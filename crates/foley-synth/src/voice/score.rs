//! A call as a score: segments of a voice's controls over time, performed by
//! an LF source with nonlinear phenomena through a waveguide tract.

use std::f32::consts::TAU;

use super::lf;
use super::tract::Tract;
use crate::{LowPass, Rng};

/// A value over a segment, by keyframes at fractions 0 to 1 of it.
#[derive(Debug, Clone)]
pub(super) struct K(Vec<(f32, f32)>);

impl K {
    pub(super) fn at(&self, t: f32) -> f32 {
        let keys = &self.0;
        let i = keys.iter().position(|k| k.0 > t).unwrap_or(keys.len());
        if i == 0 {
            return keys[0].1;
        }
        if i == keys.len() {
            return keys[keys.len() - 1].1;
        }
        let (t0, a) = keys[i - 1];
        let (t1, b) = keys[i];
        a + (b - a) * (t - t0) / (t1 - t0).max(1e-6)
    }
}

/// Keyframes.
pub(super) fn k(points: &[(f32, f32)]) -> K {
    K(points.to_vec())
}

/// A constant.
pub(super) fn c(value: f32) -> K {
    K(vec![(0.0, value)])
}

/// Mouth shapes, measured on a 17 cm tract against Peterson and Barney's men's
/// formants (F1/F2: [i] 300/2310 Hz against 270/2290, [ɛ] 510/1820 against
/// 530/1840, [æ] 640/1690 against 660/1720, [ʌ] 730/1220 against 640/1190,
/// [ɑ] 720/1110 against 730/1090): where the tongue is (0 glottis to 1 lips), its height (as
/// Pink Trombone's diameter, 2.05 close to 3.5 open), and the lips' opening.
#[derive(Debug, Clone, Copy)]
pub(super) struct Vowel(pub f32, pub f32, pub f32);

pub(super) const AA: Vowel = Vowel(0.27, 2.7, 1.0); // ɑ, as in "father"
pub(super) const AE: Vowel = Vowel(0.53, 2.9, 1.0); // æ
pub(super) const EH: Vowel = Vowel(0.58, 2.55, 0.9); // ɛ
pub(super) const EE: Vowel = Vowel(0.64, 2.05, 0.8); // i
pub(super) const UH: Vowel = Vowel(0.35, 2.8, 0.85); // ʌ
pub(super) const SCHWA: Vowel = Vowel(0.43, 3.0, 0.75); // ə
pub(super) const OH: Vowel = Vowel(0.34, 2.4, 0.5); // o, lips rounded
pub(super) const OO: Vowel = Vowel(0.48, 2.1, 0.3); // u, lips rounded

/// One stretch of a call.
#[derive(Debug, Clone)]
pub(super) struct Seg {
    pub secs: f32,
    /// Silence after it (the tract rings on).
    pub gap: f32,
    /// Pitch against the voice's usual.
    pub f0: K,
    /// Voice quality, Rd: 0.3 pressed, 1 modal, 2.7 breathy.
    pub rd: K,
    pub loud: K,
    /// How voiced: 1 voiced, 0 breath alone.
    pub voicing: K,
    /// Breath noise at the glottis.
    pub breath: K,
    /// Mouth shape over time, by keyframes of vowels.
    pub mouth: Vec<(f32, Vowel)>,
    /// Lips over the vowels' own: 1 as the vowel has them, 0 shut.
    pub lips: K,
    pub velum: K,
    /// Turbulence at a constriction (place 0 to 1, diameter cm), and how
    /// much.
    pub fric: K,
    pub fric_at: (f32, f32),
    /// Subharmonics: every `sub_n`th pulse full, the others weaker by this.
    pub sub: K,
    pub sub_n: u32,
    /// Sidebands: amplitude modulation at `sideband_hz` (30–150 Hz is
    /// roughness) by this depth.
    pub sidebands: K,
    pub sideband_hz: f32,
    /// Deterministic chaos: bursts of jitter and shimmer, and pitch jumps.
    pub chaos: K,
    /// Creak (vocal fry): irregular pulses at 20–70 Hz, blended in.
    pub creak: K,
    pub jitter: f32,
    /// Voicing on the in-breath.
    pub ingressive: bool,
    /// Vibrato and tremolo: rate (Hz) and depth.
    pub tremolo: (f32, f32),
    /// Biphonation: a second, independent source (as a horse's whinny has,
    /// Briefer et al. 2015) at `bi_ratio` times the pitch, this loud.
    pub bi: K,
    pub bi_ratio: f32,
}

impl Seg {
    pub(super) fn new(secs: f32) -> Seg {
        Seg {
            secs,
            gap: 0.0,
            f0: c(1.0),
            rd: c(1.0),
            loud: k(&[(0.0, 0.0), (0.12, 1.0), (0.8, 0.9), (1.0, 0.0)]),
            voicing: c(1.0),
            breath: c(0.05),
            mouth: vec![(0.0, SCHWA)],
            lips: c(1.0),
            velum: c(0.0),
            fric: c(0.0),
            fric_at: (0.9, 0.2),
            sub: c(0.0),
            sub_n: 2,
            sidebands: c(0.0),
            sideband_hz: 70.0,
            chaos: c(0.0),
            creak: c(0.0),
            jitter: 0.01,
            ingressive: false,
            tremolo: (0.0, 0.0),
            bi: c(0.0),
            bi_ratio: 1.0,
        }
    }

