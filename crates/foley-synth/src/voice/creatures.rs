//! Mobs with no voice sound like what they are made of: a skeleton's bones,
//! a spider's stridulation, a creeper's fuse, a bat's squeaks.

use std::f32::consts::{PI, TAU};

use super::score::{c, perform_at, Seg, Voice, UH};
use crate::{BandPass, Bubble, LowPass, Rng};

const GRAVITY: f32 = 9.81;

fn add_at(out: &mut Vec<f32>, at: usize, sound: &[f32]) {
    if out.len() < at + sound.len() {
        out.resize(at + sound.len(), 0.0);
    }
    for (o, s) in out[at..].iter_mut().zip(sound) {
        *o += s;
    }
}

/// Stick-slip friction: a creak, `hz` slips a second, wandering, rung
/// through a body that resonates at `body` Hz.
fn stick_slip(rng: &mut Rng, seconds: f32, hz: f32, body: f32, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    let n = (seconds * rate) as usize;
    let mut resonance = BandPass::new_at(body, 4.0, sample_rate);
    let mut phase = 0.0f32;
    let mut f = hz;
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            f *= 1.0 + 0.002 * rng.signed();
            phase += f * (0.8 + 0.4 * t) / rate;
            let slip = if phase >= 1.0 {
                phase -= 1.0;
                1.0 + 0.6 * rng.signed()
            } else {
                0.0
            };
            let envelope = (t * 10.0).min(1.0) * (1.0 - t).sqrt();
            4.0 * envelope * resonance.process(slip)
        })
        .collect()
}

