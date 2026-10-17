//! The streaming shore bed (M17): one continuous sound for a shoreline —
//! a shingle/foam wash under `outdoors::surf` grains scheduled by
//! storm x exposure. Tests first; implementation follows.

use crate::{BandPass, Bubble, Rng, SAMPLE_RATE};

/// How many wave grains can sound at once — also the size of the free
/// buffer pool, so a spawn on the audio thread never allocates
/// (`the_audio_thread_never_allocates` in `foley-ffi` still guards that).
///
/// Derived from the wave cadence: a grain is one full ~7 s wave
/// (`outdoors::SURF_LEN`) and surges are never closer than the 2 s
/// full-storm interval, so at most `ceil(7 s / 2 s) + 1 = 5` grains sound
/// at once; 6 slots leave a little headroom. One buffer per slot means the
/// (multi-megabyte) wave buffers are allocated once per bed, not on every
/// return to shore.
const MAX_GRAINS: usize = 6;

/// The foam/shingle wash level at full wave height at the water's edge —
/// well below the grains it runs under.
const WASH_LEVEL: f32 = 0.15;
/// Base gain of one wave grain at full height, before the distance taper.
const GRAIN_GAIN: f32 = 0.12;
/// Distance taper: a grain plays at
/// `GRAIN_GAIN × height × (WAVE_DISTANCE_FLOOR + WAVE_DISTANCE_RANGE × shore)`
/// — full level at the water's edge, a floor of 0.4 still audible far away
/// since `height > 0` already gates full silence.
const WAVE_DISTANCE_FLOOR: f32 = 0.4;
const WAVE_DISTANCE_RANGE: f32 = 0.6;
/// Seconds of linear fade at a grain's tail.
const TAIL_FADE_SECS: f32 = 0.05;

