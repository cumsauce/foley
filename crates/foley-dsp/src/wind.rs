use crate::{BandPass, Rng, SAMPLE_RATE};
use crate::{LowPass, Wobble, smooth};
#[allow(unused_imports)] // Some of these are in core; the rest come from std.
use foley_math::Float as _;

/// What the wind round the listener is like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindScene {
    /// Mean wind speed at the listener, m/s.
    pub speed: f32,
    /// How open to it the listener is: 1 out in the open, 0 shut indoors.
    pub exposure: f32,
    /// How much of what's round about is leaves, 0 to 1.
    pub leaves: f32,
}

impl WindScene {
    pub const CALM: WindScene = WindScene {
        speed: 0.0,
        exposure: 0.0,
        leaves: 0.0,
    };
}

/// Gusts: the speed wanders round its mean as an Ornstein–Uhlenbeck process,
/// with this turbulence intensity (standard deviation over mean)...
const TURBULENCE: f32 = 0.25;
/// ...and this time scale, seconds.
const GUST_TIME: f32 = 4.0;
/// How loud the leaves' patter is.
const LEAVES: f32 = 0.18;
/// What each sample keeps of a leaf's swish: about 25 ms.
const LEAF_DECAY: f32 = 0.999_17;
/// A leaf's share of the hush: the old 1.4 ms tick's over the 25 ms swish.
const LEAF_STEP: f32 = 0.056;
/// How long before the ear the trees round about hear a gust, seconds.
const GUST_LEAD: f32 = 0.8;
/// The speed is updated this often, samples.
const BLOCK: usize = 64;
/// Gust blocks the ear lags behind the leaves.
const LEAD: usize = (GUST_LEAD * SAMPLE_RATE as f32 / BLOCK as f32) as usize;

/// The wind (M1): turbulence heard as band-passed noise, its loudness rising
/// with speed cubed (aerodynamic noise power goes as U⁶) and its pitch with
/// speed (the eddies shed off things go as Strouhal's f ≈ 0.2 U / d); gusts
/// from an Ornstein–Uhlenbeck process; leaves rustling as bursts of bright
/// noise, more of them the harder it blows (rate ∝ U²). Indoors it comes in
/// low and muffled.
#[derive(Debug)]
pub struct Wind {
    rng: Rng,
    scene: WindScene,
    speed: f32,
    /// Recent speeds, a block apart: a gust sweeps through the trees upwind
    /// before it reaches the ear, so the leaves hear the speed now and the
    /// ear the speed of [`GUST_LEAD`] ago.
    /// A ring of the last [`LEAD`] speeds, the next written at `next`.
    history: [f32; LEAD],
    next: usize,
    filled: usize,
    ear: f32,
    roar: BandPass,
    whoosh: BandPass,
    rustle: BandPass,
    rustle_envelope: f32,
    until_rustle: f32,
    /// Leaves knocking together: each a few ms of bright noise.
    leaf: [BandPass; 2],
    leaf_envelope: f32,
    /// The envelopes as heard: each tick and swell rises over a fraction of
    /// a millisecond instead of in one sample, which crackled.
    leaf_heard: f32,
    rustle_heard: f32,
    until_leaf: f32,
    flutter: Wobble,
    /// The air between the ear and the leaves: dull from afar, opening as a
    /// gust sweeps the near crowns.
    leaf_air: LowPass,
    /// How near the gust in the crowns is, 0 to 1, following its swell over
    /// some 0.6 s: the sweep from afar.
    leaf_near: f32,
    /// A bough creaking in a strong wind: stick-slip at a wandering rate.
    creak: BandPass,
    creak_left: f32,
    creak_phase: f32,
    creak_hz: f32,
    samples: usize,
}

impl Wind {
    pub fn new(seed: u64) -> Wind {
        Wind {
            rng: Rng::new(seed),
            scene: WindScene::CALM,
            speed: 0.0,
            history: [0.0; LEAD],
            next: 0,
            filled: 0,
            ear: 0.0,
            roar: BandPass::new(150.0, 0.7),
            whoosh: BandPass::new(600.0, 1.2),
            rustle: BandPass::new(1800.0, 1.0),
            rustle_envelope: 0.0,
            until_rustle: 0.0,
            leaf: [BandPass::new(1300.0, 0.7), BandPass::new(2600.0, 1.0)],
            leaf_air: LowPass::new(2000.0),
            leaf_near: 0.0,
            leaf_envelope: 0.0,
            leaf_heard: 0.0,
            rustle_heard: 0.0,
            until_leaf: 0.0,
            flutter: Wobble::new(6.0, 4.0, 0.3, 1.5),
            creak: BandPass::new(900.0, 3.0),
            creak_left: 0.0,
            creak_phase: 0.0,
            creak_hz: 0.0,
            samples: 0,
        }
    }

    pub fn set_scene(&mut self, scene: WindScene) {
        // The wind is already blowing when it's first heard.
        if self.filled == 0 {
            self.speed = scene.speed.max(0.0);
        }
        self.scene = scene;
    }

