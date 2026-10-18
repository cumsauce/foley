//! Physically based sound synthesis: each sound is made from the physics
//! that dominates what is heard (docs/PLAN.md), at [`SAMPLE_RATE`], mono.

mod aeolian;
mod bell;
mod biome;
mod birds;
mod dsp;
mod life;
mod modal;
pub mod note;
mod oneshot;
mod outdoors;
mod plants;
mod primitives;
mod sculk;
mod surf;
mod thunder;
mod voice;

pub use foley_dsp::{
    absorption_cutoff, absorption_db_per_m, helmholtz, minnaert_hz, plunge_radius, to_pcm,
    BandPass, Bubble, Cave, CaveScene, Fire, FireScene, Lava, LavaScene, LowPass, Rain, RainScene,
    Rng, Surface, Underwater, UnderwaterScene, Water, WaterScene, Wind, WindScene,
    DEFAULT_SAMPLE_RATE, SAMPLE_RATE,
};

pub use crate::aeolian::aeolian;
pub use crate::bell::bell;
pub use crate::biome::{Biome, BiomeKind, BiomeScene};
pub use crate::birds::catalog::birdsong2;
pub use crate::birds::{bird_call, Bird, Birds};
pub use crate::life::{birdsong, crickets, dolbear_chirps_per_minute, frog};
pub use crate::modal::{
    contact, contact_with, fracture, fracture_with, hertz_time, Contact, Material, MaterialProps,
};
pub use crate::note::{base_hz, family, note, note_hz, Family};
pub use crate::oneshot::{bang, drip_into_water, drip_on_stone, lava_drip};
pub use crate::outdoors::{cicadas, owl, surf, surf_len, woodpecker, SURF_LEN};
pub use crate::plants::{brush, buzz, flush, ice_singing, settling_stone, Plant};
pub use crate::primitives::{bubble, gust, noise_burst, tone};
pub use crate::sculk::sculk_ring;
pub use crate::surf::{Surf, SurfScene};
pub use crate::thunder::{channel, strike, Segment, Thunder, ThunderScene};
pub use crate::voice::{
    creature_voice, howl, lf_pulse, voice, warm_voice_tables, Call, CreatureCall, CreatureSpec,
    Mob, Pulse,
};
