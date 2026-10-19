//! Engine 2 vocabulary (M13): the primitives the new species catalogue is
//! expressed in. Each is a small, named construction that starts from
//! [`crate::whistle`]/[`crate::harsh`]-style physics and adds one feature:
//! frequency modulation, amplitude-modulated trills, inhaled broadband chips
//! and Poisson-of-them rattles, low-pitch croaks, long hoots with slow decay,
//! and a `drum(rate, decay)` helper. A [`Phrase`] composes segments into a
//! sequence the [`crate::birdsong2`] entry point walks.
//!
//! The original nine species in [`super::Bird`] are routed through these
//! primitives in [`super::bird_call`]; the migration is hand-checked against
//! the engine-1 version with the correlation test in [`super::tests`].

use std::f32::consts::TAU;

use crate::dsp::{add_at, normalize};
use crate::{BandPass, LowPass, Rng};

/// Frequency-modulated whistled note. A sinusoid at `contour(t)` Hz, with a
/// sinusoidal modulator on top of depth `fm_depth_hz` at rate `fm_rate_hz`
/// Hz. The basis of every warble: wood thrush's middle phrase, bobolink's
/// burble.
///
/// `harmonic` weights the second harmonic the trachea lets through (the
/// whistle's standard brightness); `(attack, release)` are seconds.
// Eight arguments, not seven: the sample-rate work gave every primitive its
// rate, and this one was already at the limit. They are the physics of a
// note rather than a queue waiting to become a struct.
#[allow(clippy::too_many_arguments)]
pub fn fm(
    contour: &[(f32, f32)],
    fm_depth_hz: f32,
    fm_rate_hz: f32,
    seconds: f32,
    harmonic: f32,
    (attack, release): (f32, f32),
    rng: &mut Rng,
    sample_rate: u32,
) -> Vec<f32> {
    let rate = sample_rate as f32;
    let n = (seconds * rate) as usize;
    let mut phase = 0.0f32;
    let mut waver = LowPass::new_at(25.0, sample_rate);
    let attack = (attack * rate).min(n as f32 / 2.0);
    let release = (release * rate).min(n as f32 / 2.0);
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let f0 = contour_at(contour, t) * (1.0 + 0.04 * waver.process(rng.signed()));
            let mod_ = (TAU * fm_rate_hz * i as f32 / rate).sin();
            let f = f0 + fm_depth_hz * mod_;
            phase = (phase + TAU * f / rate) % TAU;
            let rise = smooth((i as f32 / attack).min(1.0));
            let fall = smooth(((n - i) as f32 / release).min(1.0));
            rise * fall * (phase.sin() + harmonic * (2.0 * phase).sin())
        })
        .collect()
}

/// An amplitude-modulated trill: a sine at `base_hz` summed with another at
/// `base_hz + sweep_hz`, with a `0.5 + 0.5·sin(2π·am_hz·t)` envelope. The
/// hermit thrush's opening spiral and the pine warbler's run sit on this.
pub fn trill(
    base_hz: f32,
    sweep_hz: f32,
    seconds: f32,
    am_hz: f32,
    rng: &mut Rng,
    sample_rate: u32,
) -> Vec<f32> {
    let rate = sample_rate as f32;
    let n = (seconds * rate) as usize;
    let mut phase1 = 0.0f32;
    let mut phase2 = 0.0f32;
    let mut waver = LowPass::new_at(20.0, sample_rate);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / rate;
        let wobble = 1.0 + 0.02 * waver.process(rng.signed());
        phase1 = (phase1 + TAU * base_hz * wobble / rate) % TAU;
        phase2 = (phase2 + TAU * (base_hz + sweep_hz) * wobble / rate) % TAU;
        let am = 0.5 + 0.5 * (TAU * am_hz * t).sin();
        out.push(am * (phase1.sin() + 0.5 * phase2.sin()));
    }
    out
}

/// A short broadband burst at `centre_hz` with quality `q`: 5 to 10 ms of
/// band-passed noise, exp attack and release. The basis of inhaled chips and
/// rattles.
pub fn chip(centre_hz: f32, q: f32, rng: &mut Rng, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    let dur_ms = 5.0 + 5.0 * rng.unit();
    let n = ((dur_ms / 1000.0) * rate) as usize;
    let mut band = BandPass::new_at(centre_hz * (1.0 + 0.04 * rng.signed()), q, sample_rate);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / n as f32;
        let env = smooth((t * 10.0).min(1.0)) * (-4.0 * t).exp();
        out.push(env * band.process(rng.signed()));
    }
    out
}

