//! Real birds (M10b), each from its call as field recordings' spectrograms
//! show it: the pitch of every note and how it glides, the timing, the
//! timbre, made by the physics of the bird's voice. A songbird's syrinx and
//! its trachea, tuned to the note, give a nearly pure whistle; a dove coos
//! with its beak shut, through its inflated throat, a soft low tone; a crow's
//! caw is the syrinx driven hard and irregularly, its harmonics shaped by the
//! throat and beak like a vowel.

use std::f32::consts::TAU;

use crate::outdoors::{add_at, normalize};
use crate::{BandPass, LowPass, Rng, SAMPLE_RATE};

/// The birds Foley knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bird {
    /// American crow: a few loud, harsh caws.
    Crow,
    /// Mourning dove: a soft, low "coo-OO-oo, oo, oo".
    Dove,
    /// American robin: caroled phrases, "cheerily, cheer-up, cheerio".
    Robin,
    /// Northern cardinal: loud slurred whistles, "cheer cheer cheer,
    /// birdy birdy birdy".
    Cardinal,
    /// Black-capped chickadee: the "fee-bee" whistle, or its
    /// "chick-a-dee-dee-dee".
    Chickadee,
}

impl Bird {
    pub const ALL: [Bird; 5] = [
        Bird::Crow,
        Bird::Dove,
        Bird::Robin,
        Bird::Cardinal,
        Bird::Chickadee,
    ];

    /// By its index in [`Bird::ALL`], wrapping round.
    pub fn of(index: u32) -> Bird {
        Bird::ALL[index as usize % Bird::ALL.len()]
    }
}

/// One call or song of `bird`; each `seed` a different individual and
/// rendition.
pub fn bird_call(bird: Bird, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0xB12D);
    let out = match bird {
        Bird::Crow => crow(&mut rng),
        Bird::Dove => dove(&mut rng),
        Bird::Robin => robin(&mut rng),
        Bird::Cardinal => cardinal(&mut rng),
        Bird::Chickadee => chickadee(&mut rng),
    };
    let mut out = out;
    out.extend(std::iter::repeat_n(
        0.0,
        (0.05 * SAMPLE_RATE as f32) as usize,
    ));
    out
}

/// A pitch contour: (fraction of a note, Hz) points.
type Contour = Vec<(f32, f32)>;

/// The pitch at `t` (0 to 1 through a note) on `contour`, a list of
/// (fraction of the note, Hz) points, gliding between them in log frequency.
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

/// A whistled note, `seconds` long, gliding along `contour`: a sine with a
/// touch of its second harmonic (what the trachea lets through), a little
/// wavering, swelling in over `attack` seconds and out over `release`.
fn whistle(
    contour: &[(f32, f32)],
    seconds: f32,
    harmonic: f32,
    (attack, release): (f32, f32),
    rng: &mut Rng,
) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let n = (seconds * rate) as usize;
    let mut phase = 0.0f32;
    let mut waver = LowPass::new(25.0);
    let attack = (attack * rate).min(n as f32 / 2.0);
    let release = (release * rate).min(n as f32 / 2.0);
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let f = contour_at(contour, t) * (1.0 + 0.04 * waver.process(rng.signed()));
            phase = (phase + TAU * f / rate) % TAU;
            let rise = smooth((i as f32 / attack).min(1.0));
            let fall = smooth(((n - i) as f32 / release).min(1.0));
            rise * fall * (phase.sin() + harmonic * (2.0 * phase).sin())
        })
        .collect()
}

/// A raised-cosine step from 0 to 1, for envelopes that never click.
fn smooth(x: f32) -> f32 {
    0.5 - 0.5 * (std::f32::consts::PI * x).cos()
}

