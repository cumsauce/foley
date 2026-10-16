//! Foley's beds (wind, rain, water, fire, lava, cave) and what they're built from,
//! rendered on the audio thread. `no_std` without `alloc`: nothing here can
//! allocate or free, and the compiler holds it to that. Voices live in
//! [`Fixed`] lists; float functions come from `foley-math`.

#![no_std]

#[cfg(test)]
extern crate std;

mod air;
mod bubble;
mod cave;
mod filter;
mod fire;
mod fixed;
mod lava;
mod rain;
mod rng;
mod underwater;
mod water;
mod wind;
mod wobble;

pub use crate::air::{absorption_cutoff, absorption_db_per_m};
pub use crate::bubble::{Bubble, minnaert_hz};
pub use crate::cave::{Cave, CaveScene};
pub use crate::filter::{BandPass, LowPass};
pub use crate::fire::{Fire, FireScene};
pub use crate::fixed::Fixed;
pub use crate::lava::{Lava, LavaScene, helmholtz};
pub use crate::rain::{Rain, RainScene, Struck, Surface};
pub use crate::rng::Rng;
pub use crate::underwater::{Underwater, UnderwaterScene};
pub use crate::water::{Water, WaterScene, plunge_radius};
pub use crate::wind::{Wind, WindScene};
pub use crate::wobble::{Wobble, smooth};

/// Samples a second, for everything Foley makes.
pub const SAMPLE_RATE: u32 = 48_000;

/// Converts to 16-bit PCM, softly limiting so loud moments never wrap.
pub fn to_pcm(sample: f32) -> i16 {
    (sample.tanh() * f32::from(i16::MAX)) as i16
}
