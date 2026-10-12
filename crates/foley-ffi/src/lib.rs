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

use foley_synth::{Bubble, RainOnWater, Rng, SAMPLE_RATE, to_pcm};

use crate::guard::{FfiError, guard, out_arg, slice_mut_arg};

/// Version of this C ABI. Java refuses a library whose version differs.
/// 1: rain beds and drips (the M0 spike).
pub const FOLEY_ABI_VERSION: u32 = 1;

pub const FOLEY_OK: i32 = 0;
/// A null pointer or out-of-range value.
pub const FOLEY_ERR_INVALID_ARGUMENT: i32 = -1;
/// The call panicked.
pub const FOLEY_ERR_PANIC: i32 = -3;

/// Opaque handle to a rain bed.
#[derive(Debug)]
pub struct FoleyRain {
    bed: RainOnWater,
    scratch: Vec<f32>,
}

/// Returns [`FOLEY_ABI_VERSION`]. Safe to call before anything else.
#[unsafe(no_mangle)]
pub extern "C" fn foley_abi_version() -> u32 {
    FOLEY_ABI_VERSION
}

/// Starts a bed of rain falling on water, at `rate` drops a second.
///
/// # Safety
/// `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foley_rain_create(seed: u64, rate: f32, out: *mut *mut FoleyRain) -> i32 {
    guard(|| {
        // SAFETY: the caller guarantees `out` is writable.
        let out = unsafe { out_arg(out, "out") }?;
        *out = std::ptr::null_mut();
        if !(rate.is_finite() && rate >= 0.0) {
            return Err(FfiError::invalid("rate out of range"));
        }
        *out = Box::into_raw(Box::new(FoleyRain {
            bed: RainOnWater::new(seed, rate),
            scratch: Vec::new(),
        }));
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
            assert_eq!(foley_rain_create(1, 400.0, &mut rain), FOLEY_OK);
            assert_eq!(
                foley_rain_render(rain, pcm.as_mut_ptr(), pcm.len()),
                FOLEY_OK
            );
            foley_rain_destroy(rain);
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
