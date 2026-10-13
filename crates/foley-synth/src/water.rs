use crate::{Bubble, LowPass, Rng, SAMPLE_RATE};

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
    until_next: f32,
    voices: Vec<Bubble>,
    roar: [LowPass; 2],
    spray: LowPass,
}

impl Water {
    pub fn new(seed: u64) -> Water {
        Water {
            rng: Rng::new(seed),
            scene: WaterScene::STILL,
            until_next: 0.0,
            voices: Vec::with_capacity(MAX_VOICES),
            roar: [LowPass::new(500.0), LowPass::new(500.0)],
            spray: LowPass::new(4000.0),
        }
    }

    pub fn set_scene(&mut self, scene: WaterScene) {
        self.scene = scene;
    }

    pub fn render(&mut self, out: &mut [f32]) {
        let dt = 1.0 / SAMPLE_RATE as f32;
        let flow = self.scene.flow.clamp(0.0, 200.0) * FLOW_BUBBLES;
        let falls = self.scene.falls.clamp(0.0, 200.0) * FALL_BUBBLES;
        let wanted = flow + falls;
        let rate = wanted.min(MAX_BUBBLES);
        let boost = if rate > 0.0 {
            (wanted / rate).sqrt().min(2.0)
        } else {
            1.0
        };
        let plunging = if wanted > 0.0 { falls / wanted } else { 0.0 };
        let gain = self.scene.gain.clamp(0.0, 1.0);
        let roar_level = 0.2 * self.scene.falls.clamp(0.0, 20.0).sqrt();
        for sample in out {
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
                        self.voices.push(Bubble::new(radius, loudness));
                    }
                }
            }
            let mut mix = 0.0;
            for voice in &mut self.voices {
                mix += voice.sample();
            }
            if roar_level > 0.0 {
                let noise = self.rng.signed();
                let low = {
                    let first = self.roar[0].process(noise);
                    self.roar[1].process(first)
                };
                let spray = self.spray.process(noise) - low;
                mix += roar_level * (2.0 * low + 0.15 * spray);
            }
            *sample = gain * mix;
        }
        self.voices.retain(|voice| !voice.done());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(scene: WaterScene) -> f32 {
        let mut water = Water::new(3);
        water.set_scene(scene);
        let mut out = vec![0.0; 2 * SAMPLE_RATE as usize];
        water.render(&mut out);
        (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt()
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
        assert!(fall > 1.5 * stream && fall < 0.5, "fall {fall}");
        let far = rms(WaterScene {
            flow: 0.0,
            falls: 10.0,
            gain: 0.1,
        });
        assert!((far / fall - 0.1).abs() < 0.01);
        assert_eq!(rms(WaterScene::STILL), 0.0);
    }
}
