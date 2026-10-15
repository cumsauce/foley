//! Parameterized primitives, for the integration API to compose into sounds
//! it invents (F2): a bubble's ring, a burst of shaped noise, a tone, a gust
//! of wind. Each is rendered to a one-shot at [`SAMPLE_RATE`], bounded and
//! peak-normalized, so anything an addon builds from them stays civil.

use crate::{BandPass, Bubble, Rng, SAMPLE_RATE};

/// A bubble's Minnaert ring (M0), radius in mm, gone in tens of
/// milliseconds; its pitch wavers a little with the seed, as a real bubble's
/// does as it wobbles.
pub fn bubble(radius_mm: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let radius = radius_mm / 1000.0 * (1.0 + 0.02 * rng.signed());
    let mut ring = Bubble::new(radius, 1.0);
    let mut out = Vec::with_capacity(SAMPLE_RATE as usize / 2);
    for _ in 0..SAMPLE_RATE / 4 {
        let sample = ring.sample();
        out.push(sample);
        if sample.abs() < 1e-4 && out.len() > 240 {
            break;
        }
    }
    crate::dsp::normalize_in_place(&mut out, 0.9);
    out
}

/// A burst of noise through a band `q` wide round `centre_hz`, over
/// `seconds`, peaking at `gain`: the texture of splashes, hisses, crunches
/// and whooshes.
pub fn noise_burst(centre_hz: f32, q: f32, seconds: f32, gain: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let rate = SAMPLE_RATE as f32;
    let length = (seconds * rate) as usize;
    let mut band = BandPass::new(centre_hz * (1.0 + 0.02 * rng.signed()), q);
    let mut out = vec![0.0f32; length];
    // A swift attack, then an exponential fall, as an impulse's energy dies.
    let decay = (-6.0 / length.max(1) as f32).exp();
    let mut envelope = 1.0f32;
    for sample in &mut out {
        *sample = envelope * band.process(rng.signed());
        envelope *= decay;
    }
    crate::dsp::normalize_in_place(&mut out, gain);
    out
}

/// A tone at `centre_hz` gliding to `target_hz` over `seconds`, peaking at
/// `gain`, with a light vibrato and a soft attack and release: whistles,
/// chimes, chirps.
pub fn tone(centre_hz: f32, target_hz: f32, seconds: f32, gain: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let rate = SAMPLE_RATE as f32;
    let length = (seconds * rate) as usize;
    let vibrato_hz = 4.0 + 3.0 * rng.unit();
    let vibrato = 0.002 + 0.004 * rng.unit();
    let mut out = vec![0.0f32; length];
    let mut phase = 0.0f32;
    let attack = (0.01 * rate) as usize;
    for (i, sample) in out.iter_mut().enumerate() {
        let t = i as f32 / length as f32;
        // Exponential glide, so the pitch's interval moves evenly.
        let hz = centre_hz * (target_hz / centre_hz).powf(t)
            * (1.0 + vibrato * (std::f32::consts::TAU * vibrato_hz * i as f32 / rate).sin());
        phase += std::f32::consts::TAU * hz / rate;
        let envelope = ((i as f32 / attack as f32).min(1.0)
            * (1.0 - (i as f32 / length as f32).powi(3)))
        .clamp(0.0, 1.0);
        *sample = envelope * phase.sin();
    }
    crate::dsp::normalize_in_place(&mut out, gain);
    out
}

/// A gust at `speed` m/s over `seconds`, peaking at `gain`: band-passed
/// noise whose pitch and level rise with the speed, surging a few times as
/// real gusts do.
pub fn gust(speed_m_s: f32, seconds: f32, gain: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let rate = SAMPLE_RATE as f32;
    let length = (seconds * rate) as usize;
    let mut out = vec![0.0f32; length];
    // Three staggered surges, each its own band round the speed's pitch.
    let surges = 3;
    for surge in 0..surges {
        let at = (length as f32 * surge as f32 / surges as f32 * 0.9) as usize;
        let span = (length / surges).max((0.3 * rate) as usize).min(length - at);
        let centre = (80.0 + 40.0 * speed_m_s) * (0.85 + 0.3 * rng.unit());
        let mut band = BandPass::new(centre, 0.7);
        let decay = (-2.0 / span as f32).exp();
        let mut envelope = 0.4 + 0.6 * rng.unit();
        for sample in &mut out[at..at + span] {
            *sample += envelope * band.process(rng.signed());
            envelope *= decay;
        }
    }
    // The louder the wind, the louder the gust (power ∝ U⁶, level ∝ U³,
    // against a 20 m/s reference).
    let level = (speed_m_s / 20.0).powi(3).clamp(0.02, 1.5);
    crate::dsp::normalize_in_place(&mut out, (gain * level).min(1.0));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_primitive_renders_bounded_sound() {
        let made: Vec<(&str, Vec<f32>)> = vec![
            ("bubble", bubble(1.0, 1)),
            ("noise", noise_burst(2000.0, 1.2, 0.3, 0.8, 2)),
            ("tone", tone(440.0, 880.0, 0.5, 0.6, 3)),
            ("gust", gust(15.0, 3.0, 1.0, 4)),
        ];
        for (what, sound) in &made {
            assert!(!sound.is_empty(), "{what}");
            assert!(*sound.last().unwrap() < 1e-3, "{what} ends on a click");
            assert!(sound.iter().all(|x| x.is_finite()), "{what}");
            let peak = sound.iter().fold(0f32, |p, x| p.max(x.abs()));
            assert!(peak <= 1.0, "{what} peaks at {peak}");
        }
        // The bubble's ring is short; the tone is as long as asked; the gust
        // sounds of wind, not of silence.
        assert!(made[0].1.len() < SAMPLE_RATE as usize / 8);
        assert_eq!(made[2].1.len(), (0.5 * SAMPLE_RATE as f32) as usize);
        assert!(made[3].1.iter().fold(0f32, |p, x| p.max(x.abs())) > 0.2);
    }

    #[test]
    fn a_bigger_bubble_rings_lower() {
        let crossings = |s: &[f32]| {
            s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f32 / s.len() as f32
        };
        assert!(crossings(&bubble(0.3, 5)) > crossings(&bubble(4.0, 5)));
    }

    #[test]
    fn a_gust_blowing_harder_is_louder() {
        let peak = |s: &[f32]| s.iter().fold(0f32, |p, x| p.max(x.abs()));
        assert!(peak(&gust(40.0, 3.0, 1.0, 6)) > 4.0 * peak(&gust(8.0, 3.0, 1.0, 6)));
    }
}
