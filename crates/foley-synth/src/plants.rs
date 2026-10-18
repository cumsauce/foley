//! Brushing through plants, lake ice singing, stones settling in caves, a
//! bird flushed from cover and insects round flowers (M11).

use std::f32::consts::TAU;

use crate::dsp::{add_at, normalize, Wobble};
use crate::{contact, BandPass, Contact, LowPass, Material, Rng};

/// What a body brushes through: for [`brush`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plant {
    /// Grass, ferns, flowers: many thin blades flicking past, bright.
    Grass,
    /// Leaves and leafy bushes: fewer, bigger leaves slapping, lower.
    Leaves,
    /// Dry stalks (a dead bush, ripe crops): crisp, crackling.
    Dry,
}

impl Plant {
    pub fn of(index: u32) -> Plant {
        match index {
            0 => Plant::Grass,
            1 => Plant::Leaves,
            _ => Plant::Dry,
        }
    }
}

/// A leg or body pushing through `plant`, about a third of a second: a swish
/// of the stems sliding past each other, swelling and dying away, made of
/// many small collisions, each a blade or leaf flicked and ringing for a
/// millisecond or so (thin blades high, leaves lower).
pub fn brush(plant: Plant, seed: u64, sample_rate: u32) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0xB2_05);
    let rate = sample_rate as f32;
    // (seconds, swish band and Q, tick band, ticks a second, tick decay a
    // sample, swish level)
    let (seconds, band, q, tick_band, ticks, decay, swish) = match plant {
        Plant::Grass => (0.32, 3500.0, 0.8, 5200.0, 450.0, 0.94, 0.5),
        Plant::Leaves => (0.42, 1500.0, 0.7, 2300.0, 150.0, 0.985, 0.7),
        Plant::Dry => (0.28, 4200.0, 1.0, 6500.0, 650.0, 0.9, 0.2),
    };
    let seconds = seconds * (0.85 + 0.3 * rng.unit());
    let n = (seconds * rate) as usize;
    let mut sweep = BandPass::new_at(band, q, sample_rate);
    let mut tick = BandPass::new_at(tick_band, 1.5, sample_rate);
    let mut flutter = Wobble::new_at(20.0, 3.0, 0.4, 1.6, sample_rate);
    let mut envelope = 0.0f32;
    let mut until = 0.0f32;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / n as f32;
        // Up quickly as the leg meets them, then a longer fall as they
        // spring back.
        let shape = if t < 0.3 {
            (t / 0.3).powi(2)
        } else {
            ((1.0 - t) / 0.7).powf(1.5)
        };
        until -= 1.0 / rate;
        if until <= 0.0 {
            envelope += 0.3 + 0.7 * rng.unit();
            until = rng.exponential(ticks * shape.max(0.05));
        }
        envelope *= decay;
        let wobble = flutter.next(rng.signed());
        let noise = rng.signed();
        out.push(
            shape * (swish * wobble * sweep.process(noise))
                + envelope.min(2.0) * tick.process(rng.signed()),
        );
    }
    normalize(out, 0.3)
}

/// Lake ice singing in the cold: the ice cracks somewhere, and the bending
/// waves it sets off run through the sheet faster the higher they are
/// (flexural waves: group speed ∝ √f), so they reach the ear as a falling
/// "pew", from some 6 kHz down to 150 Hz in under a second; often a few
/// echoes of it off the shore follow.
pub fn ice_singing(seed: u64, sample_rate: u32) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0x1CE);
    let rate = sample_rate as f32;
    let mut out = vec![0.0f32; (2.5 * rate) as usize];
    let arrivals = 1 + (rng.unit() * 3.0) as usize;
    let mut at = 0.0f32;
    for k in 0..arrivals {
        // Farther cracks arrive later and slower to fall: t(f) = d / (c √f),
        // so f(t) = top (t0 / t)².
        let t0 = 0.08 + 0.1 * rng.unit();
        let top = 5000.0 + 2000.0 * rng.unit();
        let end = t0 * (top / 150.0f32).sqrt();
        let n = ((end - t0) * rate) as usize;
        let mut phase = 0.0f32;
        let mut crack = BandPass::new_at(3000.0, 0.7, sample_rate);
        let sweep: Vec<f32> = (0..n)
            .map(|i| {
                let t = t0 + i as f32 / rate;
                let f = top * (t0 / t).powi(2);
                phase = (phase + TAU * f / rate) % TAU;
                let x = i as f32 / n as f32;
                // The crack itself, then the tone, fading as it falls.
                let snap = if i < (0.004 * rate) as usize {
                    crack.process(rng.signed()) * 3.0
                } else {
                    0.0
                };
                let fade = (x * 40.0).min(1.0) * (1.0 - x).powf(0.7);
                snap + fade * phase.sin()
            })
            .collect();
        add_at(
            &mut out,
            (at * rate) as usize,
            &sweep,
            1.0 / (1.0 + k as f32),
        );
        at += 0.15 + 0.4 * rng.unit();
    }
    normalize(out, 0.3)
}

