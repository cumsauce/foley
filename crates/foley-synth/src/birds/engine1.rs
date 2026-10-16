#![allow(dead_code, clippy::type_complexity)] // kept verbatim after fd68243 migrated the original 9 birds to engine 2
//! Engine 1 (M10b): the original 9 species' implementation, kept verbatim
//! for the correlation test in [`super::tests`] that pins engine 2's
//! migration to within an audible tolerance. The modern [`crate::bird_call`]
//! routes through [`engine2`-aware code][super]; this module exists only so
//! the test has the pre-M13 audio to compare against.
//!
//! Original module: `crates/foley-synth/src/birds.rs` at git revision
//! `84b30d9`, before the engine 2 vocabulary.

use std::f32::consts::TAU;

use crate::dsp::{add_at, normalize};
use crate::{BandPass, LowPass, Rng, SAMPLE_RATE};

/// Pitch at `t` (0 to 1 through a note) on `contour`, gliding logarithmically.
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

/// The smoothed 0→1 →0 attack/release envelope.
fn smooth(x: f32) -> f32 {
    if x <= 0.0 {
        0.0
    } else if x >= 1.0 {
        1.0
    } else {
        x * x * (3.0 - 2.0 * x)
    }
}

/// A whistled note: a sine with a touch of second harmonic, a little wavering,
/// swelling in over `attack` seconds and out over `release`.
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

/// A harsh note: the syrinx driven hard, jittery pulses with period doubling.
fn harsh(
    rng: &mut Rng,
    seconds: f32,
    pitch: impl Fn(f32) -> f32,
    formants: [f32; 3],
    doubling: f32,
) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let n = (seconds * rate) as usize;
    let mut filters = [
        BandPass::new(formants[0], 5.0),
        BandPass::new(formants[1], 6.0),
        BandPass::new(formants[2], 7.0),
    ];
    let mut breath = BandPass::new(formants[1], 1.2);
    let mut note = Vec::with_capacity(n);
    let (mut until, mut odd, mut strength, mut pulse) = (0.0f32, false, 1.0f32, 0.0f32);
    for i in 0..n {
        let t = i as f32 / n as f32;
        let f = pitch(t);
        until -= 1.0;
        if until <= 0.0 {
            odd = !odd;
            until += rate / f * (1.0 + 0.04 * rng.signed());
            strength = (0.75 + 0.25 * rng.unit()) * if odd { 1.0 } else { doubling };
            pulse = strength;
        }
        let excitation = pulse + 0.04 * strength * rng.signed();
        pulse *= 0.6;
        let envelope = smooth((i as f32 / (0.02 * rate)).min(1.0))
            * smooth(((n - i) as f32 / (0.06 * rate)).min(1.0))
            * (1.0 - 0.3 * t);
        let voiced = filters[0].process(excitation)
            + 0.8 * filters[1].process(excitation)
            + 0.4 * filters[2].process(excitation);
        note.push(envelope * (voiced + 0.04 * breath.process(rng.signed())));
    }
    note
}

fn crow(rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let f0 = 450.0 + 200.0 * rng.unit();
    let tract = 0.9 + 0.2 * rng.unit();
    let caws = 2 + (rng.unit() * 4.0) as usize;
    let mut out = Vec::new();
    let mut at = 0.0f32;
    for c in 0..caws {
        let seconds = 0.28 + 0.14 * rng.unit();
        let pitch = f0 * (1.0 + 0.04 * rng.signed());
        let caw = harsh(
            rng,
            seconds,
            |t| pitch * (1.0 + 0.06 * (t * 5.0).min(1.0) - 0.2 * t * t),
            [1200.0 * tract, 2000.0 * tract, 3100.0 * tract],
            0.55,
        );
        add_at(&mut out, (at * rate) as usize, &caw, 1.0 - 0.1 * c as f32);
        at += seconds + 0.2 + 0.2 * rng.unit();
    }
    normalize(out, 0.5)
}