/// An American crow: two to five caws a third of a second long, a little
/// apart. Each is the syrinx driven hard: pulses at 450 to 650 Hz, jittery,
/// every other one weaker (the period doubling that makes it rasp), with
/// turbulent breath, through the throat and beak's resonances near 1.2, 2
/// and 3 kHz; its pitch sags at the end.
fn crow(rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let f0 = 450.0 + 200.0 * rng.unit();
    let tract = 0.9 + 0.2 * rng.unit();
    let caws = 2 + (rng.unit() * 4.0) as usize;
    let mut out = Vec::new();
    let mut at = 0.0f32;
    for c in 0..caws {
        let seconds = 0.28 + 0.14 * rng.unit();
        let n = (seconds * rate) as usize;
        let pitch = f0 * (1.0 + 0.04 * rng.signed());
        let mut formants = [
            BandPass::new(1200.0 * tract, 5.0),
            BandPass::new(2000.0 * tract, 6.0),
            BandPass::new(3100.0 * tract, 7.0),
        ];
        let mut breath = BandPass::new(1800.0 * tract, 1.2);
        let mut caw = Vec::with_capacity(n);
        let mut until = 0.0f32;
        let mut odd = false;
        let mut strength = 1.0;
        let mut pulse = 0.0f32;
        for i in 0..n {
            let t = i as f32 / n as f32;
            let f = pitch * (1.0 + 0.06 * (t * 5.0).min(1.0) - 0.2 * t * t);
            until -= 1.0;
            if until <= 0.0 {
                odd = !odd;
                until += rate / f * (1.0 + 0.04 * rng.signed());
                strength = (0.75 + 0.25 * rng.unit()) * if odd { 1.0 } else { 0.55 };
                pulse = strength;
            }
            // Each pulse a sharp opening that closes over a fraction of a
            // millisecond.
            let excitation = pulse + 0.04 * strength * rng.signed();
            pulse *= 0.6;
            let envelope = smooth((i as f32 / (0.02 * rate)).min(1.0))
                * smooth(((n - i) as f32 / (0.06 * rate)).min(1.0))
                * (1.0 - 0.3 * t);
            let voiced = 1.0 * formants[0].process(excitation)
                + 0.8 * formants[1].process(excitation)
                + 0.4 * formants[2].process(excitation);
            caw.push(envelope * (voiced + 0.04 * breath.process(rng.signed())));
        }
        add_at(&mut out, (at * rate) as usize, &caw, 1.0 - 0.1 * c as f32);
        at += seconds + 0.2 + 0.2 * rng.unit();
    }
    normalize(out, 0.5)
}

/// A mourning dove's perch-coo: "coo-OO-oo", the second part swooping up
/// and down, then two or three lower "oo"s, each a soft, nearly pure tone
/// near 500 Hz, swelling in slowly through the shut beak, with a breath of
/// air in it.
fn dove(rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let f = 470.0 + 90.0 * rng.unit();
    // (start, length, contour as fractions of f).
    let mut notes: Vec<(f32, f32, Contour)> = vec![
        (
            0.0,
            0.6,
            vec![
                (0.0, 0.86),
                (0.25, 0.92),
                (0.45, 1.14),
                (0.7, 1.08),
                (1.0, 0.98),
            ],
        ),
        (0.85, 0.55, vec![(0.0, 0.98), (1.0, 0.92)]),
        (1.65, 0.5, vec![(0.0, 0.95), (1.0, 0.9)]),
    ];
    if rng.unit() < 0.7 {
        notes.push((2.4, 0.5, vec![(0.0, 0.94), (1.0, 0.88)]));
    }
    let mut out = Vec::new();
    let mut breath = BandPass::new(f, 4.0);
    for (start, length, contour) in notes {
        let contour: Vec<(f32, f32)> = contour.iter().map(|&(t, k)| (t, k * f)).collect();
        let mut note = whistle(
            &contour,
            length * (1.0 + 0.05 * rng.signed()),
            0.1,
            (0.08, 0.15),
            rng,
        );
        for (i, s) in note.iter_mut().enumerate() {
            let t = i as f32 / (length * rate);
            *s += 0.12 * s.abs() * breath.process(rng.signed()) * (1.0 - t).max(0.0);
        }
        add_at(&mut out, (start * rate) as usize, &note, 1.0);
    }
    normalize(out, 0.3)
}