    pub(super) fn f0(mut self, keys: &[(f32, f32)]) -> Seg {
        self.f0 = k(keys);
        self
    }

    pub(super) fn rd(mut self, rd: f32) -> Seg {
        self.rd = c(rd);
        self
    }

    pub(super) fn loud(mut self, keys: &[(f32, f32)]) -> Seg {
        self.loud = k(keys);
        self
    }

    pub(super) fn breath(mut self, breath: f32) -> Seg {
        self.breath = c(breath);
        self
    }

    pub(super) fn voicing(mut self, voicing: f32) -> Seg {
        self.voicing = c(voicing);
        self
    }

    pub(super) fn chaos(mut self, chaos: f32) -> Seg {
        self.chaos = c(chaos);
        self
    }

    pub(super) fn sub(mut self, sub: f32) -> Seg {
        self.sub = c(sub);
        self
    }

    pub(super) fn creak(mut self, keys: &[(f32, f32)]) -> Seg {
        self.creak = k(keys);
        self
    }

    /// Mouth shut, through the nose.
    pub(super) fn nasal(mut self) -> Seg {
        self.lips = c(0.0);
        self.velum = c(1.0);
        self
    }

    pub(super) fn lips(mut self, keys: &[(f32, f32)]) -> Seg {
        self.lips = k(keys);
        self
    }

    pub(super) fn velum(mut self, keys: &[(f32, f32)]) -> Seg {
        self.velum = k(keys);
        self
    }

    pub(super) fn fric(mut self, amount: f32, place: f32, width: f32) -> Seg {
        self.fric = c(amount);
        self.fric_at = (place, width);
        self
    }

    pub(super) fn inward(mut self) -> Seg {
        self.ingressive = true;
        self
    }

    pub(super) fn tremolo(mut self, rate: f32, depth: f32) -> Seg {
        self.tremolo = (rate, depth);
        self
    }

    pub(super) fn sidebands(mut self, depth: f32, hz: f32) -> Seg {
        self.sidebands = c(depth);
        self.sideband_hz = hz;
        self
    }

    pub(super) fn bi(mut self, level: f32, ratio: f32) -> Seg {
        self.bi = c(level);
        self.bi_ratio = ratio;
        self
    }

    pub(super) fn gap(mut self, gap: f32) -> Seg {
        self.gap = gap;
        self
    }

    pub(super) fn vowels(mut self, keys: &[(f32, Vowel)]) -> Seg {
        self.mouth = keys.to_vec();
        self
    }

    fn mouth_at(&self, t: f32) -> Vowel {
        let keys = &self.mouth;
        let i = keys.iter().position(|k| k.0 > t).unwrap_or(keys.len());
        if i == 0 {
            return keys[0].1;
        }
        if i == keys.len() {
            return keys[keys.len() - 1].1;
        }
        let (t0, a) = keys[i - 1];
        let (t1, b) = keys[i];
        let w = (t - t0) / (t1 - t0).max(1e-6);
        Vowel(
            a.0 + w * (b.0 - a.0),
            a.1 + w * (b.1 - a.1),
            a.2 + w * (b.2 - a.2),
        )
    }
}

/// A voice: its tract's length (m) and its usual pitch (Hz).
#[derive(Debug, Clone, Copy)]
pub(super) struct Voice {
    pub length: f32,
    pub f0: f32,
}

/// A rough normal deviate (the sum of three uniforms).
fn gauss(rng: &mut Rng) -> f32 {
    rng.signed() + rng.signed() + rng.signed()
}

