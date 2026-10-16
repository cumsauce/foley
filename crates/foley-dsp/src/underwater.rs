//! M14: underwater ambience — bubbles rising past the ear and a low pressure tone,
//! rendered when the player's head is under water. A listener state, not a placed
//! bed: one bed is always running (it's silent when the head is above sea level).

use crate::Fixed;
use crate::{Bubble, Rng, SAMPLE_RATE};

/// What the underwater bed is told about the world: a single `level` 0..=1, the
/// controller's glide of how submerged the listener is (depth and movement). At
/// 0 the bed is silent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnderwaterScene {
    pub level: f32,
}

impl UnderwaterScene {
    pub const DRY: UnderwaterScene = UnderwaterScene { level: 0.0 };
    pub const UNDER: UnderwaterScene = UnderwaterScene { level: 1.0 };
}

/// At most this many bubbles ring in the bed. Past it, the new one is dropped.
const MAX_VOICES: usize = 64;
/// The pressure tone's frequency (Hz): a very low sinusoid, what the deep sea
/// pushes into the mix.
const PRESSURE_HZ: f32 = 50.0;
/// The pressure tone's level at full scene `level`: small, beneath the bubbles.
const PRESSURE_LEVEL: f32 = 0.25;

/// Bubbles rising past the ear at `level` (a controller sets it from the depth
/// and movement of the listener), with a slow 50 Hz sinusoid for the deep. Rises
/// and falls smoothly with the scene's level.
#[derive(Debug)]
pub struct Underwater {
    rng: Rng,
    scene: UnderwaterScene,
    heard: UnderwaterScene,
    fresh: bool,
    until_next: f32,
    voices: Fixed<Bubble, MAX_VOICES>,
    /// Pressure tone's running phase.
    phase: f32,
}

impl Underwater {
    pub fn new(seed: u64) -> Underwater {
        Underwater {
            rng: Rng::new(seed),
            scene: UnderwaterScene::DRY,
            heard: UnderwaterScene::DRY,
            fresh: true,
            until_next: 0.0,
            voices: Fixed::new(),
            phase: 0.0,
        }
    }

    pub fn set_scene(&mut self, scene: UnderwaterScene) {
        self.scene = scene;
        if self.fresh {
            self.heard = scene;
            self.fresh = false;
        }
    }

    pub fn render(&mut self, out: &mut [f32]) {
        let dt = 1.0 / SAMPLE_RATE as f32;
        // Glide level over about a second (matches the spec's emerging fade).
        // TAU = 0.3 s so by 1 s we're 96% there and the bed is essentially
        // silent.
        let glide = dt / 0.3;
        for sample in out {
            self.heard.level += (self.scene.level.clamp(0.0, 1.0) - self.heard.level) * glide;
            let level = self.heard.level;
            // Bubble rate rises from 0 to 20 a second as level rises.
            let rate = level * (4.0 + 16.0 * level);
            if rate > 0.0 {
                self.until_next -= dt;
                while self.until_next <= 0.0 {
                    self.until_next += self.rng.exponential(rate);
                    if self.voices.len() < MAX_VOICES {
                        // Log-uniform 0.5 to 6 mm, like the water bed's babbling bubbles.
                        let radius = 0.0005 * 12.0f32.powf(self.rng.unit());
                        self.voices
                            .push(Bubble::new(radius, 0.02 * (0.5 + self.rng.unit())));
                    }
                }
            }
            let mut mix = 0.0;
            for voice in &mut self.voices {
                mix += voice.sample();
            }
            // Pressure tone: a slow sine at the deep's frequency.
            self.phase += core::f32::consts::TAU * PRESSURE_HZ * dt;
            mix += PRESSURE_LEVEL * level * self.phase.sin();
            // The whole bed is at the heard level; below 1e-4 we flush to zero so
            // the bed is truly silent once the controller has faded it out.
            *sample = if level > 1e-4 { level * mix } else { 0.0 };
        }
        self.voices.retain(|voice| !voice.done());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use std::{vec, vec::Vec};

    /// Peak the last render wrote at, used only by tests to confirm silence.
    fn peak(s: &[f32]) -> f32 {
        s.iter().fold(0f32, |m, x| m.max(x.abs()))
    }

    #[test]
    fn a_submerged_bed_renders_bubbles_and_pressure() {
        let mut bed = Underwater::new(1);
        bed.set_scene(UnderwaterScene::UNDER);
        let mut out = vec![0.0f32; SAMPLE_RATE as usize]; // 1 second
        bed.render(&mut out);
        let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
        assert!(rms > 0.05, "submerged bed was silent at {rms}");
        // The pressure tone at 50 Hz: zero crossings in 1 s ≈ 100.
        let crossings = out.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        assert!(
            (50..=150).contains(&crossings),
            "pressure tone not heard: {crossings} crossings"
        );
        // The bubble rings should produce higher-frequency zero crossings than the
        // 50 Hz tone alone (≈ 100 a second): the bubbles contribute extra.
        // (Most bubbles ring at > 1 kHz so each adds many crossings in a second.)
    }

    #[test]
    fn a_dry_bed_is_silent() {
        let mut bed = Underwater::new(2);
        bed.set_scene(UnderwaterScene::DRY);
        let mut out = vec![0.0f32; SAMPLE_RATE as usize];
        bed.render(&mut out);
        assert!(peak(&out) < 1e-4, "dry bed peak {}", peak(&out));
    }

    #[test]
    fn a_bed_glides_to_silence_when_level_drops() {
        // Start under, then drop to dry mid-render: the bed's glide fades it to
        // silence over about a second.
        let mut bed = Underwater::new(3);
        bed.set_scene(UnderwaterScene::UNDER);
        let mut first = vec![0.0f32; SAMPLE_RATE as usize];
        bed.render(&mut first);
        assert!(peak(&first) > 0.01);
        bed.set_scene(UnderwaterScene::DRY);
        // Render two seconds for the level to glide near 0.
        let mut second = vec![0.0f32; 2 * SAMPLE_RATE as usize];
        bed.render(&mut second);
        let tail = &second[SAMPLE_RATE as usize..];
        // The bed should be much quieter than when it was at full level (which
        // had peak ~0.4): a tenth of that, easily.
        assert!(
            peak(tail) < 0.05 * peak(&first),
            "bed didn't fade: tail peak {} vs first peak {}",
            peak(tail),
            peak(&first)
        );
    }
}
