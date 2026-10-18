//! A click-free fade and a slow random wobble.

#[allow(unused_imports)] // Some of these are in core; the rest come from std.
use foley_math::Float as _;

use crate::LowPass;

/// A raised-cosine step from 0 at `x` = 0 to 1 at `x` = 1, for envelopes that
/// never click.
pub fn smooth(x: f32) -> f32 {
    0.5 - 0.5 * (core::f32::consts::PI * x.clamp(0.0, 1.0)).cos()
}

/// A level wandering slowly and at random round 1: white noise low-passed at
/// `hz`, times `depth`, added to 1 and kept between `low` and `high`. The
/// flicker of flames, the flutter of leaves, the surge of falling water.
#[derive(Debug, Clone)]
pub struct Wobble {
    filter: LowPass,
    depth: f32,
    low: f32,
    high: f32,
}

impl Wobble {
    pub fn new(hz: f32, depth: f32, low: f32, high: f32) -> Wobble {
        Wobble::new_at(hz, depth, low, high, crate::SAMPLE_RATE)
    }

    /// As [`Wobble::new`], at the host's sample rate.
    pub fn new_at(hz: f32, depth: f32, low: f32, high: f32, sample_rate: u32) -> Wobble {
        Wobble {
            filter: LowPass::new_at(hz, sample_rate),
            depth,
            low,
            high,
        }
    }

    /// The next value, from one sample of white `noise` (-1 to 1).
    pub fn next(&mut self, noise: f32) -> f32 {
        (1.0 + self.depth * self.filter.process(noise)).clamp(self.low, self.high)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rng;

    #[test]
    fn it_fades_and_wobbles_within_bounds() {
        assert_eq!(smooth(0.0), 0.0);
        assert!((smooth(1.0) - 1.0).abs() < 1e-6 && (smooth(0.5) - 0.5).abs() < 1e-6);
        let mut wobble = Wobble::new(5.0, 4.0, 0.3, 1.5);
        let mut rng = Rng::new(1);
        assert!(
            (0..48_000)
                .map(|_| wobble.next(rng.signed()))
                .all(|w| (0.3..=1.5).contains(&w))
        );
    }
}