/// A stone working loose somewhere in a cave and settling: it knocks, then
/// bounces a few times, each bounce lower and sooner than the last (a
/// coefficient of restitution of about a half), and comes to rest.
pub fn settling_stone(seed: u64, sample_rate: u32) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0x5E77);
    let rate = sample_rate as f32;
    let mut out = Vec::new();
    let mut speed = 2.5 + 2.5 * rng.unit();
    let mut at = 0.0f32;
    let restitution = 0.4 + 0.2 * rng.unit();
    for bounce in 0..2 + (rng.unit() * 4.0) as u64 {
        let knock = contact(
            Contact::Tap,
            Material::Stone,
            speed,
            seed.wrapping_add(bounce),
            sample_rate,
        );
        add_at(&mut out, (at * rate) as usize, &knock, 1.0);
        // The time aloft between bounces goes as the speed off the ground.
        at += 2.0 * speed * restitution / 9.81;
        speed *= restitution;
        if speed < 0.3 {
            break;
        }
    }
    normalize(out, 0.35)
}

/// A bird flushed from cover: its wings clap and whirr as it takes off, a
/// dozen beats at 10 to 20 a second (a small bird beats faster), each a
/// short burst of air through the feathers, fading as it flies off.
pub fn flush(seed: u64, sample_rate: u32) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0xF1A9);
    let rate = sample_rate as f32;
    let beats_per_second = 10.0 + 10.0 * rng.unit();
    let beats = 8 + (rng.unit() * 8.0) as usize;
    let mut out = Vec::new();
    let mut at = 0.0f32;
    for k in 0..beats {
        let n = (0.03 * rate) as usize;
        let mut feathers = BandPass::new_at(1200.0 + 800.0 * rng.unit(), 0.8, sample_rate);
        let beat: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / rate;
                (t / 0.002).min(1.0) * (-t / 0.008).exp() * feathers.process(rng.signed())
            })
            .collect();
        // The first beats, pushing off, are the loudest.
        let loud = (1.0 - k as f32 / beats as f32).powf(1.5) * (0.7 + 0.3 * rng.unit());
        add_at(&mut out, (at * rate) as usize, &beat, loud);
        at += (1.0 + 0.1 * rng.signed()) / beats_per_second;
    }
    normalize(out, 0.3)
}

/// An insect on the wing round flowers: `size` 0 a bee, 1 a fly. Its wings
/// beat at about 230 Hz (a honeybee) or 190 Hz (a housefly), a buzz rich in
/// harmonics, swelling and fading and bending in pitch (Doppler) as it
/// darts nearer and away.
pub fn buzz(size: u32, seed: u64, sample_rate: u32) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0xB22);
    let rate = sample_rate as f32;
    let (f0, darting) = if size == 0 {
        (230.0, 0.6)
    } else {
        (190.0, 1.5)
    };
    let f0 = f0 * (0.92 + 0.16 * rng.unit());
    let n = ((2.0 + 1.5 * rng.unit()) * rate) as usize;
    let mut path = [
        LowPass::new_at(darting, sample_rate),
        LowPass::new_at(darting, sample_rate),
    ];
    let mut phase = 0.0f32;
    let mut last = 1.0f32;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / n as f32;
        // Its distance wanders between about 0.3 and 2 m.
        let first = path[0].process(rng.signed());
        let r = (1.0 + 30.0 * path[1].process(first)).clamp(0.3, 2.0);
        let speed = (r - last) * rate;
        last = r;
        let f = f0 * (1.0 - speed / 343.0);
        phase = (phase + f / rate) % 1.0;
        // A buzzy, saw-like wingbeat.
        let wave = 2.0 * phase - 1.0;
        let fade = (t * 10.0).min(1.0) * ((1.0 - t) * 10.0).min(1.0);
        out.push(fade * wave / r);
    }
    let mut soften = LowPass::new_at(3000.0, sample_rate);
    let out: Vec<f32> = out.iter().map(|x| soften.process(*x)).collect();
    normalize(out, 0.25)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;

    fn zero_crossings(s: &[f32]) -> f32 {
        s.windows(2)
            .filter(|w| (w[0] <= 0.0) != (w[1] <= 0.0))
            .count() as f32
            / s.len() as f32
    }

    #[test]
    fn every_sound_is_finite_bounded_and_short() {
        for sound in [
            brush(Plant::Grass, 1, SAMPLE_RATE),
            brush(Plant::Leaves, 2, SAMPLE_RATE),
            brush(Plant::Dry, 3, SAMPLE_RATE),
            ice_singing(4, SAMPLE_RATE),
            settling_stone(5, SAMPLE_RATE),
            flush(6, SAMPLE_RATE),
            buzz(0, 7, SAMPLE_RATE),
            buzz(1, 8, SAMPLE_RATE),
        ] {
            assert!(sound.iter().all(|x| x.is_finite() && x.abs() <= 0.4));
            assert!(sound.iter().any(|x| x.abs() > 0.05));
            assert!(sound.len() < 4 * SAMPLE_RATE as usize);
        }
    }

    #[test]
    fn grass_is_brighter_than_leaves_and_ice_falls_in_pitch() {
        assert!(
            zero_crossings(&brush(Plant::Grass, 1, SAMPLE_RATE))
                > zero_crossings(&brush(Plant::Leaves, 1, SAMPLE_RATE))
        );
        let ice = ice_singing(7, SAMPLE_RATE);
        let tenth = (0.1 * SAMPLE_RATE as f32) as usize;
        let early = zero_crossings(&ice[tenth / 2..tenth]);
        let late = zero_crossings(&ice[4 * tenth..5 * tenth]);
        assert!(early > 2.0 * late, "{early} then {late}");
    }
}
