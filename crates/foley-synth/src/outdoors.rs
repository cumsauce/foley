//! More life outdoors (M10a): the sea on a shore, woodpeckers drumming,
//! owls at night and cicadas in the heat.

use std::f32::consts::TAU;

use crate::water::plunge_radius;
use crate::{BandPass, Bubble, Contact, LowPass, Material, Rng, SAMPLE_RATE, contact};

fn add_at(out: &mut Vec<f32>, at: usize, sound: &[f32], gain: f32) {
    if out.len() < at + sound.len() {
        out.resize(at + sound.len(), 0.0);
    }
    for (o, s) in out[at..].iter_mut().zip(sound) {
        *o += gain * s;
    }
}

/// Peaks at `peak`, so every call comes out at a known level.
fn normalize(mut out: Vec<f32>, peak: f32) -> Vec<f32> {
    let loudest = out.iter().fold(0f32, |m, x| m.max(x.abs()));
    if loudest > 0.0 {
        out.iter_mut().for_each(|x| *x *= peak / loudest);
    }
    out
}

/// One wave breaking on a shore, `height` metres high (0.2 to 3), about
/// 7 s from its rise to the end of its backwash. As it steepens, a low roar
/// grows; it plunges, and the air it traps is torn into bubbles of every
/// size (Deane and Stokes' spectrum), heard as the crash; the foam it leaves
/// hisses as its small bubbles burst while the water runs up the beach; then
/// it drains back, drawing the shingle with it, on a stony shore.
pub fn surf(height: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0x5EA);
    let rate = SAMPLE_RATE as f32;
    let height = height.clamp(0.2, 3.0);
    let length = (7.0 * rate) as usize;
    let break_at = 1.2 + 0.6 * rng.unit();
    let shingle = rng.unit() < 0.5;
    let mut out = vec![0.0f32; length];
    let mut rumble = [LowPass::new(250.0), LowPass::new(250.0)];
    let mut crash = BandPass::new(900.0, 0.5);
    let mut foam = BandPass::new(5000.0, 0.7);
    let mut swell = LowPass::new(2.0);
    let mut bubbles: Vec<Bubble> = Vec::with_capacity(256);
    let mut until_bubble = 0.0f32;
    let mut until_pop = 0.0f32;
    let mut pop = 0.0f32;
    let dt = 1.0 / rate;
    for (i, sample) in out.iter_mut().enumerate() {
        let t = i as f32 * dt;
        let after = t - break_at;
        let noise = rng.signed();
        // The rising face: a low roar growing until it plunges.
        let rise = if after < 0.0 {
            (t / break_at).powi(2)
        } else {
            (-after / 1.5).exp()
        };
        let first = rumble[0].process(noise);
        let low = rumble[1].process(first);
        let mut s = 2.5 * rise * low;
        if after >= 0.0 {
            // The crash: broadband, falling away over a second.
            let hit = (after * 40.0).min(1.0) * (-after / 0.7).exp();
            s += 0.5 * hit * crash.process(noise) + 3.0 * hit * low;
            // The foam's hiss: bursting bubbles, brighter as the sheet runs
            // thin up the beach, gone as it soaks in.
            let sheet = (after / 0.4).min(1.0) * (-after / 2.2).exp();
            let wobble = (1.0 + 3.0 * swell.process(rng.signed())).clamp(0.4, 1.6);
            s += 0.25 * sheet * wobble * foam.process(rng.signed());
            // Tiny bubbles popping in the foam.
            until_pop -= dt;
            if until_pop <= 0.0 {
                pop = 0.2 + 0.8 * rng.unit();
                until_pop = rng.exponential(400.0 * sheet + 1.0);
            }
            pop *= 0.8;
            s += 0.05 * sheet * pop * rng.signed();
            // Trapped air, torn into bubbles that ring as they settle.
            let entrained = 4000.0 * (-after / 0.5).exp() * height.sqrt();
            until_bubble -= dt;
            while until_bubble <= 0.0 && entrained > 1.0 {
                until_bubble += rng.exponential(entrained);
                if bubbles.len() < 256 {
                    let radius = plunge_radius(&mut rng);
                    bubbles.push(Bubble::new(radius, 0.01 * (0.5 + rng.unit())));
                }
            }
        }
        for bubble in &mut bubbles {
            s += bubble.sample();
        }
        if i % 256 == 0 {
            bubbles.retain(|b| !b.done());
        }
        *sample = s;
    }
    // The backwash through shingle: pebbles knocking as the water draws
    // them down, thinning out as it drains.
    if shingle {
        let start = break_at + 2.5;
        let mut at = start;
        while at < 6.8 {
            let left = 1.0 - (at - start) / (6.8 - start);
            let f = 2500.0 + 3000.0 * rng.unit();
            let knock: Vec<f32> = (0..(0.01 * rate) as usize)
                .map(|n| {
                    let t = n as f32 * dt;
                    (-t * 600.0).exp() * (TAU * f * t).sin()
                })
                .collect();
            add_at(
                &mut out,
                (at * rate) as usize,
                &knock,
                0.06 * left * rng.unit(),
            );
            at += rng.exponential(250.0 * left + 5.0);
        }
    }
    // Fade the end so it can overlap the next wave.
    let fade = (0.5 * rate) as usize;
    for i in 0..fade {
        out[length - 1 - i] *= i as f32 / fade as f32;
    }
    normalize(out, 0.25 + 0.2 * (height / 3.0))
}

