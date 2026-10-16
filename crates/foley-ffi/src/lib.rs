//! C ABI over Foley's synthesis, called from Java through the FFM API.
//!
//! This is the only crate allowed to use `unsafe`. Run `cargo xtask header`
//! after changing any export to regenerate `include/foley.h`.
//!
//! As in Earshot and yakvc: every `int32_t` return is [`FOLEY_OK`] or a
//! negative `FOLEY_ERR_*` code, with a message from [`foley_last_error`].
//! Audio is 48 kHz mono 16-bit PCM, what the game's OpenAL streams take.

#![allow(unsafe_code)]

mod guard;
#[cfg(test)]
mod no_alloc;

use foley_synth::{
    Birds, Call, Contact, CreatureCall, CreatureSpec, Fire, FireScene, Lava, LavaScene, Material,
    MaterialProps, Mob, Plant, Rain, RainScene, SAMPLE_RATE, Surface, Thunder, ThunderScene, Water,
    WaterScene, Wind, WindScene, birdsong2, brush, bubble, buzz, cicadas, contact, contact_with,
    creature_voice, crickets, drip_into_water, drip_on_stone, flush, fracture, fracture_with, frog,
    gust, howl, ice_singing, lava_drip, noise_burst, owl, sculk_ring, settling_stone, surf, to_pcm,
    tone, voice, woodpecker,
};

use crate::guard::{FfiError, guard, out_arg, slice_arg, slice_mut_arg};

/// Version of this C ABI. Java refuses a library whose version differs.
/// 1: rain beds and drips (the M0 spike).
/// 2: rain takes a scene: its rate, what it lands on and any roof.
/// 3: wind beds.
/// 4: every bed through `foley_bed_*`, by kind, with water, fire and lava;
///    one-shots through `foley_render_one_shot`.
/// 5: thunder, a bed struck by `foley_bed_strike`.
/// 6: strikes and breaks on materials, `foley_render_contact`.
/// 7: ambient life one-shots: crickets, birdsong, frogs.
/// 8: footsteps, and snow as a material.
/// 9: mob voices, `foley_render_voice`.
/// 10: voice engine 2: more mobs (zombies' kin, skeletons, spiders, creepers,
///     bats) and calls (growls, purrs, the fuse, breathing, …).
/// 11: everyday animals and rarer hostiles (M8, M9): 52 mobs, 43 calls.
/// 12: more life outdoors: surf, woodpeckers, owls, cicadas, a wolf's howl.
/// 13: brushing through plants, lake ice singing, stones settling in caves.
/// 14: more birds (gull, loon, jay, warbler), a bird flushed, insects
///     buzzing.
/// 15: custom materials for contacts and breaks,
///     `foley_render_contact_custom` (the integration API, F1).
/// 16: primitives to compose new one-shots from, `foley_render_primitive`
///     (the integration API, F2).
/// 17: other mods' creatures, `foley_render_creature` (the integration API,
///     F3).
/// 20: active structures (M21): `FOLEY_SHOT_SCULK_RING`, a triggered low-Q
///     tone for sculk sensors.
/// 21: more birds (M13): 48-species `Birds` enum, per-biome rosters; the
///     `FOLEY_SHOT_BIRDSONG` size argument widens to cover the catalogue.
pub const FOLEY_ABI_VERSION: u32 = 21;

/// Surfaces rain lands on, in the order its scene takes their shares:
/// water, stone, leaves, wood, soft (grass, dirt, sand, wool, snow), glass,
/// metal.
pub const FOLEY_SURFACES: usize = 7;

/// Bed kinds, and the floats each one's scene takes, in order:
///
/// - rain: rate (mm/h, 0 to 200), the [`FOLEY_SURFACES`] shares of what it
///   lands on, the roof's surface (-1 for none) and its height above the ear
///   in blocks;
/// - wind: mean speed (m/s, 0 to 60), exposure (0 to 1), leaves (0 to 1);
/// - water: blocks flowing, blocks where a waterfall lands, gain (0 to 1);
/// - fire: blocks burning, gain;
/// - lava: blocks open to the air, gain;
/// - thunder: the air's temperature (°C) and relative humidity (percent),
///   and the listener's exposure to the sky (0 to 1). Each flash is added
///   with `foley_bed_strike`.
pub const FOLEY_BED_RAIN: i32 = 0;
pub const FOLEY_BED_WIND: i32 = 1;
pub const FOLEY_BED_WATER: i32 = 2;
pub const FOLEY_BED_FIRE: i32 = 3;
pub const FOLEY_BED_LAVA: i32 = 4;
pub const FOLEY_BED_THUNDER: i32 = 5;

/// One-shot kinds. `size` is the drop's (or bubble's) radius in mm for the
/// water drips (0.1 to 10), and unused for lava; the air's temperature in °C
/// for crickets (-30 to 50); the species (0 to 50) for birdsong (the M13
/// 48-species catalogue, with two headroom slots); the body length in cm for
/// a frog (2 to 20); the wave's height in m for surf (0.2 to 3); the
/// temperature for cicadas, like crickets; unused for woodpeckers, owls and
/// howls.
pub const FOLEY_SHOT_DRIP_INTO_WATER: i32 = 0;
pub const FOLEY_SHOT_DRIP_ON_STONE: i32 = 1;
pub const FOLEY_SHOT_LAVA_DRIP: i32 = 2;
pub const FOLEY_SHOT_CRICKETS: i32 = 3;
pub const FOLEY_SHOT_BIRDSONG: i32 = 4;
pub const FOLEY_SHOT_FROG: i32 = 5;
pub const FOLEY_SHOT_SURF: i32 = 6;
pub const FOLEY_SHOT_WOODPECKER: i32 = 7;
pub const FOLEY_SHOT_OWL: i32 = 8;
pub const FOLEY_SHOT_CICADAS: i32 = 9;
pub const FOLEY_SHOT_HOWL: i32 = 10;
/// Brushing through plants: `size` is the plant (0 grass, 1 leaves, 2 dry).
pub const FOLEY_SHOT_BRUSH: i32 = 11;
pub const FOLEY_SHOT_ICE_SINGING: i32 = 12;
pub const FOLEY_SHOT_SETTLING_STONE: i32 = 13;
pub const FOLEY_SHOT_FLUSH: i32 = 14;
/// An insect buzzing: `size` 0 a bee, 1 a fly.
pub const FOLEY_SHOT_BUZZ: i32 = 15;
/// A sculk sensor's triggered tone: `size` is the bowl's Q (0.5 to 20);
/// higher Q rings longer.
pub const FOLEY_SHOT_SCULK_RING: i32 = 16;

/// Primitives for `foley_render_primitive`, and the floats each takes, in
/// order:
///
/// - bubble: the radius in mm (0.05 to 20) — a Minnaert ring;
/// - noise: the band's centre (Hz, 20 to 20000), its width (Q, 0.5 to 20),
///   the seconds (0.005 to 5) and the peak's gain (0 to 1);
/// - tone: the pitch (Hz, 20 to 10000), where it glides to (Hz, 20 to
///   20000), the seconds (0.01 to 5) and the gain (0 to 1);
/// - gust: the wind's speed (m/s, 0 to 60), the seconds (0.2 to 10) and the
///   gain (0 to 1).
pub const FOLEY_PRIMITIVE_BUBBLE: i32 = 0;
pub const FOLEY_PRIMITIVE_NOISE: i32 = 1;
pub const FOLEY_PRIMITIVE_TONE: i32 = 2;
pub const FOLEY_PRIMITIVE_GUST: i32 = 3;

