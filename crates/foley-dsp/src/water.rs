#[allow(unused_imports)] // Some of these are in core; the rest come from std.
use foley_math::Float as _;

use crate::Fixed;
use crate::Wobble;
use crate::{BandPass, Bubble, LowPass, Rng};

/// The moving water near the listener.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterScene {
    /// How much water flows at the surface nearby, in blocks of it.
    pub flow: f32,
    /// How much falls nearby, in blocks where a waterfall lands.
    pub falls: f32,
    /// How loud it is at the listener: 1 within a couple of blocks, falling
    /// off as 1 / distance beyond.
    pub gain: f32,
}

impl WaterScene {
    pub const STILL: WaterScene = WaterScene {
        flow: 0.0,
        falls: 0.0,
        gain: 0.0,
    };
}

/// Bubbles entrained a second by a block of water flowing at the surface...
const FLOW_BUBBLES: f32 = 60.0;
/// ...and by a block where a waterfall plunges in.
const FALL_BUBBLES: f32 = 700.0;
/// At most this many bubbles a second are made one by one; past it, each is
/// louder, as many incoherent ones together would be.
const MAX_BUBBLES: f32 = 3000.0;
const MAX_VOICES: usize = 256;
/// The Hinze scale, m: turbulence breaks up bubbles bigger than this, so
/// their numbers fall steeply past it.
const HINZE: f32 = 0.001;
const SMALLEST: f32 = 0.000_1;
const LARGEST: f32 = 0.006;

/// A bubble's radius, in m, from what a plunging jet or breaking water
/// entrains (Deane and Stokes 2002): N(a) ∝ a^-3/2 below the Hinze scale and
/// a^-10/3 above it.
pub fn plunge_radius(rng: &mut Rng) -> f32 {
    let (k1, k2) = (-1.5f32, -10.0f32 / 3.0);
    let integral = |k: f32, lo: f32, hi: f32| (hi.powf(k + 1.0) - lo.powf(k + 1.0)) / (k + 1.0);
    let below = integral(k1, SMALLEST, HINZE);
    // Continuous at the Hinze scale.
    let above = HINZE.powf(k1 - k2) * integral(k2, HINZE, LARGEST);
    let (k, lo, hi) = if rng.unit() * (below + above) < below {
        (k1, SMALLEST, HINZE)
    } else {
        (k2, HINZE, LARGEST)
    };
    let u = rng.unit();
    (lo.powf(k + 1.0) + u * (hi.powf(k + 1.0) - lo.powf(k + 1.0))).powf(1.0 / (k + 1.0))
}

/// A bubble's radius, in m, from a stream's surface: babbling brooks ring
/// mostly from bubbles of 1 to 4 mm (their sound peaks near 1 to 3 kHz),
/// spread evenly in log size.
fn babble_radius(rng: &mut Rng) -> f32 {
    0.0008 * 5f32.powf(rng.unit())
}

/// Moving water (M2): streams and waterfalls as bubble populations. Every
/// bubble the water entrains rings at its Minnaert frequency as it settles
/// (see [`Bubble`]); a stream's few, larger bubbles babble, and a plunging
/// waterfall's many, of every size, roar, over the broadband noise of the
/// water striking water.
#[derive(Debug)]
pub struct Water {
    rng: Rng,
    scene: WaterScene,
    /// What's heard now, gliding toward the scene over a fraction of a
    /// second, so the bed never jumps when the world is sampled again.
    heard: WaterScene,
    fresh: bool,
    until_next: f32,
    voices: Fixed<Bubble, MAX_VOICES>,
    roar: [LowPass; 2],
    spray: LowPass,
    /// Sheets of falling water breaking up: the spray surges a few times a
    /// second.
    surge: Wobble,
    /// A stream's ripples slapping and splashing, 2 to 6 kHz.
    ripple: BandPass,
    rush: [BandPass; 2],
    lapping: Wobble,
    distance: [LowPass; 2],
    samples: usize,
    /// The rate this bed renders at.
    sample_rate: u32,
}

/// How fast what's heard follows the scene, seconds.
const GLIDE: f32 = 0.4;
/// Rates and filters are updated this often, samples.
const BLOCK: usize = 64;

