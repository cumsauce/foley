//! The streaming shore bed (M17): one continuous sound for a shoreline —
//! a shingle/foam wash under `outdoors::surf` grains scheduled by
//! storm x exposure. Tests first; implementation follows.

use crate::{BandPass, Rng, SAMPLE_RATE};

/// What the Java side tells the shore bed about the listener.
#[derive(Debug, Clone, Copy)]
pub struct SurfScene {
    /// Wave height in metres (0.2 to 3.0); 0 = no shore, renders silence.
    pub height: f32,
    /// 0 to 1: how close the listener is to the water's edge.
    pub shore: f32,
    /// 0 to 1: rain and thunder above the shore.
    pub storm: f32,
    /// 0 to 1: the wind the wind bed reports (never a locally re-measured wind).
    pub exposure: f32,
}

/// One active wave grain: the buffer from `outdoors::surf`, its read cursor
/// and the level it plays at.
#[derive(Debug)]
struct Grain {
    data: Vec<f32>,
    pos: usize,
    gain: f32,
}

/// One shore bed. Streams the wave cycle: a continuous shingle/foam wash
/// under `outdoors::surf` grains scheduled at a rate set by storm x exposure.
#[derive(Debug)]
pub struct Surf {
    seed: u64,
    counter: u64,
    scene: SurfScene,
    /// Seconds until the next surge starts.
    until_surge: f32,
    grains: Vec<Grain>,
    rng: Rng,
    wash: [BandPass; 3],
}