/// What the Java side tells the shore bed about the listener.
#[derive(Debug, Clone, Copy)]
pub struct SurfScene {
    /// Wave height in metres (0.2 to 3.0); 0 = no shore, renders silence.
    pub height: f32,
    /// 0 to 1: how close the listener is to the water's edge.
    pub shore: f32,
    /// 0 to 1: rain and thunder above the shore.
    pub storm: f32,
    /// 0 to 1: the wind speed the wind bed reports (never a locally re-measured wind).
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
    /// Free wave buffers: refilled by `outdoors::surf_into` and returned as
    /// grains finish, so spawning a wave costs no allocation.
    pool: Vec<Vec<f32>>,
    /// Scratch for `outdoors::surf_into`, 256 bubble slots like the one-shot's.
    bubbles: Vec<Bubble>,
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
            // Pre-sized: pushes on the audio thread must never reallocate.
            grains: Vec::with_capacity(MAX_GRAINS),
            pool: (0..MAX_GRAINS)
                .map(|_| vec![0.0f32; crate::outdoors::SURF_LEN])
                .collect(),
            bubbles: Vec::with_capacity(256),
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
        // A shorter cadence than the remaining countdown re-arms it: a storm
        // that arrives mid-calm must not wait out the old calm gap.
        let interval = self.interval_secs();
        if self.until_surge > interval {
            self.until_surge = interval;
        }
    }

    /// The silence path: no shore (`height <= 0`) or a non-finite height.
    /// Returns every live grain's buffer to the pool and re-arms the surge
    /// countdown, so returning to shore starts clean — no stale grain resumes
    /// at its old gain, and no wave fires from a leftover countdown.
    fn go_silent(&mut self, out: &mut [f32]) {
        while let Some(grain) = self.grains.pop() {
            self.pool.push(grain.data);
        }
        self.until_surge = self.interval_secs();
        out.fill(0.0);
    }

    pub fn render(&mut self, out: &mut [f32]) {
        // The client's height is not trusted, and the two failure modes are
        // deliberately different — do not "unify" them:
        //   * non-finite (NaN, ±∞) → silence: an unrepresentable height means
        //     no shore; it is never sanitised into a huge-but-sounding value;
        //   * finite → sanitised then clamped to the documented 0.2–3.0 m
        //     range, so `height = 1e20` renders exactly like a 3.0 m bed
        //     instead of saturating the wash.
        // Verified by `nan_height_is_silent` and
        // `a_huge_scene_clamps_to_the_bed_maximum`.
        let raw_height = self.scene.height;
        if !raw_height.is_finite() || raw_height <= 0.0 {
            self.go_silent(out);
            return;
        }
        let height = raw_height.clamp(0.2, 3.0);
        let shore = self.scene.shore.clamp(0.0, 1.0);
        // The foam and shingle the wave leaves behind, well below the grains.
        let wash_level = WASH_LEVEL * (height / 3.0) * shore;
        let interval = self.interval_secs();
        let dt = 1.0 / SAMPLE_RATE as f32;
        for sample in out.iter_mut() {
            self.until_surge -= dt;
            if self.until_surge <= 0.0 {
                self.spawn_grain();
                self.until_surge += interval;
            }
            // The continuous wash: band-passed noise under the waves, in
            // three cascaded 900 Hz band-pass stages, read 0 → 1 → 2 (stage
            // 0 takes the noise, stage 2 feeds the sum) — the natural
            // direction; the stages are identical, so only readability
            // depends on the order.
            let n = self.rng.signed();
            let w1 = self.wash[0].process(n);
            let w2 = self.wash[1].process(w1);
            let mut sum = wash_level * self.wash[2].process(w2);
            // Overlapping grains, summed; each retires when its buffer ends,
            // handing the buffer back to the pool for the next wave.
            for grain in &mut self.grains {
                sum += grain.gain * grain.data[grain.pos];
                grain.pos += 1;
            }
            let mut i = 0;
            while i < self.grains.len() {
                if self.grains[i].pos >= self.grains[i].data.len() {
                    let grain = self.grains.remove(i);
                    self.pool.push(grain.data);
                } else {
                    i += 1;
                }
            }
            // Soft-clip so any overlap stays in range.
            *sample = sum / (1.0 + sum.abs());
        }
    }

    /// Seconds between surges: ~12 s calm to ~2 s full storm.
    fn interval_secs(&self) -> f32 {
        let stormy = (self.scene.storm * self.scene.exposure).clamp(0.0, 1.0);
        12.0 - 10.0 * stormy
    }

    /// Spawns the next wave; one full-grain synthesis costs ≈11 ms per 0.25 s
    /// of rendered audio (~24× realtime), so the client's 0.5 s pulls cost
    /// ≈22 ms — revisit if the pull size ever shrinks.
    fn spawn_grain(&mut self) {
        // The pool has one buffer per grain slot, so an empty pool is the
        // old `grains.len() >= MAX_GRAINS` guard — and a spawn never allocates.
        let Some(mut data) = self.pool.pop() else {
            return;
        };
        let seed = self.seed ^ self.counter.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        self.counter += 1;
        // Sanitise-then-clamp at the boundary (the `render` gate means this
        // is always finite, but the raw client value never leaves `surf.rs`):
        // a finite 1e20 m bed synthesises exactly the 3.0 m wave, while
        // non-finite heights never reach here — they silence in `render`.
        let height = self.scene.height.clamp(0.2, 3.0);
        crate::outdoors::surf_into(height, seed, &mut data, &mut self.bubbles);
        // `surf_into`'s natural backwash drain already shapes the tail; this
        // 50 ms linear fade guarantees an exact-zero end so a retiring grain
        // never clicks.
        let fade = (TAIL_FADE_SECS * SAMPLE_RATE as f32) as usize;
        let n = data.len();
        for (i, s) in data.iter_mut().enumerate().skip(n.saturating_sub(fade)) {
            *s *= (n - 1 - i) as f32 / fade as f32;
        }
        // Waves attenuate with distance from the water: full level at the
        // edge (`shore = 1`), a floor of 0.4 still audible far away since
        // `height > 0` already gates full silence.
        let shore = self.scene.shore.clamp(0.0, 1.0);
        let gain = GRAIN_GAIN * height * (WAVE_DISTANCE_FLOOR + WAVE_DISTANCE_RANGE * shore);
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
    fn waves_far_from_the_water_are_quieter() {
        // Same height and storm; only proximity differs. A player 20-32 blocks
        // out must hear the crash clearly attenuated, but never silenced
        // (the bed only goes quiet when `height <= 0`).
        let r = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
        let mut at_water = Surf::new(8);
        at_water.set_scene(SurfScene {
            height: 2.2,
            shore: 1.0,
            storm: 1.0,
            exposure: 1.0,
        });
        let mut inland = Surf::new(8);
        inland.set_scene(SurfScene {
            height: 2.2,
            shore: 0.1,
            storm: 1.0,
            exposure: 1.0,
        });
        let near = render_secs(&mut at_water, 12.0);
        let far = render_secs(&mut inland, 12.0);
        let (rn, rf) = (r(&near), r(&far));
        assert!(
            rf > 0.02 * rn,
            "the far shore must still be live, rms {rf} vs {rn}"
        );
        assert!(
            rf < 0.6 * rn,
            "waves 20-odd blocks out must be clearly quieter, rms {rf} vs {rn}"
        );
    }

    #[test]
    fn nan_height_is_silent() {
        // Non-finite heights — NaN and ±∞ — are treated as no shore. They
        // are silenced on purpose, never sanitised/clamped into a huge but
        // still-audible value (`a_huge_scene_clamps_to_the_bed_maximum`
        // covers the finite counterpart).
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut s = Surf::new(9);
            s.set_scene(scene(bad, 1.0, 1.0));
            let out = render_secs(&mut s, 3.0);
            assert!(
                out.iter().all(|x| *x == 0.0),
                "height {bad} must render silence"
            );
            assert!(
                s.grains.is_empty(),
                "height {bad} must leave no live grains, got {}",
                s.grains.len()
            );
            assert_eq!(
                s.pool.len() + s.grains.len(),
                MAX_GRAINS,
                "height {bad} must return every wave buffer to the pool"
            );
        }
        // A height that turns non-finite while waves are sounding must also
        // go silent — retiring every live grain, not spraying NaN through
        // the wash or leaving a stale grain to resume.
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut s = Surf::new(9);
            s.set_scene(scene(2.2, 1.0, 1.0));
            let out = render_secs(&mut s, 3.0);
            assert!(
                out.iter().any(|x| *x != 0.0),
                "precondition: the shore sounds"
            );
            assert!(!s.grains.is_empty(), "precondition: grains are live");
            s.set_scene(scene(bad, 1.0, 1.0));
            let out = render_secs(&mut s, 3.0);
            assert!(
                out.iter().all(|x| *x == 0.0),
                "a height that turns {bad} mid-life must fall silent"
            );
            assert!(
                s.grains.is_empty(),
                "going silent must retire the live grains, got {}",
                s.grains.len()
            );
        }
    }

    #[test]
    fn a_huge_scene_clamps_to_the_bed_maximum() {
        // A *finite* 1e20 m height is not silence and not infinity: it
        // sanitises then clamps to the documented 3.0 m ceiling, so the bed
        // renders exactly what a 3.0 m bed renders for the same seed and
        // state — finite samples throughout, never the silence path.
        let mut huge = Surf::new(9);
        huge.set_scene(scene(1e20, 1.0, 1.0));
        let mut three = Surf::new(9);
        three.set_scene(scene(3.0, 1.0, 1.0));
        let a = render_secs(&mut huge, 3.0);
        let b = render_secs(&mut three, 3.0);
        assert!(
            a.iter().any(|x| *x != 0.0),
            "a 1e20 m height must not render silence"
        );
        assert!(
            a.iter().all(|x| x.is_finite()),
            "the clamped bed must stay finite"
        );
        assert!(
            a.iter().eq(b.iter()),
            "a huge height must clamp to the 3.0 m ceiling, not saturate the wash"
        );
        assert!(
            !huge.grains.is_empty(),
            "the clamped bed must keep spawning waves"
        );
        // Mid-transition too: a live bed that jumps to 1e20 keeps sounding
        // sample-for-sample like one that jumps to 3.0.
        let mut huge = Surf::new(12);
        huge.set_scene(scene(1.0, 1.0, 1.0));
        let mut three = Surf::new(12);
        three.set_scene(scene(1.0, 1.0, 1.0));
        let _ = render_secs(&mut huge, 3.0);
        let _ = render_secs(&mut three, 3.0);
        huge.set_scene(scene(1e20, 1.0, 1.0));
        three.set_scene(scene(3.0, 1.0, 1.0));
        let a = render_secs(&mut huge, 3.0);
        let b = render_secs(&mut three, 3.0);
        assert!(
            a.iter().eq(b.iter()),
            "mid-life, 1e20 must behave exactly like 3.0"
        );
    }

    #[test]
    fn returning_to_shore_starts_without_stale_grains() {
        let mut s = Surf::new(10);
        s.set_scene(scene(2.2, 1.0, 1.0));
        let out = render_secs(&mut s, 3.0);
        assert!(
            out.iter().any(|x| *x != 0.0),
            "precondition: the storm must sound"
        );
        assert!(!s.grains.is_empty(), "precondition: waves are sounding");
        // Out of range: every live grain must return to the pool.
        s.set_scene(scene(0.0, 1.0, 1.0));
        let out = render_secs(&mut s, 1.0);
        assert!(out.iter().all(|x| *x == 0.0), "no shore must be silence");
        assert!(
            s.grains.is_empty(),
            "going silent must clear the live grains, got {}",
            s.grains.len()
        );
        assert_eq!(
            s.pool.len() + s.grains.len(),
            MAX_GRAINS,
            "every wave buffer must return to the pool"
        );
        // Back to shore: the countdown is re-armed, so no stale grain
        // resumes and no wave fires from the leftover countdown.
        s.set_scene(scene(2.2, 1.0, 1.0));
        assert!(
            s.until_surge > 0.0,
            "returning to shore must re-arm the surge countdown, got {}",
            s.until_surge
        );
        let out = render_secs(&mut s, 0.5);
        assert!(
            out.iter().any(|x| *x != 0.0),
            "the returning shore must already lap"
        );
        assert!(
            s.grains.is_empty(),
            "no wave may spawn from stale state within the re-armed interval"
        );
    }

    #[test]
    fn calm_to_storm_spawns_within_the_storm_interval() {
        let mut s = Surf::new(11);
        s.set_scene(scene(2.2, 0.0, 0.0)); // calm: 12 s between surges
        let _ = render_secs(&mut s, 5.0); // partway through the calm countdown
        let before = s.counter;
        s.set_scene(scene(2.2, 1.0, 1.0)); // storm: 2 s between surges
        // The first storm wave must break within the storm interval of the
        // post-transition audio, not wait out the leftover calm countdown.
        let mut spawn = None;
        for i in 0..=2 * crate::SAMPLE_RATE as usize {
            let mut buf = [0.0f32; 1];
            s.render(&mut buf);
            if s.counter > before {
                spawn = Some((i + 1) as f32 / crate::SAMPLE_RATE as f32);
                break;
            }
        }
        let spawn = spawn.expect("a storm wave must break within the storm interval");
        assert!(
            spawn <= 2.0,
            "first storm wave at {spawn} s must be within the 2 s interval"
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
