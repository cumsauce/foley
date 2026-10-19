//! Biome-specific beds (M19): desert hiss, mushroom hum, lush/dripstone cave
//! colors, deep dark, snow hush, mangrove drone. One streaming bed, with the
//! per-biome work sharing the same envelope code. Java keeps one bed and
//! changes its scene; `kind` is snapped by the FFI's glide, and the state it
//! replaces fades out under the new one over [`CROSSFADE`] rather than being
//! cut at the switch.

use std::f32::consts::TAU;

use foley_dsp::{BandPass, LowPass, Rng, SAMPLE_RATE};

/// The seven biome bed kinds, in the order `FOLEY_BED_BIOME` takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiomeKind {
    /// Granular sand hiss underfoot, hollow wind whistle on the open plain,
    /// and (when it's blowing) the slow whoosh of a dust devil.
    Desert = 0,
    /// A low ambient hum unique to the mushroom biome, ~120 Hz + 1.6x overtone.
    Mushroom = 1,
    /// High-frequency moss hiss.
    Lush = 2,
    /// More drips, less rumble.
    Dripstone = 3,
    /// Subsonic ~30 Hz hum and scuttling sculk sensors as one-shots.
    DeepDark = 4,
    /// High-frequency absorption: a slow LPF applied to a muffled wind texture.
    Snow = 5,
    /// Humid insect drone, band-passed noise at 4 kHz.
    Mangrove = 6,
}

impl BiomeKind {
    pub fn from_index(i: u32) -> Option<Self> {
        Some(match i {
            0 => Self::Desert,
            1 => Self::Mushroom,
            2 => Self::Lush,
            3 => Self::Dripstone,
            4 => Self::DeepDark,
            5 => Self::Snow,
            6 => Self::Mangrove,
            _ => return None,
        })
    }
}

/// What the Java side tells the bed about the listener's biome.
#[derive(Debug, Clone, Copy)]
pub struct BiomeScene {
    pub kind: BiomeKind,
    /// 0 to 1: how much of the biome the listener is in (1 = centered).
    pub intensity: f32,
    /// A kind-specific count (vortex rings per 30 s for desert; scuttle count
    /// for deep_dark; the raw mangrove leaf count, 0..=200, for mangrove;
    /// leaf exposure for snow; unused for the others).
    pub sub: f32,
    /// 0 to 1: the bed's master gain.
    pub gain: f32,
}

/// One per-biome bed. Java keeps one bed and changes its scene; `kind` is
/// snapped by the FFI's glide, so a biome change swaps the state — the old one
/// fading out under the new over [`CROSSFADE`] samples.
#[derive(Debug)]
pub struct Biome {
    seed: u64,
    sample_rate: u32,
    scene: BiomeScene,
    state: BiomeState,
    /// The state a kind change replaced, still sounding through the crossfade.
    outgoing: Option<BiomeState>,
    /// Samples left in that crossfade.
    xfade_left: usize,
    /// Where the outgoing state renders while the new one fills `out`.
    scratch: Vec<f32>,
}

/// How long a kind change takes to cross, in samples: long enough that the
/// outgoing biome's tail is not cut mid-ring, short enough that the switch
/// still reads as one biome giving way to the next (an eighth of a second).
const CROSSFADE: usize = (SAMPLE_RATE / 8) as usize;

#[derive(Debug)]
enum BiomeState {
    Desert(DesertState),
    Mushroom(MushroomState),
    Lush(LushState),
    Dripstone(DripstoneState),
    DeepDark(DeepDarkState),
    Snow(SnowState),
    Mangrove(MangroveState),
}

impl BiomeState {
    /// Renders whichever biome this is; writes `out` whole.
    fn render(&mut self, out: &mut [f32], scene: BiomeScene, gain: f32, sample_rate: u32) {
        match self {
            BiomeState::Desert(s) => s.render(out, scene, gain, sample_rate),
            BiomeState::Mushroom(s) => s.render(out, scene, gain, sample_rate),
            BiomeState::Lush(s) => s.render(out, scene, gain, sample_rate),
            BiomeState::Dripstone(s) => s.render(out, scene, gain, sample_rate),
            BiomeState::DeepDark(s) => s.render(out, scene, gain, sample_rate),
            BiomeState::Snow(s) => s.render(out, scene, gain, sample_rate),
            BiomeState::Mangrove(s) => s.render(out, scene, gain, sample_rate),
        }
    }
}

