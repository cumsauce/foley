use core::f32::consts::{PI, TAU};
#[allow(unused_imports)] // Some of these are in core; the rest come from std.
use foley_math::Float as _;

use crate::Wobble;
use crate::{BandPass, Fixed, LowPass, Rng, SAMPLE_RATE};

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

/// A ringing: `amplitude` e^(-t/τ) sin(2π f t), as a phasor turned and
/// shrunk each sample, so it costs a few multiplies and no trigonometry.
#[derive(Debug, Clone, Copy)]
struct Ring {
    re: f32,
    im: f32,
    cos: f32,
    sin: f32,
    /// What each sample keeps of the last.
    decay: f32,
}

impl Ring {
    /// Rings at `hz`, losing `per_second` nepers a second.
    fn new(amplitude: f32, hz: f32, per_second: f32) -> Ring {
        let step = TAU * hz / SAMPLE_RATE as f32;
        Ring {
            re: amplitude,
            im: 0.0,
            cos: step.cos(),
            sin: step.sin(),
            decay: (-per_second / SAMPLE_RATE as f32).exp(),
        }
    }

    fn next(&mut self) -> f32 {
        let out = self.im;
        let (re, im) = (
            self.re * self.cos - self.im * self.sin,
            self.re * self.sin + self.im * self.cos,
        );
        self.re = self.decay * re;
        self.im = self.decay * im;
        out
    }
}

/// An ember ticking as it crumbles, 3 ms long.
#[derive(Debug, Clone, Copy, Default)]
struct Tick {
    start: u32,
    amplitude: f32,
    hz: f32,
}

const MAX_TICKS: usize = 25;
const TICK_SAMPLES: u32 = (0.003 * SAMPLE_RATE as f32) as u32;

/// A sound played out sample by sample from a little state: a pop, a hiss
/// of sap, a log settling.
// A settle's ticks make it the biggest, a few hundred bytes; inline, it's
// still allocation-free.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
enum Event {
    /// The burst ringing the wood round it, a few kHz, for a few ms.
    Snap { ring: Ring, left: u32 },
    /// Sap or steam escaping from a crack: a hiss for half a second to two,
    /// swelling and fading; a third of the time it whistles, a wavering tone.
    Hiss {
        band: BandPass,
        wander: LowPass,
        whistle: bool,
        hz: f32,
        phase: f32,
        at: u32,
        length: u32,
    },
    /// A log burning through and settling: a dull wooden knock, then embers
    /// crumbling and ticking for a moment.
    Settle {
        knock: Ring,
        ticks: [Tick; MAX_TICKS],
        at: u32,
        length: u32,
    },
}

impl Event {
    fn snap(rng: &mut Rng, amplitude: f32) -> Event {
        let hz = 1500.0 + 3000.0 * rng.unit();
        let per_second = 300.0 + 400.0 * rng.unit();
        Event::Snap {
            ring: Ring::new(amplitude, hz, per_second),
            left: (6.0 / per_second * SAMPLE_RATE as f32) as u32,
        }
    }

    fn hiss(rng: &mut Rng) -> Event {
        let seconds = 0.5 + 1.5 * rng.unit();
        Event::Hiss {
            band: BandPass::new(3000.0 + 3000.0 * rng.unit(), 3.0),
            wander: LowPass::new(4.0),
            whistle: rng.unit() < 0.33,
            hz: 2000.0 + 2000.0 * rng.unit(),
            phase: 0.0,
            at: 0,
            length: (seconds * SAMPLE_RATE as f32) as u32,
        }
    }

    fn settle(rng: &mut Rng) -> Event {
        let rate = SAMPLE_RATE as f32;
        let knock = Ring::new(0.12, 150.0 + 200.0 * rng.unit(), 30.0);
        let mut ticks = [Tick::default(); MAX_TICKS];
        let count = 10 + (rng.unit() * 15.0) as usize;
        for tick in &mut ticks[..count] {
            *tick = Tick {
                start: (0.4 * rng.unit().powi(2) * rate) as u32,
                amplitude: 0.01 + 0.03 * rng.unit(),
                hz: 2000.0 + 4000.0 * rng.unit(),
            };
        }
        Event::Settle {
            knock,
            ticks,
            at: 0,
            length: (0.5 * rate) as u32,
        }
    }

    /// The next sample, or None once it's over.
    fn next(&mut self, rng: &mut Rng) -> Option<f32> {
        let rate = SAMPLE_RATE as f32;
        match self {
            Event::Snap { ring, left } => {
                *left = left.checked_sub(1)?;
                Some(ring.next())
            }
            Event::Hiss {
                band,
                wander,
                whistle,
                hz,
                phase,
                at,
                length,
            } => {
                if *at >= *length {
                    return None;
                }
                let t = *at as f32 / *length as f32;
                *at += 1;
                let envelope = (PI * t).sin().powf(0.7)
                    * (1.0 + 3.0 * wander.process(rng.signed())).clamp(0.3, 1.7);
                let mut s = 0.02 * envelope * band.process(rng.signed());
                if *whistle {
                    *hz *= 1.0 + 0.0005 * rng.signed();
                    *phase = (*phase + TAU * *hz / rate) % TAU;
                    s += 0.008 * envelope * phase.sin();
                }
                Some(s)
            }
            Event::Settle {
                knock,
                ticks,
                at,
                length,
            } => {
                if *at >= *length {
                    return None;
                }
                let mut s = knock.next();
                for tick in ticks.iter().filter(|k| k.amplitude > 0.0) {
                    if let Some(i) = at.checked_sub(tick.start).filter(|i| *i < TICK_SAMPLES) {
                        let t = i as f32 / rate;
                        s += tick.amplitude * (-1500.0 * t).exp() * (TAU * tick.hz * t).sin();
                    }
                }
                *at += 1;
                Some(s)
            }
        }
    }
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
    crackles: Fixed<Crackle, MAX_VOICES>,
    last: f32,
    events: Fixed<Event, MAX_EVENTS>,
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
            crackles: Fixed::new(),
            last: 0.0,
            events: Fixed::new(),
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
        let root = size.sqrt();
        let roar_level = 0.9 * root;
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
                        if energy >= SNAP_ENERGY && !self.events.is_full() {
                            let snap = Event::snap(&mut self.rng, 0.012 * energy.sqrt());
                            self.events.push(snap);
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
            let flame = 0.06 * root.min(6.0) * flicker * self.flame.process(noise);
            if size > 0.0 {
                self.until_hiss -= dt;
                if self.until_hiss <= 0.0 {
                    self.until_hiss = self.rng.exponential(HISSES * root);
                    if !self.events.is_full() {
                        let event = Event::hiss(&mut self.rng);
                        self.events.push(event);
                    }
                }
                self.until_settle -= dt;
                if self.until_settle <= 0.0 {
                    self.until_settle = self.rng.exponential(SETTLES * root);
                    if !self.events.is_full() {
                        let event = Event::settle(&mut self.rng);
                        self.events.push(event);
                    }
                }
            }
            let mut extra = 0.0;
            let mut over = false;
            for event in &mut self.events {
                match event.next(&mut self.rng) {
                    Some(s) => extra += s,
                    None => over = true,
                }
            }
            if over {
                self.events.retain(|event| match event {
                    Event::Snap { left, .. } => *left > 0,
                    Event::Hiss { at, length, .. } | Event::Settle { at, length, .. } => {
                        at < length
                    }
                });
            }
            *sample = gain * (roar_level * puffing * roar + bright + flame + extra);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use std::{vec, vec::Vec};

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