/// The most floats any primitive takes.
pub const FOLEY_PRIMITIVE_MAX_FLOATS: usize = 4;

/// Calls of another mod's creature, for `foley_render_creature`: ambient,
/// hurt, death, growl, breath.
pub const FOLEY_CREATURE_AMBIENT: i32 = 0;
pub const FOLEY_CREATURE_HURT: i32 = 1;
pub const FOLEY_CREATURE_DEATH: i32 = 2;
pub const FOLEY_CREATURE_GROWL: i32 = 3;
pub const FOLEY_CREATURE_BREATH: i32 = 4;

/// Another mod's creature's voice, the floats `foley_render_creature` takes,
/// in order: its vocal tract's length (m, 0.03 to 1), its usual pitch (Hz,
/// 30 to 4000), how rough it goes (0 to 1), how breathy (0 to 1).
pub const FOLEY_CREATURE_FLOATS: usize = 4;

/// Materials for `foley_render_contact`: stone, wood, metal, glass, ice,
/// earth, granular (sand, gravel), plant, soft (wool), snow.
pub const FOLEY_MATERIALS: usize = 10;

/// A custom material's physics, the floats `foley_render_contact_custom`
/// takes, in order: which built-in behaviour it follows (an index below
/// [`FOLEY_MATERIALS`], whose grains, scuff, crunch and crack it keeps),
/// Young's modulus (Pa, 1e8 to 5e11), density (kg/m³, 1 to 20000), Poisson's
/// ratio (0 to 0.49), loss factor η (0.0005 to 2), the vibrating plate's
/// thickness (m, 0.001 to 1), fragments it shatters into (0 to 200),
/// restitution (0 to 0.95), and shards' thickness (m, 0 for chunks or
/// 0.0005 to 0.2).
pub const FOLEY_CUSTOM_MATERIAL_FLOATS: usize = 9;
/// Contacts for `foley_render_contact`: a tool tapping, a block set down, a
/// body landing, the block breaking, and a footstep.
pub const FOLEY_CONTACT_TAP: i32 = 0;
pub const FOLEY_CONTACT_PLACE: i32 = 1;
pub const FOLEY_CONTACT_LAND: i32 = 2;
pub const FOLEY_CONTACT_BREAK: i32 = 3;
pub const FOLEY_CONTACT_STEP: i32 = 4;

/// Mobs for `foley_render_voice`, in `foley_synth::Mob`'s order (cow, pig,
/// sheep, …; scripts/gen_voices.py lists them all).
pub const FOLEY_MOBS: usize = 52;
/// Calls, in `foley_synth::Call`'s order (ambient, hurt, death, growl, …). A
/// call a mob doesn't have is its ambient one.
pub const FOLEY_CALLS: usize = 43;

pub const FOLEY_OK: i32 = 0;
/// A null pointer or out-of-range value.
pub const FOLEY_ERR_INVALID_ARGUMENT: i32 = -1;
/// The call panicked.
pub const FOLEY_ERR_PANIC: i32 = -3;

#[derive(Debug)]
// Rain's voices are inline, so it's the biggest; a bed is boxed once.
#[allow(clippy::large_enum_variant)]
enum Synth {
    Rain(Rain),
    Wind(Wind),
    Water(Water),
    Fire(Fire),
    Lava(Lava),
    Thunder(Thunder),
}

/// Opaque handle to a bed: a sound made as it plays, endlessly.
#[derive(Debug)]
pub struct FoleyBed {
    synth: Synth,
    /// One glide block, rendered before the level guard: render never
    /// allocates, however many frames it's asked for.
    scratch: [f32; GLIDE_BLOCK],
    /// The level guard's envelope and gain.
    envelope: f32,
    guard: f32,
    /// The scene last set, and the one heard now, gliding towards it so a
    /// change (the game samples the world a few times a second) never steps.
    target: [f32; MAX_PARAMS],
    heard: [f32; MAX_PARAMS],
    /// How many params this kind takes; 0 until a scene is set.
    params: usize,
}

/// Scenes glide this often, samples, and over about this long, seconds.
const GLIDE_BLOCK: usize = 128;
const GLIDE_TIME: f32 = 0.3;

/// Beds never go past this peak: a scene that jumps (a waterfall appearing,
/// a gust) is caught before it reaches the speakers.
const BED_CEILING: f32 = 0.4;

/// Returns [`FOLEY_ABI_VERSION`]. Safe to call before anything else.
#[unsafe(no_mangle)]
pub extern "C" fn foley_abi_version() -> u32 {
    FOLEY_ABI_VERSION
}

/// Starts a bed of `kind` (`FOLEY_BED_*`), silent until
/// `foley_bed_set_scene` gives it a scene.
///
/// # Safety
/// `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_bed_create(kind: i32, seed: u64, out: *mut *mut FoleyBed) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `out` is writable.
        let out = unsafe { out_arg(out, "out") }?;
        let synth = match kind {
            FOLEY_BED_RAIN => Synth::Rain(Rain::new(seed)),
            FOLEY_BED_WIND => Synth::Wind(Wind::new(seed)),
            FOLEY_BED_WATER => Synth::Water(Water::new(seed)),
            FOLEY_BED_FIRE => Synth::Fire(Fire::new(seed)),
            FOLEY_BED_LAVA => Synth::Lava(Lava::new(seed)),
            FOLEY_BED_THUNDER => Synth::Thunder(Thunder::new()),
            _ => return Err(FfiError::invalid("unknown bed kind")),
        };
        *out = Box::into_raw(Box::new(FoleyBed {
            synth,
            scratch: [0.0; GLIDE_BLOCK],
            envelope: 0.0,
            guard: 1.0,
            target: [0.0; MAX_PARAMS],
            heard: [0.0; MAX_PARAMS],
            params: 0,
        }));
        Ok(())
    })
}

/// Sets what the world round a bed is like: `count` floats, as its kind
/// takes them (see `FOLEY_BED_RAIN`).
///
/// # Safety
/// `bed` must be live and not used on another thread during the call;
/// `params` must point to `count` floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_bed_set_scene(
    bed: *mut FoleyBed,
    params: *const f32,
    count: usize,
) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `bed` is live and used by this thread
        // alone.
        let bed = unsafe { bed.as_mut() }.ok_or_else(|| FfiError::invalid("bed is null"))?;
        // SAFETY: the caller guarantees `params` points to `count` floats.
        let p = unsafe { slice_arg(params, count, "params") }?;
        if !p.iter().all(|x| x.is_finite()) {
            return Err(FfiError::invalid("params must be finite"));
        }
        // Checked (and applied) now; from the next render on, it glides there
        // from what's heard.
        apply(&mut bed.synth, p)?;
        if bed.params == 0 {
            bed.heard[..p.len()].copy_from_slice(p);
        }
        bed.params = p.len();
        bed.target[..p.len()].copy_from_slice(p);
        Ok(())
    })
}

/// The most params any bed takes.
const MAX_PARAMS: usize = 3 + FOLEY_SURFACES;

