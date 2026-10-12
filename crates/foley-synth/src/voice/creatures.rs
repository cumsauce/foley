//! Mobs with no voice sound like what they are made of: a skeleton's bones,
//! a spider's stridulation, a creeper's fuse, a bat's squeaks.

use std::f32::consts::{PI, TAU};

use super::score::{Seg, UH, Voice, c, perform};
use crate::{BandPass, Bubble, LowPass, Rng, SAMPLE_RATE};

const GRAVITY: f32 = 9.81;

fn add_at(out: &mut Vec<f32>, at: usize, sound: &[f32]) {
    if out.len() < at + sound.len() {
        out.resize(at + sound.len(), 0.0);
    }
    for (o, s) in out[at..].iter_mut().zip(sound) {
        *o += s;
    }
}

/// Damped modes (frequency, decay a second, amplitude) and a contact click,
/// until they die away.
fn ring(modes: &[(f32, f32, f32)], click: f32, rng: &mut Rng) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let longest = modes
        .iter()
        .map(|m| 7.0 / m.1.max(1.0))
        .fold(0.004, f32::max)
        .min(1.5);
    let n = (longest * rate) as usize;
    let click_len = (0.000_4 * rate) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / rate;
            let mut s = 0.0;
            for &(f, decay, a) in modes {
                if f < 0.45 * rate {
                    s += a * (-decay * t).exp() * (TAU * f * t).sin();
                }
            }
            if i < click_len {
                s += click * rng.signed() * (1.0 - i as f32 / click_len as f32);
            }
            s
        })
        .collect()
}

/// A bone knocking on bone: a free bar's modes, f₁ = (22.37 / 2π)·(r/2)·√(E/ρ)/L²
/// for bone's E = 17 GPa and ρ = 1900 kg/m³, overtones at 2.756 and 5.404
/// times, loss factor 0.02.
fn bone(length: f32, radius: f32, amplitude: f32, rng: &mut Rng) -> Vec<f32> {
    let speed = (17e9f32 / 1900.0).sqrt();
    let f1 = 22.37 / TAU * (radius / 2.0) * speed / (length * length);
    let modes: Vec<(f32, f32, f32)> = [(1.0, 1.0), (2.756, 0.6), (5.404, 0.35)]
        .iter()
        .map(|&(ratio, gain)| {
            let f = f1 * ratio * (1.0 + 0.04 * rng.signed());
            (f, (PI * f * 0.02).max(60.0), amplitude * gain)
        })
        .collect();
    ring(&modes, 0.6 * amplitude, rng)
}

pub(super) enum Bones {
    Plain,
    Frosted,
    Mossy,
}