#[derive(Debug)]
struct DesertState {
    rng: Rng,
    until_grain: f32,
    until_vortex: f32,
    vortex: Option<Vortex>,
    whistle: BandPass,
}

#[derive(Debug)]
struct Vortex {
    length: usize,
    at: usize,
    band: BandPass,
}

#[derive(Debug)]
struct MushroomState {
    rng: Rng,
    phase_low: f32,
    phase_high: f32,
    band: BandPass,
}

#[derive(Debug)]
struct LushState {
    rng: Rng,
    until_burst: f32,
    band: BandPass,
}

#[derive(Debug)]
struct DripstoneState {
    rng: Rng,
    until_drip: f32,
    drip_env: f32,
    rumble: LowPass,
}

#[derive(Debug)]
struct DeepDarkState {
    rng: Rng,
    phase: f32,
    until_scuttle: f32,
    scuttle: Option<Scuttle>,
}

#[derive(Debug)]
struct Scuttle {
    length: usize,
    at: usize,
    band: BandPass,
}

#[derive(Debug)]
struct SnowState {
    rng: Rng,
    band: BandPass,
    lpf: LowPass,
    /// The cutoff we last set on the LPF. If the scene changes, retune.
    lpf_cutoff: f32,
}

#[derive(Debug)]
struct MangroveState {
    rng: Rng,
    band: BandPass,
}

impl Biome {
    pub fn new(seed: u64, sample_rate: u32) -> Self {
        let mut s = Self {
            seed,
            sample_rate,
            scene: BiomeScene {
                kind: BiomeKind::Desert,
                intensity: 0.0,
                sub: 0.0,
                gain: 0.0,
            },
            state: BiomeState::Desert(DesertState::new(seed, sample_rate)),
            outgoing: None,
            xfade_left: 0,
            // Reserved here, never in render: the crossfade buffer's first resize is a kind
            // change, which happens on the audio thread, and the audio thread never allocates
            // (foley-ffi's `the_audio_thread_never_allocates` holds it to that). Any render's
            // crossfade slice is at most `CROSSFADE` samples, so this capacity always fits.
            scratch: Vec::with_capacity(CROSSFADE),
        };
        s.set_scene(BiomeScene {
            kind: BiomeKind::Desert,
            intensity: 0.0,
            sub: 0.0,
            gain: 0.0,
        });
        s
    }

    pub fn set_scene(&mut self, scene: BiomeScene) {
        if scene.kind != self.scene.kind {
            let fresh = match scene.kind {
                BiomeKind::Desert => {
                    BiomeState::Desert(DesertState::new(self.seed ^ 0xDE5, self.sample_rate))
                }
                BiomeKind::Mushroom => {
                    BiomeState::Mushroom(MushroomState::new(self.seed ^ 0xC1A, self.sample_rate))
                }
                BiomeKind::Lush => {
                    BiomeState::Lush(LushState::new(self.seed ^ 0x1A5, self.sample_rate))
                }
                BiomeKind::Dripstone => {
                    BiomeState::Dripstone(DripstoneState::new(self.seed ^ 0xD21, self.sample_rate))
                }
                BiomeKind::DeepDark => {
                    BiomeState::DeepDark(DeepDarkState::new(self.seed ^ 0xD4A, self.sample_rate))
                }
                BiomeKind::Snow => {
                    BiomeState::Snow(SnowState::new(self.seed ^ 0x5AD, self.sample_rate))
                }
                BiomeKind::Mangrove => {
                    BiomeState::Mangrove(MangroveState::new(self.seed ^ 0x4A0, self.sample_rate))
                }
            };
            let old = std::mem::replace(&mut self.state, fresh);
            // The state being replaced keeps sounding under the new one: a switch that rebuilt outright
            // cut every oscillator mid-tail, a hard step at the border heard as a stutter in the bed. Only
            // a state that was actually sounding has a tail to keep — a fresh bed's placeholder (never
            // heard, gain 0) gives way at once, so a bed starting does not crossfade from a phantom.
            if self.scene.gain > 0.0 {
                self.outgoing = Some(old);
                self.xfade_left = CROSSFADE;
            }
        }
        self.scene = scene;
    }