    /// The gusting speed now, m/s.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    fn gust(&mut self) {
        let dt = BLOCK as f32 / SAMPLE_RATE as f32;
        let mean = self.scene.speed.max(0.0);
        let sigma = TURBULENCE * mean;
        // dU = (mean - U) dt / τ + σ √(2 dt / τ) dW
        let noise = self.rng.signed() * 1.732; // unit variance from uniform
        self.speed +=
            (mean - self.speed) * dt / GUST_TIME + sigma * (2.0 * dt / GUST_TIME).sqrt() * noise;
        self.speed = self.speed.max(0.0);
        self.history[self.next] = self.speed;
        self.next = (self.next + 1) % LEAD;
        self.filled = (self.filled + 1).min(LEAD);
        // The oldest kept: the first, until the ring has filled.
        self.ear = self.history[if self.filled < LEAD { 0 } else { self.next }];
        let indoors = 1.0 - self.scene.exposure.clamp(0.0, 1.0);
        // Pitch rises with speed; indoors only the low part gets through.
        let centre = (80.0 + 25.0 * self.ear) * (1.0 - 0.6 * indoors);
        self.roar.tune(centre, 0.7);
        self.whoosh.tune(centre * 4.0 * (1.0 - 0.7 * indoors), 1.2);
    }

    /// Trees round about. Leaves are up where the wind is stronger than at
    /// the ear, and overhead even where the canopy hides the sky, so they're
    /// heard under it. They rustle as many tiny collisions, more the harder it
    /// blows (rate ∝ U²), fluttering, with a gust's swell now and then; in a
    /// strong wind, boughs creak.
    fn leaves(&mut self, u: f32, exposure: f32, dt: f32) -> f32 {
        let leaves = self.scene.leaves.clamp(0.0, 1.0);
        // Only real woods: a stray bush or two adds nothing.
        let leaves = leaves * smooth((leaves - 0.1) / 0.2);
        if leaves <= 0.0 {
            return 0.0;
        }
        let reach = 0.4 + 0.6 * exposure;
        let canopy = (2.5 * u).clamp(2.5, 20.0);
        let mut out = 0.0;
        // Leaves fluttering: thousands within earshot, each a swish of some
        // 25 ms, overlapping into a hush that swells and thins, more of them
        // the harder it blows. (Ticks of a millisecond, as this once made,
        // stood apart in a breeze and crackled.) The steps are scaled so the
        // level is what the ticks gave.
        self.until_leaf -= dt;
        if self.until_leaf <= 0.0 {
            self.leaf_envelope += LEAF_STEP * (0.3 + 0.7 * self.rng.unit());
            let rate = 30.0 + 5.0 * canopy * canopy * leaves;
            self.until_leaf = self.rng.exponential(rate.min(1500.0));
        }
        self.leaf_envelope *= LEAF_DECAY;
        // About 5 ms to rise: no leaf starts with a click.
        self.leaf_heard += 0.004 * (self.leaf_envelope.min(1.5) - self.leaf_heard);
        let flutter = self.flutter.next(self.rng.signed());
        let noise = self.rng.signed();
        // Foliage rustles mostly at 1 to 3 kHz, falling away above.
        let leaf = self.leaf[0].process(noise) + 0.35 * self.leaf[1].process(noise);
        // A smooth hush sounds quieter than ticks of the same power: lifted in
        // a light breeze, as much as before by a strong one.
        let loud = LEAVES * leaves * reach * (0.6 + canopy / 40.0) * (2.0 - canopy / 20.0);
        out += loud * flutter * self.leaf_heard * leaf;
        // Gusts sweeping through the crowns.
        if u > 0.5 {
            self.until_rustle -= dt;
            if self.until_rustle <= 0.0 {
                self.rustle_envelope =
                    (self.rustle_envelope + 0.5 + 0.5 * self.rng.unit()).min(1.5);
                self.until_rustle = self.rng.exponential(0.3 * canopy * leaves);
            }
            self.rustle_envelope *= 0.99995;
            // A swell takes some 20 ms to come up through the crowns.
            self.rustle_heard += 0.001 * (self.rustle_envelope - self.rustle_heard);
            let rustle = self.rustle.process(self.rng.signed());
            out += 0.006 * self.rustle_heard * rustle * leaves * reach * (canopy / 10.0).min(2.0);
        }
        // From afar the leaves are dull; a gust sweeping in through the near
        // crowns brings their highs with it.
        if self.samples.is_multiple_of(BLOCK) {
            self.leaf_near += (self.rustle_heard.min(1.0) - self.leaf_near)
                * (BLOCK as f32 / (0.6 * SAMPLE_RATE as f32));
            self.leaf_air.tune(1500.0 + 3500.0 * self.leaf_near);
        }
        // Darker, so a little more: the same presence as the bright leaves had.
        out = 1.4 * self.leaf_air.process(out);
        // Creaks: a bough's fibres sticking and slipping, a few hundred times a
        // second, wandering in pitch, for half a second or so.
        if self.creak_left > 0.0 {
            self.creak_left -= dt;
            self.creak_hz *= 1.0 + 0.0004 * self.rng.signed();
            self.creak_phase += self.creak_hz * dt;
            let slip = if self.creak_phase >= 1.0 {
                self.creak_phase -= 1.0;
                1.0 + 0.5 * self.rng.signed()
            } else {
                0.0
            };
            let fade = (self.creak_left * 8.0).min(1.0);
            out += 0.05 * leaves * reach * fade * self.creak.process(slip);
        } else if canopy > 7.0 && self.rng.unit() < dt * 0.05 * leaves * (canopy - 7.0) / 10.0 {
            self.creak_left = 0.3 + 0.6 * self.rng.unit();
            self.creak_hz = 120.0 + 200.0 * self.rng.unit();
            self.creak.tune(500.0 + 900.0 * self.rng.unit(), 3.0);
        }
        out
    }

    /// The next `out.len()` samples.
    pub fn render(&mut self, out: &mut [f32]) {
        let dt = 1.0 / SAMPLE_RATE as f32;
        let exposure = self.scene.exposure.clamp(0.0, 1.0);
        for sample in out {
            if self.samples.is_multiple_of(BLOCK) {
                self.gust();
            }
            self.samples += 1;
            let (u, ear) = (self.speed, self.ear);
            // Amplitude ∝ U³, against a 10 m/s wind; muffled to a tenth indoors.
            // Amplitude ∝ U³, capped for gales and flight; never quite silent outside.
            let level = (0.06 * (ear / 10.0).powi(3).min(2.5)
                + if ear > 0.3 { 0.004 } else { 0.0 })
                * (0.1 + 0.9 * exposure);
            let roar = self.roar.process(self.rng.signed());
            let whoosh = self.whoosh.process(self.rng.signed());
            let mut out = level * (roar * 1.5 + whoosh * 0.6 * exposure);
            if self.scene.leaves > 0.0 {
                out += self.leaves(u, exposure, dt);
            }
            *sample = out;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use std::{vec, vec::Vec};

    fn listen(scene: WindScene) -> f32 {
        let mut wind = Wind::new(5);
        wind.set_scene(scene);
        let mut out = vec![0.0; 4 * SAMPLE_RATE as usize];
        wind.render(&mut out);
        out.iter().map(|s| s * s).sum::<f32>()
    }

    #[test]
    fn stronger_wind_is_much_louder_and_indoors_quieter() {
        let open = |speed| WindScene {
            speed,
            exposure: 1.0,
            leaves: 0.0,
        };
        let (breeze, gale) = (listen(open(4.0)), listen(open(15.0)));
        // Energy ∝ U⁶: nearly 3 000 times from 4 to 15 m/s; at least 100 here.
        // Less than U⁶'s 3 000 times: a soft floor keeps even a breeze audible.
        assert!(gale > 10.0 * breeze, "{breeze} vs {gale}");
        let inside = listen(WindScene {
            exposure: 0.0,
            ..open(15.0)
        });
        assert!(inside < gale / 20.0, "indoors {inside} vs {gale}");
        assert_eq!(listen(WindScene::CALM), 0.0);
    }

    #[test]
    fn woods_rustle_even_under_the_canopy_and_calm() {
        let under = listen(WindScene {
            speed: 1.0,
            exposure: 0.0,
            leaves: 0.8,
        });
        let bare = listen(WindScene {
            speed: 1.0,
            exposure: 0.0,
            leaves: 0.0,
        });
        // Energy over 4 s; the leaves are dark (1 to 3 kHz), so less of it
        // than their presence suggests.
        assert!(under > 4.0 * bare && under > 0.12, "{under} vs {bare}");
        let mut wind = Wind::new(2);
        wind.set_scene(WindScene {
            speed: 12.0,
            exposure: 1.0,
            leaves: 1.0,
        });
        let mut out = vec![0.0; 10 * SAMPLE_RATE as usize];
        wind.render(&mut out);
        assert!(out.iter().all(|x| x.is_finite() && x.abs() < 1.0));
    }

    #[test]
    fn gusts_wander_round_the_mean() {
        let mut wind = Wind::new(9);
        wind.set_scene(WindScene {
            speed: 10.0,
            exposure: 1.0,
            leaves: 0.5,
        });
        let mut out = vec![0.0; BLOCK];
        let mut speeds = Vec::new();
        for _ in 0..(60 * SAMPLE_RATE as usize / BLOCK) {
            wind.render(&mut out);
            speeds.push(wind.speed());
        }
        let tail = &speeds[speeds.len() / 2..];
        let mean = tail.iter().sum::<f32>() / tail.len() as f32;
        let sd = (tail.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / tail.len() as f32).sqrt();
        assert!((mean - 10.0).abs() < 2.0, "mean {mean}");
        assert!(sd > 0.8 && sd < 4.5, "gusts {sd}");
    }
}
