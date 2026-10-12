use std::f32::consts::TAU;

use crate::SAMPLE_RATE;

/// A band-pass biquad (RBJ's cookbook, constant 0 dB peak gain), for
/// colouring noise.
#[derive(Debug, Clone, Default)]
pub struct BandPass {
    b: [f32; 3],
    a: [f32; 2],
    x: [f32; 2],
    y: [f32; 2],
}

impl BandPass {
    pub fn new(centre: f32, q: f32) -> BandPass {
        let mut filter = BandPass::default();
        filter.tune(centre, q);
        filter
    }

    /// Moves the band, keeping the filter's state so it doesn't click.
    pub fn tune(&mut self, centre: f32, q: f32) {
        let w = TAU * centre.clamp(20.0, 0.45 * SAMPLE_RATE as f32) / SAMPLE_RATE as f32;
        let alpha = w.sin() / (2.0 * q.max(0.1));
        let a0 = 1.0 + alpha;
        self.b = [alpha / a0, 0.0, -alpha / a0];
        self.a = [-2.0 * w.cos() / a0, (1.0 - alpha) / a0];
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b[0] * x + self.b[1] * self.x[0] + self.b[2] * self.x[1]
            - self.a[0] * self.y[0]
            - self.a[1] * self.y[1];
        self.x = [x, self.x[0]];
        self.y = [y, self.y[0]];
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rng;

    #[test]
    fn it_passes_its_band_and_not_far_from_it() {
        let energy = |hz: f32| {
            let mut filter = BandPass::new(1000.0, 2.0);
            (0..SAMPLE_RATE)
                .map(|i| {
                    filter
                        .process((TAU * hz * i as f32 / SAMPLE_RATE as f32).sin())
                        .powi(2)
                })
                .skip(1000)
                .sum::<f32>()
        };
        assert!(energy(1000.0) > 20.0 * energy(100.0));
        assert!(energy(1000.0) > 20.0 * energy(10_000.0));
        let mut rng = Rng::new(1);
        let mut filter = BandPass::new(500.0, 1.0);
        assert!((0..48_000).all(|_| filter.process(rng.signed()).is_finite()));
    }
}