impl Water {
    pub fn new(seed: u64, sample_rate: u32) -> Water {
        Water {
            rng: Rng::new(seed),
            scene: WaterScene::STILL,
            heard: WaterScene::STILL,
            fresh: true,
            until_next: 0.0,
            voices: Fixed::new(),
            roar: [
                LowPass::new_at(500.0, sample_rate),
                LowPass::new_at(500.0, sample_rate),
            ],
            spray: LowPass::new_at(9000.0, sample_rate),
            surge: Wobble::new_at(3.0, 5.0, 0.3, 1.8, sample_rate),
            ripple: BandPass::new_at(3200.0, 0.7, sample_rate),
            rush: [
                BandPass::new_at(900.0, 0.6, sample_rate),
                BandPass::new_at(350.0, 0.9, sample_rate),
            ],
            lapping: Wobble::new_at(1.2, 6.0, 0.4, 1.8, sample_rate),
            distance: [
                LowPass::new_at(16000.0, sample_rate),
                LowPass::new_at(16000.0, sample_rate),
            ],
            samples: 0,
            sample_rate,
        }
    }

    pub fn set_scene(&mut self, scene: WaterScene) {
        self.scene = scene;
        if self.fresh {
            self.heard = scene;
            self.fresh = false;
        }
    }

    pub fn render(&mut self, out: &mut [f32]) {
        let sample_rate = self.sample_rate;
        let dt = 1.0 / sample_rate as f32;
        let glide = BLOCK as f32 * dt / GLIDE;
        let (mut rate, mut boost, mut plunging, mut roar_level, mut rush_level) =
            (0.0, 1.0, 0.0, 0.0, 0.0);
        // From afar, single bubbles blur into the surface's rush: their share
        // falls and the rush's grows, so a far stream murmurs on rather than
        // plinking with silence between. Near (gain 1) as it was.
        let (mut plinks, mut murmur) = (1.0, 1.0);
        for sample in out {
            if self.samples.is_multiple_of(BLOCK) {
                let (to, h) = (self.scene, &mut self.heard);
                h.flow += (to.flow.clamp(0.0, 200.0) - h.flow) * glide;
                h.falls += (to.falls.clamp(0.0, 200.0) - h.falls) * glide;
                h.gain += (to.gain.clamp(0.0, 1.0) - h.gain) * glide;
                let flow = h.flow * FLOW_BUBBLES;
                let falls = h.falls * FALL_BUBBLES;
                let wanted = flow + falls;
                rate = wanted.min(MAX_BUBBLES);
                // Past the cap, many bubbles blur into a hiss: a little louder
                // each, and the rush below takes over.
                boost = if rate > 0.0 {
                    (wanted / rate).sqrt().min(1.5)
                } else {
                    1.0
                };
                plunging = if wanted > 0.0 { falls / wanted } else { 0.0 };
                roar_level = 0.15 * h.falls.min(12.0).sqrt();
                // A river's surface: turbulence heard as a soft, wide rush,
                // growing slowly with how much of it there is.
                rush_level = 0.006 * h.flow.min(100.0).sqrt();
                let far = 1.0 - h.gain.clamp(0.0, 1.0);
                plinks = 1.0 - 0.6 * far;
                murmur = 1.0 + 2.0 * far;
                // Further away, water loses a little of its sparkle (the ground
                // takes some highs; Earshot adds the air's own absorption).
                let cutoff = 6000.0 + 14000.0 * h.gain.sqrt();
                self.distance
                    .iter_mut()
                    .for_each(|f| f.tune_at(cutoff, sample_rate));
            }
            self.samples += 1;
            // Free finished bubbles as it plays: done only once a read, a
            // half-second read filled the voices in milliseconds and dropped
            // every bubble after, plinks cut out twice a second.
            if self.samples.is_multiple_of(BLOCK) {
                self.voices.retain(|voice| !voice.done());
            }
            if rate > 0.0 {
                self.until_next -= dt;
                while self.until_next <= 0.0 {
                    self.until_next += self.rng.exponential(rate);
                    if self.voices.len() < MAX_VOICES {
                        // Plunged bubbles are mostly tiny, so quiet one by one
                        // (amplitude ∝ r^1.5); they count in their thousands.
                        let (radius, loudness) = if self.rng.unit() < plunging {
                            (plunge_radius(&mut self.rng), 0.015)
                        } else {
                            (babble_radius(&mut self.rng), 0.02)
                        };
                        let loudness = loudness * boost * (0.5 + self.rng.unit());
                        self.voices
                            .push(Bubble::new_at(radius, loudness, self.sample_rate));
                    }
                }
            }
            let mut mix = 0.0;
            for voice in &mut self.voices {
                mix += voice.sample();
            }
            mix *= plinks;
            let noise = self.rng.signed();
            // Always run, even silent: skipped and taken up again, their
            // filters started from stale state, a click as a fall came near.
            {
                let low = {
                    let first = self.roar[0].process(noise);
                    self.roar[1].process(first)
                };
                let spray = self.spray.process(noise) - low;
                let surge = self.surge.next(self.rng.signed());
                mix += roar_level * (2.0 * low + 0.22 * surge * spray);
            }
            {
                // Lapping: the rush swells and ebbs a little, about once a second.
                let lap = self.lapping.next(self.rng.signed());
                let rush = self.rush[0].process(noise)
                    + 0.7 * self.rush[1].process(noise)
                    + 0.35 * self.ripple.process(noise);
                mix += murmur * rush_level * lap * rush;
            }
            let first = self.distance[0].process(mix);
            let near = self.distance[1].process(first);
            *sample = self.heard.gain * near;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;
    #[allow(unused_imports)]
    use std::{vec, vec::Vec};

    fn rms(scene: WaterScene) -> f32 {
        let mut water = Water::new(3, SAMPLE_RATE);
        water.set_scene(scene);
        let mut out = vec![0.0; 2 * SAMPLE_RATE as usize];
        water.render(&mut out);
        (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt()
    }

    /// The mod reads half a second at a time; the bed must sound the same as
    /// read in small blocks, so finished bubbles must free their voices as it
    /// plays, not only at the end of a read.
    #[test]
    fn a_long_read_sounds_like_short_ones() {
        let render = |chunk: usize| {
            let mut water = Water::new(9, SAMPLE_RATE);
            water.set_scene(WaterScene {
                flow: 20.0,
                falls: 6.0,
                gain: 1.0,
            });
            let mut out = vec![0.0; SAMPLE_RATE as usize];
            out.chunks_mut(chunk).for_each(|c| water.render(c));
            out
        };
        let (long, short) = (render(SAMPLE_RATE as usize / 2), render(64));
        assert!(long == short, "a 0.5 s read differs from 64-sample ones");
    }

    #[test]
    fn plunging_bubbles_follow_deane_and_stokes() {
        let mut rng = Rng::new(1);
        let radii: Vec<f32> = (0..100_000).map(|_| plunge_radius(&mut rng)).collect();
        assert!(radii.iter().all(|r| (SMALLEST..=LARGEST).contains(r)));
        // Under a^-3/2 from 0.1 to 1 mm, about two thirds are under 0.3 mm.
        let small = radii.iter().filter(|r| **r < 0.0003).count() as f32 / radii.len() as f32;
        assert!((0.5..0.75).contains(&small), "{small}");
        // Past the Hinze scale they fall away steeply: under a tenth (8.9 %
        // from the two integrals).
        let big = radii.iter().filter(|r| **r > HINZE).count() as f32 / radii.len() as f32;
        assert!((0.07..0.11).contains(&big), "{big}");
    }

    #[test]
    fn a_waterfall_roars_over_a_stream_and_distance_quietens_it() {
        let stream = rms(WaterScene {
            flow: 10.0,
            falls: 0.0,
            gain: 1.0,
        });
        let fall = rms(WaterScene {
            flow: 0.0,
            falls: 10.0,
            gain: 1.0,
        });
        assert!(stream > 0.01 && stream < 0.15, "stream {stream}");
        // 1.5x held only while a long read starved the stream of bubbles; read
        // as the game reads (half a second at a time) it was ~1.2x then too.
        assert!(
            fall > 1.1 * stream && fall < 0.5,
            "fall {fall} stream {stream}"
        );
        let far = rms(WaterScene {
            flow: 0.0,
            falls: 10.0,
            gain: 0.1,
        });
        // A tenth the gain, and darker for being further.
        assert!(far < 0.1 * fall && far > 0.04 * fall, "{far} vs {fall}");
        assert_eq!(rms(WaterScene::STILL), 0.0);
    }
}
