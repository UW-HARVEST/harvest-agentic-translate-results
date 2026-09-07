//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (must match the C shared library exactly):
//!   * `void synth_pair(mp3d_sample_t *pcm, int nch, const float *z);`
//!
//! `mp3d_sample_t` is `int16_t` (see `include/lib.h`).

use std::ffi::c_int;

/// `typedef int16_t mp3d_sample_t;`
#[allow(non_camel_case_types)]
pub type mp3d_sample_t = i16;

/// Translation of the C `static int16_t mp3d_scale_pcm(float sample)`.
///
/// The original:
/// ```c
/// static int16_t mp3d_scale_pcm(float sample) {
///     if (sample >= 32766.5)
///         return (int16_t)32767;
///     if (sample <= -32767.5)
///         return (int16_t)-32768;
///     int16_t s = (int16_t)(sample + .5f);
///     s -= (s < 0);
///     return s;
/// }
/// ```
///
/// Notes on faithfulness:
/// * The literals `32766.5` / `-32767.5` are `double` in C, so `sample` is
///   promoted to `double` for the comparisons. Both constants are exactly
///   representable in `f32` as well, so the comparison outcome is identical,
///   but we perform it in `f64` to mirror the C promotion exactly.
/// * `sample + .5f` is a single-precision addition (FLT_EVAL_METHOD == 0 on the
///   target ABI), then converted to `int16_t` by truncation toward zero. The two
///   guards above bound the sum strictly inside (-32767.0, 32767.0), so the
///   truncation always fits in `int16_t` and no undefined/out-of-range
///   conversion can occur; the intermediate `as i32` mirrors the C compiler's
///   float -> int -> narrow sequence.
/// * `s -= (s < 0);` subtracts exactly 1 when `s` is negative (C `bool` -> `int`).
#[inline]
fn mp3d_scale_pcm(sample: f32) -> i16 {
    if sample as f64 >= 32766.5 {
        return 32767i16;
    }
    if sample as f64 <= -32767.5 {
        return -32768i16;
    }
    let mut s: i16 = (sample + 0.5f32) as i32 as i16;
    s -= (s < 0) as i16;
    s
}

/// Translation of the C `void synth_pair(mp3d_sample_t *pcm, int nch, const float *z)`.
///
/// The exported linker symbol is `synth_pair` (the public header declares no
/// namespace-renaming macros).
///
/// # Safety
///
/// Mirrors the C contract verbatim: `pcm` must be writable at indices `0` and
/// `16 * nch`, and `z` must be readable at indices `k * 64` and `2 + k * 64`
/// for `k` in `0..=14`. No validation is performed, exactly as in the C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn synth_pair(pcm: *mut mp3d_sample_t, nch: c_int, z: *const f32) {
    // Read helper mirroring C's `z[i]` (pointer arithmetic in units of float).
    #[inline(always)]
    unsafe fn g(z: *const f32, i: isize) -> f32 {
        unsafe { *z.offset(i) }
    }

    let mut a: f32;

    // a  = (z[14*64] - z[0])      * 29;
    a = (unsafe { g(z, 14 * 64) } - unsafe { g(z, 0) }) * 29f32;
    // a += (z[1*64]  + z[13*64])  * 213;
    a += (unsafe { g(z, 1 * 64) } + unsafe { g(z, 13 * 64) }) * 213f32;
    // a += (z[12*64] - z[2*64])   * 459;
    a += (unsafe { g(z, 12 * 64) } - unsafe { g(z, 2 * 64) }) * 459f32;
    // a += (z[3*64]  + z[11*64])  * 2037;
    a += (unsafe { g(z, 3 * 64) } + unsafe { g(z, 11 * 64) }) * 2037f32;
    // a += (z[10*64] - z[4*64])   * 5153;
    a += (unsafe { g(z, 10 * 64) } - unsafe { g(z, 4 * 64) }) * 5153f32;
    // a += (z[5*64]  + z[9*64])   * 6574;
    a += (unsafe { g(z, 5 * 64) } + unsafe { g(z, 9 * 64) }) * 6574f32;
    // a += (z[8*64]  - z[6*64])   * 37489;
    a += (unsafe { g(z, 8 * 64) } - unsafe { g(z, 6 * 64) }) * 37489f32;
    // a += z[7*64] * 75038;
    a += unsafe { g(z, 7 * 64) } * 75038f32;

    // pcm[0] = mp3d_scale_pcm(a);
    unsafe {
        *pcm.offset(0) = mp3d_scale_pcm(a);
    }

    // z += 2;
    let z = unsafe { z.offset(2) };

    // a  = z[14*64] * 104;
    a = unsafe { g(z, 14 * 64) } * 104f32;
    // a += z[12*64] * 1567;
    a += unsafe { g(z, 12 * 64) } * 1567f32;
    // a += z[10*64] * 9727;
    a += unsafe { g(z, 10 * 64) } * 9727f32;
    // a += z[8*64]  * 64019;
    a += unsafe { g(z, 8 * 64) } * 64019f32;
    // a += z[6*64]  * -9975;
    a += unsafe { g(z, 6 * 64) } * -9975f32;
    // a += z[4*64]  * -45;
    a += unsafe { g(z, 4 * 64) } * -45f32;
    // a += z[2*64]  * 146;
    a += unsafe { g(z, 2 * 64) } * 146f32;
    // a += z[0*64]  * -5;
    a += unsafe { g(z, 0 * 64) } * -5f32;

    // pcm[16 * nch] = mp3d_scale_pcm(a);
    unsafe {
        *pcm.offset(16isize * nch as isize) = mp3d_scale_pcm(a);
    }
}
