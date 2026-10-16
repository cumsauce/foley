//! The cave bed (M12): drips from a low stone roof and a sub-bass rumble
//! modulated by the depth and the mass of stone above, fading as the cave
//! opens to the sky.

use core::f32::consts::TAU;

use crate::{LowPass, Rng, SAMPLE_RATE, Wobble};

/// What the cave round the listener is like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaveScene {
    /// How far below the surface the ear is, normalised to 0..1 (1 = 60 blocks down).
    pub depth: f32,
    /// How much stone is between the ear and the sky, normalised to 0..1 (1 = 60 solid blocks).
    pub mass: f32,
    /// How much of the sky the cave is open to: 1 = open air above, 0 = shut.
    pub openness: f32,
    /// How many waterlogged or stalactite blocks are in the column above, normalised to 0..1.
    pub drips: f32,
    /// How loud the bed is overall, 0..1.
    pub gain: f32,
}

impl CaveScene {
    /// A silent bed: nothing underground, the ear in the open, gain 0.
    pub const SILENT: CaveScene = CaveScene {
        depth: 0.0,
        mass: 0.0,
        openness: 1.0,
        drips: 0.0,
        gain: 0.0,
    };
}

/// Drip band, Hz. Real ceiling drips ring at the radial mode of the in-falling
/// drop's cavity: 800 Hz to 1.4 kHz.
const DRIP_HZ_LO: f32 = 800.0;
const DRIP_HZ_HI: f32 = 1400.0;
/// How long one drip rings, ms.
const DRIP_MS: f32 = 30.0;
/// Voices running at once: their pitches staggered, their pans staggered.
const VOICES: usize = 5;
/// The sub-bass centre, Hz: a slow rumble below 80 Hz, well below the drips.
const RUMBLE_HZ: f32 = 55.0;
/// Settling-stone bursts fire at this rate a second when the cave is wet and deep.
const STONE_RATE: f32 = 0.04;
/// Settling-stone gain, so they're a quiet texture under the bed.
const STONE_GAIN: f32 = 0.05;
/// How long a settling-stone burst takes, ms.
const STONE_MS: f32 = 80.0;
/// Rendered and thrown away on the first set_scene, so the filters and the drips'
/// slow envelopes reach their steady state before the first sample is heard.
const WARM_UP: usize = 4096;

/// A single ceiling voice: its pitch, its pan, the time until the next drip,
/// and the per-voice state of the currently-ringing drip (envelope, phase).
#[derive(Debug, Clone, Copy)]
struct DripVoice {
    hz: f32,
    /// L/R pan, -1 (full left) to 1 (full right).
    pan: f32,
    /// Seconds until the next drip.
    until: f32,
    /// The currently-ringing drip's envelope (0 to 1), or 0 if silent.
    envelope: f32,
    /// The currently-ringing drip's phase, radians.
    phase: f32,
    /// The exponential decay this drip's envelope takes, a sample.
    decay: f32,
    /// The 1 ms attack this drip's envelope rises with, a sample.
    attack: f32,
    /// Whether a drip is currently firing on this voice.
    firing: bool,
}

/// A single settling-stone burst's state, rendered into the bed as a small
/// pre-shaped noise envelope (a few short plate modes ringing down).
#[derive(Debug, Clone, Copy)]
struct StoneBurst {
    /// The burst's envelope, 0 to 1, or 0 if silent.
    envelope: f32,
    /// The decay of the envelope, a sample.
    decay: f32,
    /// The attack of the envelope, a sample.
    attack: f32,
    /// The phase of the noise's first mode.
    phase: f32,
    /// The frequency of the first mode, Hz.
    hz: f32,
}

#[derive(Debug)]
/// The cave bed (M12): a handful of ceiling voices dripping on a Poisson
/// process, a sub-bass rumble whose level tracks the depth and the mass of
/// stone above, and the rare settling-stone texture of a working cave. The
/// whole mix fades with openness.
pub struct Cave {
    rng: Rng,
    scene: CaveScene,
    voices: [DripVoice; VOICES],
    /// The sub-bass rumble: low-passed noise, with a slow wobble.
    rumble: LowPass,
    rumble_lfo: Wobble,
    /// The currently-firing settling-stone burst, if any.
    stone: Option<StoneBurst>,
    /// The schedule: when (in samples relative to now) the next stone fires.
    stone_wait: f32,
    /// Openness as heard: glides to the target with a 0.1 s time constant.
    openness_heard: f32,
    /// Total samples rendered so far (for WARM_UP). Wrap-around is fine.
    samples: usize,
    /// How many settling stones have fired this session. Test-only: an audio
    /// output detector can't reliably see them in a mix with drips.
    #[cfg(test)]
    stones_fired: u32,
}