/// An American robin: three to six caroled phrases of two or three notes,
/// each note a quick slur between 1.8 and 3.4 kHz; a few kinds of phrase per bird, never the same twice running.
fn robin(rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let low = 1800.0 + 300.0 * rng.unit();
    let high = low * (1.5 + 0.2 * rng.unit());
    let pick = |rng: &mut Rng| low * (high / low).powf(rng.unit());
    let kinds: Vec<Vec<Vec<(f32, f32)>>> = (0..3 + (rng.unit() * 2.0) as usize)
        .map(|_| {
            (0..2 + (rng.unit() * 2.0) as usize)
                .map(|_| {
                    let (a, b, c) = (pick(rng), pick(rng), pick(rng));
                    // Mostly a chevron, up then down.
                    vec![(0.0, a.min(b)), (0.35, a.max(b).max(c)), (1.0, c.min(a))]
                })
                .collect()
        })
        .collect();
    let phrases = 3 + (rng.unit() * 4.0) as usize;
    let mut out = Vec::new();
    let mut at = 0.0f32;
    let mut last = usize::MAX;
    for _ in 0..phrases {
        let mut kind = (rng.unit() * kinds.len() as f32) as usize;
        if kind == last {
            kind = (kind + 1) % kinds.len();
        }
        last = kind;
        for contour in &kinds[kind] {
            let seconds = 0.09 + 0.05 * rng.unit();
            let note = whistle(contour, seconds, 0.2, (0.01, 0.025), rng);
            add_at(
                &mut out,
                (at * rate) as usize,
                &note,
                0.8 + 0.2 * rng.unit(),
            );
            at += seconds + 0.03 + 0.02 * rng.unit();
        }
        at += 0.25 + 0.25 * rng.unit();
    }
    normalize(out, 0.35)
}

/// A northern cardinal: three to six loud slurred whistles ("cheer", down
/// from 4 to 2 kHz, or "wheet", up), then often a quick run of "birdy"s.
fn cardinal(rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let k = 0.92 + 0.16 * rng.unit();
    let cheer: Vec<(f32, f32)> = if rng.unit() < 0.65 {
        vec![(0.0, 3900.0 * k), (0.15, 4300.0 * k), (1.0, 1900.0 * k)]
    } else {
        vec![(0.0, 1800.0 * k), (0.8, 3600.0 * k), (1.0, 3400.0 * k)]
    };
    let mut out = Vec::new();
    let mut at = 0.0f32;
    let repeats = 3 + (rng.unit() * 4.0) as usize;
    let seconds = 0.22 + 0.08 * rng.unit();
    for _ in 0..repeats {
        let note = whistle(&cheer, seconds, 0.05, (0.015, 0.04), rng);
        add_at(&mut out, (at * rate) as usize, &note, 1.0);
        at += seconds + 0.08 + 0.05 * rng.unit();
    }
    if rng.unit() < 0.6 {
        let birdy = [(0.0, 2200.0 * k), (0.6, 3900.0 * k), (1.0, 3300.0 * k)];
        for _ in 0..5 + (rng.unit() * 5.0) as usize {
            let note = whistle(&birdy, 0.07, 0.05, (0.008, 0.015), rng);
            add_at(&mut out, (at * rate) as usize, &note, 0.85);
            at += 0.11;
        }
    }
    normalize(out, 0.4)
}