/// A Poisson of [`chip`] calls at `rate_hz` for `seconds`, each at a slightly
/// perturbed `centre_hz`. Used for snow bunting's rattle and varied thrush's
/// second note.
pub fn rattle(
    centre_hz: f32,
    rate_hz: f32,
    seconds: f32,
    rng: &mut Rng,
    sample_rate: u32,
) -> Vec<f32> {
    let rate = sample_rate as f32;
    let total = (seconds * rate) as usize;
    let mut out = vec![0.0f32; total];
    let mut t = 0.0f32;
    while t < seconds {
        let center = centre_hz * (1.0 + 0.05 * rng.signed());
        let burst = chip(center, 1.0, rng, sample_rate);
        let at = (t * rate) as usize;
        let gain = 0.6 + 0.4 * rng.unit();
        add_at(&mut out, at, &burst, gain);
        t += rng.exponential(rate_hz);
    }
    out
}

/// A croak/rasp: `pulse_count` short, low-pitch pulses with strong breath
/// noise — hornbills, guan, roadrunner's clatter.
pub fn croak(
    pulse_count: usize,
    base_hz: f32,
    breath_db: f32,
    rng: &mut Rng,
    sample_rate: u32,
) -> Vec<f32> {
    let rate = sample_rate as f32;
    let mut out = Vec::new();
    let mut noise = BandPass::new_at(base_hz, 1.5, sample_rate);
    let breath = (breath_db / 20.0).exp();
    for _ in 0..pulse_count {
        let pulse_dur = 0.025 + 0.02 * rng.unit();
        let gap = 0.04 + 0.05 * rng.unit();
        let pulse_n = (pulse_dur * rate) as usize;
        for i in 0..pulse_n {
            let t = i as f32 / pulse_n as f32;
            let env = (t * 12.0).min(1.0) * (-3.0 * t).exp();
            let voiced = env * (TAU * base_hz * i as f32 / rate).sin();
            let breath = breath * env * noise.process(rng.signed());
            out.push(0.5 * voiced + 0.5 * breath);
        }
        // Gap: small noise tail.
        let gap_n = (gap * rate) as usize;
        for _ in 0..gap_n {
            out.push(0.05 * noise.process(rng.signed()));
        }
    }
    normalize(out, 0.5)
}

/// A long pure-tone pulse with a slow amplitude decay: boreal owl, great grey
/// owl, snowy owl.
pub fn hoot(centre_hz: f32, seconds: f32, rng: &mut Rng, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    let n = (seconds * rate) as usize;
    let mut phase = 0.0f32;
    let tau = seconds / 3.0;
    let attack = (0.04 * rate) as usize;
    let attack = attack.min(n / 2);
    let mut phase2 = 0.0f32;
    let mut waver = LowPass::new_at(8.0, sample_rate);
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let env = smooth((i as f32 / attack as f32).min(1.0)) * (-t / tau).exp();
            let f0 = centre_hz * (1.0 + 0.02 * waver.process(rng.signed()));
            phase = (phase + TAU * f0 / rate) % TAU;
            phase2 = (phase2 + TAU * 2.0 * f0 / rate) % TAU;
            env * (phase.sin() + 0.15 * phase2.sin())
        })
        .collect()
}

/// A drum: `decay_s` seconds of taps at `rate_hz` strikes/s, loudness falling
/// over the decay. Gila woodpecker's faster roll vs. the spruce grouse's
/// slower one.
pub fn drum(rate_hz: f32, decay_s: f32, rng: &mut Rng, sample_rate: u32) -> Vec<f32> {
    use crate::{Contact, Material, contact};
    let total = (decay_s * sample_rate as f32) as usize;
    let mut out = vec![0.0f32; total];
    let mut at = 0.0f32;
    let interval = 1.0 / rate_hz;
    let mut strike = 0u64;
    while at < decay_s {
        let tap = contact(Contact::Tap, Material::Wood, 6.0, strike, sample_rate);
        let env = (-4.0 * at / decay_s).exp();
        let sample = (at * sample_rate as f32) as usize;
        add_at(&mut out, sample, &tap, env);
        at += interval;
        strike += 1;
        let _ = rng; // rng deterministically seeded through `contact`'s seed
    }
    normalize(out, 0.4)
}

