#[allow(unused_imports)] // Some of these are in core; the rest come from std.
use foley_math::Float as _;

use crate::Fixed;
use core::f32::consts::TAU;

use crate::{BandPass, Bubble, Rng, SAMPLE_RATE};

/// What rain falls on, as sound sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Surface {
    /// Ponds, rivers, the sea: drops trap ringing bubbles.
    Water,
    /// Stone, concrete, bricks: a sharp tick.
    Stone,
    /// Leaves: a soft, damped patter.
    Leaves,
    /// Planks and logs: a dull knock.
    Wood,
    /// Grass, dirt, sand, wool, snow: little more than a thud.
    Soft,
    /// Glass: a bright ring.
    Glass,
    /// Metal: a tinny ring.
    Metal,
}

impl Surface {
    pub const ALL: [Surface; 7] = [
        Surface::Water,
        Surface::Stone,
        Surface::Leaves,
        Surface::Wood,
        Surface::Soft,
        Surface::Glass,
        Surface::Metal,
    ];

    pub fn from_index(index: usize) -> Option<Surface> {
        Surface::ALL.get(index).copied()
    }

    /// How bright the hiss of many far drops on it is, 0 (dull) to 1.
    fn brightness(self) -> f32 {
        match self {
            Surface::Water => 0.8,
            Surface::Stone | Surface::Glass | Surface::Metal => 0.7,
            Surface::Wood => 0.45,
            Surface::Leaves => 0.35,
            Surface::Soft => 0.25,
        }
    }

    /// How loud a drop on it is, against stone.
    fn loudness(self) -> f32 {
        match self {
            Surface::Water | Surface::Stone => 1.0,
            Surface::Glass | Surface::Metal => 1.2,
            Surface::Wood => 0.8,
            Surface::Leaves => 0.6,
            Surface::Soft => 0.15,
        }
    }
}

/// What the rain around the listener is like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RainScene {
    /// Rain rate, mm/h: 2 is light, 10 moderate, 50 a downpour.
    pub rate: f32,
    /// The share of what the rain lands on near the listener, by [`Surface`].
    pub surfaces: [f32; 7],
    /// What the listener is sheltering under, if anything, and how far above
    /// the ear, in blocks.
    pub roof: Option<(Surface, f32)>,
}

impl RainScene {
    pub const DRY: RainScene = RainScene {
        rate: 0.0,
        surfaces: [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        roof: None,
    };
}

/// Marshall–Palmer's drop size distribution, N(D) = N₀ e^(-ΛD), has
/// Λ = 4.1 R^-0.21 per mm for a rain rate R in mm/h.
fn marshall_palmer_lambda(rate: f32) -> f32 {
    4.1 * rate.max(0.1).powf(-0.21)
}

/// A drop's terminal speed in m/s for a diameter in mm (Gunn and Kinzer's
/// measurements, as fitted by Atlas et al.).
fn terminal_speed(diameter: f32) -> f32 {
    (9.65 - 10.3 * (-0.6 * diameter).exp()).max(0.1)
}

/// Drops near enough to be heard one by one: at most this many a second.
const MAX_DROPS: f32 = 2500.0;
/// Of the ground's drop flux, those on this much area (m²) right round the
/// listener, within about 40 cm, stand out one by one; the rest blur into
/// the texture. Moderate rain (10 mm/h, about 4 500 drops per m² a second)
/// gives some 2 250 distinct drops a second.
const NEAR_AREA: f32 = 0.5;
/// Marshall–Palmer's N₀, drops per m³ per mm of diameter.
const MP_N0: f32 = 8000.0;
/// At most this many drops ring at once.
const MAX_VOICES: usize = 384;

/// One drop's sound: a bubble, or a struck surface's modes with a click.
#[derive(Debug)]
enum Voice {
    Bubble(Bubble),
    Struck(Struck),
}

/// For [`Fixed`]'s empty slots.
impl Default for Voice {
    fn default() -> Self {
        Voice::Bubble(Bubble::default())
    }
}

/// A struck surface: two damped modes and a short noise burst.
#[derive(Debug)]
pub struct Struck {
    /// Each mode as a phasor (re, im) turned by (cos, sin) of its step a
    /// sample and shrunk by its decay, so a mode costs a few multiplies a
    /// sample and no trigonometry.
    modes: [[f32; 4]; 2],
    click: f32,
    click_step: f32,
    age: f32,
    decay: f32,
    /// 1 / rate: how much one sample advances time.
    dt: f32,
}

impl Struck {
    pub fn new(rng: &mut Rng, surface: Surface, amplitude: f32) -> Struck {
        Struck::new_at(rng, surface, amplitude, SAMPLE_RATE)
    }

