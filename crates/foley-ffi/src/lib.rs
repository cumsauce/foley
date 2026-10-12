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

use foley_synth::{Bubble, Rain, RainScene, Rng, SAMPLE_RATE, Surface, Wind, WindScene, to_pcm};

use crate::guard::{FfiError, guard, out_arg, slice_arg, slice_mut_arg};

/// Version of this C ABI. Java refuses a library whose version differs.
/// 1: rain beds and drips (the M0 spike).
/// 2: rain takes a scene: its rate, what it lands on and any roof.
/// 3: wind beds.
pub const FOLEY_ABI_VERSION: u32 = 3;

/// Surfaces rain lands on, in the order `foley_rain_set_scene` takes their
/// shares: water, stone, leaves, wood, soft (grass, dirt, sand, wool,
/// snow), glass, metal.
pub const FOLEY_SURFACES: usize = 7;

pub const FOLEY_OK: i32 = 0;
/// A null pointer or out-of-range value.
pub const FOLEY_ERR_INVALID_ARGUMENT: i32 = -1;
/// The call panicked.
pub const FOLEY_ERR_PANIC: i32 = -3;

/// Opaque handle to a rain bed.
#[derive(Debug)]
pub struct FoleyRain {
    bed: Rain,
    scratch: Vec<f32>,
}

/// Opaque handle to a wind bed.
#[derive(Debug)]
pub struct FoleyWind {
    bed: Wind,
    scratch: Vec<f32>,
}

/// Returns [`FOLEY_ABI_VERSION`]. Safe to call before anything else.
#[unsafe(no_mangle)]
pub extern "C" fn foley_abi_version() -> u32 {
    FOLEY_ABI_VERSION
}

/// Starts a rain bed, silent until `foley_rain_set_scene` gives it rain.
///
/// # Safety
/// `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_rain_create(seed: u64, out: *mut *mut FoleyRain) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `out` is writable.
        let out = unsafe { out_arg(out, "out") }?;
        *out = Box::into_raw(Box::new(FoleyRain {
            bed: Rain::new(seed),
            scratch: Vec::new(),
        }));
        Ok(())
    })
}

/// Sets what the rain round the listener is like: `rate` in mm/h (0 for
/// none, up to 200), the shares of what it lands on ([`FOLEY_SURFACES`]
/// floats in their order), and the roof the listener shelters under, as a
/// surface index and its height above the ear in blocks (`roof` -1 for none).
///
/// # Safety
/// `rain` must be live and not used on another thread during the call;
/// `surfaces` must point to `FOLEY_SURFACES` floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_rain_set_scene(
    rain: *mut FoleyRain,
    rate: f32,
    surfaces: *const f32,
    roof: i32,
    roof_height: f32,
) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `rain` is live and used by this
        // thread alone.
        let rain = unsafe { rain.as_mut() }.ok_or_else(|| FfiError::invalid("rain is null"))?;
        // SAFETY: the caller guarantees `surfaces` points to
        // `FOLEY_SURFACES` floats.
        let shares = unsafe { slice_arg(surfaces, FOLEY_SURFACES, "surfaces") }?;
        if !(0.0..=200.0).contains(&rate) || !shares.iter().all(|s| s.is_finite() && *s >= 0.0) {
            return Err(FfiError::invalid("rate or surfaces out of range"));
        }
        let roof = match roof {
            -1 => None,
            index => Some((
                usize::try_from(index)
                    .ok()
                    .and_then(Surface::from_index)
                    .ok_or_else(|| FfiError::invalid("unknown roof surface"))?,
                roof_height.clamp(0.5, 64.0),
            )),
        };
        let mut scene = RainScene {
            rate,
            surfaces: [0.0; FOLEY_SURFACES],
            roof,
        };
        scene.surfaces.copy_from_slice(shares);
        rain.bed.set_scene(scene);
        Ok(())
    })
}

/// Frees a rain bed. Null is ignored.
///
/// # Safety
/// `rain` must come from `foley_rain_create` and not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_rain_destroy(rain: *mut FoleyRain) {
    if !rain.is_null() {
        // SAFETY: `rain` came from `Box::into_raw` and is never used again.
        drop(unsafe { Box::from_raw(rain) });
    }
}