/// A segment kind: what kind of note to render. The [`Phrase`] composes a
/// list of these into a single sequence.
#[derive(Debug, Clone)]
pub enum SegmentKind {
    /// A whistled note along a [`crate::whistle`]-style contour, with attack and release.
    Whistle {
        contour: Vec<(f32, f32)>,
        attack: f32,
        release: f32,
        harmonic: f32,
    },
    /// A frequency-modulated note (see [`fm`]).
    Fm {
        contour: Vec<(f32, f32)>,
        depth_hz: f32,
        rate_hz: f32,
        attack: f32,
        release: f32,
    },
    /// A trill (see [`trill`]).
    Trill {
        base_hz: f32,
        sweep_hz: f32,
        am_hz: f32,
    },
    /// A single broadband chip (see [`chip`]).
    #[allow(dead_code)]
    Chip { centre_hz: f32, q: f32 },
    /// A rattle of chips (see [`rattle`]).
    Rattle { centre_hz: f32, rate_hz: f32 },
    /// A low croak (see [`croak`]).
    Croak {
        pulse_count: usize,
        base_hz: f32,
        breath_db: f32,
    },
    /// A long hoot (see [`hoot`]).
    Hoot { centre_hz: f32 },
    /// A drum roll (see [`drum`]).
    Drum { rate_hz: f32, decay_s: f32 },
}

/// A [`SegmentKind`] with how many seconds of audio it renders, plus the
/// gain to lay it into the phrase at.
#[derive(Debug, Clone)]
pub struct Segment {
    pub kind: SegmentKind,
    pub seconds: f32,
    pub gain: f32,
}

/// A composition: a list of [`Segment`]s rendered in sequence.
#[derive(Debug, Clone, Default)]
pub struct Phrase {
    pub segments: Vec<Segment>,
}

impl Phrase {
    pub fn new(segments: Vec<Segment>) -> Self {
        Phrase { segments }
    }

    /// Renders this phrase to PCM. Per-segment randomness is seeded from
    /// `seed`; the same `seed` reproduces the same call.
    pub fn render(&self, seed: u64, sample_rate: u32) -> Vec<f32> {
        let mut rng = Rng::new(seed);
        let mut out = Vec::new();
        let mut at = 0.0f32;
        for seg in &self.segments {
            let rendered = render_segment(&mut rng, seg, sample_rate);
            let sample_pos = (at * sample_rate as f32) as usize;
            add_at(&mut out, sample_pos, &rendered, seg.gain);
            at += seg.seconds;
        }
        if out.is_empty() {
            out.push(0.0);
        }
        out
    }
}

fn render_segment(rng: &mut Rng, seg: &Segment, sample_rate: u32) -> Vec<f32> {
    let seconds = seg.seconds;
    match &seg.kind {
        SegmentKind::Whistle {
            contour,
            attack,
            release,
            harmonic,
        } => crate::birds::whistle_internal(
            contour,
            seconds,
            *harmonic,
            (*attack, *release),
            rng,
            sample_rate,
        ),
        SegmentKind::Fm {
            contour,
            depth_hz,
            rate_hz,
            attack,
            release,
        } => fm(
            contour,
            *depth_hz,
            *rate_hz,
            seconds,
            0.1,
            (*attack, *release),
            rng,
            sample_rate,
        ),
        SegmentKind::Trill {
            base_hz,
            sweep_hz,
            am_hz,
        } => trill(*base_hz, *sweep_hz, seconds, *am_hz, rng, sample_rate),
        SegmentKind::Chip { centre_hz, q } => chip(*centre_hz, *q, rng, sample_rate),
        SegmentKind::Rattle { centre_hz, rate_hz } => {
            rattle(*centre_hz, *rate_hz, seconds, rng, sample_rate)
        }
        SegmentKind::Croak {
            pulse_count,
            base_hz,
            breath_db,
        } => croak(*pulse_count, *base_hz, *breath_db, rng, sample_rate),
        SegmentKind::Hoot { centre_hz } => hoot(*centre_hz, seconds, rng, sample_rate),
        SegmentKind::Drum { rate_hz, decay_s } => drum(*rate_hz, *decay_s, rng, sample_rate),
    }
}

/// The smoothed 0→1 →0 attack/release envelope used by [`crate::birds::whistle`].
fn smooth(x: f32) -> f32 {
    if x <= 0.0 {
        0.0
    } else if x >= 1.0 {
        1.0
    } else {
        x * x * (3.0 - 2.0 * x)
    }
}

