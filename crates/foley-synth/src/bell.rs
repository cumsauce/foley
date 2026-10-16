use foley_dsp::Rng;
use foley_dsp::SAMPLE_RATE;

use std::f32::consts::TAU;

/// A village bell's ring: a high-Q bowl ring at ~440 Hz, with two
/// inharmonic overtones (the "strike" and the "hum") that give the bell its
/// characteristic shimmer. `q` (0.5 to 20) controls how long it rings — a
/// real bell rings for a few seconds; a low Q thuds out fast. `seed` mixes
/// a small starting-phase offset so each bell block has its own distinct
/// ring.
pub fn bell(q: f32, seed: u64) -> Vec<f32> {
    // Clamp to the spec range; out-of-range callers get a default.
    let q = q.clamp(0.5, 20.0);
    // Time constant: at q=8, tau=0.5 s, so the bell rings for ~2 s.
    // Higher q rings proportionally longer.
    let tau = 0.5 * (q / 8.0);
    // The bell's fundamental ~440 Hz (concert A above middle C).
    let f0 = 440.0;
    // The strike tone at 1.18× (the bell's "strike" mode), louder than the hum.
    // The hum at 1.49×, smaller. Real bells have many more modes, but these three
    // dominate the audible ring.
    let length = (3.0 * SAMPLE_RATE as f32) as usize;
    // A small phase offset per seed so two bells don't ring exactly in step.
    let mut rng = Rng::new(seed);
    let phase0 = rng.unit() * 2.0;
    let phase1 = rng.unit() * 2.0;
    let phase2 = rng.unit() * 2.0;
    let decay_step = (-1.0 / (tau * SAMPLE_RATE as f32)).exp();
    // An 8 ms attack so the bell "swells" rather than clicks on.
    let attack = (0.008 * SAMPLE_RATE as f32) as usize;
    let mut out = Vec::with_capacity(length);
    let mut amp = 1.0f32;
    let mut phase_a = phase0;
    let mut phase_b = phase1;
    let mut phase_c = phase2;
    for i in 0..length {
        // Linear attack across `attack` samples, then exponential decay.
        let env = if i < attack {
            i as f32 / attack as f32
        } else {
            amp
        };
        let s = env * (phase_a.sin() + 0.4 * phase_b.sin() + 0.2 * phase_c.sin());
        out.push(s);
        amp *= decay_step;
        phase_a += TAU * f0 / SAMPLE_RATE as f32;
        phase_b += TAU * f0 * 1.18 / SAMPLE_RATE as f32;
        phase_c += TAU * f0 * 1.49 / SAMPLE_RATE as f32;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let w_end = (loudest + 480).min(s.len());
        let w = &s[w_start..w_end];
        let crossings = w.windows(2).filter(|x| x[0] <= 0.0 && x[1] > 0.0).count() as f32;
        crossings * SAMPLE_RATE as f32 / w.len() as f32
    }

    #[test]
    fn bell_is_a_higher_damped_tone_than_sculk() {
        let out = bell(8.0, 1);
        let p = pitch(&out);
        // Bell is at ~440 Hz. The 440 fundamental must dominate the zero-crossing count.
        assert!((350.0..=550.0).contains(&p), "bell pitch {p} Hz");
        assert!(out.iter().all(|x| x.is_finite() && x.abs() <= 2.0));
        assert!(out.len() < 4 * SAMPLE_RATE as usize);
        assert!(rms(&out) > 0.02, "bell too quiet");
    }

    #[test]
    fn higher_q_rings_longer() {
        let low = bell(0.5, 2);
        let high = bell(20.0, 2);
        let half = |v: &[f32]| rms(&v[v.len() / 2..]);
        assert!(
            half(&high) > 4.0 * half(&low),
            "high q {} vs low q {}",
            half(&high),
            half(&low)
        );
    }

    #[test]
    fn same_seed_is_deterministic() {
        assert_eq!(bell(8.0, 42), bell(8.0, 42));
    }
}