/// Checks `p` for this kind of bed and gives it to the synth.
fn apply(synth: &mut Synth, p: &[f32]) -> Result<(), FfiError> {
    let wanted = match synth {
        Synth::Rain(_) => 3 + FOLEY_SURFACES,
        Synth::Wind(_) | Synth::Water(_) | Synth::Thunder(_) => 3,
        Synth::Fire(_) | Synth::Lava(_) => 2,
    };
    if p.len() != wanted {
        return Err(FfiError::invalid("wrong number of params for this bed"));
    }
    let gain = |x: f32| {
        (0.0..=1.0)
            .contains(&x)
            .then_some(x)
            .ok_or_else(|| FfiError::invalid("gain out of range"))
    };
    let amount = |x: f32| {
        (0.0..=10_000.0)
            .contains(&x)
            .then_some(x)
            .ok_or_else(|| FfiError::invalid("amount out of range"))
    };
    match synth {
        Synth::Rain(rain) => {
            let rate = p[0];
            let shares = &p[1..=FOLEY_SURFACES];
            if !(0.0..=200.0).contains(&rate) || !shares.iter().all(|s| *s >= 0.0) {
                return Err(FfiError::invalid("rate or surfaces out of range"));
            }
            let roof = match p[FOLEY_SURFACES + 1] as i32 {
                -1 => None,
                index => Some((
                    usize::try_from(index)
                        .ok()
                        .and_then(Surface::from_index)
                        .ok_or_else(|| FfiError::invalid("unknown roof surface"))?,
                    p[FOLEY_SURFACES + 2].clamp(0.5, 64.0),
                )),
            };
            let mut surfaces = [0.0; FOLEY_SURFACES];
            surfaces.copy_from_slice(shares);
            rain.set_scene(RainScene {
                rate,
                surfaces,
                roof,
            });
        }
        Synth::Wind(wind) => {
            if !(0.0..=60.0).contains(&p[0]) {
                return Err(FfiError::invalid("wind speed out of range"));
            }
            wind.set_scene(WindScene {
                speed: p[0],
                exposure: gain(p[1])?,
                leaves: gain(p[2])?,
            });
        }
        Synth::Water(water) => water.set_scene(WaterScene {
            flow: amount(p[0])?,
            falls: amount(p[1])?,
            gain: gain(p[2])?,
        }),
        Synth::Fire(fire) => fire.set_scene(FireScene {
            size: amount(p[0])?,
            gain: gain(p[1])?,
        }),
        Synth::Lava(lava) => lava.set_scene(LavaScene {
            area: amount(p[0])?,
            gain: gain(p[1])?,
        }),
        Synth::Thunder(thunder) => {
            if !(-50.0..=60.0).contains(&p[0]) || !(0.0..=100.0).contains(&p[1]) {
                return Err(FfiError::invalid("temperature or humidity out of range"));
            }
            thunder.set_scene(ThunderScene {
                celsius: p[0],
                humidity: p[1],
                exposure: gain(p[2])?,
            });
        }
    }
    Ok(())
}

/// Whether the param at `index` is a choice, not an amount, so it's taken at
/// once rather than glided: a rain bed's roof surface and room size.
fn is_choice(synth: &Synth, index: usize) -> bool {
    matches!(synth, Synth::Rain(_)) && index > FOLEY_SURFACES
}

/// Renders the next `frames` samples of `bed` into `out`, as 16-bit PCM.
///
/// # Safety
/// `bed` must be live and not used on another thread during the call; `out`
/// must be writable for `frames` samples.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_bed_render(bed: *mut FoleyBed, out: *mut i16, frames: usize) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `bed` is live and used by this thread
        // alone.
        let bed = unsafe { bed.as_mut() }.ok_or_else(|| FfiError::invalid("bed is null"))?;
        // SAFETY: the caller guarantees `out` is writable for `frames`.
        let out = unsafe { slice_mut_arg(out, frames, "out") }?;
        let step = 1.0 - (-(GLIDE_BLOCK as f32) / (GLIDE_TIME * SAMPLE_RATE as f32)).exp();
        let n = bed.params;
        for out in out.chunks_mut(GLIDE_BLOCK) {
            let block = &mut bed.scratch[..out.len()];
            if bed.heard[..n] != bed.target[..n] {
                for (i, (heard, target)) in
                    bed.heard[..n].iter_mut().zip(&bed.target[..n]).enumerate()
                {
                    *heard = if is_choice(&bed.synth, i) || (*target - *heard).abs() < 1e-4 {
                        *target
                    } else {
                        *heard + (*target - *heard) * step
                    };
                }
                // In range: between two scenes that were.
                apply(&mut bed.synth, &bed.heard[..n])?;
            }
            match &mut bed.synth {
                Synth::Rain(synth) => synth.render(block),
                Synth::Wind(synth) => synth.render(block),
                Synth::Water(synth) => synth.render(block),
                Synth::Fire(synth) => synth.render(block),
                Synth::Lava(synth) => synth.render(block),
                Synth::Thunder(synth) => synth.render(block),
            }
            // The level guard: the envelope rises at once and falls over
            // about half a second; the gain follows it down quickly and back
            // up slowly.
            for (out, sample) in out.iter_mut().zip(block.iter()) {
                bed.envelope = sample.abs().max(bed.envelope * 0.99996);
                let wanted = (BED_CEILING / bed.envelope.max(1e-6)).min(1.0);
                let rate = if wanted < bed.guard { 0.01 } else { 0.0001 };
                bed.guard += (wanted - bed.guard) * rate;
                *out = to_pcm(sample * bed.guard);
            }
        }
        Ok(())
    })
}

/// A flash of lightning striking the ground at (`x`, `y`, `z`) metres from
/// the listener (y up), now: a thunder bed makes its thunder, arriving as
/// sound would. Other beds refuse it.
///
/// # Safety
/// `bed` must be live and not used on another thread during the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_bed_strike(
    bed: *mut FoleyBed,
    x: f32,
    y: f32,
    z: f32,
    seed: u64,
) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `bed` is live and used by this thread
        // alone.
        let bed = unsafe { bed.as_mut() }.ok_or_else(|| FfiError::invalid("bed is null"))?;
        let Synth::Thunder(thunder) = &mut bed.synth else {
            return Err(FfiError::invalid("only thunder is struck"));
        };
        if ![x, y, z]
            .iter()
            .all(|c| c.is_finite() && c.abs() <= 100_000.0)
        {
            return Err(FfiError::invalid("strike out of range"));
        }
        thunder.strike([x, y, z], seed);
        Ok(())
    })
}

/// Frees a bed. Null is ignored.
///
/// # Safety
/// `bed` must come from `foley_bed_create` and not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_bed_destroy(bed: *mut FoleyBed) {
    if !bed.is_null() {
        // SAFETY: `bed` came from `Box::into_raw` and is never used again.
        drop(unsafe { Box::from_raw(bed) });
    }
}

