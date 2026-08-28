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
/// Notes on faithfulness (all confirmed against the GCC output for
/// `c_src/src/lib.c`, see `SYMBOLS.md`):
/// * The literals `32766.5` / `-32767.5` are `double` in C, so `sample` is
///   promoted to `double` for the comparisons. Both constants are exactly
///   representable in `f32` too (GCC in fact folds both comparisons to `comiss`
///   against `f32` constants), so the outcome is identical; the `as f64` here
///   mirrors the abstract C promotion.
/// * Both comparisons are false for NaN (`comiss` sets CF when unordered, and
///   `jb` is taken), so a NaN `sample` reaches the conversion below. C calls
///   that undefined; the emitted `cvttss2si` yields `0x80000000`, whose low 16
///   bits are `0`. Rust's saturating `as i32` maps NaN to `0`, which narrows to
///   the same `0`.
/// * `sample + .5f` is a single-precision addition (`FLT_EVAL_METHOD == 0`),
///   then converted to `int16_t` by truncation toward zero. The two guards bound
///   the sum inside `(-32767.0, 32767.0)`, so the narrowing always fits; the
///   intermediate `as i32` mirrors GCC's `cvttss2si %xmm0,%eax; mov %ax,...`.
/// * `s -= (s < 0);` subtracts exactly 1 when `s` is negative (C `bool` -> `int`)
///   and is evaluated in `int` before being truncated back to 16 bits, i.e. it
///   wraps rather than trapping.
#[inline]
fn mp3d_scale_pcm(sample: f32) -> i16 {
    if sample as f64 >= 32766.5 {
        return 32767i16;
    }
    if sample as f64 <= -32767.5 {
        return -32768i16;
    }
    let s: i16 = (sample + 0.5f32) as i32 as i16;
    // The guards make `s == i16::MIN` unreachable, but use wrapping arithmetic
    // so a debug-mode overflow panic can never diverge from C.
    s.wrapping_sub((s < 0) as i16)
}

/// Translation of the C `void synth_pair(mp3d_sample_t *pcm, int nch, const float *z)`.
///
/// The exported linker symbol is `synth_pair` (the public header declares no
/// namespace-renaming macros).
///
/// # Safety
///
/// Mirrors the C contract verbatim: `pcm` must be writable at indices `0` and
/// `16 * nch`, and `z` must be readable at `z[k * 64]` for `k in 0..=14` and at
/// `z[2 + k * 64]` for even `k in 0..=14` (i.e. `z[0 ..= 898]`, 899 floats).
/// No validation is performed, exactly as in the C.
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
    //
    // `16 * nch` is an `int` multiplication in C, and only the 32-bit result is
    // sign-extended to a pointer offset (GCC emits `shl $0x4,%eax; cltq`).
    // Reproduce the wrap-around explicitly so the index matches for every `int`
    // value of `nch`.
    let idx = 16i32.wrapping_mul(nch) as isize;
    unsafe {
        *pcm.wrapping_offset(idx) = mp3d_scale_pcm(a);
    }
}