    /// As [`Struck::new`], at the host's sample rate.
    pub fn new_at(rng: &mut Rng, surface: Surface, amplitude: f32, sample_rate: u32) -> Struck {
        // Modes (frequency range, decay per second) for each surface.
        let (low, high, decay, click): (f32, f32, f32, f32) = match surface {
            Surface::Stone => (3000.0, 7000.0, 900.0, 0.8),
            Surface::Leaves => (500.0, 1800.0, 220.0, 0.25),
            Surface::Wood => (280.0, 900.0, 260.0, 0.35),
            Surface::Glass => (2000.0, 6000.0, 90.0, 0.4),
            Surface::Metal => (700.0, 3200.0, 35.0, 0.3),
            Surface::Soft | Surface::Water => (250.0, 1200.0, 700.0, 0.35),
        };
        let rate = sample_rate as f32;
        let mut mode = |gain: f32| {
            let f = low * (high / low).powf(rng.unit());
            let mode_decay = decay * (0.8 + 0.4 * rng.unit());
            let shrink = (-mode_decay / rate).exp();
            let step = TAU * f / rate;
            let a = amplitude * gain * (0.6 + 0.4 * rng.unit());
            // Starting at amplitude a on the real axis, so the mode starts at
            // zero in the imaginary part it is heard from.
            [a, 0.0, shrink * step.cos(), shrink * step.sin()]
        };
        let modes = [mode(1.0), mode(0.5)];
        Struck {
            modes,
            click: amplitude * click,
            click_step: (-4000.0 / rate).exp(),
            dt: 1.0 / rate,
            age: 0.0,
            decay,
        }
    }

    pub fn sample(&mut self, noise: f32) -> f32 {
        let dt = self.dt;
        // Decays as running products: the same curve as e^(-dt), far cheaper.
        self.click *= self.click_step;
        let mut out = self.click * noise;
        for [re, im, c, s] in &mut self.modes {
            let (r, i) = (*re * *c - *im * *s, *re * *s + *im * *c);
            *re = r;
            *im = i;
            out += i;
        }
        self.age += dt;
        out
    }

    pub fn done(&self) -> bool {
        self.decay * self.age > 7.0
    }
}

impl Default for Struck {
    fn default() -> Self {
        Struck {
            modes: [[0.0; 4]; 2],
            click: 0.0,
            click_step: 1.0,
            age: 0.0,
            decay: 0.0,
            dt: 1.0 / 48_000.0,
        }
    }
}

/// Rain around the listener (M1): drops near enough to be heard one by one,
/// as events on whatever they land on, over the hiss of all the farther ones,
/// whose colour follows the surfaces round about.
///
/// The near drops' sizes follow Marshall–Palmer for the rain rate, and their
/// impact energy goes with mass times speed squared (D³ v²). On water, drops
/// of 0.8 to 1.1 mm trap a small bubble ringing near 14 kHz (Medwin and
/// Pumphrey's regular entrainment), the sizzle that makes rain on water sound
/// like it does; drops over 2.2 mm sometimes trap a big, lower one. Other
/// surfaces ring with their own modes. Under a roof, most near drops are on
/// the roof, and the rest of the rain comes in muffled.
#[derive(Debug)]
pub struct Rain {
    rng: Rng,
    scene: RainScene,
    until_next: f32,
    voices: Fixed<Voice, MAX_VOICES>,
    hiss: f32,
    hiss_low: f32,
    /// How far under the roof the ear is, 0 to 1, gliding over some 0.3 s:
    /// a roof is a choice, so it comes and goes in a tick, and the far hiss
    /// and sizzle cut in or out at once clicked at every doorway.
    shelter: f32,
    /// The sizzle of the whole surface of water round about, near 14 kHz.
    sizzle: BandPass,
    samples: u64,
    /// The rate this bed renders at.
    sample_rate: u32,
}

impl Rain {
    pub fn new(seed: u64, sample_rate: u32) -> Rain {
        Rain {
            rng: Rng::new(seed),
            scene: RainScene::DRY,
            until_next: 0.0,
            voices: Fixed::new(),
            hiss: 0.0,
            hiss_low: 0.0,
            shelter: 0.0,
            sizzle: BandPass::new_at(13_500.0, 1.2, sample_rate),
            samples: 0,
            sample_rate,
        }
    }

    pub fn set_scene(&mut self, scene: RainScene) {
        self.scene = scene;
    }