/// Renders a one-shot of `kind` (`FOLEY_SHOT_*`) from `seed`. Writes it to
/// `out` if `cap` samples are enough, and how many it needs to `*needed`
/// either way.
///
/// # Safety
/// `out` must be writable for `cap` samples; `needed` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_render_one_shot(
    kind: i32,
    size: f32,
    seed: u64,
    out: *mut i16,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `needed` is writable.
        let needed = unsafe { out_arg(needed, "needed") }?;
        // SAFETY: the caller guarantees `out` is writable for `cap` samples.
        let out = unsafe { slice_mut_arg(out, cap, "out") }?;
        let drop_size = || {
            (0.1..=10.0)
                .contains(&size)
                .then_some(size / 1000.0)
                .ok_or_else(|| FfiError::invalid("size out of range"))
        };
        let sound = match kind {
            FOLEY_SHOT_DRIP_INTO_WATER => drip_into_water(drop_size()?, seed),
            FOLEY_SHOT_DRIP_ON_STONE => drip_on_stone(drop_size()?, seed),
            FOLEY_SHOT_LAVA_DRIP => lava_drip(seed),
            FOLEY_SHOT_CRICKETS if (-30.0..=50.0).contains(&size) => crickets(size, seed),
            FOLEY_SHOT_BIRDSONG if (0.0..=Birds::ALL.len() as f32 - 1.0).contains(&size) => {
                birdsong2(Birds::of(size as u32), seed)
            }
            FOLEY_SHOT_FROG if (2.0..=20.0).contains(&size) => frog(size, seed),
            FOLEY_SHOT_SURF if (0.2..=3.0).contains(&size) => surf(size, seed),
            FOLEY_SHOT_WOODPECKER => woodpecker(seed),
            FOLEY_SHOT_OWL => owl(seed),
            FOLEY_SHOT_CICADAS if (-30.0..=50.0).contains(&size) => cicadas(size, seed),
            FOLEY_SHOT_HOWL => howl(seed),
            FOLEY_SHOT_BRUSH if (0.0..=2.0).contains(&size) => brush(Plant::of(size as u32), seed),
            FOLEY_SHOT_ICE_SINGING => ice_singing(seed),
            FOLEY_SHOT_SETTLING_STONE => settling_stone(seed),
            FOLEY_SHOT_FLUSH => flush(seed),
            FOLEY_SHOT_BUZZ if (0.0..=1.0).contains(&size) => buzz(size as u32, seed),
            FOLEY_SHOT_SCULK_RING if (0.5..=20.0).contains(&size) => sculk_ring(size, seed),
            FOLEY_SHOT_CRICKETS | FOLEY_SHOT_BIRDSONG | FOLEY_SHOT_FROG | FOLEY_SHOT_SURF
            | FOLEY_SHOT_CICADAS | FOLEY_SHOT_BRUSH | FOLEY_SHOT_BUZZ => {
                return Err(FfiError::invalid("size out of range"));
            }
            FOLEY_SHOT_SCULK_RING => {
                return Err(FfiError::invalid("size out of range"));
            }
            _ => return Err(FfiError::invalid("unknown one-shot kind")),
        };
        if let Some(dest) = out.get_mut(..sound.len()) {
            for (out, sample) in dest.iter_mut().zip(&sound) {
                *out = to_pcm(*sample);
            }
        }
        *needed = sound.len();
        Ok(())
    })
}

/// Renders a `contact` (`FOLEY_CONTACT_*`) with a block of `material` (an
/// index below [`FOLEY_MATERIALS`]) at `speed` m/s (0.1 to 30; unused for a
/// break), from `seed`. Writes it to `out` if `cap` samples are enough, and
/// how many it needs to `*needed` either way.
///
/// # Safety
/// `out` must be writable for `cap` samples; `needed` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_render_contact(
    kind: i32,
    material: i32,
    speed: f32,
    seed: u64,
    out: *mut i16,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `needed` is writable.
        let needed = unsafe { out_arg(needed, "needed") }?;
        // SAFETY: the caller guarantees `out` is writable for `cap` samples.
        let out = unsafe { slice_mut_arg(out, cap, "out") }?;
        let material = usize::try_from(material)
            .ok()
            .and_then(Material::from_index)
            .ok_or_else(|| FfiError::invalid("unknown material"))?;
        if !(0.1..=30.0).contains(&speed) {
            return Err(FfiError::invalid("speed out of range"));
        }
        let sound = match kind {
            FOLEY_CONTACT_BREAK => fracture(material, seed),
            FOLEY_CONTACT_TAP => contact(Contact::Tap, material, speed, seed),
            FOLEY_CONTACT_PLACE => contact(Contact::Place, material, speed, seed),
            FOLEY_CONTACT_LAND => contact(Contact::Land, material, speed, seed),
            FOLEY_CONTACT_STEP => contact(Contact::Step, material, speed, seed),
            _ => return Err(FfiError::invalid("unknown contact")),
        };
        if let Some(dest) = out.get_mut(..sound.len()) {
            for (out, sample) in dest.iter_mut().zip(&sound) {
                *out = to_pcm(*sample);
            }
        }
        *needed = sound.len();
        Ok(())
    })
}

/// Renders a `contact` (`FOLEY_CONTACT_*`) on a custom material's physics
/// (the integration API): `props` is [`FOLEY_CUSTOM_MATERIAL_FLOATS`] floats
/// (see it for the order and ranges), `speed` as in
/// [`foley_render_contact`]. Writes it to `out` if `cap` samples are enough,
/// and how many it needs to `*needed` either way.
///
/// # Safety
/// `props` must point to `count` floats; `out` must be writable for `cap`
/// samples; `needed` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_render_contact_custom(
    props: *const f32,
    count: usize,
    kind: i32,
    speed: f32,
    seed: u64,
    out: *mut i16,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `needed` is writable.
        let needed = unsafe { out_arg(needed, "needed") }?;
        // SAFETY: the caller guarantees `out` is writable for `cap` samples.
        let out = unsafe { slice_mut_arg(out, cap, "out") }?;
        // SAFETY: the caller guarantees `props` points to `count` floats.
        let p = unsafe { slice_arg(props, count, "props") }?;
        if p.len() != FOLEY_CUSTOM_MATERIAL_FLOATS {
            return Err(FfiError::invalid(
                "wrong number of floats for a custom material",
            ));
        }
        let behaviour = usize::try_from(p[0].round() as i64)
            .ok()
            .and_then(Material::from_index)
            .ok_or_else(|| FfiError::invalid("unknown behaviour material"))?;
        let range = |what: &str, x: f32, lo: f32, hi: f32| -> Result<f32, FfiError> {
            (lo..=hi)
                .contains(&x)
                .then_some(x)
                .ok_or_else(|| FfiError::invalid(format!("{what} out of range")))
        };
        let material = MaterialProps {
            behavior: behaviour,
            young: range("Young's modulus", p[1], 1e8, 5e11)?,
            density: range("density", p[2], 1.0, 20_000.0)?,
            poisson: range("Poisson's ratio", p[3], 0.0, 0.49)?,
            loss: range("loss factor", p[4], 0.0005, 2.0)?,
            thickness: range("plate thickness", p[5], 0.001, 1.0)?,
            fragments: range("fragments", p[6], 0.0, 200.0)? as usize,
            restitution: range("restitution", p[7], 0.0, 0.95)?,
            shard: if p[8] == 0.0 {
                0.0
            } else {
                range("shard thickness", p[8], 0.0005, 0.2)?
            },
        };
        if !(0.1..=30.0).contains(&speed) {
            return Err(FfiError::invalid("speed out of range"));
        }
        let sound = match kind {
            FOLEY_CONTACT_BREAK => fracture_with(&material, seed),
            FOLEY_CONTACT_TAP => contact_with(Contact::Tap, &material, speed, seed),
            FOLEY_CONTACT_PLACE => contact_with(Contact::Place, &material, speed, seed),
            FOLEY_CONTACT_LAND => contact_with(Contact::Land, &material, speed, seed),
            FOLEY_CONTACT_STEP => contact_with(Contact::Step, &material, speed, seed),
            _ => return Err(FfiError::invalid("unknown contact")),
        };
        if let Some(dest) = out.get_mut(..sound.len()) {
            for (out, sample) in dest.iter_mut().zip(&sound) {
                *out = to_pcm(*sample);
            }
        }
        *needed = sound.len();
        Ok(())
    })
}