    pub fn render(&mut self, out: &mut [f32]) {
        let gain = self.scene.gain;
        if gain <= 0.0 {
            for s in out.iter_mut() {
                *s = 0.0;
            }
            return;
        }
        self.state.render(out, self.scene, gain, self.sample_rate);
        if self.xfade_left == 0 {
            return;
        }
        let Biome {
            outgoing,
            scratch,
            scene,
            xfade_left,
            sample_rate,
            ..
        } = self;
        let Some(old) = outgoing.as_mut() else {
            *xfade_left = 0;
            return;
        };
        let n = out.len().min(*xfade_left);
        if scratch.len() < n {
            scratch.resize(n, 0.0);
        }
        let under = &mut scratch[..n];
        old.render(under, *scene, gain, *sample_rate);
        for (i, sample) in out[..n].iter_mut().enumerate() {
            // The new kind fades up over the old one's fade down: whole old at
            // the switch, whole new at the end of the crossfade.
            let progress = (CROSSFADE - *xfade_left + i + 1) as f32 / CROSSFADE as f32;
            *sample = *sample * progress + under[i] * (1.0 - progress);
        }
        *xfade_left -= n;
        if *xfade_left == 0 {
            *outgoing = None;
        }
    }
}

impl DesertState {
    fn new(seed: u64, sample_rate: u32) -> Self {
        Self {
            rng: Rng::new(seed),
            until_grain: 0.0,
            until_vortex: 0.0,
            vortex: None,
            whistle: BandPass::new_at(1500.0, 8.0, sample_rate),
        }
    }

    fn render(&mut self, out: &mut [f32], scene: BiomeScene, gain: f32, sample_rate: u32) {
        let rate = sample_rate as f32;
        let intensity = scene.intensity.clamp(0.0, 1.0);
        let grain_rate = (200.0 * intensity).max(0.0);
        let mean_grain_dt = if grain_rate > 0.0 {
            rate / grain_rate
        } else {
            f32::INFINITY
        };
        let vortex_total = scene.sub.clamp(0.0, 10.0) as u32;
        let vortex_period = if vortex_total > 0 {
            30.0 * rate / vortex_total as f32
        } else {
            f32::INFINITY
        };
        let whistle = scene.sub > 0.0;
        let whistle_gain = 0.08 * intensity;
        for sample in out.iter_mut() {
            // Grain (granular hiss): a sample of noise every few ms, scaled by intensity.
            self.until_grain -= 1.0;
            let mut grain = 0.0f32;
            if grain_rate > 0.0 && self.until_grain <= 0.0 {
                self.until_grain += self.rng.exponential(mean_grain_dt);
                grain = 0.4 * intensity * self.rng.signed();
            }
            // Vortex ring.
            self.until_vortex -= 1.0;
            if vortex_total > 0 && self.vortex.is_none() && self.until_vortex <= 0.0 {
                self.until_vortex += vortex_period;
                self.vortex = Some(Vortex {
                    length: (1.5 * rate) as usize,
                    at: 0,
                    band: BandPass::new_at(200.0, 1.2, sample_rate),
                });
            }
            let mut vortex_s = 0.0f32;
            if let Some(v) = &mut self.vortex {
                if v.at < v.length {
                    let t = v.at as f32 / v.length as f32;
                    let env = (t * 6.0).min(1.0) * (1.0 - t).powf(1.5);
                    let sweep = 0.7 + 0.6 * (TAU * 0.4 * t).sin();
                    vortex_s = env * sweep * v.band.process(self.rng.signed());
                    v.at += 1;
                } else {
                    self.vortex = None;
                }
            }
            // Open-plain whistle.
            let whistle_s = if whistle {
                whistle_gain * self.whistle.process(self.rng.signed())
            } else {
                0.0
            };
            *sample = ((grain + vortex_s + whistle_s) * gain).clamp(-1.0, 1.0);
        }
    }
}

