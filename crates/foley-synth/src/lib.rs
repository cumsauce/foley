//! Physically based sound synthesis: each sound is made from the physics
//! that dominates what is heard (docs/PLAN.md), at [`SAMPLE_RATE`], mono.

mod birds;
mod dsp;
mod life;
mod modal;
mod oneshot;
mod outdoors;
mod plants;
mod primitives;
mod sculk;
mod thunder;
mod voice;

pub use foley_dsp::{
    BandPass, Bubble, Fire, FireScene, Lava, LavaScene, LowPass, Rain, RainScene, Rng, SAMPLE_RATE,
    Surface, Underwater, UnderwaterScene, Water, WaterScene, Wind, WindScene, absorption_cutoff,
    absorption_db_per_m, helmholtz, minnaert_hz, plunge_radius, to_pcm,
};

pub use crate::birds::{Bird, bird_call};
pub use crate::life::{birdsong, crickets, dolbear_chirps_per_minute, frog};
pub use crate::modal::{
    Contact, Material, MaterialProps, contact, contact_with, fracture, fracture_with, hertz_time,
};
pub use crate::oneshot::{drip_into_water, drip_on_stone, lava_drip};
pub use crate::outdoors::{cicadas, owl, surf, woodpecker};
pub use crate::plants::{Plant, brush, buzz, flush, ice_singing, settling_stone};
pub use crate::primitives::{bubble, gust, noise_burst, tone};
pub use crate::sculk::sculk_ring;
pub use crate::thunder::{Segment, Thunder, ThunderScene, channel, strike};
pub use crate::voice::{
    Call, CreatureCall, CreatureSpec, Mob, Pulse, creature_voice, howl, lf_pulse, voice,
};
