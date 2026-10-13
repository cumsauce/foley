use std::f32::consts::{PI, TAU};

use crate::dsp::Wobble;
use crate::{BandPass, LowPass, Rng, SAMPLE_RATE};

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
/// At most this many snaps, hisses and settles play at once.
const MAX_EVENTS: usize = 16;

/// One crackle: a burst of noise as a pocket of resin or steam bursts.
#[derive(Debug)]
struct Crackle {
    amplitude: f32,
    step: f32,
}

/// A sound made whole when it starts, played out sample by sample: a pop, a
/// hiss of sap, a log settling.
#[derive(Debug)]
struct Event {
    sound: Vec<f32>,
    at: usize,
}

/// Crackles this energetic or more snap: the burst rings the wood round it.
const SNAP_ENERGY: f32 = 30.0;
/// Sap or steam escaping from a crack: hisses a second from a block of fire.
const HISSES: f32 = 0.05;
/// Logs burning through and settling: knocks a second from a block of fire.
const SETTLES: f32 = 0.03;

/// Fire (M2): combustion roar and crackles.
///
/// - **Roar:** turbulent combustion is a low rumble, its spectrum peaking at
///   a few hundred hertz and lower the bigger the fire, puffing (the flame's
///   own oscillation) at f ≈ 1.5 / √D Hz for a fire D metres across
///   (Cetegen and Ahmed 1993).
/// - **Crackles:** short bursts as pockets in the fuel burst, a Poisson
///   process whose sizes follow a power law, as crackling noise does
///   (P(E) ∝ E^-1.7): many small ticks, now and then a pop that snaps, ringing
///   the wood round it.
/// - **Flames:** a flutter of brighter noise over the roar, as the flame
///   front flickers some ten times a second.
/// - **Sap and steam:** now and then gas escapes from a crack in the wood,
///   hissing for a second or two, sometimes whistling as it goes.
/// - **Settling:** now and then a log burns through and drops: a dull
///   wooden knock and embers crumbling.
#[derive(Debug)]
pub struct Fire {
    rng: Rng,
    scene: FireScene,
    roar: [LowPass; 2],
    puff: f32,
    until_crackle: f32,
    crackles: Vec<Crackle>,
    last: f32,
    events: Vec<Event>,
    samples: usize,
    flicker: Wobble,
    flame: BandPass,
    until_hiss: f32,
    until_settle: f32,
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
            events: Vec::with_capacity(MAX_EVENTS),
            samples: 0,
            flicker: Wobble::new(10.0, 4.0, 0.2, 2.0),
            flame: BandPass::new(1500.0, 0.8),
            until_hiss: 0.0,
            until_settle: 0.0,
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
                        if energy >= SNAP_ENERGY && self.events.len() < MAX_EVENTS {
                            let sound = snap(&mut self.rng, 0.012 * energy.sqrt());
                            self.events.push(Event { sound, at: 0 });
                        }
                    }
                }
            }
            let mut crackle = 0.0;
            for voice in &mut self.crackles {
                voice.amplitude *= voice.step;
                crackle += voice.amplitude * self.rng.signed();
            }
            self.samples += 1;
            // Spent crackles go every so often, not every sample.
            if self.crackles.len() == MAX_VOICES || self.samples.is_multiple_of(64) {
                self.crackles.retain(|voice| voice.amplitude > 1e-4);
            }
            // A first difference: bursts are bright, without the roar's lows.
            let bright = crackle - self.last;
            self.last = crackle;
            // The flames' flicker: brighter noise, its level wandering about
            // ten times a second.
            let flicker = self.flicker.next(self.rng.signed());
            let flame = 0.06 * size.sqrt().min(6.0) * flicker * self.flame.process(noise);
            if size > 0.0 {
                self.until_hiss -= dt;
                if self.until_hiss <= 0.0 {
                    self.until_hiss = self.rng.exponential(HISSES * size.sqrt());
                    if self.events.len() < MAX_EVENTS {
                        let sound = hiss(&mut self.rng);
                        self.events.push(Event { sound, at: 0 });
                    }
                }
                self.until_settle -= dt;
                if self.until_settle <= 0.0 {
                    self.until_settle = self.rng.exponential(SETTLES * size.sqrt());
                    if self.events.len() < MAX_EVENTS {
                        let sound = settle(&mut self.rng);
                        self.events.push(Event { sound, at: 0 });
                    }
                }
            }
            let mut extra = 0.0;
            for event in &mut self.events {
                extra += event.sound[event.at];
                event.at += 1;
            }
            self.events.retain(|e| e.at < e.sound.len());
            *sample = gain * (roar_level * puffing * roar + bright + flame + extra);
        }
    }
}

/// A snap: the burst rings the wood round it, a few kHz, for a few ms.
fn snap(rng: &mut Rng, amplitude: f32) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let f = 1500.0 + 3000.0 * rng.unit();
    let decay = 300.0 + 400.0 * rng.unit();
    let n = (6.0 / decay * rate) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / rate;
            amplitude * (-decay * t).exp() * (TAU * f * t).sin()
        })
        .collect()
}

/// Sap or steam escaping from a crack: a hiss for half a second to two,
/// swelling and fading; a third of the time it whistles, a wavering tone.
fn hiss(rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let seconds = 0.5 + 1.5 * rng.unit();
    let n = (seconds * rate) as usize;
    let mut band = BandPass::new(3000.0 + 3000.0 * rng.unit(), 3.0);
    let mut wander = LowPass::new(4.0);
    let whistle = rng.unit() < 0.33;
    let mut hz = 2000.0 + 2000.0 * rng.unit();
    let mut phase = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let envelope = (PI * t).sin().powf(0.7)
                * (1.0 + 3.0 * wander.process(rng.signed())).clamp(0.3, 1.7);
            let mut s = 0.02 * envelope * band.process(rng.signed());
            if whistle {
                hz *= 1.0 + 0.0005 * rng.signed();
                phase += TAU * hz / rate;
                s += 0.008 * envelope * phase.sin();
            }
            s
        })
        .collect()
}

/// A log burning through and settling: a dull wooden knock, then embers
/// crumbling and ticking for a moment.
fn settle(rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let n = (0.5 * rate) as usize;
    let f = 150.0 + 200.0 * rng.unit();
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / rate;
            0.12 * (-30.0 * t).exp() * (TAU * f * t).sin()
        })
        .collect();
    for _ in 0..10 + (rng.unit() * 15.0) as usize {
        let at = (0.4 * rng.unit().powi(2) * rate) as usize;
        let tick_f = 2000.0 + 4000.0 * rng.unit();
        let amplitude = 0.01 + 0.03 * rng.unit();
        for i in 0..(0.003 * rate) as usize {
            if let Some(o) = out.get_mut(at + i) {
                let t = i as f32 / rate;
                *o += amplitude * (-1500.0 * t).exp() * (TAU * tick_f * t).sin();
            }
        }
    }
    out
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