/// A skeleton: teeth chattering, bones knocking, and now and then a faint,
/// dry breath it has no lungs for.
pub(super) fn skeleton(rng: &mut Rng, call: Call3, kind: Bones) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let mut out = Vec::new();
    match call {
        Call3::Ambient => {
            let chatter = 4 + (rng.unit() * 6.0) as usize;
            let gap = 1.0 / (9.0 + 4.0 * rng.unit());
            for i in 0..chatter {
                let tooth = bone(0.03 + 0.02 * rng.unit(), 0.006, 0.3, rng);
                add_at(&mut out, ((0.05 + i as f32 * gap) * rate) as usize, &tooth);
            }
            for _ in 0..2 + (rng.unit() * 4.0) as usize {
                let knock = bone(
                    0.12 + 0.2 * rng.unit(),
                    0.006 + 0.006 * rng.unit(),
                    0.5,
                    rng,
                );
                add_at(&mut out, (0.6 * rng.unit() * rate) as usize, &knock);
            }
            if rng.unit() < 0.5 {
                let mut breath = Seg::new(0.5 + 0.3 * rng.unit()).vowels(&[(0.0, UH)]);
                breath.voicing = c(0.0);
                breath.breath = c(0.25);
                breath.fric = c(0.08);
                breath.fric_at = (0.2, 0.5);
                let sound = perform(
                    rng,
                    Voice {
                        length: 0.17,
                        f0: 100.0,
                    },
                    &[breath],
                );
                let quiet: Vec<f32> = sound.iter().map(|x| 0.4 * x).collect();
                add_at(&mut out, (0.1 * rate) as usize, &quiet);
            }
        }
        Call3::Hurt => {
            let big = bone(0.3, 0.012, 1.0, rng);
            add_at(&mut out, 0, &big);
            for _ in 0..8 + (rng.unit() * 8.0) as usize {
                let rattle = bone(
                    0.05 + 0.25 * rng.unit(),
                    0.005 + 0.007 * rng.unit(),
                    0.4 + 0.3 * rng.unit(),
                    rng,
                );
                add_at(
                    &mut out,
                    (0.25 * rng.unit().powi(2) * rate) as usize,
                    &rattle,
                );
            }
        }
        Call3::Death => {
            // Coming apart: every bone falls and bounces.
            for _ in 0..25 + (rng.unit() * 15.0) as usize {
                let length = 0.05 + 0.35 * rng.unit();
                let radius = 0.004 + 0.012 * rng.unit();
                let mut speed = (2.0 * GRAVITY * (0.2 + 0.8 * rng.unit())).sqrt();
                let mut at = (2.0 * speed / GRAVITY) * 0.5 * rng.unit() + 0.05 * rng.unit();
                let mut loud = 0.5 + 0.5 * rng.unit();
                while loud > 0.05 && at < 1.8 {
                    let hit = bone(length, radius, loud, rng);
                    add_at(&mut out, (at * rate) as usize, &hit);
                    speed *= 0.4;
                    at += 2.0 * speed / GRAVITY;
                    loud *= 0.6;
                }
            }
        }
    }
    match kind {
        Bones::Plain => {}
        Bones::Frosted => {
            // Rime on the bones: a few glassy ticks.
            for _ in 0..3 + (rng.unit() * 5.0) as usize {
                let f = 6000.0 + 4000.0 * rng.unit();
                let tick = ring(&[(f, 40.0, 0.12)], 0.05, rng);
                add_at(&mut out, (0.5 * rng.unit() * rate) as usize, &tick);
            }
        }
        Bones::Mossy => {
            // Damp and overgrown: duller knocks, and water dripping off.
            let mut lp = LowPass::new(2500.0);
            out.iter_mut().for_each(|x| *x = lp.process(*x));
            for _ in 0..1 + (rng.unit() * 2.0) as usize {
                let mut bubble = Bubble::new(0.001 + 0.002 * rng.unit(), 0.3);
                let mut drip = Vec::new();
                while !bubble.done() {
                    drip.push(bubble.sample());
                }
                add_at(&mut out, (0.6 * rng.unit() * rate) as usize, &drip);
            }
        }
    }
    out
}

/// Ambient, hurt or death, for the mobs here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Call3 {
    Ambient,
    Hurt,
    Death,
}

