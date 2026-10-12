use std::f32::consts::{PI, TAU};

use crate::{LowPass, Rng, SAMPLE_RATE};

/// The lava near the listener.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LavaScene {
    /// How much lava is open to the air nearby, in blocks.
    pub area: f32,
    /// How loud it is at the listener, as for water.
    pub gain: f32,
}

impl LavaScene {
    pub const NONE: LavaScene = LavaScene {
        area: 0.0,
        gain: 0.0,
    };
}

/// Bubbles bursting a second from a block of lava.
const BURSTS: f32 = 0.35;
const MAX_BURSTS: f32 = 30.0;
const MAX_VOICES: usize = 32;
const SPEED_OF_SOUND: f32 = 343.0;

/// One bursting bubble: the cavity it leaves rings as a Helmholtz resonator
/// (a mode), under a low thump, with a sizzle of spatter.
#[derive(Debug)]
struct Burst {
    /// Each mode as a phasor (re, im, cos, sin) shrunk by its decay.
    modes: [[f32; 4]; 2],
    sizzle: f32,
    sizzle_step: f32,
}

impl Burst {
    fn new(rng: &mut Rng) -> Burst {
        let rate = SAMPLE_RATE as f32;
        // Bubble radius, m: 1 to 6 cm, mostly small.
        let radius = 0.01 * 6f32.powf(rng.unit().powf(1.5));
        let opening = radius * (0.3 + 0.4 * rng.unit());
        let (frequency, q) = helmholtz(radius, opening);
        // Lava's viscosity damps it further than radiation alone.
        let decay = PI * frequency / (0.5 * q).clamp(1.5, 10.0);
        let amplitude = 0.25 * radius / 0.03;
        let mode = |f: f32, decay: f32, a: f32| {
            let shrink = (-decay / rate).exp();
            let step = TAU * f / rate;
            [a, 0.0, shrink * step.cos(), shrink * step.sin()]
        };
        Burst {
            modes: [
                mode(frequency, decay, amplitude),
                mode(0.3 * frequency, 2.0 * decay, 1.5 * amplitude),
            ],
            sizzle: 0.15 * amplitude,
            sizzle_step: (-1.0 / (0.04 * rate)).exp(),
        }
    }

    fn sample(&mut self, noise: f32) -> f32 {
        let mut out = self.sizzle * noise;
        self.sizzle *= self.sizzle_step;
        for [re, im, c, s] in &mut self.modes {
            let (r, i) = (*re * *c - *im * *s, *re * *s + *im * *c);
            *re = r;
            *im = i;
            out += i;
        }
        out
    }

    fn done(&self) -> bool {
        self.sizzle < 1e-4 && self.modes.iter().all(|m| m[0].abs() + m[1].abs() < 1e-4)
    }
}

/// The resonance of the cavity a burst bubble leaves, a Helmholtz resonator
/// with a hemispherical volume V = ⅔πR³ and an opening of radius a (area
/// A = πa², end-corrected neck L ≈ 1.7a): f = (c / 2π)·√(A / (V L)), and its
/// radiation quality Q = 2π·√(V (L / A)³). A 3 cm bubble opening 1.5 cm rings
/// near 1.2 kHz.
pub fn helmholtz(radius: f32, opening: f32) -> (f32, f32) {
    let area = PI * opening * opening;
    let volume = 2.0 / 3.0 * PI * radius.powi(3);
    let neck = 1.7 * opening;
    let frequency = SPEED_OF_SOUND / TAU * (area / (volume * neck)).sqrt();
    let q = TAU * (volume * (neck / area).powi(3)).sqrt();
    (frequency, q)
}

/// Lava (M2): a low rumble of convection, and bubbles of gas bursting at the
/// surface as a Poisson process, each ringing as the Helmholtz resonator its
/// cavity makes (after Vergniolle and Brandeis's model of Strombolian
/// bubbles), heavily damped by the melt.
#[derive(Debug)]
pub struct Lava {
    rng: Rng,
    scene: LavaScene,
    rumble: [LowPass; 2],
    until_burst: f32,
    bursts: Vec<Burst>,
}

impl Lava {
    pub fn new(seed: u64) -> Lava {
        Lava {
            rng: Rng::new(seed),
            scene: LavaScene::NONE,
            rumble: [LowPass::new(60.0), LowPass::new(60.0)],
            until_burst: 0.0,
            bursts: Vec::with_capacity(MAX_VOICES),
        }
    }

    pub fn set_scene(&mut self, scene: LavaScene) {
        self.scene = scene;
    }

    pub fn render(&mut self, out: &mut [f32]) {
        let dt = 1.0 / SAMPLE_RATE as f32;
        let area = self.scene.area.clamp(0.0, 1000.0);
        let gain = self.scene.gain.clamp(0.0, 1.0);
        let rate = (BURSTS * area).min(MAX_BURSTS);
        let rumble_level = 2.5 * area.sqrt().min(10.0);
        for sample in out {
            if rate > 0.0 {
                self.until_burst -= dt;
                while self.until_burst <= 0.0 {
                    self.until_burst += self.rng.exponential(rate);
                    if self.bursts.len() < MAX_VOICES {
                        self.bursts.push(Burst::new(&mut self.rng));
                    }
                }
            }
            let noise = self.rng.signed();
            let mut mix = rumble_level * {
                let first = self.rumble[0].process(noise);
                self.rumble[1].process(first)
            };
            for burst in &mut self.bursts {
                mix += burst.sample(noise);
            }
            *sample = gain * mix;
        }
        self.bursts.retain(|burst| !burst.done());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cavity_rings_as_a_helmholtz_resonator() {
        let (f, q) = helmholtz(0.03, 0.015);
        assert!((1100.0..1400.0).contains(&f), "{f} Hz");
        assert!((5.0..15.0).contains(&q), "Q {q}");
        // A bigger cavity is lower.
        assert!(helmholtz(0.06, 0.03).0 < f);
    }

    #[test]
    fn lava_bubbles_and_rumbles() {
        let mut lava = Lava::new(2);
        lava.set_scene(LavaScene {
            area: 20.0,
            gain: 1.0,
        });
        let mut out = vec![0.0; 4 * SAMPLE_RATE as usize];
        lava.render(&mut out);
        let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
        assert!(rms > 0.005 && rms < 0.3, "{rms}");
        assert!(out.iter().all(|s| s.is_finite()));
    }
}