impl MushroomState {
    fn new(seed: u64, sample_rate: u32) -> Self {
        Self {
            rng: Rng::new(seed),
            phase_low: 0.0,
            phase_high: 0.0,
            band: BandPass::new_at(300.0, 4.0, sample_rate),
        }
    }

    fn render(&mut self, out: &mut [f32], scene: BiomeScene, gain: f32, sample_rate: u32) {
        let rate = sample_rate as f32;
        let intensity = scene.intensity.clamp(0.0, 1.0);
        let tone_gain = 0.05 * intensity;
        let dt = 1.0 / rate;
        for sample in out.iter_mut() {
            self.phase_low += TAU * 120.0 * dt;
            self.phase_high += TAU * 192.0 * dt;
            let tone = 0.7 * self.phase_low.sin() + 0.3 * self.phase_high.sin();
            let air = 0.3 * self.band.process(self.rng.signed());
            *sample = (tone_gain * (tone + air) * gain).clamp(-1.0, 1.0);
        }
    }
}

impl LushState {
    fn new(seed: u64, sample_rate: u32) -> Self {
        Self {
            rng: Rng::new(seed),
            until_burst: 0.0,
            band: BandPass::new_at(4000.0, 0.7, sample_rate),
        }
    }

    fn render(&mut self, out: &mut [f32], scene: BiomeScene, gain: f32, sample_rate: u32) {
        let rate = sample_rate as f32;
        let intensity = scene.intensity.clamp(0.0, 1.0);
        let burst_period = 0.3 * rate;
        let burst_gain = 0.4 * intensity;
        for sample in out.iter_mut() {
            self.until_burst -= 1.0;
            let mut burst = 0.0f32;
            if self.until_burst <= 0.0 {
                self.until_burst += burst_period;
                burst = burst_gain * self.band.process(self.rng.signed());
            }
            let cont = 0.1 * intensity * self.band.process(self.rng.signed());
            *sample = ((burst + cont) * gain).clamp(-1.0, 1.0);
        }
    }
}

impl DripstoneState {
    fn new(seed: u64, sample_rate: u32) -> Self {
        Self {
            rng: Rng::new(seed),
            until_drip: 0.0,
            drip_env: 0.0,
            rumble: LowPass::new_at(80.0, sample_rate),
        }
    }

    fn render(&mut self, out: &mut [f32], scene: BiomeScene, gain: f32, sample_rate: u32) {
        let rate = sample_rate as f32;
        let intensity = scene.intensity.clamp(0.0, 1.0);
        let drip_period = 2.0 * rate;
        let rumble_gain = 0.05 * intensity;
        for sample in out.iter_mut() {
            self.until_drip -= 1.0;
            if self.until_drip <= 0.0 {
                self.until_drip += drip_period;
                self.drip_env = 1.0;
            }
            // Drip envelope: a sharp attack and a fast decay (~3 ms).
            self.drip_env *= 0.9995;
            if self.drip_env < 0.001 {
                self.drip_env = 0.0;
            }
            let drip_s = 0.6 * intensity * self.drip_env * self.rng.signed();
            let rumble_s = rumble_gain * self.rumble.process(self.rng.signed());
            *sample = ((drip_s + rumble_s) * gain).clamp(-1.0, 1.0);
        }
    }
}

impl DeepDarkState {
    fn new(seed: u64, _sample_rate: u32) -> Self {
        Self {
            rng: Rng::new(seed),
            phase: 0.0,
            until_scuttle: 0.0,
            scuttle: None,
        }
    }