/// Renders the next `frames` samples of `rain` into `out`, as 16-bit PCM.
///
/// # Safety
/// `rain` must be live and not used on another thread during the call;
/// `out` must be writable for `frames` samples.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_rain_render(
    rain: *mut FoleyRain,
    out: *mut i16,
    frames: usize,
) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `rain` is live and used by this
        // thread alone.
        let rain = unsafe { rain.as_mut() }.ok_or_else(|| FfiError::invalid("rain is null"))?;
        // SAFETY: the caller guarantees `out` is writable for `frames`.
        let out = unsafe { slice_mut_arg(out, frames, "out") }?;
        rain.scratch.resize(frames, 0.0);
        rain.bed.render(&mut rain.scratch);
        for (out, sample) in out.iter_mut().zip(&rain.scratch) {
            *out = to_pcm(*sample);
        }
        Ok(())
    })
}

/// Starts a wind bed, calm until `foley_wind_set_scene` gives it wind.
///
/// # Safety
/// `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_wind_create(seed: u64, out: *mut *mut FoleyWind) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `out` is writable.
        let out = unsafe { out_arg(out, "out") }?;
        *out = Box::into_raw(Box::new(FoleyWind {
            bed: Wind::new(seed),
            scratch: Vec::new(),
        }));
        Ok(())
    })
}

/// Sets the wind round the listener: its mean `speed` in m/s (0 to 60), how
/// `exposed` to it the listener is (0 shut in to 1 in the open) and how much
/// of what's round about is `leaves` (0 to 1).
///
/// # Safety
/// `wind` must be live and not used on another thread during the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_wind_set_scene(
    wind: *mut FoleyWind,
    speed: f32,
    exposure: f32,
    leaves: f32,
) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `wind` is live and used by this
        // thread alone.
        let wind = unsafe { wind.as_mut() }.ok_or_else(|| FfiError::invalid("wind is null"))?;
        if !(0.0..=60.0).contains(&speed)
            || !(0.0..=1.0).contains(&exposure)
            || !(0.0..=1.0).contains(&leaves)
        {
            return Err(FfiError::invalid("speed, exposure or leaves out of range"));
        }
        wind.bed.set_scene(WindScene {
            speed,
            exposure,
            leaves,
        });
        Ok(())
    })
}

/// Renders the next `frames` samples of `wind` into `out`, as 16-bit PCM.
///
/// # Safety
/// `wind` must be live and not used on another thread during the call;
/// `out` must be writable for `frames` samples.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_wind_render(
    wind: *mut FoleyWind,
    out: *mut i16,
    frames: usize,
) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `wind` is live and used by this
        // thread alone.
        let wind = unsafe { wind.as_mut() }.ok_or_else(|| FfiError::invalid("wind is null"))?;
        // SAFETY: the caller guarantees `out` is writable for `frames`.
        let out = unsafe { slice_mut_arg(out, frames, "out") }?;
        wind.scratch.resize(frames, 0.0);
        wind.bed.render(&mut wind.scratch);
        for (out, sample) in out.iter_mut().zip(&wind.scratch) {
            *out = to_pcm(*sample);
        }
        Ok(())
    })
}

/// Frees a wind bed. Null is ignored.
///
/// # Safety
/// `wind` must come from `foley_wind_create` and not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_wind_destroy(wind: *mut FoleyWind) {
    if !wind.is_null() {
        // SAFETY: `wind` came from `Box::into_raw` and is never used again.
        drop(unsafe { Box::from_raw(wind) });
    }
}

