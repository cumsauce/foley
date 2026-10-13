//! Physically based sound synthesis: each sound is made from the physics
//! that dominates what is heard (docs/PLAN.md), at [`SAMPLE_RATE`], mono.

mod air;
mod bubble;
mod filter;
mod fire;
mod lava;
mod life;
mod modal;
mod oneshot;
mod outdoors;
mod rain;
mod rng;
mod thunder;
mod voice;
mod water;
mod wind;

pub use crate::air::{absorption_cutoff, absorption_db_per_m};
pub use crate::bubble::{Bubble, minnaert_hz};
pub use crate::filter::{BandPass, LowPass};
pub use crate::fire::{Fire, FireScene};
pub use crate::lava::{Lava, LavaScene, helmholtz};
pub use crate::life::{birdsong, crickets, dolbear_chirps_per_minute, frog};
pub use crate::modal::{Contact, Material, contact, fracture, hertz_time};
pub use crate::oneshot::{drip_into_water, drip_on_stone, lava_drip};
pub use crate::outdoors::{cicadas, owl, surf, woodpecker};
pub use crate::rain::{Rain, RainScene, Surface};
pub use crate::rng::Rng;
pub use crate::thunder::{Segment, Thunder, ThunderScene, channel, strike};
pub use crate::voice::{Call, Mob, Pulse, howl, lf_pulse, voice};
pub use crate::water::{Water, WaterScene, plunge_radius};
pub use crate::wind::{Wind, WindScene};

/// Samples a second, for everything Foley makes.
pub const SAMPLE_RATE: u32 = 48_000;

/// Converts to 16-bit PCM, softly limiting so loud moments never wrap.
pub fn to_pcm(sample: f32) -> i16 {
    (sample.tanh() * f32::from(i16::MAX)) as i16
}