    fn render(&mut self, out: &mut [f32], scene: BiomeScene, gain: f32, sample_rate: u32) {
        let rate = sample_rate as f32;
        let intensity = scene.intensity.clamp(0.0, 1.0);
        let hum_gain = 0.4 * intensity;
        let scuttle_total = (scene.sub.clamp(0.0, 2.0)) as u32;
        let scuttle_period = if scuttle_total > 0 {
            30.0 * rate / scuttle_total as f32
        } else {
            f32::INFINITY
        };
        let dt = 1.0 / rate;
        for sample in out.iter_mut() {
            self.phase += TAU * 30.0 * dt;
            let hum = hum_gain * self.phase.sin();
            // Scuttle.
            self.until_scuttle -= 1.0;
            if scuttle_total > 0 && self.scuttle.is_none() && self.until_scuttle <= 0.0 {
                self.until_scuttle += scuttle_period;
                self.scuttle = Some(Scuttle {
                    length: (0.4 * rate) as usize,
                    at: 0,
                    band: BandPass::new_at(2000.0, 0.5, sample_rate),
                });
            }
            let mut scuttle_s = 0.0f32;
            if let Some(sc) = &mut self.scuttle {
                if sc.at < sc.length {
                    let t = sc.at as f32 / sc.length as f32;
                    let env = (t * 12.0).min(1.0) * (1.0 - t);
                    scuttle_s = 0.3 * env * sc.band.process(self.rng.signed());
                    sc.at += 1;
                } else {
                    self.scuttle = None;
                }
            }
            *sample = ((hum + scuttle_s) * gain).clamp(-1.0, 1.0);
        }
    }
}

impl SnowState {
    fn new(seed: u64, sample_rate: u32) -> Self {
        Self {
            rng: Rng::new(seed),
            band: BandPass::new_at(2000.0, 0.5, sample_rate),
            lpf: LowPass::new_at(4000.0, sample_rate),
            lpf_cutoff: 4000.0,
        }
    }

    fn render(&mut self, out: &mut [f32], scene: BiomeScene, gain: f32, sample_rate: u32) {
        let intensity = scene.intensity.clamp(0.0, 1.0);
        // The LPF: 4 kHz at intensity 0, 2 kHz at intensity 1 (more absorption).
        let cutoff = 4000.0 - 2000.0 * intensity;
        if (self.lpf_cutoff - cutoff).abs() > 1.0 {
            self.lpf.tune_at(cutoff, sample_rate);
            self.lpf_cutoff = cutoff;
        }
        let texture_gain = 0.2 * intensity;
        for sample in out.iter_mut() {
            let s = self.band.process(self.rng.signed());
            let s = self.lpf.process(s);
            *sample = (texture_gain * s * gain).clamp(-1.0, 1.0);
        }
    }
}

impl MangroveState {
    fn new(seed: u64, sample_rate: u32) -> Self {
        Self {
            rng: Rng::new(seed),
            band: BandPass::new_at(4000.0, 0.6, sample_rate),
        }
    }