/// A black-capped chickadee. Mostly its song, "fee-bee": two pure whistles,
/// the second a whole tone lower, near 4 and 3.5 kHz. Otherwise its call,
/// "chick-a-dee-dee-dee": two quick high notes, then buzzy "dee"s, each a
/// stack of harmonics of about 450 Hz, loudest near 3.5 kHz.
fn chickadee(rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let k = 0.95 + 0.1 * rng.unit();
    let mut out = Vec::new();
    if rng.unit() < 0.6 {
        let fee = whistle(
            &[(0.0, 4000.0 * k), (1.0, 3900.0 * k)],
            0.35,
            0.02,
            (0.03, 0.05),
            rng,
        );
        let bee = whistle(
            &[
                (0.0, 3500.0 * k),
                (0.15, 3350.0 * k),
                (0.3, 3480.0 * k),
                (1.0, 3400.0 * k),
            ],
            0.32,
            0.02,
            (0.02, 0.06),
            rng,
        );
        add_at(&mut out, 0, &fee, 1.0);
        add_at(&mut out, (0.47 * rate) as usize, &bee, 0.8);
        return normalize(out, 0.3);
    }
    let chick = whistle(
        &[(0.0, 7000.0 * k), (1.0, 4500.0 * k)],
        0.04,
        0.0,
        (0.005, 0.01),
        rng,
    );
    let a = whistle(
        &[(0.0, 4200.0 * k), (1.0, 3200.0 * k)],
        0.035,
        0.0,
        (0.005, 0.01),
        rng,
    );
    add_at(&mut out, 0, &chick, 0.8);
    add_at(&mut out, (0.07 * rate) as usize, &a, 0.6);
    let mut at = 0.14f32;
    let f0 = 430.0 + 50.0 * rng.unit();
    for _ in 0..2 + (rng.unit() * 4.0) as usize {
        let n = (0.13 * rate) as usize;
        let mut dee = vec![0.0f32; n];
        for h in 1..=16 {
            let f = f0 * h as f32;
            let weight = (-((f - 3500.0 * k) / 900.0).powi(2)).exp();
            if weight < 0.01 {
                continue;
            }
            let start = rng.unit() * TAU;
            for (i, s) in dee.iter_mut().enumerate() {
                *s += weight * (start + TAU * f * i as f32 / rate).sin();
            }
        }
        for (i, s) in dee.iter_mut().enumerate() {
            *s *= smooth((i as f32 / (0.015 * rate)).min(1.0))
                * smooth(((n - i) as f32 / (0.03 * rate)).min(1.0));
        }
        add_at(&mut out, (at * rate) as usize, &dee, 0.35);
        at += 0.17 + 0.03 * rng.unit();
    }
    normalize(out, 0.3)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pitch round the loudest moment, from zero crossings.
    fn pitch(s: &[f32]) -> f32 {
        let loudest = s
            .iter()
            .enumerate()
            .fold(
                (0, 0f32),
                |m, (i, x)| if x.abs() > m.1 { (i, x.abs()) } else { m },
            )
            .0;
        let window = &s[loudest.saturating_sub(480)..(loudest + 480).min(s.len())];
        window
            .windows(2)
            .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
            .count() as f32
            / (window.len() as f32 / SAMPLE_RATE as f32)
    }

    #[test]
    fn every_bird_is_finite_bounded_and_short() {
        for bird in Bird::ALL {
            for seed in 0..6 {
                let s = bird_call(bird, seed);
                assert!(
                    s.iter().all(|x| x.is_finite() && x.abs() <= 0.5),
                    "{bird:?}"
                );
                assert!(s.len() > SAMPLE_RATE as usize / 4, "{bird:?}");
                assert!(s.len() < 6 * SAMPLE_RATE as usize, "{bird:?} {}", s.len());
            }
        }
    }

    #[test]
    fn a_dove_coos_low_and_songbirds_whistle_high() {
        let dove = pitch(&bird_call(Bird::Dove, 1));
        assert!((380.0..700.0).contains(&dove), "dove {dove} Hz");
        let cardinal = pitch(&bird_call(Bird::Cardinal, 1));
        assert!(
            (1500.0..5000.0).contains(&cardinal),
            "cardinal {cardinal} Hz"
        );
        let robin = pitch(&bird_call(Bird::Robin, 1));
        assert!((1500.0..4500.0).contains(&robin), "robin {robin} Hz");
    }
}