/// A woodpecker drumming on a dead branch: 10 to 30 strikes at about 20 a
/// second, louder then softer, slowing a little at the end, each the beak
/// striking wood at some 6 m/s and the branch ringing.
pub fn woodpecker(seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0xD0D0);
    let rate = SAMPLE_RATE as f32;
    let strikes = 10 + (rng.unit() * 20.0) as usize;
    let mut hz = 16.0 + 10.0 * rng.unit();
    let mut out = vec![0.0f32; (0.3 * rate) as usize];
    let mut at = 0.05;
    for n in 0..strikes {
        let x = n as f32 / strikes as f32;
        let loud = (x * 4.0).min(1.0) * (1.0 - 0.6 * x);
        let knock = contact(
            Contact::Tap,
            Material::Wood,
            6.0,
            seed.wrapping_add(n as u64 % 4),
        );
        add_at(&mut out, (at * rate) as usize, &knock, loud);
        if x > 0.7 {
            hz *= 0.97;
        }
        at += 1.0 / hz;
    }
    // A branch is a resonant body: a touch of its hollow ring.
    let mut body = BandPass::new(600.0 + 400.0 * rng.unit(), 6.0);
    let hollow: Vec<f32> = out.iter().map(|x| body.process(*x)).collect();
    for (o, h) in out.iter_mut().zip(hollow) {
        *o += 0.8 * h;
    }
    normalize(out, 0.45)
}