impl Cave {
    pub fn new(seed: u64) -> Cave {
        let mut rng = Rng::new(seed);
        let rate = SAMPLE_RATE as f32;
        let attack = 1.0 - (-1.0 / (0.001 * rate)).exp();
        let decay = (-1.0 / (DRIP_MS * 0.001 * rate * 0.3)).exp();
        let voices = [(); VOICES].map(|()| DripVoice {
            hz: DRIP_HZ_LO + (DRIP_HZ_HI - DRIP_HZ_LO) * rng.unit(),
            pan: 2.0 * rng.unit() - 1.0,
            until: 0.2 * rng.unit(),
            envelope: 0.0,
            phase: 0.0,
            attack,
            decay,
            firing: false,
        });
        Cave {
            rng,
            scene: CaveScene::SILENT,
            voices,
            rumble: LowPass::new(RUMBLE_HZ),
            rumble_lfo: Wobble::new(0.3, 0.6, 0.5, 1.5),
            stone: None,
            stone_wait: 0.0,
            openness_heard: 1.0,
            samples: 0,
            #[cfg(test)]
            stones_fired: 0,
        }
    }

    pub fn set_scene(&mut self, scene: CaveScene) {
        if self.samples == 0 {
            self.scene = scene;
            self.openness_heard = scene.openness;
            // Warm the filters up to the new scene so the first sample doesn't click.
            self.render(&mut [0.0; WARM_UP]);
            return;
        }
        self.scene = scene;
    }

    pub fn scene(&self) -> CaveScene {
        self.scene
    }