/// Performs `segments` with `voice` at a caller-chosen sample rate. The source
/// is the LF model at each segment's Rd, its period set by the pitch with
/// jitter, its pulses scaled
/// by shimmer; subharmonics weaken (and lengthen) all but every `sub_n`th
/// pulse; sidebands modulate its amplitude; chaos adds bursts of jitter and
/// shimmer and occasional pitch jumps; creak pulls the rate down to 20–70 Hz,
/// irregularly. Ingressive voicing plays each pulse backwards, breathier and
/// quieter. Breath noise follows the glottal flow. Turbulence is injected at
/// a constriction. The tract moves towards each instant's mouth shape in a
/// few milliseconds.
pub(super) fn perform_at(
    rng: &mut Rng,
    voice: Voice,
    segments: &[Seg],
    sample_rate: u32,
) -> Vec<f32> {
    let rate = sample_rate as f32;
    let mut tract = Tract::new(voice.length);
    let mut out = Vec::new();
    let mut phase = 0.0f32;
    let mut bi_phase = 0.0f32;
    let mut scale = 1.0f32;
    let mut amp = 1.0f32;
    let mut pulse = 0u32;
    let mut jump = 1.0f32;
    let creak_hz = 25.0 + 40.0 * rng.unit();
    let mut breath_lp = [LowPass::new(4000.0), LowPass::new(6000.0)];
    let mut fric_lp = LowPass::new(1200.0);
    let mut time = 0.0f32;
    for seg in segments {
        let samples = (seg.secs * rate) as usize;
        let place = tract.index_of(seg.fric_at.0);
        for i in 0..samples {
            let t = i as f32 / samples.max(1) as f32;
            time += 1.0 / rate;
            if i % 32 == 0 {
                let m = seg.mouth_at(t);
                let fric = seg.fric.at(t);
                let constriction = (fric > 0.02).then_some(seg.fric_at);
                tract.shape(
                    m.0,
                    m.1,
                    m.2 * seg.lips.at(t),
                    seg.velum.at(t),
                    constriction,
                );
            }
            tract.settle(0.01);
            let chaos = seg.chaos.at(t);
            let creak = seg.creak.at(t);
            let mut f0 = voice.f0 * seg.f0.at(t) * scale * jump;
            if seg.tremolo.0 > 0.0 {
                f0 *= 1.0 + 0.05 * seg.tremolo.1 * (TAU * seg.tremolo.0 * time).sin();
            }
            f0 = f0 * (1.0 - creak) + creak_hz * creak;
            phase += f0 / rate;
            if phase >= 1.0 {
                phase -= 1.0;
                pulse += 1;
                let jitter = seg.jitter + 0.08 * chaos + 0.2 * creak;
                scale = (1.0 + jitter * gauss(rng)).clamp(0.5, 1.6);
                amp = (1.0 + (0.05 + 0.4 * chaos + 0.3 * creak) * gauss(rng)).max(0.0);
                let sub = seg.sub.at(t);
                if sub > 0.0 && !pulse.is_multiple_of(seg.sub_n.max(2)) {
                    amp *= 1.0 - sub;
                    scale *= 1.0 + 0.15 * sub;
                }
                if chaos > 0.3 && rng.unit() < 0.015 * chaos {
                    jump = if jump != 1.0 {
                        1.0
                    } else if rng.unit() < 0.5 {
                        1.35
                    } else {
                        0.78
                    };
                }
            }
            let mut rd = seg.rd.at(t);
            let mut gain = 1.0;
            let mut cycle = phase;
            if seg.ingressive {
                cycle = 1.0 - phase;
                rd += 0.5;
                gain = 0.6;
            }
            if creak > 0.0 {
                rd = rd * (1.0 - creak) + 0.4 * creak;
            }
            let (dg, flow) = lf::lookup(rd, cycle);
            let voicing = seg.voicing.at(t);
            let mut voiced = dg * amp * voicing * gain;
            let bi = seg.bi.at(t);
            if bi > 0.0 {
                bi_phase = (bi_phase + f0 * seg.bi_ratio / rate).fract();
                voiced += bi * lf::lookup(rd, bi_phase).0 * voicing * gain;
            }
            let sidebands = seg.sidebands.at(t);
            if sidebands > 0.0 {
                voiced *= 1.0 + sidebands * (TAU * seg.sideband_hz * time).sin();
            }
            let noise = rng.signed();
            let opening = if voicing > 0.1 { 0.2 + flow } else { 0.7 };
            // Breath noise sits well below the voice (about −10 dB at full breath).
            let first = breath_lp[0].process(noise);
            let coloured = breath_lp[1].process(first);
            let ingress = if seg.ingressive { 1.5 } else { 1.0 };
            let aspiration = 0.3 * coloured * seg.breath.at(t) * opening * ingress;
            let loud = seg.loud.at(t);
            let glottis = loud * (0.5 * voiced + aspiration);
            let fric = seg.fric.at(t);
            let turbulence = if fric > 0.0 {
                let n = rng.signed();
                Some((place, loud * fric * (n - fric_lp.process(n))))
            } else {
                None
            };
            out.push(tract.run(glottis, turbulence));
        }
        for _ in 0..(seg.gap * rate) as usize {
            time += 1.0 / rate;
            tract.settle(0.01);
            out.push(tract.run(0.0, None));
        }
    }
    out
}

/// Takes off the lowest rumble (below about 40 Hz) and scales to `peak`,
/// with short fades so it never clicks, at a caller-chosen sample rate.
pub(super) fn finish_at(mut out: Vec<f32>, peak: f32, sample_rate: u32) -> Vec<f32> {
    let mut low = LowPass::new_at(40.0, sample_rate);
    for x in &mut out {
        *x -= low.process(*x);
    }
    out.extend(std::iter::repeat_n(0.0, sample_rate as usize / 50));
    let loudest = out.iter().fold(0f32, |m, x| m.max(x.abs()));
    if loudest > 0.0 {
        out.iter_mut().for_each(|x| *x *= peak / loudest);
    }
    let fade = (sample_rate as usize / 500).min(out.len() / 2);
    let len = out.len();
    for i in 0..fade {
        let g = i as f32 / fade as f32;
        out[i] *= g;
        out[len - 1 - i] *= g;
    }
    out
}