/// The pitch at `t` (0 to 1 through a note) on a `(fraction, Hz)` contour,
/// gliding logarithmically.
fn contour_at(contour: &[(f32, f32)], t: f32) -> f32 {
    for pair in contour.windows(2) {
        let ((t0, f0), (t1, f1)) = (pair[0], pair[1]);
        if t <= t1 {
            let x = ((t - t0) / (t1 - t0).max(1e-6)).clamp(0.0, 1.0);
            return f0 * (f1 / f0).powf(x);
        }
    }
    contour.last().map_or(0.0, |p| p.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt()
    }

    #[test]
    fn fm_glides_and_modulates() {
        let out = fm(
            &[(0.0, 1000.0), (1.0, 2000.0)],
            200.0,
            12.0,
            0.4,
            0.05,
            (0.02, 0.05),
            &mut Rng::new(1),
            SAMPLE_RATE,
        );
        assert!(out.iter().all(|x| x.is_finite() && x.abs() <= 1.5));
        assert!(rms(&out) > 0.01);
        assert!(out.len() > 1000 && out.len() < 5 * SAMPLE_RATE as usize);
    }

    #[test]
    fn trill_is_rapidly_amplitude_modulated() {
        let out = trill(2500.0, 100.0, 0.4, 10.0, &mut Rng::new(2), SAMPLE_RATE);
        // 0.4s × 10Hz × 2 peaks each cycle = ~8 peaks
        let mut peaks = 0;
        for w in out.windows(80) {
            if w[40] > 0.0 && w[40] > w[20] && w[40] > w[60] {
                peaks += 1;
            }
        }
        assert!(peaks > 4, "{}", peaks);
    }

    #[test]
    fn chip_is_short_and_broadband() {
        let out = chip(5000.0, 1.0, &mut Rng::new(3), SAMPLE_RATE);
        assert!(out.len() < 800);
        assert!(out.iter().any(|x| x.abs() > 0.05));
    }

    #[test]
    fn rattle_has_many_chips() {
        let out = rattle(4000.0, 30.0, 0.3, &mut Rng::new(4), SAMPLE_RATE);
        // 30 Hz × 0.3 s = 9 chips expected. Detect distinct runs of sound
        // (a chip's amplitude > 0.1) separated by > 5 ms of quiet.
        let mut onsets = 0;
        let mut quiet = (SAMPLE_RATE / 200) as usize; // already quiet
        for s in &out {
            if s.abs() > 0.1 {
                if quiet > (SAMPLE_RATE / 200) as usize {
                    onsets += 1;
                }
                quiet = 0;
            } else {
                quiet += 1;
            }
        }
        assert!(
            (3..=15).contains(&onsets),
            "expected 3..=15 chips, got {}",
            onsets
        );
    }

    #[test]
    fn croak_is_low_and_breathy() {
        let out = croak(4, 400.0, 0.6, &mut Rng::new(5), SAMPLE_RATE);
        assert!(out.len() < SAMPLE_RATE as usize);
        assert!(rms(&out) > 0.01);
    }

    #[test]
    fn hoot_is_a_long_pure_tone() {
        let out = hoot(400.0, 1.0, &mut Rng::new(6), SAMPLE_RATE);
        assert!(rms(&out) > 0.1);
    }

    #[test]
    fn drum_strikes_at_the_given_rate() {
        let out = drum(20.0, 0.5, &mut Rng::new(7), SAMPLE_RATE);
        // ~10 strikes; count distinct onset peaks above 0.05 separated by
        // > 5 ms of quiet.
        let mut onsets = 0;
        let quiet_needed = (SAMPLE_RATE / 200) as usize;
        let mut quiet = quiet_needed; // start quiet
        for s in &out {
            if s.abs() > 0.05 {
                if quiet > quiet_needed {
                    onsets += 1;
                }
                quiet = 0;
            } else {
                quiet += 1;
            }
        }
        assert!(
            (3..=30).contains(&onsets),
            "expected 3..=30 strikes, got {}",
            onsets
        );
    }

    #[test]
    fn phrase_renders_its_segments_in_sequence() {
        let phrase = Phrase::new(vec![
            Segment {
                kind: SegmentKind::Whistle {
                    contour: vec![(0.0, 1000.0), (1.0, 1500.0)],
                    attack: 0.02,
                    release: 0.05,
                    harmonic: 0.1,
                },
                seconds: 0.15,
                gain: 0.8,
            },
            Segment {
                kind: SegmentKind::Trill {
                    base_hz: 2500.0,
                    sweep_hz: 100.0,
                    am_hz: 10.0,
                },
                seconds: 0.2,
                gain: 0.7,
            },
        ]);
        let out = phrase.render(8, SAMPLE_RATE);
        // Phrase has 0.15 + 0.2 = 0.35 s of audio at 48 kHz.
        assert!(out.len() > 5_000 && out.len() < (1.0 * SAMPLE_RATE as f32) as usize);
    }
}
