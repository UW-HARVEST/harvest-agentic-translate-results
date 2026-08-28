//! Rust translation of `c_src/src/lib.c`.
//!
//! The header (`c_src/include/lib.h`) declares:
//!
//! ```c
//! typedef int16_t mp3d_sample_t;
//! void synth_pair(mp3d_sample_t *pcm, int nch, const float *z);
//! ```
//!
//! There is no namespace/renaming macro, so the exported linker symbol is
//! literally `synth_pair`.

use std::ffi::c_int;

/// `mp3d_sample_t` from `lib.h`.
#[allow(non_camel_case_types)]
type mp3d_sample_t = i16;

/// Faithful port of the `static int16_t mp3d_scale_pcm(float sample)` helper.
///
/// The C comparisons are against `double` literals (`32766.5`, `-32767.5`),
/// so the `float` argument is promoted to `double` first; both constants are
/// exactly representable in either format, so the promotion is modelled by
/// comparing as `f64`.
///
/// `(int16_t)(sample + .5f)` is a C float-to-integer conversion, i.e.
/// truncation toward zero. The early returns guarantee the value is well
/// inside the `i16` range at that point, so Rust's saturating `as i16`
/// behaves identically to C's truncation here.
fn mp3d_scale_pcm(sample: f32) -> i16 {
    if sample as f64 >= 32766.5 {
        return 32767i32 as i16;
    }
    if sample as f64 <= -32767.5 {
        return -32768i32 as i16;
    }
    let mut s: i16 = (sample + 0.5f32) as i16;
    // C: `s -= (s < 0);` -- integer promotion, subtract 0 or 1, truncate back.
    s = s.wrapping_sub(if s < 0 { 1 } else { 0 });
    s
}

/// `void synth_pair(mp3d_sample_t *pcm, int nch, const float *z)`
///
/// # Safety
///
/// Same contract as the C original: `pcm` must be writable at offsets `0` and
/// `16 * nch`, and `z` must be readable at the sampled offsets.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn synth_pair(pcm: *mut mp3d_sample_t, nch: c_int, z: *const f32) {
    // Read helper mirroring the C subscript `z[i]`.
    let zi = |i: isize| -> f32 { unsafe { *z.offset(i) } };

    let mut a: f32;
    a = (zi(14 * 64) - zi(0)) * 29f32;
    a += (zi(1 * 64) + zi(13 * 64)) * 213f32;
    a += (zi(12 * 64) - zi(2 * 64)) * 459f32;
    a += (zi(3 * 64) + zi(11 * 64)) * 2037f32;
    a += (zi(10 * 64) - zi(4 * 64)) * 5153f32;
    a += (zi(5 * 64) + zi(9 * 64)) * 6574f32;
    a += (zi(8 * 64) - zi(6 * 64)) * 37489f32;
    a += zi(7 * 64) * 75038f32;
    unsafe {
        *pcm.offset(0) = mp3d_scale_pcm(a);
    }

    // C: `z += 2;` -- every subsequent subscript is shifted by two floats.
    let zi = |i: isize| -> f32 { unsafe { *z.offset(2 + i) } };

    a = zi(14 * 64) * 104f32;
    a += zi(12 * 64) * 1567f32;
    a += zi(10 * 64) * 9727f32;
    a += zi(8 * 64) * 64019f32;
    a += zi(6 * 64) * -9975f32;
    a += zi(4 * 64) * -45f32;
    a += zi(2 * 64) * 146f32;
    a += zi(0 * 64) * -5f32;
    unsafe {
        *pcm.offset(16 * nch as isize) = mp3d_scale_pcm(a);
    }
}
