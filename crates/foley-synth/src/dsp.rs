//! Small building blocks the one-shots share: mixing one sound into
//! another, and levelling. The fade and the wobble live in `foley-dsp`.

pub(crate) use foley_dsp::{Wobble, smooth};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers_mix_level_and_fade() {
        let mut out = vec![1.0];
        add_at(&mut out, 2, &[1.0, 2.0], 0.5);
        assert_eq!(out, [1.0, 0.0, 0.5, 1.0]);
        assert_eq!(peak(&normalize(out, 0.5)), 0.5);
    }
}