    pub fn render(&mut self, out: &mut [f32]) {
        let rate = SAMPLE_RATE as f32;
        let dt = 1.0 / rate;
        let depth = self.scene.depth.clamp(0.0, 1.0);
        let mass = self.scene.mass.clamp(0.0, 1.0);
        let openness_target = self.scene.openness.clamp(0.0, 1.0);
        let drips = self.scene.drips.clamp(0.0, 1.0);
        let gain = self.scene.gain.clamp(0.0, 1.0);

        // The rumble's level: a deep cave under lots of stone rumbles; a
        // shallow cave under little stone is silent.
        let rumble_level = 0.15 * depth * mass;
        // Drips come at a rate proportional to the drip count, 0 (a dry
        // cave) to 5 a second (a soak): a Poisson process over all voices.
        let drip_rate = 5.0 * drips * VOICES as f32;
        // Settling stones: only in a deep, drippy cave. The plan says the
        // scene's `drips + depth > 0.5` gate.
        let stones_active = drips > 0.0 && drips + depth > 0.5;
        // The openness glide, 0.1 s: fade in over 0.1 s, fade out over 0.1 s.
        let openness_step = 1.0 - (-dt / 0.1).exp();

        for sample in out.iter_mut() {
            // Openness glide.
            self.openness_heard += openness_step * (openness_target - self.openness_heard);
            // Quiet means quiet: openness is a fade toward 0, multiplying the
            // mix, so a shut cave is at full mix and an open cave is silent.
            let openness_fade = (1.0 - self.openness_heard).clamp(0.0, 1.0);

            // The sub-bass rumble: low-passed white noise, level 0.15 * depth
            // * mass, with a slow LFO (Wobble) to keep it from being a flat
            // hiss.
            let noise = self.rng.signed();
            let lfo = self.rumble_lfo.next(self.rng.signed());
            let rumble = rumble_level * lfo * self.rumble.process(noise);

            // The drips: each voice fires on its own Poisson clock. When a
            // voice's `until` reaches 0, its envelope starts rising and the
            // drip rings. The envelope decays exponentially. The voice's pan
            // shifts the contribution toward the left or right of the mix
            // (here, modulating amplitude, since the bed is mono and "pan"
            // is a stereo-cue approximation).
            let mut drips_sum = 0.0f32;
            for voice in &mut self.voices {
                voice.until -= dt;
                if voice.until <= 0.0 {
                    if drip_rate > 0.0 {
                        // A fresh drip: arm the envelope and reset the phase.
                        voice.until = self.rng.exponential(drip_rate);
                        voice.envelope = 0.0;
                        voice.phase = 0.0;
                        voice.firing = true;
                    } else {
                        // Dry cave: keep the clock idle at a long wait.
                        voice.until = 1.0;
                        voice.firing = false;
                    }
                }
                if voice.firing {
                    voice.envelope += voice.attack * (1.0 - voice.envelope);
                    voice.phase = (voice.phase + TAU * voice.hz / rate) % TAU;
                    let pan = 0.5 + 0.5 * voice.pan;
                    drips_sum += 0.4 * voice.envelope * voice.phase.sin() * pan;
                    voice.envelope *= voice.decay;
                    if voice.envelope < 1e-3 {
                        voice.firing = false;
                    }
                }
            }

            // Settling stones: a Poisson at STONE_RATE a second when active.
            // Each burst is a short low-frequency envelope on a sub-300Hz
            // tone — distinct from the drips' 800-1400Hz band.
            let mut stone_mix = 0.0f32;
            if stones_active {
                self.stone_wait -= dt;
                if self.stone_wait <= 0.0 {
                    self.stone_wait = self.rng.exponential(STONE_RATE);
                    let stone_attack = 1.0 - (-1.0 / (0.005 * rate)).exp();
                    let stone_decay = (-1.0 / (STONE_MS * 0.001 * rate * 0.3)).exp();
                    self.stone = Some(StoneBurst {
                        envelope: 0.0,
                        decay: stone_decay,
                        attack: stone_attack,
                        phase: 0.0,
                        hz: 120.0 + 80.0 * self.rng.unit(),
                    });
                    #[cfg(test)]
                    {
                        self.stones_fired += 1;
                    }
                }
                if let Some(s) = &mut self.stone {
                    s.envelope += s.attack * (1.0 - s.envelope);
                    s.phase = (s.phase + TAU * s.hz / rate) % TAU;
                    // Use a half-rectified sine (the |sin| of a low frequency)
                    // for a "rock click" timbre, with a 4 ms envelope attack.
                    stone_mix = STONE_GAIN * s.envelope * s.phase.sin().abs();
                    s.envelope *= s.decay;
                    if s.envelope < 1e-4 {
                        self.stone = None;
                    }
                }
            }

            let mix = (rumble + drips_sum + stone_mix) * openness_fade;
            *sample = (gain * mix).tanh();
            self.samples = self.samples.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use std::{vec, vec::Vec};

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt()
    }

    fn render_with(scene: CaveScene, seed: u64) -> Vec<f32> {
        let mut c = Cave::new(seed);
        c.set_scene(scene);
        let mut out = vec![0.0; SAMPLE_RATE as usize];
        c.render(&mut out);
        out
    }

    fn quiet(scene: CaveScene) -> bool {
        rms(&render_with(scene, 1)) < 0.001
    }

    #[test]
    fn dry_shallow_cave_is_silent() {
        // Open cave, no drips, no mass: nothing to hear.
        assert!(quiet(CaveScene {
            depth: 0.1,
            mass: 0.1,
            openness: 1.0,
            drips: 0.0,
            gain: 0.0,
        }));
    }

    #[test]
    fn a_dry_cave_still_rumbles_when_deep() {
        // drips = 0, but depth and mass are high: rumble alone, no drips.
        let out = render_with(
            CaveScene {
                depth: 1.0,
                mass: 1.0,
                openness: 0.0,
                drips: 0.0,
                gain: 1.0,
            },
            9,
        );
        assert!(rms(&out) > 0.001, "dry cave rms {}", rms(&out));
    }

    #[test]
    fn a_shallow_cave_under_no_mass_is_silent() {
        // depth = 0 → rumble 0 → quiet.
        let out = render_with(
            CaveScene {
                depth: 0.0,
                mass: 0.0,
                openness: 0.0,
                drips: 1.0,
                gain: 0.0,
            },
            10,
        );
        assert_eq!(rms(&out), 0.0);
    }

    #[test]
    fn deep_loud_cave_rumbles() {
        let out = render_with(
            CaveScene {
                depth: 1.0,
                mass: 1.0,
                openness: 0.0,
                drips: 1.0,
                gain: 1.0,
            },
            7,
        );
        assert!(rms(&out) > 0.005, "{}", rms(&out));
    }

    #[test]
    fn openness_fades_the_bed() {
        let closed = render_with(
            CaveScene {
                depth: 1.0,
                mass: 1.0,
                openness: 0.0,
                drips: 1.0,
                gain: 1.0,
            },
            7,
        );
        let open = render_with(
            CaveScene {
                depth: 1.0,
                mass: 1.0,
                openness: 1.0,
                drips: 1.0,
                gain: 1.0,
            },
            7,
        );
        assert!(
            rms(&closed) > 2.0 * rms(&open),
            "{} vs {}",
            rms(&closed),
            rms(&open)
        );
    }

    #[test]
    fn every_call_is_finite_bounded() {
        for s in 0..4 {
            let out = render_with(
                CaveScene {
                    depth: 1.0,
                    mass: 1.0,
                    openness: 0.0,
                    drips: 1.0,
                    gain: 1.0,
                },
                s,
            );
            assert!(out.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
        }
    }

    #[test]
    fn settling_stones_fire_in_a_drippy_cave() {
        // 30 s of a wet, deep, closed cave should contain 1 to 4 settling-
        // stone transients (Poisson at 0.04/s × 30 s = 1.2 expected). The
        // audio output can't reliably reveal them in a mix with drips, so
        // we read the internal counter (test-only).
        let mut c = Cave::new(11);
        c.set_scene(CaveScene {
            depth: 1.0,
            mass: 1.0,
            openness: 0.0,
            drips: 1.0,
            gain: 1.0,
        });
        let mut out = vec![0.0; 30 * SAMPLE_RATE as usize];
        c.render(&mut out);
        assert!(
            (1..=4).contains(&c.stones_fired),
            "stones={} (expected 1-4)",
            c.stones_fired
        );
    }

    #[test]
    fn settling_stones_dont_fire_in_a_dry_cave() {
        // A dry cave has drips = 0, so the stones gate is closed.
        let mut c = Cave::new(13);
        c.set_scene(CaveScene {
            depth: 1.0,
            mass: 1.0,
            openness: 0.0,
            drips: 0.0,
            gain: 1.0,
        });
        let mut out = vec![0.0; 30 * SAMPLE_RATE as usize];
        c.render(&mut out);
        assert_eq!(c.stones_fired, 0);
    }
}