fn gull(rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let f0 = 700.0 + 200.0 * rng.unit();
    let formants = [1700.0, 2900.0, 4200.0];
    let mut out = Vec::new();
    let mut at = 0.0f32;
    for _ in 0..1 + (rng.unit() * 2.0) as usize {
        let note = harsh(
            rng,
            0.38,
            |t| f0 * (1.0 + 0.5 * (std::f32::consts::PI * t).sin()),
            formants,
            0.85,
        );
        add_at(&mut out, (at * rate) as usize, &note, 1.0);
        at += 0.5;
    }
    for k in 0..4 + (rng.unit() * 6.0) as usize {
        let note = harsh(rng, 0.11, |t| f0 * 1.3 * (1.0 - 0.15 * t), formants, 0.85);
        add_at(&mut out, (at * rate) as usize, &note, 0.9 - 0.05 * k as f32);
        at += 0.16;
    }
    normalize(out, 0.45)
}

fn jay(rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let f0 = 950.0 + 250.0 * rng.unit();
    let mut out = Vec::new();
    let mut at = 0.0f32;
    for _ in 0..2 + (rng.unit() * 3.0) as usize {
        let note = harsh(
            rng,
            0.22,
            |t| f0 * (1.15 - 0.35 * t),
            [2500.0, 3600.0, 5000.0],
            0.7,
        );
        add_at(&mut out, (at * rate) as usize, &note, 1.0);
        at += 0.35 + 0.15 * rng.unit();
    }
    normalize(out, 0.45)
}

fn loon(rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let f = 580.0 + 80.0 * rng.unit();
    let mut out = Vec::new();
    let mut at = 0.0f32;
    let notes: [&[(f32, f32)]; 3] = [
        &[(0.0, 1.0), (0.3, 1.0), (0.5, 1.6), (1.0, 1.55)],
        &[(0.0, 1.55), (0.2, 1.75), (0.6, 1.7), (1.0, 1.5)],
        &[(0.0, 1.5), (0.5, 1.6), (1.0, 1.2)],
    ];
    for contour in &notes[..1 + (rng.unit() * 3.0) as usize] {
        let contour: Vec<(f32, f32)> = contour.iter().map(|&(t, k)| (t, k * f)).collect();
        let seconds = 1.6 + 0.6 * rng.unit();
        let note = whistle(&contour, seconds, 0.3, (0.15, 0.4), rng);
        add_at(&mut out, (at * rate) as usize, &note, 1.0);
        at += seconds + 0.3;
    }
    normalize(out, 0.35)
}

fn warbler(rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let k = 0.9 + 0.2 * rng.unit();
    let mut out = Vec::new();
    let mut at = 0.0f32;
    let notes = 5 + (rng.unit() * 4.0) as usize;
    for i in 0..notes {
        let last = i + 1 == notes;
        let contour = if last {
            vec![(0.0, 4000.0 * k), (1.0, 6800.0 * k)]
        } else {
            vec![(0.0, 6200.0 * k), (0.5, 4300.0 * k), (1.0, 5200.0 * k)]
        };
        let seconds = if last { 0.16 } else { 0.08 };
        let note = whistle(&contour, seconds, 0.05, (0.008, 0.02), rng);
        add_at(&mut out, (at * rate) as usize, &note, 1.0);
        at += seconds + 0.12 * (1.0 - 0.07 * i as f32);
    }
    normalize(out, 0.3)
}

fn dove(rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let f = 470.0 + 90.0 * rng.unit();
    let mut notes: Vec<(f32, f32, Vec<(f32, f32)>)> = vec![
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

/// The 9 species' calls under the original engine 1: same API as the
/// engine-2 [`super::bird_call`], kept verbatim for the correlation test.
pub fn bird_call(species: super::Bird, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0xB12D);
    let out = match species {
        super::Bird::Crow => crow(&mut rng),
        super::Bird::Dove => dove(&mut rng),
        super::Bird::Robin => robin(&mut rng),
        super::Bird::Cardinal => cardinal(&mut rng),
        super::Bird::Chickadee => chickadee(&mut rng),
        super::Bird::Gull => gull(&mut rng),
        super::Bird::Loon => loon(&mut rng),
        super::Bird::Jay => jay(&mut rng),
        super::Bird::Warbler => warbler(&mut rng),
    };
    let mut out = out;
    out.extend(std::iter::repeat_n(
        0.0,
        (0.05 * SAMPLE_RATE as f32) as usize,
    ));
    out
}
