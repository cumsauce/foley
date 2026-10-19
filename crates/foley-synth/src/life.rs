use std::f32::consts::{PI, TAU};

use crate::{BandPass, Bird, Rng, bird_call};

/// A field cricket's chirps a minute at `celsius`: Dolbear's law (1897),
/// T°F = 50 + (N − 40)/4, which is N ≈ 7.2·T°C − 32. None below about 8 °C.
pub fn dolbear_chirps_per_minute(celsius: f32) -> f32 {
    (7.2 * celsius - 32.0).max(0.0)
}

/// A field cricket chirping for 3 s at `celsius` (M5). Each chirp is 3 or 4
/// syllables, one for each closing stroke of the wings, 15 ms long and
/// 35 ms apart; the file and scraper drive the harp, which rings a nearly
/// pure tone near 4.5 kHz (a little higher when warm). Chirps come at
/// Dolbear's rate.
/// A few field crickets chirping together for 3 s at `celsius`, each at its
/// own pitch, pace and distance: the chorus of a warm night.
pub fn crickets(celsius: f32, seed: u64, sample_rate: u32) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0xC0C0);
    let count = 3 + (rng.unit() * 4.0) as usize;
    let mut out = vec![0.0; (3.0 * sample_rate as f32) as usize];
    for k in 0..count {
        let one = cricket(
            celsius + 2.0 * rng.signed(),
            seed.wrapping_add(k as u64 * 7919),
            sample_rate,
        );
        let distance = 0.3 + 0.7 * rng.unit();
        for (o, s) in out.iter_mut().zip(&one) {
            *o += distance * s / (count as f32).sqrt();
        }
    }
    out
}

fn cricket(celsius: f32, seed: u64, sample_rate: u32) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let rate = sample_rate as f32;
    let length = (3.0 * rate) as usize;
    let mut out = vec![0.0; length];
    let per_second = dolbear_chirps_per_minute(celsius) / 60.0;
    if per_second <= 0.0 {
        return out;
    }
    let carrier = (4500.0 + 40.0 * (celsius - 20.0)) * (1.0 + 0.04 * rng.signed());
    let syllables = 3 + (rng.unit() * 2.0) as usize;
    let mut at = rng.unit() / per_second;
    while at < 2.8 {
        for s in 0..syllables {
            let start = ((at + s as f32 * 0.035) * rate) as usize;
            let pulse = (0.015 * rate) as usize;
            for i in 0..pulse {
                let Some(sample) = out.get_mut(start + i) else {
                    break;
                };
                let t = i as f32 / pulse as f32;
                // A fast rise and an exponential fall; the tone glides down
                // 3 % as the stroke slows.
                let envelope = (t * 10.0).min(1.0) * (-4.0 * t).exp();
                let f = carrier * (1.0 - 0.03 * t);
                *sample += 0.5 * envelope * (TAU * f * i as f32 / rate).sin();
            }
        }
        at += (1.0 + 0.1 * rng.signed()) / per_second;
    }
    out
}

/// A bird singing (M5): `species` picks one of the real birds Foley knows
/// ([`Bird::of`]), `seed` the individual and rendition.
pub fn birdsong(species: u32, seed: u64, sample_rate: u32) -> Vec<f32> {
    bird_call(Bird::of(species), seed, sample_rate)
}

