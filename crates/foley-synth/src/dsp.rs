//! Small building blocks every synth shares: mixing one sound into another,
//! levelling, click-free fades, and a slow random wobble.

use crate::LowPass;

/// Adds `sound`, times `gain`, into `out` from sample `at`, growing `out` to
/// fit. For one-shots, made off the audio thread.
pub(crate) fn add_at(out: &mut Vec<f32>, at: usize, sound: &[f32], gain: f32) {
    if out.len() < at + sound.len() {
        out.resize(at + sound.len(), 0.0);
    }
    for (o, s) in out[at..].iter_mut().zip(sound) {
        *o += gain * s;
    }
}

/// The largest absolute sample.
pub(crate) fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0f32, |m, x| m.max(x.abs()))
}

/// Scales `out` so its peak is `level`, so every call of a one-shot comes out
/// at a known level.
pub(crate) fn normalize(mut out: Vec<f32>, level: f32) -> Vec<f32> {
    normalize_in_place(&mut out, level);
    out
}

/// [`normalize`], in place.
pub(crate) fn normalize_in_place(out: &mut [f32], level: f32) {
    let loudest = peak(out);
    if loudest > 0.0 {
        let scale = level / loudest;
        out.iter_mut().for_each(|x| *x *= scale);
    }
}

/// A raised-cosine step from 0 at `x` = 0 to 1 at `x` = 1, for envelopes that
/// never click.
pub(crate) fn smooth(x: f32) -> f32 {
    0.5 - 0.5 * (std::f32::consts::PI * x.clamp(0.0, 1.0)).cos()
}

/// A level wandering slowly and at random round 1: white noise low-passed at
/// `hz`, times `depth`, added to 1 and kept between `low` and `high`. The
/// flicker of flames, the flutter of leaves, the surge of falling water.
#[derive(Debug, Clone)]
pub(crate) struct Wobble {
    filter: LowPass,
    depth: f32,
    low: f32,
    high: f32,
}

impl Wobble {
    pub(crate) fn new(hz: f32, depth: f32, low: f32, high: f32) -> Wobble {
        Wobble {
            filter: LowPass::new(hz),
            depth,
            low,
            high,
        }
    }

    /// The next value, from one sample of white `noise` (-1 to 1).
    pub(crate) fn next(&mut self, noise: f32) -> f32 {
        (1.0 + self.depth * self.filter.process(noise)).clamp(self.low, self.high)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rng;

    #[test]
    fn helpers_mix_level_and_fade() {
        let mut out = vec![1.0];
        add_at(&mut out, 2, &[1.0, 2.0], 0.5);
        assert_eq!(out, [1.0, 0.0, 0.5, 1.0]);
        assert_eq!(peak(&normalize(out, 0.5)), 0.5);
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
