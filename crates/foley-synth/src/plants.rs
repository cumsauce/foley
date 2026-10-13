//! Brushing through plants, lake ice singing, and stones settling in caves
//! (M11).

use std::f32::consts::TAU;

use crate::outdoors::{add_at, normalize};
use crate::{BandPass, Contact, LowPass, Material, Rng, SAMPLE_RATE, contact};

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
pub fn brush(plant: Plant, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0xB2_05);
    let rate = SAMPLE_RATE as f32;
    // (seconds, swish band and Q, tick band, ticks a second, tick decay a
    // sample, swish level)
    let (seconds, band, q, tick_band, ticks, decay, swish) = match plant {
        Plant::Grass => (0.32, 3500.0, 0.8, 5200.0, 450.0, 0.94, 0.5),
        Plant::Leaves => (0.42, 1500.0, 0.7, 2300.0, 150.0, 0.985, 0.7),
        Plant::Dry => (0.28, 4200.0, 1.0, 6500.0, 650.0, 0.9, 0.2),
    };
    let seconds = seconds * (0.85 + 0.3 * rng.unit());
    let n = (seconds * rate) as usize;
    let mut sweep = BandPass::new(band, q);
    let mut tick = BandPass::new(tick_band, 1.5);
    let mut flutter = LowPass::new(20.0);
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
        let wobble = (1.0 + 3.0 * flutter.process(rng.signed())).clamp(0.4, 1.6);
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
pub fn ice_singing(seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0x1CE);
    let rate = SAMPLE_RATE as f32;
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
        let mut crack = BandPass::new(3000.0, 0.7);
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
pub fn settling_stone(seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0x5E77);
    let rate = SAMPLE_RATE as f32;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn zero_crossings(s: &[f32]) -> f32 {
        s.windows(2)
            .filter(|w| (w[0] <= 0.0) != (w[1] <= 0.0))
            .count() as f32
            / s.len() as f32
    }

    #[test]
    fn every_sound_is_finite_bounded_and_short() {
        for sound in [
            brush(Plant::Grass, 1),
            brush(Plant::Leaves, 2),
            brush(Plant::Dry, 3),
            ice_singing(4),
            settling_stone(5),
        ] {
            assert!(sound.iter().all(|x| x.is_finite() && x.abs() <= 0.4));
            assert!(sound.iter().any(|x| x.abs() > 0.05));
            assert!(sound.len() < 3 * SAMPLE_RATE as usize);
        }
    }

    #[test]
    fn grass_is_brighter_than_leaves_and_ice_falls_in_pitch() {
        assert!(zero_crossings(&brush(Plant::Grass, 1)) > zero_crossings(&brush(Plant::Leaves, 1)));
        let ice = ice_singing(7);
        let tenth = (0.1 * SAMPLE_RATE as f32) as usize;
        let early = zero_crossings(&ice[tenth / 2..tenth]);
        let late = zero_crossings(&ice[4 * tenth..5 * tenth]);
        assert!(early > 2.0 * late, "{early} then {late}");
    }
}