/// Renders a primitive of `kind` (`FOLEY_PRIMITIVE_*`) from `seed`:
/// `params` are its floats (see the kinds for the order and ranges). Writes
/// it to `out` if `cap` samples are enough, and how many it needs to
/// `*needed` either way.
///
/// # Safety
/// `params` must point to `count` floats; `out` must be writable for `cap`
/// samples; `needed` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_render_primitive(
    kind: i32,
    params: *const f32,
    count: usize,
    seed: u64,
    out: *mut i16,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `needed` is writable.
        let needed = unsafe { out_arg(needed, "needed") }?;
        // SAFETY: the caller guarantees `out` is writable for `cap` samples.
        let out = unsafe { slice_mut_arg(out, cap, "out") }?;
        // SAFETY: the caller guarantees `params` points to `count` floats.
        let p = unsafe { slice_arg(params, count, "params") }?;
        if p.len() > FOLEY_PRIMITIVE_MAX_FLOATS || !p.iter().all(|x| x.is_finite()) {
            return Err(FfiError::invalid(
                "wrong number of floats, or a non-finite one",
            ));
        }
        let given = |i: usize| -> Result<f32, FfiError> {
            p.get(i)
                .copied()
                .ok_or_else(|| FfiError::invalid("too few floats for this primitive"))
        };
        let range = |what: &str, x: f32, lo: f32, hi: f32| -> Result<f32, FfiError> {
            (lo..=hi)
                .contains(&x)
                .then_some(x)
                .ok_or_else(|| FfiError::invalid(format!("{what} out of range")))
        };
        let gain = |i: usize| -> Result<f32, FfiError> { range("gain", given(i)?, 0.0, 1.0) };
        let sound = match kind {
            FOLEY_PRIMITIVE_BUBBLE => bubble(range("radius_mm", given(0)?, 0.05, 20.0)?, seed),
            FOLEY_PRIMITIVE_NOISE => noise_burst(
                range("centre", given(0)?, 20.0, 20_000.0)?,
                range("q", given(1)?, 0.5, 20.0)?,
                range("seconds", given(2)?, 0.005, 5.0)?,
                gain(3)?,
                seed,
            ),
            FOLEY_PRIMITIVE_TONE => tone(
                range("centre", given(0)?, 20.0, 10_000.0)?,
                range("target", given(1)?, 20.0, 20_000.0)?,
                range("seconds", given(2)?, 0.01, 5.0)?,
                gain(3)?,
                seed,
            ),
            FOLEY_PRIMITIVE_GUST => gust(
                range("speed", given(0)?, 0.0, 60.0)?,
                range("seconds", given(1)?, 0.2, 10.0)?,
                gain(2)?,
                seed,
            ),
            _ => return Err(FfiError::invalid("unknown primitive")),
        };
        if let Some(dest) = out.get_mut(..sound.len()) {
            for (out, sample) in dest.iter_mut().zip(&sound) {
                *out = to_pcm(*sample);
            }
        }
        *needed = sound.len();
        Ok(())
    })
}

/// Renders a `call` (`FOLEY_CREATURE_*`) of another mod's creature: `spec`
/// is its voice ([`FOLEY_CREATURE_FLOATS`] floats, see them for the order
/// and ranges), made up from `seed`. Writes it to `out` if `cap` samples are
/// enough, and how many it needs to `*needed` either way.
///
/// # Safety
/// `spec` must point to `count` floats; `out` must be writable for `cap`
/// samples; `needed` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_render_creature(
    spec: *const f32,
    count: usize,
    call: i32,
    seed: u64,
    out: *mut i16,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `needed` is writable.
        let needed = unsafe { out_arg(needed, "needed") }?;
        // SAFETY: the caller guarantees `out` is writable for `cap` samples.
        let out = unsafe { slice_mut_arg(out, cap, "out") }?;
        // SAFETY: the caller guarantees `spec` points to `count` floats.
        let p = unsafe { slice_arg(spec, count, "spec") }?;
        if p.len() != FOLEY_CREATURE_FLOATS || !p.iter().all(|x| x.is_finite()) {
            return Err(FfiError::invalid(
                "wrong number of floats for a creature, or a non-finite one",
            ));
        }
        let range = |what: &str, x: f32, lo: f32, hi: f32| -> Result<f32, FfiError> {
            (lo..=hi)
                .contains(&x)
                .then_some(x)
                .ok_or_else(|| FfiError::invalid(format!("{what} out of range")))
        };
        let creature = CreatureSpec {
            length: range("tract length", p[0], 0.03, 1.0)?,
            f0: range("pitch", p[1], 30.0, 4000.0)?,
            rough: range("rough", p[2], 0.0, 1.0)?,
            breathy: range("breathy", p[3], 0.0, 1.0)?,
        };
        let call = match call {
            FOLEY_CREATURE_AMBIENT => CreatureCall::Ambient,
            FOLEY_CREATURE_HURT => CreatureCall::Hurt,
            FOLEY_CREATURE_DEATH => CreatureCall::Death,
            FOLEY_CREATURE_GROWL => CreatureCall::Growl,
            FOLEY_CREATURE_BREATH => CreatureCall::Breath,
            _ => return Err(FfiError::invalid("unknown creature call")),
        };
        let mut rng = foley_synth::Rng::new(seed);
        let sound = creature_voice(&mut rng, &creature, call);
        if let Some(dest) = out.get_mut(..sound.len()) {
            for (out, sample) in dest.iter_mut().zip(&sound) {
                *out = to_pcm(*sample);
            }
        }
        *needed = sound.len();
        Ok(())
    })
}

/// Renders a `call` (`FOLEY_CALL_*`) of `mob` (an index below
/// [`FOLEY_MOBS`]), made up from `seed`. Writes it to `out` if `cap` samples
/// are enough, and how many it needs to `*needed` either way.
///
/// # Safety
/// `out` must be writable for `cap` samples; `needed` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_render_voice(
    mob: i32,
    call: i32,
    seed: u64,
    out: *mut i16,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `needed` is writable.
        let needed = unsafe { out_arg(needed, "needed") }?;
        // SAFETY: the caller guarantees `out` is writable for `cap` samples.
        let out = unsafe { slice_mut_arg(out, cap, "out") }?;
        let mob = usize::try_from(mob)
            .ok()
            .and_then(Mob::from_index)
            .ok_or_else(|| FfiError::invalid("unknown mob"))?;
        let call = usize::try_from(call)
            .ok()
            .and_then(Call::from_index)
            .ok_or_else(|| FfiError::invalid("unknown call"))?;
        let sound = voice(mob, call, seed);
        if let Some(dest) = out.get_mut(..sound.len()) {
            for (out, sample) in dest.iter_mut().zip(&sound) {
                *out = to_pcm(*sample);
            }
        }
        *needed = sound.len();
        Ok(())
    })
}