/// Damped modes (frequency, decay a second, amplitude) and a contact click,
/// until they die away.
fn ring(modes: &[(f32, f32, f32)], click: f32, rng: &mut Rng, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
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
fn bone(length: f32, radius: f32, amplitude: f32, rng: &mut Rng, sample_rate: u32) -> Vec<f32> {
    let speed = (17e9f32 / 1900.0).sqrt();
    let f1 = 22.37 / TAU * (radius / 2.0) * speed / (length * length);
    let modes: Vec<(f32, f32, f32)> = [(1.0, 1.0), (2.756, 0.6), (5.404, 0.35)]
        .iter()
        .map(|&(ratio, gain)| {
            let f = f1 * ratio * (1.0 + 0.04 * rng.signed());
            (f, (PI * f * 0.02).max(60.0), amplitude * gain)
        })
        .collect();
    ring(&modes, 0.6 * amplitude, rng, sample_rate)
}

pub(super) enum Bones {
    Plain,
    Frosted,
    Mossy,
}

/// A skeleton: teeth chattering, bones knocking, and now and then a faint,
/// dry breath it has no lungs for.
pub(super) fn skeleton(rng: &mut Rng, call: Call3, kind: Bones, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    let mut out = Vec::new();
    match call {
        Call3::Ambient => {
            let chatter = 4 + (rng.unit() * 6.0) as usize;
            let gap = 1.0 / (9.0 + 4.0 * rng.unit());
            for i in 0..chatter {
                let tooth = bone(0.03 + 0.02 * rng.unit(), 0.006, 0.3, rng, sample_rate);
                add_at(&mut out, ((0.05 + i as f32 * gap) * rate) as usize, &tooth);
            }
            for _ in 0..2 + (rng.unit() * 4.0) as usize {
                let knock = bone(
                    0.12 + 0.2 * rng.unit(),
                    0.006 + 0.006 * rng.unit(),
                    0.5,
                    rng,
                    sample_rate,
                );
                add_at(&mut out, (0.6 * rng.unit() * rate) as usize, &knock);
            }
            if rng.unit() < 0.5 {
                let mut breath = Seg::new(0.5 + 0.3 * rng.unit()).vowels(&[(0.0, UH)]);
                breath.voicing = c(0.0);
                breath.breath = c(0.25);
                breath.fric = c(0.08);
                breath.fric_at = (0.2, 0.5);
                let sound = perform_at(
                    rng,
                    Voice {
                        length: 0.17,
                        f0: 100.0,
                    },
                    &[breath],
                    sample_rate,
                );
                let quiet: Vec<f32> = sound.iter().map(|x| 0.4 * x).collect();
                add_at(&mut out, (0.1 * rate) as usize, &quiet);
            }
            if rng.unit() < 0.35 {
                // Air through the ribs and the empty skull: a faint, hollow
                // whistle, like blowing across a bottle.
                let seconds = 0.6 + 0.6 * rng.unit();
                let n = (seconds * rate) as usize;
                let mut tube = BandPass::new_at(350.0 + 300.0 * rng.unit(), 12.0, sample_rate);
                let mut wander = LowPass::new_at(3.0, sample_rate);
                let hollow: Vec<f32> = (0..n)
                    .map(|i| {
                        let t = i as f32 / n as f32;
                        let swell =
                            (PI * t).sin().powi(2) * (1.0 + 3.0 * wander.process(rng.signed()));
                        0.5 * swell * tube.process(rng.signed())
                    })
                    .collect();
                add_at(&mut out, (0.2 * rng.unit() * rate) as usize, &hollow);
            }
            if rng.unit() < 0.3 {
                // Its bow, drawn a little: the string and limbs creaking.
                let (seconds, hz) = (0.25 + 0.2 * rng.unit(), 90.0 + 60.0 * rng.unit());
                let creak = stick_slip(rng, seconds, hz, 1600.0, sample_rate);
                let quiet: Vec<f32> = creak.iter().map(|x| 0.5 * x).collect();
                add_at(&mut out, (0.4 * rng.unit() * rate) as usize, &quiet);
            }
        }
        Call3::Hurt => {
            let big = bone(0.3, 0.012, 1.0, rng, sample_rate);
            add_at(&mut out, 0, &big);
            for _ in 0..8 + (rng.unit() * 8.0) as usize {
                let rattle = bone(
                    0.05 + 0.25 * rng.unit(),
                    0.005 + 0.007 * rng.unit(),
                    0.4 + 0.3 * rng.unit(),
                    rng,
                    sample_rate,
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
                    let hit = bone(length, radius, loud, rng, sample_rate);
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
                let tick = ring(&[(f, 40.0, 0.12)], 0.05, rng, sample_rate);
                add_at(&mut out, (0.5 * rng.unit() * rate) as usize, &tick);
            }
        }
        Bones::Mossy => {
            // Damp and overgrown: duller knocks, and water dripping off.
            let mut lp = LowPass::new_at(2500.0, sample_rate);
            out.iter_mut().for_each(|x| *x = lp.process(*x));
            for _ in 0..1 + (rng.unit() * 2.0) as usize {
                let mut bubble = Bubble::new_at(0.001 + 0.002 * rng.unit(), 0.3, sample_rate);
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
pub(super) fn spider(rng: &mut Rng, call: Call3, scale: f32, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    let mut out = Vec::new();
    let chirp =
        |rng: &mut Rng, out: &mut Vec<f32>, at: f32, teeth_hz: f32, seconds: f32, loud: f32| {
            let f = (2500.0 + 2500.0 * rng.unit()) * scale;
            let tooth = ring(
                &[(f, 900.0, 1.0), (1.7 * f, 1300.0, 0.5)],
                0.6,
                rng,
                sample_rate,
            );
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
                    sample_rate,
                );
                add_at(&mut out, (at * rng.unit() * rate) as usize, &tick);
            }
            if rng.unit() < 0.4 {
                let seconds = 0.3 + 0.3 * rng.unit();
                hiss(rng, &mut out, 0.0, seconds, 0.25);
            }
            // Eight feet: a quick, soft patter in fours, as it shifts its weight.
            if rng.unit() < 0.6 {
                let mut step_at = 0.1 + 0.4 * rng.unit();
                for _ in 0..1 + (rng.unit() * 3.0) as usize {
                    for foot in 0..4 {
                        let f = (350.0 + 300.0 * rng.unit()) / scale.sqrt();
                        let tap = ring(
                            &[(f, 250.0, 0.25), (2.3 * f, 400.0, 0.1)],
                            0.2,
                            rng,
                            sample_rate,
                        );
                        let at = step_at + foot as f32 * (0.018 + 0.012 * rng.unit());
                        add_at(&mut out, (at * rate) as usize, &tap);
                    }
                    step_at += 0.15 + 0.15 * rng.unit();
                }
            }
            // Mandibles working: wet, quick double clicks.
            if rng.unit() < 0.5 {
                let at = rng.unit() * 0.5;
                for j in 0..2 {
                    let f = (1800.0 + 900.0 * rng.unit()) * scale;
                    let click = ring(&[(f, 2500.0, 0.35)], 0.25, rng, sample_rate);
                    add_at(&mut out, ((at + 0.03 * j as f32) * rate) as usize, &click);
                }
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
                let tap = ring(
                    &[(200.0 + 200.0 * rng.unit(), 120.0, 0.6)],
                    0.4,
                    rng,
                    sample_rate,
                );
                add_at(&mut out, ((0.3 + 0.7 * rng.unit()) * rate) as usize, &tap);
            }
        }
    }
    out
}

/// A creeper: its fuse, a turbulent hiss through a narrow opening that
/// swells over the 1.5 s before it goes off, fluttering, with sparks and a
/// faint rising whistle; or, struck, the crunch of its leafy body.
pub(super) fn creeper(rng: &mut Rng, call: CreeperCall, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    match call {
        CreeperCall::Primed => {
            let n = (1.6 * rate) as usize;
            let mut bands = [
                BandPass::new_at(3200.0, 2.0, sample_rate),
                BandPass::new_at(6000.0, 1.5, sample_rate),
            ];
            let mut flutter = LowPass::new_at(10.0, sample_rate);
            let mut whistle = 0.0f32;
            let mut until_spark = 0.0f32;
            let mut spark = 0.0f32;
            let mut leaves = BandPass::new_at(4500.0, 1.2, sample_rate);
            let mut shiver_phase = 0.0f32;
            let mut rumble_phase = 0.0f32;
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
                    // Its leafy body shivering, faster and harder as it swells,
                    // and a low pressure building under it all.
                    shiver_phase += (12.0 + 18.0 * t) / rate;
                    let tremble = 0.5 + 0.5 * (TAU * shiver_phase).sin();
                    let shiver = 0.5 * t * tremble * leaves.process(rng.signed());
                    rumble_phase += TAU * (55.0 + 50.0 * t * t) / rate;
                    let rumble = 0.12 * t * t * rumble_phase.sin();
                    fade * (swell * (wobble * hiss + 0.06 * t * whistle.sin() + fizz)
                        + shiver
                        + rumble)
                })
                .collect()
        }
        CreeperCall::Hurt | CreeperCall::Death => {
            let seconds = if call == CreeperCall::Hurt { 0.2 } else { 0.9 };
            let n = (seconds * rate) as usize;
            let mut rustle = BandPass::new_at(4000.0, 1.0, sample_rate);
            let mut sigh = BandPass::new_at(3000.0, 1.5, sample_rate);
            let thud = ring(
                &[(110.0 + 40.0 * rng.unit(), 30.0, 1.0)],
                0.5,
                rng,
                sample_rate,
            );
            let mut out: Vec<f32> = (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let crunch = (-t * 8.0).exp() * rustle.process(rng.signed());
                    sigh.tune_at(3000.0 - 1500.0 * t, 1.5, sample_rate);
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
pub(super) fn bat(rng: &mut Rng, call: Call3, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
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
                let mut lp = LowPass::new_at(800.0, sample_rate);
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
pub(super) fn underwater(rng: &mut Rng, voice: Vec<f32>, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    let mut lp = [
        LowPass::new_at(700.0, sample_rate),
        LowPass::new_at(700.0, sample_rate),
    ];
    let mut out: Vec<f32> = voice
        .iter()
        .map(|x| {
            let first = lp[0].process(*x);
            1.5 * lp[1].process(first)
        })
        .collect();
    let length = out.len() as f32 / rate;
    for _ in 0..10 + (rng.unit() * 15.0) as usize {
        let mut bubble = Bubble::new_at(0.002 + 0.004 * rng.unit(), 0.25, sample_rate);
        let mut sound = Vec::new();
        while !bubble.done() {
            sound.push(bubble.sample());
        }
        add_at(&mut out, (length * rng.unit() * rate) as usize, &sound);
    }
    out
}

/// `count` bubbles rising through `sound`, quietly.
pub(super) fn bubbles(
    rng: &mut Rng,
    mut sound: Vec<f32>,
    count: usize,
    sample_rate: u32,
) -> Vec<f32> {
    let rate = sample_rate as f32;
    let length = sound.len() as f32 / rate;
    let peak = sound.iter().fold(0f32, |m, x| m.max(x.abs()));
    for _ in 0..count {
        let mut bubble = Bubble::new_at(0.003 + 0.004 * rng.unit(), 1.0, sample_rate);
        let mut pop = Vec::new();
        while !bubble.done() {
            pop.push(bubble.sample());
        }
        let loudest = pop.iter().fold(0f32, |m, x| m.max(x.abs())).max(1e-6);
        let quiet: Vec<f32> = pop.iter().map(|x| x * 0.15 * peak / loudest).collect();
        add_at(&mut sound, (length * rng.unit() * rate) as usize, &quiet);
    }
    sound
}

/// A wet throat: a few bubbles bursting in it as it groans.
pub(super) fn gurgle(rng: &mut Rng, mut voice: Vec<f32>, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    let length = voice.len() as f32 / rate;
    for _ in 0..4 + (rng.unit() * 6.0) as usize {
        let mut bubble = Bubble::new_at(0.003 + 0.004 * rng.unit(), 0.15, sample_rate);
        let mut sound = Vec::new();
        while !bubble.done() {
            sound.push(bubble.sample());
        }
        add_at(&mut voice, (length * rng.unit() * rate) as usize, &sound);
    }
    voice
}

/// A bee's buzz: its wings beating about 230 times a second (faster when
/// angry), each stroke a pressure pulse rich in harmonics, wavering. One
/// second that loops seamlessly: whole wingbeats, whole wavers, and its end
/// crossfaded into its start.
pub(super) fn bee_loop(rng: &mut Rng, angry: bool, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    let beat = if angry { 255.0 } else { 230.0 } + (rng.unit() * 6.0).floor();
    let wobble = if angry { 5.0 } else { 2.0 };
    let n = rate as usize;
    let tail = n / 20;
    let mut phase = 0.0f32;
    let mut noise_lp = LowPass::new_at(2000.0, sample_rate);
    let make = |i: usize, phase: &mut f32, noise: f32| {
        let t = i as f32 / rate;
        // FM over whole cycles of the waver nets to zero: the loop stays in tune.
        let f = beat * (1.0 + 0.015 * (TAU * wobble * t).sin());
        *phase += f / rate;
        let mut s = 0.0;
        let harmonics = if angry { 40 } else { 30 };
        for h in 1..=harmonics {
            if beat * h as f32 > 0.45 * rate {
                break;
            }
            s += (TAU * h as f32 * *phase).sin() / (h as f32).powf(if angry { 0.7 } else { 0.9 });
        }
        let level = 1.0 + 0.2 * (TAU * 3.0 * t).sin();
        level * (0.25 * s + 0.15 * noise)
    };
    let mut out: Vec<f32> = (0..n + tail)
        .map(|i| make(i, &mut phase, noise_lp.process(rng.signed())))
        .collect();
    for i in 0..tail {
        let w = i as f32 / tail as f32;
        out[i] = out[i] * w + out[n + i] * (1.0 - w);
    }
    out.truncate(n);
    out
}

/// A bee hurt (its buzz spiking) or dying (its buzz falling away).
pub(super) fn bee_cry(rng: &mut Rng, death: bool, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    let seconds = if death { 0.7 } else { 0.3 };
    let n = (seconds * rate) as usize;
    let mut phase = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let f = if death {
                300.0 * (1.0 - 0.6 * t)
            } else {
                260.0 * (1.0 + 0.5 * (PI * t).sin())
            };
            phase += f / rate;
            let mut s = 0.0;
            for h in 1..30 {
                s += (TAU * h as f32 * phase).sin() / (h as f32).powf(0.8);
            }
            let envelope = if death {
                (1.0 - t).powi(2)
            } else {
                (PI * t).sin()
            };
            envelope * (0.25 * s + 0.1 * rng.signed())
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Slime {
    Squish,
    Jump,
    Attack,
    Hurt,
    Death,
}

/// A slime: a body of gel whose low modes wobble, heavily damped, under the
/// wet, sticky crackle of its surface parting and the pop of small bubbles.
/// `size` 1 for a big one; smaller ones are higher.
pub(super) fn slime(rng: &mut Rng, call: Slime, size: f32, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    let scale = 1.0 / size.max(0.3);
    let (seconds, impact, wet) = match call {
        Slime::Squish => (0.35, 0.8, 0.6),
        Slime::Jump => (0.3, 0.3, 0.8),
        Slime::Attack => (0.3, 1.0, 0.7),
        Slime::Hurt => (0.4, 0.9, 0.9),
        Slime::Death => (0.8, 1.0, 1.0),
    };
    let wobble: Vec<(f32, f32, f32)> = (0..3)
        .map(|i| {
            let f = (70.0 + 60.0 * i as f32) * scale * (1.0 + 0.1 * rng.signed());
            (f, PI * f / 3.0, impact / (1.0 + i as f32))
        })
        .collect();
    let mut out = ring(&wobble, 0.2 * impact, rng, sample_rate);
    let n = (seconds * rate) as usize;
    let mut band = BandPass::new(1200.0 * scale.sqrt(), 1.0);
    let mut stick = 0.0f32;
    let mut until = 0.0f32;
    let film: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            until -= 1.0 / rate;
            if until <= 0.0 {
                stick = 0.5 + 0.5 * rng.unit();
                let rate = 40.0 + 40.0 * rng.unit();
                until = rng.exponential(rate);
            }
            stick *= 0.995;
            let envelope = if call == Slime::Jump {
                t.sqrt() * (1.0 - t) * 2.0
            } else {
                (1.0 - t).powi(2)
            };
            wet * envelope * stick * band.process(rng.signed())
        })
        .collect();
    add_at(&mut out, 0, &film);
    // Peeling off the ground to jump: the seal breaks with a suction "thwop",
    // a low pulse rising as the gap opens, and the film tearing.
    if call == Slime::Jump {
        let n = (0.07 * rate) as usize;
        let (low, high) = (70.0 * scale, 220.0 * scale);
        let mut phase = 0.0f32;
        let mut tear = BandPass::new(900.0 * scale.sqrt(), 1.5);
        let thwop: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / n as f32;
                phase += TAU * (low + (high - low) * t) / rate;
                let envelope = (t * 12.0).min(1.0) * (1.0 - t).powi(2);
                envelope * (0.6 * phase.sin() + 0.4 * tear.process(rng.signed()))
            })
            .collect();
        add_at(&mut out, (0.03 * rate) as usize, &thwop);
    }
    // Landing or striking: the splat of it hitting, and bits flung off
    // landing round it a moment later.
    if matches!(call, Slime::Squish | Slime::Attack | Slime::Death) {
        let n = (0.025 * rate) as usize;
        let mut dull = LowPass::new(1500.0 * scale.sqrt());
        let splat: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / n as f32;
                0.5 * impact * (t * 25.0).min(1.0) * (1.0 - t).powi(3) * dull.process(rng.signed())
            })
            .collect();
        add_at(&mut out, (0.003 * rate) as usize, &splat);
        let flung = if call == Slime::Death {
            8
        } else {
            2 + (rng.unit() * 4.0) as usize
        };
        for _ in 0..flung {
            let at = ((0.03 + 0.17 * rng.unit()) * rate) as usize;
            let mut drop = Bubble::new((0.0008 + 0.0012 * rng.unit()) * size.sqrt(), 0.04);
            let mut plip = Vec::new();
            while !drop.done() {
                plip.push(drop.sample());
            }
            add_at(&mut out, at, &plip);
        }
    }
    let bubbles = if call == Slime::Death { 8 } else { 3 };
    for _ in 0..bubbles {
        let mut bubble = Bubble::new((0.001 + 0.002 * rng.unit()) * size.sqrt(), 0.03);
        let mut pop = Vec::new();
        while !bubble.done() {
            pop.push(bubble.sample());
        }
        add_at(&mut out, (seconds * rng.unit() * rate) as usize, &pop);
    }
    out
}

/// An iron golem, struck (its iron and stone ring, and its joints creak by
/// stick–slip) or falling apart.
pub(super) fn iron_golem(rng: &mut Rng, call: Call3, seed: u64, sample_rate: u32) -> Vec<f32> {
    use crate::{contact, fracture, Contact, Material};
    let mut out = Vec::new();
    match call {
        Call3::Death => {
            add_at(&mut out, 0, &fracture(Material::Metal, seed, sample_rate));
            add_at(
                &mut out,
                (0.05 * sample_rate as f32) as usize,
                &fracture(Material::Stone, seed + 1, sample_rate),
            );
        }
        _ => {
            add_at(
                &mut out,
                0,
                &contact(Contact::Tap, Material::Metal, 2.0, seed, sample_rate),
            );
            add_at(
                &mut out,
                0,
                &contact(Contact::Tap, Material::Stone, 2.0, seed + 1, sample_rate),
            );
            // Stick–slip: the joint grinds in jerks, slower and slower.
            let mut at = 0.05;
            let mut step = 0.015;
            for _ in 0..8 + (rng.unit() * 8.0) as usize {
                let f = 900.0 + 600.0 * rng.unit();
                let click = ring(
                    &[(f, 300.0, 0.15), (2.3 * f, 500.0, 0.08)],
                    0.05,
                    rng,
                    sample_rate,
                );
                add_at(&mut out, (at * sample_rate as f32) as usize, &click);
                at += step;
                step *= 1.15;
            }
        }
    }
    out
}

/// A snow golem: snow creaking (ambient), crunching (hurt), or collapsing.
pub(super) fn snow_golem(call: Call3, seed: u64, sample_rate: u32) -> Vec<f32> {
    use crate::{contact, fracture, Contact, Material};
    match call {
        Call3::Ambient => contact(Contact::Step, Material::Snow, 0.4, seed, sample_rate)
            .iter()
            .map(|x| 0.5 * x)
            .collect(),
        Call3::Hurt => contact(Contact::Land, Material::Snow, 2.0, seed, sample_rate),
        Call3::Death => fracture(Material::Snow, seed, sample_rate),
    }
}

/// A guardian flopping on land: a wet slap.
pub(super) fn flop(rng: &mut Rng, big: bool, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    let f = if big { 90.0 } else { 160.0 } * (1.0 + 0.1 * rng.signed());
    let mut out = ring(
        &[(f, 40.0, 1.0), (2.7 * f, 90.0, 0.4)],
        0.8,
        rng,
        sample_rate,
    );
    let n = (0.12 * rate) as usize;
    let mut band = BandPass::new_at(1500.0, 0.8, sample_rate);
    let slap: Vec<f32> = (0..n)
        .map(|i| (1.0 - i as f32 / n as f32).powi(3) * band.process(rng.signed()))
        .collect();
    add_at(&mut out, 0, &slap);
    out
}

/// A phantom's wings: a few broad, soft beats; or its swoop, air rushing
/// past and away.
pub(super) fn phantom_air(rng: &mut Rng, swoop: bool, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    if swoop {
        let n = (0.9 * rate) as usize;
        let mut band = BandPass::new_at(800.0, 0.7, sample_rate);
        (0..n)
            .map(|i| {
                let t = i as f32 / n as f32;
                band.tune_at(1800.0 - 1200.0 * t, 0.7, sample_rate);
                (PI * t).sin().powi(2) * band.process(rng.signed())
            })
            .collect()
    } else {
        let mut out = Vec::new();
        let mut lp = LowPass::new_at(500.0, sample_rate);
        for beat in 0..2 + (rng.unit() * 2.0) as usize {
            let n = (0.08 * rate) as usize;
            let thump: Vec<f32> = (0..n)
                .map(|i| (PI * i as f32 / n as f32).sin() * lp.process(rng.signed()) * 3.0)
                .collect();
            add_at(&mut out, (beat as f32 * 0.35 * rate) as usize, &thump);
        }
        out
    }
}
