//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `c_src/include/lib.h`):
//!
//! ```c
//! typedef int16_t mp3d_sample_t;
//! void synth_pair(mp3d_sample_t *pcm, int nch, const float *z);
//! ```
//!
//! There are no namespace-renaming preprocessor macros in the public header, so
//! the linker symbol is plain `synth_pair`, exactly as reported by `nm -D` on the
//! C shared library.

#![allow(non_camel_case_types)]

use std::ffi::c_int;

/// `typedef int16_t mp3d_sample_t;`
pub type mp3d_sample_t = i16;

/// Translation of the file-static C helper:
///
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
/// The comparisons in C are performed in `double` (the float operand is
/// converted to the type of the `double` literal), which is reproduced here by
/// widening `sample` before comparing. Both literals are exactly representable
/// in `f32` as well, so the widening cannot change the outcome, but it keeps the
/// translation literal.
///
/// The float -> integer conversion follows the C/x86-64 lowering: truncate
/// toward zero into a 32-bit integer, then narrow to 16 bits. `sample` is
/// bounded by the two range checks above, so no out-of-range conversion can
/// occur; a NaN input yields 0 in both the C code (`cvttss2si` -> `INT_MIN`,
/// then truncated to 16 bits) and here.
#[inline]
fn mp3d_scale_pcm(sample: f32) -> i16 {
    if (sample as f64) >= 32766.5 {
        return 32767i32 as i16;
    }
    if (sample as f64) <= -32767.5 {
        return -32768i32 as i16;
    }
    let s: i16 = ((sample + 0.5f32) as i32) as i16;
    // `s -= (s < 0);` -- C promotes to `int`, subtracts, then converts back to
    // `int16_t`; `wrapping_sub` reproduces that (two's-complement) narrowing.
    s.wrapping_sub((s < 0) as i16)
}

/// ```c
/// void synth_pair(mp3d_sample_t *pcm, int nch, const float *z);
/// ```
///
/// The accumulation order of the `float` arithmetic is preserved verbatim so the
/// rounding of every intermediate result -- and therefore the emitted PCM -- is
/// bit-identical to the C implementation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn synth_pair(pcm: *mut mp3d_sample_t, nch: c_int, z: *const f32) {
    unsafe {
        let mut z = z;
        let mut a: f32;

        a = (*z.add(14 * 64) - *z.add(0)) * 29f32;
        a += (*z.add(1 * 64) + *z.add(13 * 64)) * 213f32;
        a += (*z.add(12 * 64) - *z.add(2 * 64)) * 459f32;
        a += (*z.add(3 * 64) + *z.add(11 * 64)) * 2037f32;
        a += (*z.add(10 * 64) - *z.add(4 * 64)) * 5153f32;
        a += (*z.add(5 * 64) + *z.add(9 * 64)) * 6574f32;
        a += (*z.add(8 * 64) - *z.add(6 * 64)) * 37489f32;
        a += *z.add(7 * 64) * 75038f32;
        *pcm.add(0) = mp3d_scale_pcm(a);

        z = z.add(2);

        a = *z.add(14 * 64) * 104f32;
        a += *z.add(12 * 64) * 1567f32;
        a += *z.add(10 * 64) * 9727f32;
        a += *z.add(8 * 64) * 64019f32;
        a += *z.add(6 * 64) * -9975f32;
        a += *z.add(4 * 64) * -45f32;
        a += *z.add(2 * 64) * 146f32;
        a += *z.add(0 * 64) * -5f32;
        // `pcm[16 * nch]` -- the C index is computed in `int`.
        *pcm.offset((16i32.wrapping_mul(nch)) as isize) = mp3d_scale_pcm(a);
    }
}
