use crate::{Bubble, Rng, SAMPLE_RATE};

/// The smallest and largest bubbles a raindrop traps on hitting water, in
/// metres (about 0.2 to 1.5 mm, which ring at about 16 to 2 kHz).
const SMALLEST: f32 = 0.0002;
const LARGEST: f32 = 0.0015;
/// At most this many bubbles ring at once; past it, new drops are skipped.
const MAX_BUBBLES: usize = 256;

/// Rain falling on water (the M0 spike of the rain bed, docs/PLAN.md): drops
/// arrive as a Poisson process, each trapping a bubble whose size is drawn
/// log-uniformly, so the sound is the sum of many Minnaert pings, plus the
/// splash's broadband hiss.
#[derive(Debug)]
pub struct RainOnWater {
    rng: Rng,
    /// Drops a second.
    rate: f32,
    until_next: f32,
    bubbles: Vec<Bubble>,
    hiss: f32,
}

impl RainOnWater {
    pub fn new(seed: u64, rate: f32) -> RainOnWater {
        let mut rng = Rng::new(seed);
        let until_next = rng.exponential(rate);
        RainOnWater {
            rng,
            rate,
            until_next,
            bubbles: Vec::new(),
            hiss: 0.0,
        }
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate.max(0.0);
    }

    /// The next `out.len()` samples.
    pub fn render(&mut self, out: &mut [f32]) {
        let dt = 1.0 / SAMPLE_RATE as f32;
        for sample in out {
            self.until_next -= dt;
            while self.until_next <= 0.0 {
                if self.bubbles.len() < MAX_BUBBLES && self.rate > 0.0 {
                    let radius = SMALLEST * (LARGEST / SMALLEST).powf(self.rng.unit());
                    self.bubbles
                        .push(Bubble::new(radius, 0.02 + 0.03 * self.rng.unit()));
                }
                self.until_next += self.rng.exponential(self.rate.max(1e-3));
            }
            let mut mix = 0.0;
            for bubble in &mut self.bubbles {
                mix += bubble.sample();
            }
            self.bubbles.retain(|b| !b.done());
            // The splashes' hiss: white noise smoothed a little, louder with more drops.
            self.hiss += 0.3 * (self.rng.signed() - self.hiss);
            *sample = mix + self.hiss * 0.0015 * self.rate.sqrt();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heavier_rain_is_louder_and_never_silent() {
        let energy = |rate: f32| {
            let mut bed = RainOnWater::new(1, rate);
            let mut out = vec![0.0; SAMPLE_RATE as usize];
            bed.render(&mut out);
            out.iter().map(|s| s * s).sum::<f32>()
        };
        let (light, heavy) = (energy(50.0), energy(800.0));
        assert!(light > 0.0 && heavy > 2.0 * light, "{light} vs {heavy}");
    }
}