/// A spider: stridulation (a file of teeth scraped at hundreds a second,
/// ringing its chitin), mandibles ticking, and a hiss. `scale` raises it for
/// a smaller spider.
pub(super) fn spider(rng: &mut Rng, call: Call3, scale: f32) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let mut out = Vec::new();
    let chirp =
        |rng: &mut Rng, out: &mut Vec<f32>, at: f32, teeth_hz: f32, seconds: f32, loud: f32| {
            let f = (2500.0 + 2500.0 * rng.unit()) * scale;
            let tooth = ring(&[(f, 900.0, 1.0), (1.7 * f, 1300.0, 0.5)], 0.6, rng);
            let n = (seconds * teeth_hz) as usize;
            for i in 0..n {
                let envelope = (PI * i as f32 / n.max(1) as f32).sin();
                let scaled: Vec<f32> = tooth
                    .iter()
                    .map(|x| x * loud * envelope * (0.7 + 0.3 * rng.unit()))
                    .collect();
                add_at(out, ((at + i as f32 / teeth_hz) * rate) as usize, &scaled);
            }
        };
    let hiss = |rng: &mut Rng, out: &mut Vec<f32>, at: f32, seconds: f32, loud: f32| {
        let mut band = BandPass::new(4500.0 * scale.sqrt(), 0.8);
        let n = (seconds * rate) as usize;
        let sound: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / n as f32;
                let envelope = (t * 15.0).min(1.0) * (1.0 - t).powi(2);
                loud * envelope * band.process(rng.signed())
            })
            .collect();
        add_at(out, (at * rate) as usize, &sound);
    };
    match call {
        Call3::Ambient => {
            let mut at = 0.0;
            for _ in 0..2 + (rng.unit() * 4.0) as usize {
                let teeth = (300.0 + 400.0 * rng.unit()) * scale;
                let seconds = 0.05 + 0.07 * rng.unit();
                chirp(rng, &mut out, at, teeth, seconds, 0.5);
                at += seconds + 0.08 + 0.12 * rng.unit();
            }
            for _ in 0..2 + (rng.unit() * 4.0) as usize {
                let tick = ring(
                    &[((1200.0 + 800.0 * rng.unit()) * scale, 1500.0, 0.4)],
                    0.3,
                    rng,
                );
                add_at(&mut out, (at * rng.unit() * rate) as usize, &tick);
            }
            if rng.unit() < 0.4 {
                let seconds = 0.3 + 0.3 * rng.unit();
                hiss(rng, &mut out, 0.0, seconds, 0.25);
            }
        }
        Call3::Hurt => {
            hiss(rng, &mut out, 0.0, 0.25, 1.0);
            chirp(rng, &mut out, 0.02, 900.0 * scale, 0.15, 0.6);
        }
        Call3::Death => {
            hiss(rng, &mut out, 0.0, 0.6, 0.7);
            // The chitter slowing as it fails.
            let mut at = 0.1;
            let mut teeth = 700.0 * scale;
            for _ in 0..5 {
                chirp(rng, &mut out, at, teeth, 0.06, 0.4);
                at += 0.12 + 0.1 * rng.unit();
                teeth *= 0.75;
            }
            for _ in 0..3 + (rng.unit() * 3.0) as usize {
                let tap = ring(&[(200.0 + 200.0 * rng.unit(), 120.0, 0.6)], 0.4, rng);
                add_at(&mut out, ((0.3 + 0.7 * rng.unit()) * rate) as usize, &tap);
            }
        }
    }
    out
}

/// A creeper: its fuse, a turbulent hiss through a narrow opening that
/// swells over the 1.5 s before it goes off, fluttering, with sparks and a
/// faint rising whistle; or, struck, the crunch of its leafy body.
pub(super) fn creeper(rng: &mut Rng, call: CreeperCall) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    match call {
        CreeperCall::Primed => {
            let n = (1.6 * rate) as usize;
            let mut bands = [BandPass::new(3200.0, 2.0), BandPass::new(6000.0, 1.5)];
            let mut flutter = LowPass::new(10.0);
            let mut whistle = 0.0f32;
            let mut until_spark = 0.0f32;
            let mut spark = 0.0f32;
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let swell = 0.3 + 0.7 * t * t;
                    let wobble = 1.0 + 0.35 * flutter.process(rng.signed()) * 8.0;
                    let noise = rng.signed();
                    let hiss = bands[0].process(noise) + 0.6 * bands[1].process(noise);
                    whistle += TAU * (2800.0 + 800.0 * t) / rate;
                    until_spark -= 1.0 / rate;
                    if until_spark <= 0.0 {
                        spark = 0.4 + 0.6 * rng.unit();
                        until_spark = rng.exponential(20.0 + 40.0 * t);
                    }
                    spark *= 0.97;
                    let fizz = spark * rng.signed() * 0.4;
                    let fade = ((n - i) as f32 / (0.02 * rate)).min(1.0);
                    fade * swell * (wobble * hiss + 0.06 * t * whistle.sin() + fizz)
                })
                .collect()
        }
        CreeperCall::Hurt | CreeperCall::Death => {
            let seconds = if call == CreeperCall::Hurt { 0.2 } else { 0.9 };
            let n = (seconds * rate) as usize;
            let mut rustle = BandPass::new(4000.0, 1.0);
            let mut sigh = BandPass::new(3000.0, 1.5);
            let thud = ring(&[(110.0 + 40.0 * rng.unit(), 30.0, 1.0)], 0.5, rng);
            let mut out: Vec<f32> = (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let crunch = (-t * 8.0).exp() * rustle.process(rng.signed());
                    sigh.tune(3000.0 - 1500.0 * t, 1.5);
                    let deflate = if call == CreeperCall::Death {
                        (1.0 - t) * sigh.process(rng.signed()) * 0.6
                    } else {
                        0.0
                    };
                    crunch + deflate
                })
                .collect();
            add_at(
                &mut out,
                0,
                &thud.iter().map(|x| 0.5 * x).collect::<Vec<_>>(),
            );
            out
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CreeperCall {
    Primed,
    Hurt,
    Death,
}