impl Surf {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            counter: 0,
            scene: SurfScene {
                height: 0.0,
                shore: 0.0,
                storm: 0.0,
                exposure: 0.0,
            },
            // The first wave breaks the moment the bed starts.
            until_surge: 0.0,
            grains: Vec::new(),
            rng: Rng::new(seed ^ 0x5F17),
            wash: [
                BandPass::new(900.0, 0.5),
                BandPass::new(900.0, 0.5),
                BandPass::new(900.0, 0.5),
            ],
        }
    }

    pub fn set_scene(&mut self, scene: SurfScene) {
        self.scene = scene;
    }

    pub fn render(&mut self, out: &mut [f32]) {
        if self.scene.height <= 0.0 {
            out.fill(0.0);
            return;
        }
        let shore = self.scene.shore.clamp(0.0, 1.0);
        // The foam and shingle the wave leaves behind, well below the grains.
        let wash_level = 0.15 * (self.scene.height / 3.0) * shore;
        let interval = self.interval_secs();
        let dt = 1.0 / SAMPLE_RATE as f32;
        for sample in out.iter_mut() {
            self.until_surge -= dt;
            if self.until_surge <= 0.0 {
                self.spawn_grain();
                self.until_surge += interval;
            }
            // The continuous wash: band-passed noise under the waves.
            let n = self.rng.signed();
            let w1 = self.wash[2].process(n);
            let w2 = self.wash[1].process(w1);
            let mut sum = wash_level * self.wash[0].process(w2);
            // Overlapping grains, summed; each retires when its buffer ends.
            for grain in &mut self.grains {
                sum += grain.gain * grain.data[grain.pos];
                grain.pos += 1;
            }
            self.grains.retain(|g| g.pos < g.data.len());
            // Soft-clip so any overlap stays in range.
            *sample = sum / (1.0 + sum.abs());
        }
    }

    /// Seconds between surges: ~12 s calm to ~2 s full storm.
    fn interval_secs(&self) -> f32 {
        let stormy = (self.scene.storm * self.scene.exposure).clamp(0.0, 1.0);
        12.0 - 10.0 * stormy
    }

    fn spawn_grain(&mut self) {
        if self.grains.len() >= 16 {
            return;
        }
        let seed = self.seed ^ self.counter.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        self.counter += 1;
        let mut data = crate::outdoors::surf(self.scene.height, seed);
        // Fade the buffer's tail so a grain ending under a newer one never clicks.
        let fade = (0.05 * SAMPLE_RATE as f32) as usize;
        let n = data.len();
        for (i, s) in data.iter_mut().enumerate().skip(n.saturating_sub(fade)) {
            *s *= (n - 1 - i) as f32 / fade as f32;
        }
        let gain = 0.12 * self.scene.height.clamp(0.2, 3.0);
        self.grains.push(Grain { data, pos: 0, gain });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(height: f32, storm: f32, exposure: f32) -> SurfScene {
        SurfScene {
            height,
            shore: 1.0,
            storm,
            exposure,
        }
    }

    fn render_secs(s: &mut Surf, secs: f32) -> Vec<f32> {
        let mut out = vec![0.0f32; (secs * crate::SAMPLE_RATE as f32) as usize];
        s.render(&mut out);
        out
    }

    #[test]
    fn silent_with_no_shore() {
        let mut s = Surf::new(1);
        s.set_scene(scene(0.0, 1.0, 1.0));
        let out = render_secs(&mut s, 3.0);
        assert!(
            out.iter().all(|x| *x == 0.0),
            "a bed with no shore must render silence"
        );
    }

    #[test]
    fn a_calm_shore_still_sounds() {
        let mut s = Surf::new(2);
        s.set_scene(scene(0.3, 0.0, 0.0));
        let out = render_secs(&mut s, 5.0);
        let rms = (out.iter().map(|x| x * x).sum::<f32>() / out.len() as f32).sqrt();
        assert!(rms > 1e-4, "a calm lake must still lap, rms {rms}");
    }

    #[test]
    fn a_storm_has_more_surges_than_a_calm_sea() {
        // Count envelope peaks above a threshold over 60 s.
        fn peaks(mut s: Surf, storm: f32) -> usize {
            s.set_scene(scene(2.2, storm, 1.0));
            let out = render_secs(&mut s, 60.0);
            let win = (crate::SAMPLE_RATE / 10) as usize;
            let mut over = false;
            let mut n = 0;
            for chunk in out.chunks(win) {
                let r = (chunk.iter().map(|x| x * x).sum::<f32>() / chunk.len() as f32).sqrt();
                if r > 0.02 && !over {
                    n += 1;
                    over = true;
                }
                if r <= 0.02 {
                    over = false;
                }
            }
            n
        }
        let calm = peaks(Surf::new(3), 0.0);
        let storm = peaks(Surf::new(3), 1.0);
        assert!(storm > calm, "storm surges {storm} must exceed calm {calm}");
    }

    #[test]
    fn the_foam_wash_is_heard_between_waves() {
        // With no storm the next surge is 12 s away and the first grain has
        // ended by ~7 s, so 8..11 s holds only the continuous foam/shingle wash.
        fn between_waves_rms(shore: f32) -> f32 {
            let mut s = Surf::new(7);
            s.set_scene(SurfScene {
                height: 1.0,
                shore,
                storm: 0.0,
                exposure: 0.0,
            });
            let out = render_secs(&mut s, 12.0);
            let a = (8.0 * crate::SAMPLE_RATE as f32) as usize;
            let b = (11.0 * crate::SAMPLE_RATE as f32) as usize;
            let w = &out[a..b];
            (w.iter().map(|x| x * x).sum::<f32>() / w.len() as f32).sqrt()
        }
        let at_water = between_waves_rms(1.0);
        assert!(
            at_water > 1e-4,
            "the foam wash must be heard between waves, rms {at_water}"
        );
        // The wash scales with `shore`: at the water's edge it laps; ashore it is gone.
        let inland = between_waves_rms(0.0);
        assert!(
            inland == 0.0,
            "with no proximity to the water there is no wash, rms {inland}"
        );
    }

    #[test]
    fn waves_keep_coming_without_a_moving_listener() {
        // No scene change after the first: the cycle must not stall.
        let mut s = Surf::new(4);
        s.set_scene(scene(1.0, 1.0, 1.0));
        let out = render_secs(&mut s, 120.0);
        let rms = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
        let last = &out[out.len() - crate::SAMPLE_RATE as usize..];
        assert!(
            rms(last) > 1e-3,
            "waves must keep arriving while standing still, rms {}",
            rms(last)
        );
        // Arrivals, not just noise: every 10 s stretch must contain a surge
        // (a scheduler that fires once and stalls would leave only the wash).
        let win = (crate::SAMPLE_RATE / 10) as usize;
        let surge = 0.010;
        let mut arrivals = 0;
        for seg in out.chunks(10 * crate::SAMPLE_RATE as usize) {
            if seg.chunks(win).any(|c| rms(c) > surge) {
                arrivals += 1;
            }
        }
        assert!(
            arrivals >= 12,
            "a surge must arrive in each of the 12 ten-second stretches, got {arrivals}"
        );
    }

    #[test]
    fn overlapping_waves_stay_bounded_and_seamless() {
        let mut s = Surf::new(5);
        s.set_scene(scene(2.2, 1.0, 1.0));
        let out = render_secs(&mut s, 90.0);
        assert!(
            out.iter().all(|x| x.is_finite() && x.abs() <= 1.0),
            "output must stay in range"
        );
        // No single-sample jump larger than a step within a wave.
        let max_step = out
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(
            max_step < 0.5,
            "a click at a grain boundary, max step {max_step}"
        );
    }

    #[test]
    fn a_higher_wave_is_louder() {
        let mut low = Surf::new(6);
        low.set_scene(scene(0.3, 0.5, 0.5));
        let mut high = Surf::new(6);
        high.set_scene(scene(2.2, 0.5, 0.5));
        let r = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
        let lo = render_secs(&mut low, 20.0);
        let hi = render_secs(&mut high, 20.0);
        assert!(r(&hi) > r(&lo), "a 2.2 m sea must beat a 0.3 m lap");
    }
}
