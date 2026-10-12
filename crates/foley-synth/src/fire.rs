use std::f32::consts::{PI, TAU};

use crate::{LowPass, Rng, SAMPLE_RATE};

/// The fire near the listener.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FireScene {
    /// How much is burning, in blocks of fire.
    pub size: f32,
    /// How loud it is at the listener, as for water.
    pub gain: f32,
}

impl FireScene {
    pub const OUT: FireScene = FireScene {
        size: 0.0,
        gain: 0.0,
    };
}

/// Crackles a second from a block of fire.
const CRACKLES: f32 = 7.0;
const MAX_CRACKLES: f32 = 400.0;
const MAX_VOICES: usize = 64;

/// One crackle: a burst of noise as a pocket of resin or steam bursts.
#[derive(Debug)]
struct Crackle {
    amplitude: f32,
    step: f32,
}

/// Fire (M2): combustion roar and crackles.
///
/// - **Roar:** turbulent combustion is a low rumble, its spectrum peaking at
///   a few hundred hertz and lower the bigger the fire, puffing (the flame's
///   own oscillation) at f ≈ 1.5 / √D Hz for a fire D metres across
///   (Cetegen and Ahmed 1993).
/// - **Crackles:** short bursts as pockets in the fuel burst, a Poisson
///   process whose sizes follow a power law, as crackling noise does
///   (P(E) ∝ E^-1.7): many small ticks, now and then a pop.
#[derive(Debug)]
pub struct Fire {
    rng: Rng,
    scene: FireScene,
    roar: [LowPass; 2],
    puff: f32,
    until_crackle: f32,
    crackles: Vec<Crackle>,
    last: f32,
}

impl Fire {
    pub fn new(seed: u64) -> Fire {
        Fire {
            rng: Rng::new(seed),
            scene: FireScene::OUT,
            roar: [LowPass::new(300.0), LowPass::new(300.0)],
            puff: 0.0,
            until_crackle: 0.0,
            crackles: Vec::with_capacity(MAX_VOICES),
            last: 0.0,
        }
    }

    pub fn set_scene(&mut self, scene: FireScene) {
        self.scene = scene;
        let size = scene.size.clamp(0.0, 256.0);
        let cutoff = 350.0 / (1.0 + 0.25 * size.sqrt());
        for filter in &mut self.roar {
            filter.tune(cutoff);
        }
    }

    pub fn render(&mut self, out: &mut [f32]) {
        let dt = 1.0 / SAMPLE_RATE as f32;
        let size = self.scene.size.clamp(0.0, 256.0);
        let gain = self.scene.gain.clamp(0.0, 1.0);
        // The fire's width, m, if its blocks were a disc.
        let across = (4.0 * size / PI).sqrt().max(0.3);
        let puff_hz = 1.5 / across.sqrt();
        let roar_level = 0.9 * size.sqrt();
        let rate = (CRACKLES * size).min(MAX_CRACKLES);
        for sample in out {
            self.puff = (self.puff + puff_hz * dt * (0.8 + 0.4 * self.rng.unit())) % 1.0;
            let puffing = 1.0 + 0.45 * (TAU * self.puff).sin();
            let noise = self.rng.signed();
            let roar = {
                let first = self.roar[0].process(noise);
                self.roar[1].process(first)
            };
            if rate > 0.0 {
                self.until_crackle -= dt;
                while self.until_crackle <= 0.0 {
                    self.until_crackle += self.rng.exponential(rate);
                    if self.crackles.len() < MAX_VOICES {
                        // E from 1 to 300 with P(E) ∝ E^-1.7, by inverting its distribution.
                        let k = -0.7f32;
                        let energy = (1.0 + self.rng.unit() * (300f32.powf(k) - 1.0)).powf(1.0 / k);
                        // Bigger bursts are louder and last longer: 0.2 ms to about 2 ms.
                        let time = 0.000_2 * energy.powf(0.4);
                        self.crackles.push(Crackle {
                            amplitude: 0.05 * energy.sqrt(),
                            step: (-dt / time).exp(),
                        });
                    }
                }
            }
            let mut crackle = 0.0;
            for voice in &mut self.crackles {
                voice.amplitude *= voice.step;
                crackle += voice.amplitude * self.rng.signed();
            }
            self.crackles.retain(|voice| voice.amplitude > 1e-4);
            // A first difference: bursts are bright, without the roar's lows.
            let bright = crackle - self.last;
            self.last = crackle;
            *sample = gain * (roar_level * puffing * roar + bright);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(scene: FireScene) -> f32 {
        let mut fire = Fire::new(3);
        fire.set_scene(scene);
        let mut out = vec![0.0; 2 * SAMPLE_RATE as usize];
        fire.render(&mut out);
        (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt()
    }

    #[test]
    fn a_bigger_fire_is_louder_and_out_is_silent() {
        let one = rms(FireScene {
            size: 1.0,
            gain: 1.0,
        });
        let blaze = rms(FireScene {
            size: 30.0,
            gain: 1.0,
        });
        assert!(one > 0.005 && one < 0.15, "one block {one}");
        assert!(blaze > 2.0 * one && blaze < 0.6, "blaze {blaze}");
        assert_eq!(rms(FireScene::OUT), 0.0);
    }

    #[test]
    fn crackles_are_mostly_small() {
        let mut fire = Fire::new(1);
        fire.set_scene(FireScene {
            size: 10.0,
            gain: 1.0,
        });
        let mut out = vec![0.0; 4 * SAMPLE_RATE as usize];
        fire.render(&mut out);
        // Peaks well above the roar: a heavy-tailed distribution of bursts.
        let peak = out.iter().fold(0f32, |p, s| p.max(s.abs()));
        let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
        assert!(peak > 4.0 * rms, "peak {peak} rms {rms}");
    }
}