    /// Drops a second heard one by one, from the ground's drop flux
    /// ∫ N(D) v(D) dD over the near area.
    fn near_rate(&self) -> f32 {
        if self.scene.rate <= 0.0 {
            return 0.0;
        }
        let lambda = marshall_palmer_lambda(self.scene.rate);
        let flux: f32 = (1..=60)
            .map(|i| {
                let d = i as f32 * 0.1;
                MP_N0 * (-lambda * d).exp() * terminal_speed(d) * 0.1
            })
            .sum();
        (flux * NEAR_AREA).min(MAX_DROPS)
    }

    /// A drop's diameter in mm, drawn from Marshall–Palmer (an exponential),
    /// between 0.3 and 5 mm.
    fn diameter(&mut self) -> f32 {
        let lambda = marshall_palmer_lambda(self.scene.rate);
        (0.3 + self.rng.exponential(lambda)).min(5.0)
    }

    fn surface(&mut self) -> Surface {
        let total: f32 = self.scene.surfaces.iter().sum();
        let mut pick = self.rng.unit() * total.max(1e-6);
        for (i, share) in self.scene.surfaces.iter().enumerate() {
            pick -= share;
            if pick <= 0.0 {
                return Surface::ALL[i];
            }
        }
        Surface::Soft
    }

    fn drop(&mut self) {
        if self.voices.len() >= MAX_VOICES {
            return;
        }
        let d = self.diameter();
        // Energy ∝ D³ v², against a 2 mm drop; heard 0.3 to 1.5 m away.
        let energy = (d / 2.0).powi(3) * (terminal_speed(d) / terminal_speed(2.0)).powi(2);
        let distance = 0.3 + 1.2 * self.rng.unit().sqrt();
        let mut amplitude = 0.05 * energy.sqrt() / distance;
        let surface = match self.scene.roof {
            Some((roof, height)) if self.rng.unit() < 0.85 => {
                // On the roof: as near as the roof is.
                amplitude *= 1.5 / height.max(1.0);
                roof
            }
            Some(_) => {
                amplitude *= 0.25;
                self.surface()
            }
            None => self.surface(),
        };
        amplitude *= surface.loudness();
        if surface == Surface::Water {
            // Rain on water is heard mostly as the small bubbles' sizzle, its
            // spectrum peaking near 14 kHz some 10 dB over the rest (Medwin
            // et al.); the big bubbles of heavy drops are the odd plink in it,
            // barely rising, not a run of bloops like a cup filling.
            if (0.8..=1.1).contains(&d) {
                let radius = 0.00020 + 0.00005 * self.rng.unit();
                self.voices.push(Voice::Bubble(
                    Bubble::new_at(radius, amplitude * 6.0, self.sample_rate).with_rise(0.02),
                ));
            } else if d > 2.2 && self.rng.unit() < 0.15 {
                let radius = 0.0006 + 0.0009 * self.rng.unit();
                self.voices.push(Voice::Bubble(
                    Bubble::new_at(radius, amplitude * 0.1, self.sample_rate).with_rise(0.02),
                ));
            }
            // The splash itself, softly.
            self.voices.push(Voice::Struck(Struck::new_at(
                &mut self.rng,
                Surface::Soft,
                amplitude * 0.25,
                self.sample_rate,
            )));
            return;
        }
        let struck = Struck::new_at(&mut self.rng, surface, amplitude, self.sample_rate);
        self.voices.push(Voice::Struck(struck));
    }

