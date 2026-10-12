//! Physically based sound synthesis: each sound is made from the physics
//! that dominates what is heard (docs/PLAN.md), at [`SAMPLE_RATE`], mono.

mod bubble;
mod filter;
mod rain;
mod rng;
mod wind;

pub use crate::bubble::{Bubble, minnaert_hz};
pub use crate::filter::BandPass;
pub use crate::rain::{Rain, RainScene, Surface};
pub use crate::rng::Rng;
pub use crate::wind::{Wind, WindScene};

/// Samples a second, for everything Foley makes.
pub const SAMPLE_RATE: u32 = 48_000;

/// Converts to 16-bit PCM, softly limiting so loud moments never wrap.
pub fn to_pcm(sample: f32) -> i16 {
    (sample.tanh() * f32::from(i16::MAX)) as i16
}