/// A frog of body length `size_cm` calling (M5): glottal pulses, 60 to
/// 150 a second, driving the inflated vocal sac, which rings like a
/// Helmholtz resonator, lower the bigger the frog (about 2.5 kHz for a 3 cm
/// tree frog to 500 Hz for a 15 cm bullfrog), in notes of 0.1 to 0.5 s.
pub fn frog(size_cm: f32, seed: u64, sample_rate: u32) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let rate = sample_rate as f32;
    let sac = 2500.0 * (3.0 / size_cm.clamp(2.0, 20.0)) * (1.0 + 0.05 * rng.signed());
    // Some frogs croak, slow pulses in a few notes; others trill, fast ones.
    let trill = size_cm < 8.0 && rng.unit() < 0.35;
    let pulse_rate = if trill {
        180.0 + 80.0 * rng.unit()
    } else {
        60.0 + 90.0 * rng.unit()
    };
    let notes = if trill {
        1
    } else {
        1 + (rng.unit() * 3.0) as usize
    };
    let mut out = Vec::new();
    let mut resonance = [
        BandPass::new_at(sac, 8.0, sample_rate),
        BandPass::new_at(2.3 * sac, 6.0, sample_rate),
    ];
    let mut breath = BandPass::new_at(sac * 0.5, 1.0, sample_rate);
    for _ in 0..notes {
        let seconds = if trill {
            0.6 + 0.8 * rng.unit()
        } else {
            0.1 + 0.4 * rng.unit()
        };
        let samples = (seconds * rate) as usize;
        let width = (0.001 * rate) as usize;
        // Each pulse a little early or late and a little louder or softer
        // than the last, as a living larynx is.
        let mut next = 0usize;
        let mut pulse_at = usize::MAX;
        let mut strength = 1.0;
        for i in 0..samples {
            let t = i as f32 / samples as f32;
            if i >= next {
                pulse_at = i;
                strength = 0.75 + 0.5 * rng.unit();
                let period = rate / pulse_rate * (1.0 + 0.05 * rng.signed());
                next = i + period as usize;
                // The sac swells through the note, and rings a little higher.
                let f = sac * (1.0 + 0.06 * t);
                resonance[0].tune_at(f, 8.0, sample_rate);
                resonance[1].tune_at(2.3 * f, 6.0, sample_rate);
            }
            let envelope = (t * 8.0).min(1.0) * ((1.0 - t) * 6.0).min(1.0);
            let phase = i.wrapping_sub(pulse_at);
            let pulse = if phase < width {
                strength * (PI * phase as f32 / width as f32).sin()
            } else {
                0.0
            };
            let excitation = envelope * pulse;
            out.push(
                4.0 * resonance[0].process(excitation) + 1.5 * resonance[1].process(excitation),
            );
        }
        // Air drawn back from the sac into the lungs between notes.
        let gap = ((0.05 + 0.15 * rng.unit()) * rate) as usize;
        for i in 0..gap {
            let t = i as f32 / gap as f32;
            out.push(0.04 * (PI * t).sin() * breath.process(rng.signed()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;

    fn peak(s: &[f32]) -> f32 {
        s.iter().fold(0f32, |p, x| p.max(x.abs()))
    }

    #[test]
    fn crickets_chirp_faster_when_warm() {
        assert!((dolbear_chirps_per_minute(20.0) - 112.0).abs() < 1.0);
        assert_eq!(dolbear_chirps_per_minute(4.0), 0.0);
        // Count chirps: runs of sound separated by more than 50 ms of quiet.
        let chirps = |s: &[f32]| {
            let mut count = 0;
            let mut quiet = usize::MAX;
            for x in s {
                if x.abs() > 0.01 {
                    if quiet > 2400 {
                        count += 1;
                    }
                    quiet = 0;
                } else {
                    quiet = quiet.saturating_add(1);
                }
            }
            count
        };
        let (cool, warm) = (
            chirps(&cricket(14.0, 1, SAMPLE_RATE)),
            chirps(&cricket(28.0, 1, SAMPLE_RATE)),
        );
        assert!(warm > cool, "{cool} then {warm}");
        assert!(crickets(4.0, 1, SAMPLE_RATE).iter().all(|x| *x == 0.0));
        assert!(peak(&crickets(25.0, 2, SAMPLE_RATE)) > 0.1);
    }

    #[test]
    fn a_big_frog_croaks_lower() {
        let crossings = |s: &[f32]| {
            s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f32 / s.len() as f32
        };
        let (small, big) = (frog(3.0, 1, SAMPLE_RATE), frog(15.0, 1, SAMPLE_RATE));
        assert!(crossings(&big) < crossings(&small));
        assert!(peak(&small) > 0.05 && peak(&big) > 0.05);
        assert!(small.iter().chain(&big).all(|x| x.is_finite()));
    }
}