    fn render(&mut self, out: &mut [f32], scene: BiomeScene, gain: f32, _sample_rate: u32) {
        let intensity = scene.intensity.clamp(0.0, 1.0);
        let drone_gain = (0.005 * scene.sub.clamp(0.0, 200.0)).min(0.5) * intensity;
        for sample in out.iter_mut() {
            let s = self.band.process(self.rng.signed());
            *sample = (drone_gain * s * gain).clamp(-1.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foley_dsp::SAMPLE_RATE;

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt()
    }

    fn render(
        kind: BiomeKind,
        intensity: f32,
        sub: f32,
        gain: f32,
        seed: u64,
        blocks: usize,
    ) -> Vec<f32> {
        let mut bed = Biome::new(seed, SAMPLE_RATE);
        bed.set_scene(BiomeScene {
            kind,
            intensity,
            sub,
            gain,
        });
        let mut out = vec![0f32; blocks * 480];
        bed.render(&mut out);
        out
    }

    #[test]
    fn every_kind_renders_something_finite() {
        for k in [
            BiomeKind::Desert,
            BiomeKind::Mushroom,
            BiomeKind::Lush,
            BiomeKind::Dripstone,
            BiomeKind::DeepDark,
            BiomeKind::Snow,
            BiomeKind::Mangrove,
        ] {
            let out = render(k, 1.0, 50.0, 1.0, 1, 50);
            assert!(
                out.iter().all(|x| x.is_finite() && x.abs() <= 1.0),
                "{k:?} not bounded"
            );
            assert!(rms(&out) > 1e-5, "{k:?} silent (rms={})", rms(&out));
        }
    }

    #[test]
    fn desert_hiss_is_higher_frequency_than_deep_dark_drone() {
        let desert = render(BiomeKind::Desert, 1.0, 0.0, 1.0, 1, 1);
        let dark = render(BiomeKind::DeepDark, 1.0, 0.0, 1.0, 1, 1);
        let zc = |s: &[f32]| s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        let zd = zc(&desert);
        let zk = zc(&dark);
        assert!(zd > zk * 2, "desert {zd} dark {zk}");
    }

    #[test]
    fn same_seed_same_sound() {
        let a = render(BiomeKind::Mushroom, 0.5, 0.0, 1.0, 7, 1);
        let b = render(BiomeKind::Mushroom, 0.5, 0.0, 1.0, 7, 1);
        assert_eq!(a, b);
    }

    #[test]
    fn gain_zero_is_silent() {
        let out = render(BiomeKind::Snow, 1.0, 0.0, 0.0, 1, 5);
        assert!(out.iter().all(|x| *x == 0.0));
    }

    #[test]
    fn kind_from_index_round_trips() {
        for i in 0..=6 {
            let k = BiomeKind::from_index(i).unwrap();
            assert_eq!(k as u32, i);
        }
        assert!(BiomeKind::from_index(7).is_none());
    }

    /// A kind change must not cut the outgoing biome's tail. The bed
    /// crossfades: at the switch the new kind starts from nothing and the old
    /// keeps sounding through the fade, so the splice steps no more than the
    /// stream steps on its own, and the new kind's first samples are *not* the
    /// fresh bed's (which is what a rebuild from scratch produces).
    #[test]
    fn a_kind_change_crossfades_rather_than_cutting() {
        const WARM: usize = 480;
        const AFTER: usize = 2 * WARM + 960;
        let desert = BiomeScene {
            kind: BiomeKind::Desert,
            intensity: 1.0,
            sub: 0.0,
            gain: 1.0,
        };
        let mushroom = BiomeScene {
            kind: BiomeKind::Mushroom,
            intensity: 1.0,
            sub: 0.0,
            gain: 1.0,
        };
        // Two beds in lockstep: one switches, one keeps deserting.
        let mut switched = Biome::new(3, SAMPLE_RATE);
        let mut unswitched = Biome::new(3, SAMPLE_RATE);
        switched.set_scene(desert);
        unswitched.set_scene(desert);
        let mut warm_a = vec![0.0f32; WARM];
        let mut warm_b = vec![0.0f32; WARM];
        switched.render(&mut warm_a);
        unswitched.render(&mut warm_b);
        assert_eq!(warm_a, warm_b, "lockstep beds must agree before the switch");

        switched.set_scene(mushroom);
        let mut after_a = vec![0.0f32; AFTER];
        let mut after_b = vec![0.0f32; AFTER];
        switched.render(&mut after_a);
        unswitched.render(&mut after_b);

        // A third bed that is mushroom from scratch: what a state rebuild at
        // the switch hands out — the new kind restarted at its first sample.
        let mut fresh = Biome::new(3, SAMPLE_RATE);
        fresh.set_scene(mushroom);
        let mut fresh_out = vec![0.0f32; AFTER];
        fresh.render(&mut fresh_out);

        // 1) The new kind must not restart from scratch at the switch.
        assert_ne!(
            after_a, fresh_out,
            "a kind change restarted the new biome from nothing: the tail was cut"
        );

        // 2) The splice steps no more than the stream does on its own.
        let step = (after_a[0] - warm_a[WARM - 1]).abs();
        let natural = warm_a
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max);
        println!("splice step {step:.5}; the stream's own largest step {natural:.5}");
        assert!(
            step <= natural + 1e-4,
            "the switch stepped the output by {step}, over the stream's own {natural}"
        );

        // 3) The outgoing biome still sounds straight after the switch: for
        //    the first 10 ms the switched bed stays near the bed that never
        //    switched (the crossfade has barely begun).
        let window = after_a[..480.min(AFTER)].to_vec();
        let drift = window
            .iter()
            .zip(&after_b)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        let peak = after_b.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
        println!("first-10-ms drift from the unswitched bed {drift:.5}; desert peak {peak:.5}");
        assert!(
            drift <= 0.25 * peak + 1e-4,
            "the outgoing biome vanished at the switch: drift {drift} against a peak of {peak}"
        );
    }
}
