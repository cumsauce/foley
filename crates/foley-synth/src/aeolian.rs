//! Aeolian tones (M18): Strouhal whistles from obstacles the air blows past.
//! f ≈ 0.2·U/d, silent below U = 2 m/s (the vortex shedding is too weak to hear).

const MIN_SPEED: f32 = 2.0;

/// An Aeolian tone: a Strouhal whistle at `f ≈ 0.2·U/d`, where `d` is the
/// obstacle's characteristic diameter (an iron bar ~40 mm, a fence rail ~25 mm,
/// a lantern finial ~80 mm, a bell rim ~150 mm, a bamboo shoot ~50 mm), and
/// `U` is the wind speed (m/s). Silent when `U < 2 m/s`. The whistle is a
/// pure sinusoid at the Strouhal frequency with a slow amplitude vibrato so it
/// breathes rather than drones, plus a small octave overtone so it has a body,
/// not a pure tone. 50 ms attack, 200 ms release.
pub fn aeolian(
    diameter_m: f32,
    wind_speed_m_s: f32,
    seconds: f32,
    gain: f32,
    seed: u64,
    sample_rate: u32,
) -> Vec<f32> {
    let rate = sample_rate as f32;
    // Below the threshold: return a silent short buffer so the dispatcher doesn't waste cycles.
    if wind_speed_m_s < MIN_SPEED {
        return vec![0.0; (0.05 * rate) as usize];
    }
    let f = 0.2 * wind_speed_m_s / diameter_m;
    let length = (seconds * rate) as usize;
    let mut out = vec![0.0f32; length];
    // Mix the seed into the phase so each call has a stable but varied starting phase.
    let phase_seed = (seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)) as f32 / u64::MAX as f32;
    let mut phase = phase_seed * std::f32::consts::TAU;
    let mut octave_phase = phase_seed * std::f32::consts::TAU * 0.7;
    let attack = (0.05 * rate) as usize;
    let release = (0.20 * rate) as usize;
    let vibrato_hz = 4.0;
    let vibrato_depth = 0.04;
    for (i, sample) in out.iter_mut().enumerate() {
        let t = i as f32 / rate;
        // 50 ms rise, sustain, 200 ms fall.
        let env = if i < attack {
            i as f32 / attack as f32
        } else if i > length - release {
            (length - i) as f32 / release as f32
        } else {
            1.0
        };
        // Slow amplitude vibrato so the whistle breathes, not drones.
        let vibrato = 1.0 + vibrato_depth * (std::f32::consts::TAU * vibrato_hz * t).sin();
        // Tiny frequency vibrato: a few cents up and down so the carrier isn't perfectly steady.
        let f_now = f * (1.0 + 0.005 * (std::f32::consts::TAU * 5.0 * t).sin());
        phase += std::f32::consts::TAU * f_now / rate;
        octave_phase += std::f32::consts::TAU * 2.0 * f_now / rate;
        *sample = env * vibrato * (phase.sin() + 0.15 * octave_phase.sin());
    }
    crate::dsp::normalize_in_place(&mut out, gain);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;

    fn loudest_at(out: &[f32]) -> f32 {
        out.iter().fold(0f32, |m, x| m.max(x.abs()))
    }

    /// Zero-crossing pitch in a 200 ms window after the attack.
    fn pitch(out: &[f32]) -> f32 {
        let rate = SAMPLE_RATE as f32;
        let start = (0.05 * rate) as usize;
        let end = start + (0.2 * rate) as usize;
        let w = &out[start.min(out.len())..end.min(out.len())];
        w.windows(2).filter(|x| x[0] <= 0.0 && x[1] > 0.0).count() as f32 / (w.len() as f32 / rate)
    }

    #[test]
    fn silent_below_two_metres_per_second() {
        // At 1.5 m/s with a 40 mm iron bar, the impl must return a near-silent buffer.
        let out = aeolian(0.040, 1.5, 0.5, 1.0, 1, SAMPLE_RATE);
        assert!(out.len() < 4 * SAMPLE_RATE as usize);
        assert!(
            loudest_at(&out) < 0.05,
            "silent below 2 m/s: peak={}",
            loudest_at(&out)
        );
    }

    #[test]
    fn whistle_pitch_is_strouhal() {
        // d = 25 mm, U = 10 m/s: Strouhal f ≈ 0.2·10/0.025 = 80 Hz.
        let out = aeolian(0.025, 10.0, 0.8, 1.0, 2, SAMPLE_RATE);
        let p = pitch(&out);
        assert!((p - 80.0).abs() < 8.0, "pitch {p} Hz, want ~80");
        assert!(loudest_at(&out) > 0.1);
    }

    #[test]
    fn smaller_diameter_pitches_higher() {
        // d = 25 mm vs d = 100 mm at the same U = 8 m/s: the small one pitches higher.
        let small = aeolian(0.025, 8.0, 0.8, 1.0, 3, SAMPLE_RATE);
        let large = aeolian(0.100, 8.0, 0.8, 1.0, 3, SAMPLE_RATE);
        assert!(pitch(&small) > pitch(&large));
    }

    #[test]
    fn same_seed_is_deterministic() {
        assert_eq!(
            aeolian(0.025, 10.0, 0.5, 1.0, 7, SAMPLE_RATE),
            aeolian(0.025, 10.0, 0.5, 1.0, 7, SAMPLE_RATE)
        );
    }

    #[test]
    fn bounded_and_short_enough() {
        let out = aeolian(0.040, 12.0, 0.5, 1.0, 9, SAMPLE_RATE);
        assert!(out.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
        assert!(out.len() <= (2.0 * SAMPLE_RATE as f32) as usize);
    }
}