    /// The next `out.len()` samples.
    pub fn render(&mut self, out: &mut [f32]) {
        let sample_rate = self.sample_rate;
        let dt = 1.0 / sample_rate as f32;
        let rate = self.near_rate();
        let brightness: f32 = {
            let total: f32 = self.scene.surfaces.iter().sum::<f32>().max(1e-6);
            Surface::ALL
                .iter()
                .zip(self.scene.surfaces)
                .map(|(s, share)| s.brightness() * share)
                .sum::<f32>()
                / total
        };
        // The far drops' hiss: noise low-passed from 1 to 8 kHz by brightness,
        // and to 600 Hz through a roof; louder with the rain rate.
        let open_cutoff = 1000.0 + 7000.0 * brightness;
        let roofed = if self.scene.roof.is_some() { 1.0 } else { 0.0 };
        let smoothing_at = |shelter: f32| {
            let cutoff = open_cutoff + (600.0 - open_cutoff) * shelter;
            1.0 - (-TAU * cutoff / sample_rate as f32).exp()
        };
        let mut smoothing = smoothing_at(self.shelter);
        let level = 0.012 * (self.scene.rate / 10.0).sqrt() * (0.3 + brightness);
        // The far drops on water: a lake or the sea round about sizzles as a
        // whole, not just where the near drops fall. Not through a roof.
        let water = self.scene.surfaces[Surface::Water as usize]
            / self.scene.surfaces.iter().sum::<f32>().max(1e-6);
        let sizzle = 0.07 * (self.scene.rate / 10.0).sqrt() * water;
        for sample in out {
            if self.samples.is_multiple_of(64) {
                self.shelter += (roofed - self.shelter) * (64.0 / (0.3 * sample_rate as f32));
                smoothing = smoothing_at(self.shelter);
            }
            if rate > 0.0 {
                self.until_next -= dt;
                while self.until_next <= 0.0 {
                    self.drop();
                    self.until_next += self.rng.exponential(rate);
                }
            }
            // One noise sample serves every click: they're short and randomly
            // timed, so nothing audible correlates.
            let noise = self.rng.signed();
            let mut mix = 0.0;
            for voice in &mut self.voices {
                mix += match voice {
                    Voice::Bubble(bubble) => bubble.sample(),
                    Voice::Struck(struck) => struck.sample(noise),
                };
            }
            self.samples += 1;
            if self.samples.is_multiple_of(64) {
                self.voices.retain(|voice| match voice {
                    Voice::Bubble(bubble) => !bubble.done(),
                    Voice::Struck(struck) => !struck.done(),
                });
            }
            self.hiss += smoothing * (self.rng.signed() - self.hiss);
            self.hiss_low += smoothing * (self.hiss - self.hiss_low);
            let far = (1.0 - self.shelter) * sizzle * self.sizzle.process(self.rng.signed());
            *sample = mix + level * self.hiss_low * 3.0 + far;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use std::{vec, vec::Vec};

    #[test]
    fn default_struck_has_positive_dt() {
        let s = Struck::default();
        assert!(s.dt > 0.0);
    }

    fn scene(rate: f32, surface: Surface) -> RainScene {
        let mut surfaces = [0.0; 7];
        surfaces[surface as usize] = 1.0;
        RainScene {
            rate,
            surfaces,
            roof: None,
        }
    }

    /// Energy and zero-crossing rate (a measure of brightness) of 2 s.
    fn listen(scene: RainScene) -> (f32, f32) {
        let mut rain = Rain::new(3, SAMPLE_RATE);
        rain.set_scene(scene);
        let mut out = vec![0.0; 2 * SAMPLE_RATE as usize];
        rain.render(&mut out);
        let energy = out.iter().map(|s| s * s).sum::<f32>();
        let crossings = out
            .windows(2)
            .filter(|w| (w[0] <= 0.0) != (w[1] <= 0.0))
            .count();
        (energy, crossings as f32 / out.len() as f32)
    }

    #[test]
    fn the_drop_flux_follows_marshall_palmer() {
        let mut rain = Rain::new(1, SAMPLE_RATE);
        rain.set_scene(scene(10.0, Surface::Stone));
        // About 4 500 drops per m² per second at 10 mm/h: N₀ ∫ e^(-ΛD) v(D) dD.
        let flux = rain.near_rate() / NEAR_AREA;
        assert!((4_000.0..5_000.0).contains(&flux), "{flux}");
        assert!(terminal_speed(2.0) > 6.0 && terminal_speed(2.0) < 7.0);
    }

    #[test]
    fn surfaces_sound_different() {
        let (stone, stone_bright) = listen(scene(10.0, Surface::Stone));
        let (leaves, leaves_bright) = listen(scene(10.0, Surface::Leaves));
        let (snow, _) = listen(scene(10.0, Surface::Soft));
        let (_, water_bright) = listen(scene(10.0, Surface::Water));
        assert!(stone_bright > leaves_bright, "stone ticks, leaves patter");
        assert!(water_bright > leaves_bright, "water sizzles");
        assert!(
            snow < stone / 4.0,
            "snow and grass deaden it: {snow} vs {stone}"
        );
        assert!(leaves > 0.0);
    }

    #[test]
    fn heavier_rain_is_louder_and_a_roof_muffles_the_rest() {
        let (light, _) = listen(scene(2.0, Surface::Stone));
        let (heavy, _) = listen(scene(50.0, Surface::Stone));
        assert!(heavy > light, "{light} vs {heavy}");
        let mut roofed = scene(10.0, Surface::Stone);
        roofed.roof = Some((Surface::Wood, 3.0));
        let (_, open_bright) = listen(scene(10.0, Surface::Stone));
        let (_, roof_bright) = listen(roofed);
        assert!(roof_bright < open_bright, "under wood it drums duller");
        assert_eq!(listen(RainScene::DRY).0, 0.0, "no rain, no sound");
    }
}
