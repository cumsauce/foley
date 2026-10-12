use std::f32::consts::TAU;

use crate::SAMPLE_RATE;

/// Minnaert's resonance of a gas bubble of radius `radius` metres in water at
/// the surface: f = (1 / 2πr) · √(3γp / ρ), with air's γ = 1.4, p = 101 325 Pa
/// and ρ = 998 kg/m³. A 1 mm bubble rings at about 3.3 kHz.
pub fn minnaert_hz(radius: f32) -> f32 {
    const STIFFNESS: f32 = 20.65; // √(3 · 1.4 · 101 325 / 998), m/s
    STIFFNESS / (TAU * radius)
}

/// A bubble's total damping constant δ (thermal, viscous and radiation losses,
/// after Devin 1959): about 0.04 to 0.05 for the millimetre bubbles drops
/// and drips trap. Its amplitude decays at π f δ a second.
const DAMPING: f32 = 0.045;

/// A bubble born in water ringing as it settles (after van den Doel's
/// liquid-sound model): a sine at the Minnaert frequency, decaying at
/// π f δ, rising in pitch as it nears the surface (by ξ times the decay rate
/// over a second, ξ = 0.1), louder the bigger it is.
#[derive(Debug, Clone)]
pub struct Bubble {
    frequency: f32,
    damping: f32,
    rise: f32,
    amplitude: f32,
    /// e^(-damping / rate): the envelope's step a sample.
    step: f32,
    /// The ring as a phasor (re, im) turned by (cos, sin) of its step a
    /// sample, refreshed every [`REFRESH`] samples as the pitch rises.
    phasor: [f32; 2],
    turn: [f32; 2],
    age: f32,
    samples: u32,
}

/// The phasor's turn follows the rising pitch every this many samples.
const REFRESH: u32 = 32;

impl Bubble {
    pub fn new(radius: f32, loudness: f32) -> Bubble {
        let frequency = minnaert_hz(radius);
        let damping = std::f32::consts::PI * frequency * DAMPING;
        Bubble {
            frequency,
            damping,
            rise: 0.1 * damping,
            // Bigger bubbles radiate more: amplitude grows with r^1.5.
            amplitude: loudness * (radius / 0.001).powf(1.5),
            step: (-damping / SAMPLE_RATE as f32).exp(),
            phasor: [1.0, 0.0],
            turn: [1.0, 0.0],
            age: 0.0,
            samples: 0,
        }
    }

    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    /// The next sample.
    pub fn sample(&mut self) -> f32 {
        let dt = 1.0 / SAMPLE_RATE as f32;
        if self.samples.is_multiple_of(REFRESH) {
            let frequency = self.frequency * (1.0 + self.rise * self.age);
            let angle = TAU * frequency * dt;
            self.turn = [angle.cos(), angle.sin()];
            // Keep the phasor on the unit circle against rounding.
            let norm = self.phasor[0].hypot(self.phasor[1]);
            self.phasor = [self.phasor[0] / norm, self.phasor[1] / norm];
        }
        self.samples += 1;
        let [re, im] = self.phasor;
        let [c, s] = self.turn;
        self.phasor = [re * c - im * s, re * s + im * c];
        self.age += dt;
        self.amplitude *= self.step;
        self.amplitude * self.phasor[1]
    }

    /// Whether it has died away below hearing (-60 dB).
    pub fn done(&self) -> bool {
        self.damping * self.age > 6.9
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_millimetre_bubble_rings_at_minnaerts_frequency() {
        assert!(
            (minnaert_hz(0.001) - 3286.0).abs() < 5.0,
            "{}",
            minnaert_hz(0.001)
        );
        // Half the size, twice the pitch.
        assert!((minnaert_hz(0.0005) / minnaert_hz(0.001) - 2.0).abs() < 1e-4);
    }

    #[test]
    fn a_bubble_rings_then_dies_away_rising_in_pitch() {
        let mut bubble = Bubble::new(0.002, 1.0);
        // Count zero crossings in the first and a later 5 ms.
        let mut samples = Vec::new();
        while !bubble.done() {
            samples.push(bubble.sample());
        }
        let window = SAMPLE_RATE as usize / 200;
        let crossings = |s: &[f32]| s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        let early = crossings(&samples[..window]);
        let late = crossings(&samples[samples.len() - window..]);
        assert!(late >= early, "pitch rises: {early} then {late}");
        let peak = |s: &[f32]| s.iter().fold(0f32, |p, x| p.max(x.abs()));
        assert!(peak(&samples[samples.len() - window..]) < 0.01 * peak(&samples[..window]));
        // Gone within a few hundred milliseconds.
        assert!(
            samples.len() < SAMPLE_RATE as usize / 2,
            "{} samples",
            samples.len()
        );
    }
}
