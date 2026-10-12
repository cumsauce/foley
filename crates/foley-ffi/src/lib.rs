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

use foley_synth::{
    Call, Contact, Fire, FireScene, Lava, LavaScene, Material, Mob, Rain, RainScene, Surface,
    Thunder, ThunderScene, Water, WaterScene, Wind, WindScene, birdsong, contact, crickets,
    drip_into_water, drip_on_stone, fracture, frog, lava_drip, to_pcm, voice,
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
pub const FOLEY_ABI_VERSION: u32 = 11;

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
/// for crickets (-30 to 50); the species (0 to 1000) for birdsong; the body
/// length in cm for a frog (2 to 20).
pub const FOLEY_SHOT_DRIP_INTO_WATER: i32 = 0;
pub const FOLEY_SHOT_DRIP_ON_STONE: i32 = 1;
pub const FOLEY_SHOT_LAVA_DRIP: i32 = 2;
pub const FOLEY_SHOT_CRICKETS: i32 = 3;
pub const FOLEY_SHOT_BIRDSONG: i32 = 4;
pub const FOLEY_SHOT_FROG: i32 = 5;

/// Materials for `foley_render_contact`: stone, wood, metal, glass, ice,
/// earth, granular (sand, gravel), plant, soft (wool), snow.
pub const FOLEY_MATERIALS: usize = 10;
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
    scratch: Vec<f32>,
}

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
            scratch: Vec::new(),
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
        let wanted = match bed.synth {
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
        match &mut bed.synth {
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
    })
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
        bed.scratch.resize(frames, 0.0);
        match &mut bed.synth {
            Synth::Rain(synth) => synth.render(&mut bed.scratch),
            Synth::Wind(synth) => synth.render(&mut bed.scratch),
            Synth::Water(synth) => synth.render(&mut bed.scratch),
            Synth::Fire(synth) => synth.render(&mut bed.scratch),
            Synth::Lava(synth) => synth.render(&mut bed.scratch),
            Synth::Thunder(synth) => synth.render(&mut bed.scratch),
        }
        for (out, sample) in out.iter_mut().zip(&bed.scratch) {
            *out = to_pcm(*sample);
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
            FOLEY_SHOT_BIRDSONG if (0.0..=1000.0).contains(&size) => birdsong(size as u32, seed),
            FOLEY_SHOT_FROG if (2.0..=20.0).contains(&size) => frog(size, seed),
            FOLEY_SHOT_CRICKETS | FOLEY_SHOT_BIRDSONG | FOLEY_SHOT_FROG => {
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
        let mut pcm = vec![0i16; 2 * 48_000];
        let scene = [15.0f32, 90.0, 1.0];
        // SAFETY: every pointer is valid for its length; `bed` is freed last.
        unsafe {
            assert_eq!(foley_bed_create(FOLEY_BED_THUNDER, 1, &mut bed), FOLEY_OK);
            assert_eq!(foley_bed_set_scene(bed, scene.as_ptr(), 3), FOLEY_OK);
            assert_eq!(foley_bed_strike(bed, 200.0, 0.0, 0.0, 3), FOLEY_OK);
            assert_eq!(foley_bed_render(bed, pcm.as_mut_ptr(), pcm.len()), FOLEY_OK);
            foley_bed_destroy(bed);
            let mut wind = std::ptr::null_mut();
            assert_eq!(foley_bed_create(FOLEY_BED_WIND, 1, &mut wind), FOLEY_OK);
            assert_eq!(
                foley_bed_strike(wind, 200.0, 0.0, 0.0, 3),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            foley_bed_destroy(wind);
        }
        assert!(pcm.iter().any(|&s| s.abs() > 5000), "a crack");
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
}