/// An owl hooting at night: a great horned owl's "hoo, h'hoo-hoo, hoo,
/// hoo", low and soft, each note a nearly pure tone from the syrinx with a
/// little breath, rising into it and falling away.
pub fn owl(seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0x0071);
    let rate = SAMPLE_RATE as f32;
    // A great horned owl hoots low: about 280 to 360 Hz.
    let f0 = 280.0 + 80.0 * rng.unit();
    // (start, length, pitch, loudness).
    let pattern: &[(f32, f32, f32, f32)] = if rng.unit() < 0.5 {
        &[
            (0.0, 0.35, 1.0, 0.9),
            (0.55, 0.12, 0.98, 0.6),
            (0.7, 0.3, 1.02, 1.0),
            (1.25, 0.32, 0.97, 0.85),
            (1.85, 0.3, 0.95, 0.75),
        ]
    } else {
        &[
            (0.0, 0.5, 1.0, 1.0),
            (0.9, 0.18, 1.04, 0.7),
            (1.15, 0.18, 1.02, 0.7),
            (1.4, 0.55, 0.96, 0.9),
        ]
    };
    let mut out = vec![0.0f32; (2.6 * rate) as usize];
    let mut breath = BandPass::new(f0 * 2.0, 2.0);
    for &(start, length, pitch, loud) in pattern {
        let n = (length * rate) as usize;
        let mut phase = 0.0f32;
        let note: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / n as f32;
                // Up into the note and down out of it.
                let f = f0 * pitch * (0.94 + 0.08 * (t * 4.0).min(1.0) - 0.06 * t * t);
                phase += TAU * f / rate;
                let envelope = (t * 8.0).min(1.0) * (1.0 - t).powf(0.6);
                let tone = phase.sin() + 0.12 * (2.0 * phase).sin();
                envelope * (tone + 0.15 * breath.process(rng.signed()))
            })
            .collect();
        add_at(&mut out, (start * rate) as usize, &note, loud);
    }
    normalize(out, 0.35)
}

/// Cicadas in the heat for 6 s at `celsius`: a few males, each buckling its
/// tymbals hundreds of times a second (faster when warmer), every click
/// ringing the hollow abdomen near 5 kHz, in long swells; together, the
/// drone of a hot afternoon. Below about 20 °C they're silent.
pub fn cicadas(celsius: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0xC1CA);
    let rate = SAMPLE_RATE as f32;
    let length = (6.0 * rate) as usize;
    let mut out = vec![0.0f32; length];
    if celsius < 20.0 {
        return out;
    }
    let count = 3 + (rng.unit() * 5.0) as usize;
    for _ in 0..count {
        let clicks = (150.0 + 12.0 * (celsius - 20.0)) * (1.0 + 0.2 * rng.signed());
        let ring = 4500.0 + 1500.0 * rng.unit();
        let mut body = [BandPass::new(ring, 6.0), BandPass::new(ring, 6.0)];
        let swell_hz = 0.15 + 0.3 * rng.unit();
        let offset = rng.unit() * TAU;
        let near = 0.3 + 0.7 * rng.unit();
        let mut phase = 0.0f32;
        for (i, o) in out.iter_mut().enumerate() {
            let t = i as f32 / rate;
            phase += clicks / rate;
            let click = if phase >= 1.0 {
                phase -= 1.0;
                1.0
            } else {
                0.0
            };
            let swell = (0.5 + 0.5 * (TAU * swell_hz * t + offset).sin()).powi(2);
            let first = body[0].process(click);
            *o += near * swell * body[1].process(first);
        }
    }
    let fade = (0.3 * rate) as usize;
    for i in 0..fade {
        let g = i as f32 / fade as f32;
        out[i] *= g;
        out[length - 1 - i] *= g;
    }
    normalize(out, 0.3)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt()
    }

    #[test]
    fn every_call_is_finite_bounded_and_audible() {
        for sound in [
            surf(1.0, 1),
            surf(3.0, 2),
            surf(0.2, 3),
            woodpecker(4),
            owl(5),
            owl(6),
            cicadas(30.0, 7),
        ] {
            assert!(sound.iter().all(|x| x.is_finite() && x.abs() <= 0.5));
            assert!(rms(&sound) > 0.005, "{}", rms(&sound));
            assert!(sound.len() < 10 * SAMPLE_RATE as usize);
        }
    }

    #[test]
    fn a_wave_crashes_after_it_rises_and_cicadas_need_heat() {
        let wave = surf(1.5, 9);
        let second =
            |s: usize| rms(&wave[s * SAMPLE_RATE as usize..(s + 1) * SAMPLE_RATE as usize]);
        // Its loudest second is the crash, not the start or the end.
        assert!(second(2) > second(0) && second(2) > second(6));
        assert!(cicadas(15.0, 1).iter().all(|x| *x == 0.0));
    }
}