/// A bat: squeaks (its audible social calls, sweeping down from about
/// 15 kHz to 9), in short trains, sometimes with its wings' flutter.
pub(super) fn bat(rng: &mut Rng, call: Call3) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let mut out = Vec::new();
    let squeak = |from: f32, to: f32, seconds: f32, harsh: f32, rng: &mut Rng| -> Vec<f32> {
        let n = (seconds * rate) as usize;
        let mut phase = 0.0f32;
        (0..n)
            .map(|i| {
                let t = i as f32 / n as f32;
                let f = from * (to / from).powf(t) * (1.0 + harsh * 0.05 * rng.signed());
                phase += TAU * f / rate;
                let envelope = (PI * t).sin().powf(0.7);
                envelope * (phase.sin() + 0.3 * (2.0 * phase).sin() + harsh * 0.4 * rng.signed())
            })
            .collect()
    };
    match call {
        Call3::Ambient => {
            let mut at = 0.0;
            for _ in 0..2 + (rng.unit() * 4.0) as usize {
                let s = squeak(
                    14_000.0 + 2000.0 * rng.unit(),
                    9000.0 + 2000.0 * rng.unit(),
                    0.008 + 0.012 * rng.unit(),
                    0.0,
                    rng,
                );
                add_at(&mut out, (at * rate) as usize, &s);
                at += 0.03 + 0.06 * rng.unit();
            }
            if rng.unit() < 0.3 {
                let mut lp = LowPass::new(800.0);
                let n = (0.3 * rate) as usize;
                let flutter: Vec<f32> = (0..n)
                    .map(|i| {
                        let beat = (TAU * 11.0 * i as f32 / rate).sin().max(0.0);
                        0.6 * beat * lp.process(rng.signed())
                    })
                    .collect();
                add_at(&mut out, 0, &flutter);
            }
        }
        Call3::Hurt => {
            for i in 0..2 {
                let s = squeak(12_000.0, 7000.0, 0.04, 0.6, rng);
                add_at(&mut out, (i as f32 * 0.06 * rate) as usize, &s);
            }
        }
        Call3::Death => {
            let s = squeak(12_000.0, 5500.0, 0.07, 0.4, rng);
            add_at(&mut out, 0, &s);
        }
    }
    out
}

/// Under water: muffled, with the breath going up as bubbles.
pub(super) fn underwater(rng: &mut Rng, voice: Vec<f32>) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let mut lp = [LowPass::new(700.0), LowPass::new(700.0)];
    let mut out: Vec<f32> = voice
        .iter()
        .map(|x| {
            let first = lp[0].process(*x);
            1.5 * lp[1].process(first)
        })
        .collect();
    let length = out.len() as f32 / rate;
    for _ in 0..10 + (rng.unit() * 15.0) as usize {
        let mut bubble = Bubble::new(0.002 + 0.004 * rng.unit(), 0.25);
        let mut sound = Vec::new();
        while !bubble.done() {
            sound.push(bubble.sample());
        }
        add_at(&mut out, (length * rng.unit() * rate) as usize, &sound);
    }
    out
}

/// A wet throat: a few bubbles bursting in it as it groans.
pub(super) fn gurgle(rng: &mut Rng, mut voice: Vec<f32>) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let length = voice.len() as f32 / rate;
    for _ in 0..4 + (rng.unit() * 6.0) as usize {
        let mut bubble = Bubble::new(0.003 + 0.004 * rng.unit(), 0.15);
        let mut sound = Vec::new();
        while !bubble.done() {
            sound.push(bubble.sample());
        }
        add_at(&mut voice, (length * rng.unit() * rate) as usize, &sound);
    }
    voice
}