/// A water drip: a drop's tick on landing, then the bubble it traps ringing
/// at its Minnaert frequency. `radius` is the bubble's, in millimetres (0.1
/// to 10). Writes it to `out` if `cap` samples are enough, and how many it
/// needs to `*needed` either way.
///
/// # Safety
/// `out` must be writable for `cap` samples; `needed` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_render_drip(
    radius: f32,
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
        if !(0.1..=10.0).contains(&radius) {
            return Err(FfiError::invalid("radius out of range"));
        }
        let pcm = drip(radius / 1000.0, seed);
        if let Some(dest) = out.get_mut(..pcm.len()) {
            dest.copy_from_slice(&pcm);
        }
        *needed = pcm.len();
        Ok(())
    })
}

fn drip(radius: f32, seed: u64) -> Vec<i16> {
    let mut rng = Rng::new(seed);
    let mut bubble = Bubble::new(radius, 0.5);
    // The drop's tick as it lands: 2 ms of decaying noise, then the bubble a
    // few milliseconds later, once the cavity pinches off.
    let tick = SAMPLE_RATE as usize / 500;
    let gap = SAMPLE_RATE as usize / 250;
    let mut out = Vec::new();
    for i in 0..tick {
        out.push(to_pcm(0.3 * rng.signed() * (1.0 - i as f32 / tick as f32)));
    }
    out.extend(std::iter::repeat_n(0, gap));
    while !bubble.done() {
        out.push(to_pcm(bubble.sample()));
    }
    out
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

    #[test]
    fn rain_renders_pcm() {
        let mut rain = std::ptr::null_mut();
        let mut pcm = vec![0i16; 4800];
        // SAFETY: every pointer is valid for its length; `rain` is freed last.
        unsafe {
            assert_eq!(foley_rain_create(1, &mut rain), FOLEY_OK);
            let surfaces = [1.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
            assert_eq!(
                foley_rain_set_scene(rain, 10.0, surfaces.as_ptr(), -1, 0.0),
                FOLEY_OK
            );
            assert_eq!(
                foley_rain_set_scene(rain, 10.0, surfaces.as_ptr(), 9, 2.0),
                FOLEY_ERR_INVALID_ARGUMENT,
                "unknown roof"
            );
            assert_eq!(
                foley_rain_render(rain, pcm.as_mut_ptr(), pcm.len()),
                FOLEY_OK
            );
            foley_rain_destroy(rain);
        }
        assert!(pcm.iter().any(|&s| s != 0));
    }

    #[test]
    fn wind_renders_pcm() {
        let mut wind = std::ptr::null_mut();
        let mut pcm = vec![0i16; 48_000];
        // SAFETY: every pointer is valid for its length; `wind` is freed last.
        unsafe {
            assert_eq!(foley_wind_create(1, &mut wind), FOLEY_OK);
            assert_eq!(foley_wind_set_scene(wind, 12.0, 1.0, 0.5), FOLEY_OK);
            assert_eq!(
                foley_wind_set_scene(wind, 12.0, 2.0, 0.5),
                FOLEY_ERR_INVALID_ARGUMENT
            );
            assert_eq!(
                foley_wind_render(wind, pcm.as_mut_ptr(), pcm.len()),
                FOLEY_OK
            );
            foley_wind_destroy(wind);
        }
        assert!(pcm.iter().any(|&s| s != 0));
    }

    #[test]
    fn a_drip_says_how_long_it_is_then_fills_the_buffer() {
        let mut needed = 0;
        // SAFETY: a null buffer with capacity 0 is not written.
        unsafe {
            assert_eq!(
                foley_render_drip(1.0, 7, std::ptr::null_mut(), 0, &mut needed),
                FOLEY_OK
            );
        }
        // About 15 ms of ringing after the tick: a millimetre drip.
        assert!(needed > 900, "{needed} samples");
        let mut pcm = vec![0i16; needed];
        // SAFETY: `pcm` is writable for `needed` samples.
        unsafe {
            assert_eq!(
                foley_render_drip(1.0, 7, pcm.as_mut_ptr(), needed, &mut needed),
                FOLEY_OK
            );
            assert_eq!(
                foley_render_drip(50.0, 7, pcm.as_mut_ptr(), needed, &mut needed),
                FOLEY_ERR_INVALID_ARGUMENT
            );
        }
        assert!(pcm.iter().any(|&s| s.abs() > 1000));
    }
}