/// Copies this thread's last error message into `buf` (truncated to `cap`)
/// and returns its full length, or 0 if there is none.
///
/// # Safety
/// `buf` must be writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_last_error(buf: *mut u8, cap: usize) -> usize {
    let message = guard::last_error();
    // SAFETY: the caller guarantees `buf` is writable for `cap` bytes.
    if let Ok(buf) = unsafe { slice_mut_arg(buf, cap, "buf") } {
        let n = message.floor_char_boundary(buf.len());
        buf[..n].copy_from_slice(&message.as_bytes()[..n]);
    }
    message.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::no_alloc::allocations;

    /// Allocates on a real-time thread on purpose. CI runs it under
    /// RealtimeSanitizer expecting it to be stopped: proof the sanitizer is
    /// on, so its pass on the test below means something.
    #[test]
    #[ignore = "fails by design under RealtimeSanitizer"]
    fn realtime_sanitizer_catches_an_allocation() {
        rtsan_standalone::realtime_enter();
        drop(std::hint::black_box(vec![1u8; 16]));
        rtsan_standalone::realtime_exit();
    }

    /// Everything the stream thread calls (render, a new scene, a strike)
    /// allocates and frees nothing, for every bed, as scenes change and
    /// voices come and go. With `RTSAN_ENABLE=1` (in CI), RealtimeSanitizer
    /// also watches those calls for anything else that can block: locks,
    /// sleeps, system calls.
    #[test]
    fn the_audio_thread_never_allocates() {
        let rain = |rate: f32| [rate, 0.4, 0.2, 0.1, 0.1, 0.1, 0.05, 0.05, 2.0, 4.0];
        let beds: [(i32, Vec<Vec<f32>>); 6] = [
            (
                FOLEY_BED_RAIN,
                vec![rain(5.0).to_vec(), rain(150.0).to_vec(), rain(0.0).to_vec()],
            ),
            (
                FOLEY_BED_WIND,
                vec![
                    vec![2.0, 1.0, 0.8],
                    vec![40.0, 0.3, 1.0],
                    vec![0.0, 0.0, 0.0],
                ],
            ),
            (
                FOLEY_BED_WATER,
                vec![
                    vec![50.0, 8.0, 1.0],
                    vec![0.0, 200.0, 0.5],
                    vec![0.0, 0.0, 0.0],
                ],
            ),
            (
                FOLEY_BED_FIRE,
                vec![vec![1.0, 1.0], vec![200.0, 1.0], vec![0.0, 0.0]],
            ),
            (
                FOLEY_BED_LAVA,
                vec![vec![5.0, 1.0], vec![500.0, 1.0], vec![0.0, 0.0]],
            ),
            (
                FOLEY_BED_THUNDER,
                vec![vec![20.0, 70.0, 1.0], vec![5.0, 30.0, 0.2]],
            ),
        ];
        // The counter itself works.
        assert!(allocations(|| drop(std::hint::black_box(vec![1u8]))) > 0);
        // As the game reads it: 0.25 s at a time.
        let mut pcm = vec![0i16; 12_000];
        for (kind, scenes) in beds {
            let mut bed = std::ptr::null_mut();
            // SAFETY: `bed` is writable, every pointer is valid for its
            // length, and `bed` is freed last.
            unsafe {
                assert_eq!(foley_bed_create(kind, 7, &mut bed), FOLEY_OK);
                let mut heard = false;
                for (n, scene) in scenes.iter().cycle().take(3 * scenes.len()).enumerate() {
                    let made = allocations(|| {
                        assert_eq!(
                            foley_bed_set_scene(bed, scene.as_ptr(), scene.len()),
                            FOLEY_OK
                        );
                        if kind == FOLEY_BED_THUNDER && n < 2 {
                            assert_eq!(
                                foley_bed_strike(bed, 300.0, 0.0, 400.0, n as u64),
                                FOLEY_OK
                            );
                        }
                        for _ in 0..40 {
                            assert_eq!(
                                foley_bed_render(bed, pcm.as_mut_ptr(), pcm.len()),
                                FOLEY_OK
                            );
                            heard |= pcm.iter().any(|s| *s != 0);
                        }
                        rtsan_standalone::realtime_exit();
                    });
                    assert_eq!(made, 0, "bed {kind}, scene {n}: allocated {made} times");
                    // Gives the thunder thread time to make and free.
                    if kind == FOLEY_BED_THUNDER {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                }
                assert!(heard, "bed {kind} was silent");
                foley_bed_destroy(bed);
            }
        }
    }

    fn render(kind: i32, params: &[f32]) -> Vec<i16> {
        let mut bed = std::ptr::null_mut();
        let mut pcm = vec![0i16; 48_000];
        // SAFETY: every pointer is valid for its length; `bed` is freed last.
        unsafe {
            assert_eq!(foley_bed_create(kind, 1, &mut bed), FOLEY_OK);
            assert_eq!(
                foley_bed_set_scene(bed, params.as_ptr(), params.len()),
                FOLEY_OK
            );
            assert_eq!(
                foley_bed_set_scene(bed, params.as_ptr(), params.len() - 1),
                FOLEY_ERR_INVALID_ARGUMENT,
                "too few params"
            );
            assert_eq!(foley_bed_render(bed, pcm.as_mut_ptr(), pcm.len()), FOLEY_OK);
            foley_bed_destroy(bed);
        }
        pcm
    }

    #[test]
    fn every_bed_renders_pcm() {
        let rain = [10.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0];
        for (kind, params) in [
            (FOLEY_BED_RAIN, &rain[..]),
            (FOLEY_BED_WIND, &[12.0, 1.0, 0.5][..]),
            (FOLEY_BED_WATER, &[5.0, 5.0, 1.0][..]),
            (FOLEY_BED_FIRE, &[4.0, 1.0][..]),
            (FOLEY_BED_LAVA, &[20.0, 1.0][..]),
        ] {
            assert!(render(kind, params).iter().any(|&s| s != 0), "bed {kind}");
        }
        let mut bed = std::ptr::null_mut();
        // SAFETY: `bed` is writable, and freed after.
        unsafe {
            assert_eq!(
                foley_bed_create(99, 1, &mut bed),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            assert_eq!(foley_bed_create(FOLEY_BED_WIND, 1, &mut bed), FOLEY_OK);
            let bad = [12.0f32, 2.0, 0.5];
            assert_eq!(
                foley_bed_set_scene(bed, bad.as_ptr(), bad.len()),
                FOLEY_ERR_INVALID_ARGUMENT,
                "exposure over 1"
            );
            foley_bed_destroy(bed);
        }
    }

    #[test]
    fn thunder_is_struck() {
        let mut bed = std::ptr::null_mut();
        // The thunder is made off the stream's thread, so render a block at a
        // time as the game does until it has come in and been heard.
        let mut pcm = vec![0i16; 4800];
        let mut loudest = 0i16;
        let scene = [15.0f32, 90.0, 1.0];
        // SAFETY: every pointer is valid for its length; `bed` is freed last.
        unsafe {
            assert_eq!(foley_bed_create(FOLEY_BED_THUNDER, 1, &mut bed), FOLEY_OK);
            assert_eq!(foley_bed_set_scene(bed, scene.as_ptr(), 3), FOLEY_OK);
            assert_eq!(foley_bed_strike(bed, 200.0, 0.0, 0.0, 3), FOLEY_OK);
            for _ in 0..200 {
                for _ in 0..200 {
                    assert_eq!(foley_bed_render(bed, pcm.as_mut_ptr(), pcm.len()), FOLEY_OK);
                    loudest = pcm.iter().fold(loudest, |m, s| m.max(s.saturating_abs()));
                    if loudest > 5000 {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                loudest = pcm.iter().fold(loudest, |m, s| m.max(s.saturating_abs()));
                if loudest > 5000 {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            foley_bed_destroy(bed);
            let mut wind = std::ptr::null_mut();
            assert_eq!(foley_bed_create(FOLEY_BED_WIND, 1, &mut wind), FOLEY_OK);
            assert_eq!(
                foley_bed_strike(wind, 200.0, 0.0, 0.0, 3),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            foley_bed_destroy(wind);
        }
        assert!(loudest > 5000, "a crack");
    }

    #[test]
    fn every_contact_on_every_material_renders() {
        for kind in [
            FOLEY_CONTACT_TAP,
            FOLEY_CONTACT_PLACE,
            FOLEY_CONTACT_LAND,
            FOLEY_CONTACT_BREAK,
            FOLEY_CONTACT_STEP,
        ] {
            for material in 0..FOLEY_MATERIALS as i32 {
                let mut needed = 0;
                // SAFETY: a null buffer with capacity 0 is not written.
                unsafe {
                    assert_eq!(
                        foley_render_contact(
                            kind,
                            material,
                            3.0,
                            1,
                            std::ptr::null_mut(),
                            0,
                            &mut needed
                        ),
                        FOLEY_OK
                    );
                }
                let mut pcm = vec![0i16; needed];
                // SAFETY: `pcm` is writable for `needed` samples.
                unsafe {
                    assert_eq!(
                        foley_render_contact(
                            kind,
                            material,
                            3.0,
                            1,
                            pcm.as_mut_ptr(),
                            needed,
                            &mut needed
                        ),
                        FOLEY_OK
                    );
                }
                assert!(pcm.iter().any(|&s| s.abs() > 1000), "{kind} on {material}");
            }
        }
        let mut needed = 0;
        // SAFETY: a null buffer with capacity 0 is not written.
        unsafe {
            assert_eq!(
                foley_render_contact(0, 10, 3.0, 1, std::ptr::null_mut(), 0, &mut needed),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            assert_eq!(
                foley_render_contact(7, 0, 3.0, 1, std::ptr::null_mut(), 0, &mut needed),
                FOLEY_ERR_INVALID_ARGUMENT
            );
        }
    }

    #[test]
    fn creatures_speak_and_are_checked() {
        // A human-like creature: tract, pitch, rough, breathy.
        let spec = [0.17f32, 120.0, 0.5, 0.5];
        for call in 0..=4 {
            let mut needed = 0;
            // SAFETY: a null buffer with capacity 0 is not written; `spec`
            // and `needed` are valid.
            unsafe {
                assert_eq!(
                    foley_render_creature(
                        spec.as_ptr(),
                        FOLEY_CREATURE_FLOATS,
                        call,
                        7,
                        std::ptr::null_mut(),
                        0,
                        &mut needed
                    ),
                    FOLEY_OK
                );
            }
            assert!(needed > 100, "{call}: {needed}");
            let mut pcm = vec![0i16; needed];
            // SAFETY: `pcm` is writable for `needed` samples.
            unsafe {
                assert_eq!(
                    foley_render_creature(
                        spec.as_ptr(),
                        FOLEY_CREATURE_FLOATS,
                        call,
                        7,
                        pcm.as_mut_ptr(),
                        needed,
                        &mut needed
                    ),
                    FOLEY_OK
                );
            }
            assert!(pcm.iter().any(|&s| s.abs() > 500), "{call}");
        }
        let mut needed = 0;
        // SAFETY: the checks refuse before writing; every pointer is valid.
        unsafe {
            let too_long = [2.0f32, 120.0, 0.5, 0.5];
            assert_eq!(
                foley_render_creature(
                    too_long.as_ptr(),
                    FOLEY_CREATURE_FLOATS,
                    0,
                    7,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            let too_rough = [0.17f32, 120.0, 1.5, 0.5];
            assert_eq!(
                foley_render_creature(
                    too_rough.as_ptr(),
                    FOLEY_CREATURE_FLOATS,
                    0,
                    7,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            assert_eq!(
                foley_render_creature(
                    spec.as_ptr(),
                    FOLEY_CREATURE_FLOATS - 1,
                    0,
                    7,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            assert_eq!(
                foley_render_creature(
                    spec.as_ptr(),
                    FOLEY_CREATURE_FLOATS,
                    9,
                    7,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
        }
    }

    #[test]
    fn primitives_render_and_are_checked() {
        let cases: [(i32, Vec<f32>); 4] = [
            (FOLEY_PRIMITIVE_BUBBLE, vec![1.0]),
            (FOLEY_PRIMITIVE_NOISE, vec![2000.0, 1.2, 0.3, 0.8]),
            (FOLEY_PRIMITIVE_TONE, vec![440.0, 880.0, 0.5, 0.6]),
            (FOLEY_PRIMITIVE_GUST, vec![15.0, 3.0, 1.0]),
        ];
        for (kind, params) in &cases {
            let mut needed = 0;
            // SAFETY: a null buffer with capacity 0 is not written; `params`
            // and `needed` are valid.
            unsafe {
                assert_eq!(
                    foley_render_primitive(
                        *kind,
                        params.as_ptr(),
                        params.len(),
                        7,
                        std::ptr::null_mut(),
                        0,
                        &mut needed
                    ),
                    FOLEY_OK
                );
            }
            assert!(needed > 100, "{kind}: {needed}");
            let mut pcm = vec![0i16; needed];
            // SAFETY: `pcm` is writable for `needed` samples.
            unsafe {
                assert_eq!(
                    foley_render_primitive(
                        *kind,
                        params.as_ptr(),
                        params.len(),
                        7,
                        pcm.as_mut_ptr(),
                        needed,
                        &mut needed
                    ),
                    FOLEY_OK
                );
            }
            assert!(pcm.iter().any(|&s| s.abs() > 500), "{kind}");
            // The primitives repeat themselves from the same seed.
            let mut again = vec![0i16; needed];
            // SAFETY: `again` is writable for `needed` samples.
            unsafe {
                assert_eq!(
                    foley_render_primitive(
                        *kind,
                        params.as_ptr(),
                        params.len(),
                        7,
                        again.as_mut_ptr(),
                        needed,
                        &mut needed
                    ),
                    FOLEY_OK
                );
            }
            assert_eq!(pcm, again);
        }
        let mut needed = 0;
        // SAFETY: the checks refuse before writing; every pointer is valid.
        unsafe {
            let too_few = [2000.0f32, 1.2];
            assert_eq!(
                foley_render_primitive(
                    FOLEY_PRIMITIVE_NOISE,
                    too_few.as_ptr(),
                    too_few.len(),
                    7,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            let loud = [2000.0f32, 1.2, 0.3, 2.0];
            assert_eq!(
                foley_render_primitive(
                    FOLEY_PRIMITIVE_NOISE,
                    loud.as_ptr(),
                    loud.len(),
                    7,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            let huge = [1.0f32, 1.0, 1.0, 1.0, 1.0];
            assert_eq!(
                foley_render_primitive(
                    FOLEY_PRIMITIVE_BUBBLE,
                    huge.as_ptr(),
                    huge.len(),
                    7,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            assert_eq!(
                foley_render_primitive(
                    9,
                    [1.0f32].as_ptr(),
                    1,
                    7,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
        }
    }

    #[test]
    fn a_custom_material_renders_and_is_checked() {
        // Wood's physics as a custom material would send them: behaviour
        // wood, E, ρ, ν, η, plate, fragments, restitution, shard.
        let props =
            |fragments: f32| vec![1.0, 10e9, 500.0, 0.3, 0.025, 0.025, fragments, 0.4, 0.01];
        for kind in 0..=4 {
            let mut needed = 0;
            // SAFETY: a null buffer with capacity 0 is not written; `props`
            // and `needed` are valid.
            unsafe {
                assert_eq!(
                    foley_render_contact_custom(
                        props(6.0).as_ptr(),
                        FOLEY_CUSTOM_MATERIAL_FLOATS,
                        kind,
                        3.0,
                        1,
                        std::ptr::null_mut(),
                        0,
                        &mut needed
                    ),
                    FOLEY_OK
                );
            }
            assert!(needed > 100, "{kind}: {needed}");
            let mut pcm = vec![0i16; needed];
            // SAFETY: `pcm` is writable for `needed` samples.
            unsafe {
                assert_eq!(
                    foley_render_contact_custom(
                        props(6.0).as_ptr(),
                        FOLEY_CUSTOM_MATERIAL_FLOATS,
                        kind,
                        3.0,
                        1,
                        pcm.as_mut_ptr(),
                        needed,
                        &mut needed
                    ),
                    FOLEY_OK
                );
            }
            assert!(pcm.iter().any(|&s| s.abs() > 1000), "{kind}");
        }
        let mut needed = 0;
        // SAFETY: the checks refuse before writing; every pointer is valid.
        unsafe {
            // Too few floats.
            assert_eq!(
                foley_render_contact_custom(
                    props(6.0).as_ptr(),
                    FOLEY_CUSTOM_MATERIAL_FLOATS - 1,
                    0,
                    3.0,
                    1,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            // Unknown behaviour, Young's modulus out of range, too many
            // fragments, an unknown contact.
            let mut behaviour = props(6.0);
            behaviour[0] = FOLEY_MATERIALS as f32 + 1.0;
            assert_eq!(
                foley_render_contact_custom(
                    behaviour.as_ptr(),
                    FOLEY_CUSTOM_MATERIAL_FLOATS,
                    0,
                    3.0,
                    1,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            let mut young = props(6.0);
            young[1] = 1e12;
            assert_eq!(
                foley_render_contact_custom(
                    young.as_ptr(),
                    FOLEY_CUSTOM_MATERIAL_FLOATS,
                    0,
                    3.0,
                    1,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            let mut fragments = props(6.0);
            fragments[6] = 300.0;
            assert_eq!(
                foley_render_contact_custom(
                    fragments.as_ptr(),
                    FOLEY_CUSTOM_MATERIAL_FLOATS,
                    0,
                    3.0,
                    1,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            assert_eq!(
                foley_render_contact_custom(
                    props(6.0).as_ptr(),
                    FOLEY_CUSTOM_MATERIAL_FLOATS,
                    7,
                    3.0,
                    1,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
        }
    }

    #[test]
    fn every_mob_speaks() {
        for mob in 0..FOLEY_MOBS as i32 {
            for call in 0..3 {
                let mut needed = 0;
                // SAFETY: a null buffer with capacity 0 is not written.
                unsafe {
                    assert_eq!(
                        foley_render_voice(mob, call, 1, std::ptr::null_mut(), 0, &mut needed),
                        FOLEY_OK
                    );
                }
                let mut pcm = vec![0i16; needed];
                // SAFETY: `pcm` is writable for `needed` samples.
                unsafe {
                    assert_eq!(
                        foley_render_voice(mob, call, 1, pcm.as_mut_ptr(), needed, &mut needed),
                        FOLEY_OK
                    );
                }
                assert!(pcm.iter().any(|&s| s.abs() > 5000), "{mob} {call}");
            }
        }
        let mut needed = 0;
        // SAFETY: a null buffer with capacity 0 is not written.
        unsafe {
            assert_eq!(
                foley_render_voice(52, 0, 1, std::ptr::null_mut(), 0, &mut needed),
                FOLEY_ERR_INVALID_ARGUMENT
            );
        }
    }

    #[test]
    fn a_one_shot_says_how_long_it_is_then_fills_the_buffer() {
        for (kind, size) in [
            (FOLEY_SHOT_DRIP_INTO_WATER, 1.0),
            (FOLEY_SHOT_DRIP_ON_STONE, 1.0),
            (FOLEY_SHOT_LAVA_DRIP, 1.0),
            (FOLEY_SHOT_CRICKETS, 22.0),
            (FOLEY_SHOT_BIRDSONG, 5.0),
            (FOLEY_SHOT_FROG, 8.0),
            (FOLEY_SHOT_SURF, 1.0),
            (FOLEY_SHOT_WOODPECKER, 0.0),
            (FOLEY_SHOT_OWL, 0.0),
            (FOLEY_SHOT_CICADAS, 30.0),
            (FOLEY_SHOT_HOWL, 0.0),
            (FOLEY_SHOT_BRUSH, 1.0),
            (FOLEY_SHOT_ICE_SINGING, 0.0),
            (FOLEY_SHOT_SETTLING_STONE, 0.0),
            (FOLEY_SHOT_FLUSH, 0.0),
            (FOLEY_SHOT_BUZZ, 1.0),
            (FOLEY_SHOT_SCULK_RING, 6.0),
        ] {
            let mut needed = 0;
            // SAFETY: a null buffer with capacity 0 is not written.
            unsafe {
                assert_eq!(
                    foley_render_one_shot(kind, size, 7, std::ptr::null_mut(), 0, &mut needed),
                    FOLEY_OK
                );
            }
            assert!(needed > 100, "{kind}: {needed} samples");
            let mut pcm = vec![0i16; needed];
            // SAFETY: `pcm` is writable for `needed` samples.
            unsafe {
                assert_eq!(
                    foley_render_one_shot(kind, size, 7, pcm.as_mut_ptr(), needed, &mut needed),
                    FOLEY_OK
                );
            }
            assert!(pcm.iter().any(|&s| s.abs() > 1000), "{kind}");
        }
        let mut needed = 0;
        // SAFETY: a null buffer with capacity 0 is not written.
        unsafe {
            assert_eq!(
                foley_render_one_shot(
                    FOLEY_SHOT_DRIP_ON_STONE,
                    50.0,
                    7,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
        }
    }

    /// M13: every species in the M13 catalogue renders via `FOLEY_SHOT_BIRDSONG`,
    /// and a species index past the catalogue's last entry is rejected.
    #[test]
    fn every_birdsong_species_renders() {
        // Every index up to (and including) the catalogue's last renders.
        for n in 0..Birds::ALL.len() {
            let mut needed = 0;
            // SAFETY: null buffer with capacity 0 is not written.
            unsafe {
                assert_eq!(
                    foley_render_one_shot(
                        FOLEY_SHOT_BIRDSONG,
                        n as f32,
                        7,
                        std::ptr::null_mut(),
                        0,
                        &mut needed
                    ),
                    FOLEY_OK,
                    "species {n}"
                );
            }
            assert!(needed > 100, "species {n}: {needed} samples");
        }
        // One past the last must be rejected (size is `Birds::ALL.len() - 1`).
        let mut needed = 0;
        // SAFETY: the check refuses before writing.
        unsafe {
            assert_eq!(
                foley_render_one_shot(
                    FOLEY_SHOT_BIRDSONG,
                    Birds::ALL.len() as f32,
                    7,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                FOLEY_ERR_INVALID_ARGUMENT
            );
        }
    }
}
