use crate::{BandPass, Rng, SAMPLE_RATE};

/// What the wind round the listener is like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindScene {
    /// Mean wind speed at the listener, m/s.
    pub speed: f32,
    /// How open to it the listener is: 1 out in the open, 0 shut indoors.
    pub exposure: f32,
    /// How much of what's round about is leaves, 0 to 1.
    pub leaves: f32,
}

impl WindScene {
    pub const CALM: WindScene = WindScene {
        speed: 0.0,
        exposure: 0.0,
        leaves: 0.0,
    };
}

/// Gusts: the speed wanders round its mean as an Ornstein–Uhlenbeck process,
/// with this turbulence intensity (standard deviation over mean)...
const TURBULENCE: f32 = 0.25;
/// ...and this time scale, seconds.
const GUST_TIME: f32 = 4.0;
/// The speed is updated this often, samples.
const BLOCK: usize = 64;

/// The wind (M1): turbulence heard as band-passed noise, its loudness rising
/// with speed cubed (aerodynamic noise power goes as U⁶) and its pitch with
/// speed (the eddies shed off things go as Strouhal's f ≈ 0.2 U / d); gusts
/// from an Ornstein–Uhlenbeck process; leaves rustling as bursts of bright
/// noise, more of them the harder it blows (rate ∝ U²). Indoors it comes in
/// low and muffled.
#[derive(Debug)]
pub struct Wind {
    rng: Rng,
    scene: WindScene,
    speed: f32,
    roar: BandPass,
    whoosh: BandPass,
    rustle: BandPass,
    rustle_envelope: f32,
    until_rustle: f32,
    samples: usize,
}

impl Wind {
    pub fn new(seed: u64) -> Wind {
        Wind {
            rng: Rng::new(seed),
            scene: WindScene::CALM,
            speed: 0.0,
            roar: BandPass::new(150.0, 0.7),
            whoosh: BandPass::new(600.0, 1.2),
            rustle: BandPass::new(4000.0, 1.5),
            rustle_envelope: 0.0,
            until_rustle: 0.0,
            samples: 0,
        }
    }

    pub fn set_scene(&mut self, scene: WindScene) {
        self.scene = scene;
    }

    /// The gusting speed now, m/s.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    fn gust(&mut self) {
        let dt = BLOCK as f32 / SAMPLE_RATE as f32;
        let mean = self.scene.speed.max(0.0);
        let sigma = TURBULENCE * mean;
        // dU = (mean - U) dt / τ + σ √(2 dt / τ) dW
        let noise = self.rng.signed() * 1.732; // unit variance from uniform
        self.speed +=
            (mean - self.speed) * dt / GUST_TIME + sigma * (2.0 * dt / GUST_TIME).sqrt() * noise;
        self.speed = self.speed.max(0.0);
        let indoors = 1.0 - self.scene.exposure.clamp(0.0, 1.0);
        // Pitch rises with speed; indoors only the low part gets through.
        let centre = (80.0 + 25.0 * self.speed) * (1.0 - 0.6 * indoors);
        self.roar.tune(centre, 0.7);
        self.whoosh.tune(centre * 4.0 * (1.0 - 0.7 * indoors), 1.2);
    }

    /// The next `out.len()` samples.
    pub fn render(&mut self, out: &mut [f32]) {
        let dt = 1.0 / SAMPLE_RATE as f32;
        let exposure = self.scene.exposure.clamp(0.0, 1.0);
        for sample in out {
            if self.samples.is_multiple_of(BLOCK) {
                self.gust();
            }
            self.samples += 1;
            let u = self.speed;
            // Amplitude ∝ U³, against a 10 m/s wind; muffled to a tenth indoors.
            let level = 0.06 * (u / 10.0).powi(3).min(8.0) * (0.1 + 0.9 * exposure);
            let roar = self.roar.process(self.rng.signed());
            let whoosh = self.whoosh.process(self.rng.signed());
            let mut out = level * (roar * 1.5 + whoosh * 0.6 * exposure);
            // Leaves: bursts at a rate ∝ U², each a few tens of ms of bright noise.
            if self.scene.leaves > 0.0 && u > 0.5 {
                self.until_rustle -= dt;
                if self.until_rustle <= 0.0 {
                    self.rustle_envelope =
                        (self.rustle_envelope + 0.5 + 0.5 * self.rng.unit()).min(1.5);
                    let rate = 2.0 * u * u * self.scene.leaves;
                    self.until_rustle = self.rng.exponential(rate);
                }
                self.rustle_envelope *= 0.9993;
                let rustle = self.rustle.process(self.rng.signed());
                out += 0.02
                    * self.rustle_envelope
                    * rustle
                    * self.scene.leaves
                    * (u / 10.0).min(2.0)
                    * exposure;
            }
            *sample = out;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listen(scene: WindScene) -> f32 {
        let mut wind = Wind::new(5);
        wind.set_scene(scene);
        let mut out = vec![0.0; 4 * SAMPLE_RATE as usize];
        wind.render(&mut out);
        out.iter().map(|s| s * s).sum::<f32>()
    }

    #[test]
    fn stronger_wind_is_much_louder_and_indoors_quieter() {
        let open = |speed| WindScene {
            speed,
            exposure: 1.0,
            leaves: 0.0,
        };
        let (breeze, gale) = (listen(open(4.0)), listen(open(15.0)));
        // Energy ∝ U⁶: nearly 3 000 times from 4 to 15 m/s; at least 100 here.
        assert!(gale > 100.0 * breeze, "{breeze} vs {gale}");
        let inside = listen(WindScene {
            exposure: 0.0,
            ..open(15.0)
        });
        assert!(inside < gale / 20.0, "indoors {inside} vs {gale}");
        assert_eq!(listen(WindScene::CALM), 0.0);
    }

    #[test]
    fn gusts_wander_round_the_mean() {
        let mut wind = Wind::new(9);
        wind.set_scene(WindScene {
            speed: 10.0,
            exposure: 1.0,
            leaves: 0.5,
        });
        let mut out = vec![0.0; BLOCK];
        let mut speeds = Vec::new();
        for _ in 0..(60 * SAMPLE_RATE as usize / BLOCK) {
            wind.render(&mut out);
            speeds.push(wind.speed());
        }
        let tail = &speeds[speeds.len() / 2..];
        let mean = tail.iter().sum::<f32>() / tail.len() as f32;
        let sd = (tail.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / tail.len() as f32).sqrt();
        assert!((mean - 10.0).abs() < 2.0, "mean {mean}");
        assert!(sd > 0.8 && sd < 4.5, "gusts {sd}");
    }
}
