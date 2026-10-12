use std::f32::consts::{PI, TAU};

use crate::{BandPass, Rng, SAMPLE_RATE};

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
pub fn crickets(celsius: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let rate = SAMPLE_RATE as f32;
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

/// One syllable of a song: a pitch contour, how long, and how it's sung.
#[derive(Debug, Clone, Copy)]
struct Syllable {
    from: f32,
    to: f32,
    seconds: f32,
    /// A trill's rate (Hz) and depth (fraction of pitch), or 0.
    trill: (f32, f32),
}

/// A species' song: its syllables and the order it sings them in.
#[derive(Debug, Clone)]
struct Song {
    syllables: Vec<Syllable>,
    order: Vec<usize>,
    gap: f32,
}

impl Song {
    /// A species, made up from `species`: 2 to 4 kinds of syllable between
    /// 1.5 and 8 kHz (whistles, sweeps up and down, trills), sung in a
    /// motif of 4 to 10 with repeats.
    fn of(species: u32) -> Song {
        let mut rng = Rng::new(0xB12D_0000 + u64::from(species));
        let low = 1500.0 + 2500.0 * rng.unit();
        let span = 1.0 + 1.0 * rng.unit();
        let mut syllables = Vec::new();
        for _ in 0..2 + (rng.unit() * 3.0) as usize {
            let a = low * span.powf(rng.unit());
            let b = low * span.powf(rng.unit());
            let trill = if rng.unit() < 0.3 {
                (20.0 + 40.0 * rng.unit(), 0.05 + 0.1 * rng.unit())
            } else {
                (0.0, 0.0)
            };
            syllables.push(Syllable {
                from: a,
                to: b,
                seconds: 0.04 + 0.25 * rng.unit(),
                trill,
            });
        }
        let mut order = Vec::new();
        let motif = 4 + (rng.unit() * 7.0) as usize;
        while order.len() < motif {
            let kind = (rng.unit() * syllables.len() as f32) as usize;
            for _ in 0..1 + (rng.unit() * 3.0) as usize {
                order.push(kind);
            }
        }
        Song {
            syllables,
            order,
            gap: 0.02 + 0.08 * rng.unit(),
        }
    }
}

/// A bird of `species` singing its song once (M5), through a model of the
/// syrinx: its labia as a self-sustained (van der Pol) oscillator,
/// x″ = μω(p − x²)x′ − ω²x, set going by the air sac's pressure p and tuned
/// by the syringeal muscles' tension (ω), as in Mindlin and Laje's work.
/// Below threshold pressure it falls silent; the pitch follows each
/// syllable's contour. The trachea and beak, which songbirds tune to the
/// note, take off most of the overtones.
pub fn birdsong(species: u32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let song = Song::of(species);
    let rate = SAMPLE_RATE as f32;
    let dt = 1.0 / rate;
    let detune = 1.0 + 0.03 * rng.signed();
    // Sometimes a song is cut short.
    let sung = if rng.unit() < 0.25 {
        (song.order.len() / 2).max(2)
    } else {
        song.order.len()
    };
    let mut out = Vec::new();
    let (mut x, mut v) = (0.01f32, 0.0f32);
    let mut tract = BandPass::new(3000.0, 3.0);
    for &kind in &song.order[..sung] {
        let syllable = song.syllables[kind];
        let seconds = syllable.seconds * (1.0 + 0.08 * rng.signed());
        let samples = (seconds * rate) as usize;
        let gap = ((song.gap * (1.0 + 0.2 * rng.signed())) * rate) as usize;
        for i in 0..samples + gap {
            let t = i as f32 / samples.max(1) as f32;
            let singing = i < samples;
            // Pressure: up in 5 ms, held, down in the last 10 ms; below
            // zero between syllables, which quenches the labia.
            let p = if singing {
                let attack = (i as f32 / (0.005 * rate)).min(1.0);
                let release = ((samples - i) as f32 / (0.01 * rate)).min(1.0);
                attack * release
            } else {
                -1.0
            };
            let mut f = syllable.from * (syllable.to / syllable.from).powf(t.min(1.0)) * detune;
            if syllable.trill.0 > 0.0 {
                f *= 1.0 + syllable.trill.1 * (TAU * syllable.trill.0 * i as f32 * dt).sin();
            }
            // Pre-warped so the integrator rings at f.
            let omega = 2.0 * rate * (PI * f * dt).sin();
            let mu = 0.15;
            // Semi-implicit Euler: stable for ωdt < 2.
            v += dt * (mu * omega * (p - x * x) * v - omega * omega * x);
            x += dt * v;
            if !singing && x.abs() < 1e-4 {
                x = 0.01;
                v = 0.0;
            }
            tract.tune(f, 3.0);
            let s = 0.25 * (0.7 * tract.process(x) + 0.3 * x * p.max(0.0));
            out.push(s);
        }
    }
    out.extend(std::iter::repeat_n(0.0, (0.05 * rate) as usize));
    out
}

/// A frog of body length `size_cm` calling (M5): glottal pulses, 60 to
/// 150 a second, driving the inflated vocal sac, which rings like a
/// Helmholtz resonator, lower the bigger the frog (about 2.5 kHz for a 3 cm
/// tree frog to 500 Hz for a 15 cm bullfrog), in notes of 0.1 to 0.5 s.
pub fn frog(size_cm: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let rate = SAMPLE_RATE as f32;
    let sac = 2500.0 * (3.0 / size_cm.clamp(2.0, 20.0)) * (1.0 + 0.05 * rng.signed());
    let pulse_rate = 60.0 + 90.0 * rng.unit();
    let notes = 1 + (rng.unit() * 3.0) as usize;
    let mut out = Vec::new();
    let mut resonance = [BandPass::new(sac, 8.0), BandPass::new(2.3 * sac, 6.0)];
    for _ in 0..notes {
        let seconds = 0.1 + 0.4 * rng.unit();
        let samples = (seconds * rate) as usize;
        let period = (rate / pulse_rate) as usize;
        let width = (0.001 * rate) as usize;
        for i in 0..samples {
            let t = i as f32 / samples as f32;
            let envelope = (t * 8.0).min(1.0) * ((1.0 - t) * 6.0).min(1.0);
            let phase = i % period;
            let pulse = if phase < width {
                (PI * phase as f32 / width as f32).sin()
            } else {
                0.0
            };
            let excitation = envelope * pulse;
            out.push(
                4.0 * resonance[0].process(excitation) + 1.5 * resonance[1].process(excitation),
            );
        }
        out.extend(std::iter::repeat_n(
            0.0,
            ((0.05 + 0.15 * rng.unit()) * rate) as usize,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let (cool, warm) = (chirps(&crickets(14.0, 1)), chirps(&crickets(28.0, 1)));
        assert!(warm > cool, "{cool} then {warm}");
        assert!(crickets(4.0, 1).iter().all(|x| *x == 0.0));
        assert!(peak(&crickets(25.0, 2)) > 0.1);
    }

    #[test]
    fn a_species_sings_its_own_song() {
        let a = birdsong(3, 1);
        let b = birdsong(3, 2);
        let c = birdsong(7, 1);
        for s in [&a, &b, &c] {
            assert!(s.iter().all(|x| x.is_finite()));
            assert!(peak(s) > 0.05 && peak(s) < 2.0, "{}", peak(s));
            assert!(s.len() < 6 * SAMPLE_RATE as usize);
        }
        // Pitch in the birds' range: zero crossings in the loudest 20 ms.
        let i = a
            .iter()
            .enumerate()
            .fold(
                (0, 0f32),
                |m, (i, x)| if x.abs() > m.1 { (i, x.abs()) } else { m },
            )
            .0;
        let window = &a[i.saturating_sub(480)..(i + 480).min(a.len())];
        let hz = window
            .windows(2)
            .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
            .count() as f32
            / (window.len() as f32 / SAMPLE_RATE as f32);
        assert!((1200.0..9000.0).contains(&hz), "{hz} Hz");
    }

    #[test]
    fn a_big_frog_croaks_lower() {
        let crossings = |s: &[f32]| {
            s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f32 / s.len() as f32
        };
        let (small, big) = (frog(3.0, 1), frog(15.0, 1));
        assert!(crossings(&big) < crossings(&small));
        assert!(peak(&small) > 0.05 && peak(&big) > 0.05);
        assert!(small.iter().chain(&big).all(|x| x.is_finite()));
    }
}
