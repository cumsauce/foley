use foley_dsp::Rng;

use std::f32::consts::TAU;

/// A sculk sensor's triggered tone: a low-Q bowl ring at ~200 Hz, with a
/// small inharmonic overtone that gives it the bowl's "shimmer". The
/// amplitude decays exponentially. `q` is the bowl's quality factor (0.5 to
/// 20): a low Q thuds out fast, a high Q rings for a few seconds. `seed`
/// mixes a small starting-phase offset so each call (each sensor) is
/// distinct.
pub fn sculk_ring(q: f32, seed: u64, sample_rate: u32) -> Vec<f32> {
    // Clamp to the spec range; out-of-range callers get a default.
    let q = q.clamp(0.5, 20.0);
    let rate = sample_rate as f32;
    // Time constant: at q=6, tau=0.25 s, which matches a moderately ringy
    // bowl. Higher q rings proportionally longer.
    let tau = 0.25 * (q / 6.0);
    // Bowl's fundamental ~200 Hz, an inharmonic overtone at 1.6x with 0.1
    // amplitude.
    let f0 = 200.0;
    let length = (2.5 * rate) as usize;
    // A small phase offset (0..1 rad) per seed so two sensors don't ring
    // exactly in step. The Rng mixes the seed deterministically.
    let mut rng = Rng::new(seed);
    let phase0 = rng.unit() * 2.0;
    let phase1 = rng.unit() * 2.0;
    let decay_step = (-1.0 / (tau * rate)).exp();
    // A 30 ms attack so the bowl "swells" rather than clicks on.
    let attack = (0.03 * rate) as usize;
    let mut out = Vec::with_capacity(length);
    let mut amp = 1.0f32;
    let mut phase_a = phase0;
    let mut phase_b = phase1;
    for i in 0..length {
        // Linear attack across `attack` samples, then exponential decay.
        let env = if i < attack {
            i as f32 / attack as f32
        } else {
            amp
        };
        let s = env * (phase_a.sin() + 0.1 * phase_b.sin());
        out.push(s);
        amp *= decay_step;
        phase_a += TAU * f0 / rate;
        phase_b += TAU * f0 * 1.6 / rate;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt()
    }

    fn pitch(s: &[f32]) -> f32 {
        // Zero-crossing rate on the loudest 10 ms window.
        let loudest = s
            .iter()
            .enumerate()
            .fold(
                (0usize, 0f32),
                |m, (i, x)| if x.abs() > m.1 { (i, x.abs()) } else { m },
            )
            .0;
        let w_start = loudest.saturating_sub(240);
        let w_end = (loudest + 240).min(s.len());
        let w = &s[w_start..w_end];
        let crossings = w.windows(2).filter(|x| x[0] <= 0.0 && x[1] > 0.0).count() as f32;
        crossings * SAMPLE_RATE as f32 / w.len() as f32
    }

    #[test]
    fn ring_is_a_low_pitched_damped_tone() {
        let out = sculk_ring(6.0, 1, SAMPLE_RATE);
        let p = pitch(&out);
        assert!((150.0..=300.0).contains(&p), "ring pitch {p} Hz");
        assert!(out.iter().all(|x| x.is_finite() && x.abs() <= 1.2));
        assert!(out.len() < 4 * SAMPLE_RATE as usize);
        // Not silent.
        assert!(rms(&out) > 0.01);
    }

    #[test]
    fn higher_q_rings_longer() {
        let low = sculk_ring(0.5, 2, SAMPLE_RATE);
        let high = sculk_ring(20.0, 2, SAMPLE_RATE);
        let half = |v: &[f32]| rms(&v[v.len() / 2..]);
        // High Q must be at least 4x louder in the second half.
        assert!(
            half(&high) > 4.0 * half(&low),
            "high q {} vs low q {}",
            half(&high),
            half(&low)
        );
    }

    #[test]
    fn same_seed_is_deterministic() {
        assert_eq!(
            sculk_ring(6.0, 42, SAMPLE_RATE),
            sculk_ring(6.0, 42, SAMPLE_RATE)
        );
    }
}
