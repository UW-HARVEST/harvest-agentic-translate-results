//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `normalize`  -- `void normalize(float *dest, const float *src, int size);`
//!
//! The translation reproduces the original semantics bit-for-bit:
//!   * the sum of squares is accumulated sequentially in `f32` (no widening,
//!     no reassociation), matching the C `float sum` accumulator;
//!   * the reciprocal square root uses IEEE-754 correctly rounded `sqrtf`
//!     followed by a single `f32` division, exactly like `1.0f / sqrtf(sum)`;
//!   * a non-positive (or NaN) sum falls through to the `memset` branch, which
//!     is only taken when `dest != src` (pointer comparison);
//!   * the `memset` length reproduces C's `size * sizeof(float)`, where the
//!     `int` operand is first converted to `size_t` (sign extension) and the
//!     multiplication wraps modulo 2^64 -- including for negative sizes.

#![allow(clippy::missing_safety_doc)]

use core::mem::size_of;
use std::ffi::c_int;

/// ```c
/// void normalize(float *dest, const float *src, int size) {
///     float sum = 0.0f;
///     int i;
///     for (i = 0; i < size; i++)
///         sum += src[i] * src[i];
///     if (sum > 0.0f) {
///         sum = 1.0f / sqrtf(sum);
///         for (i = 0; i < size; i++)
///             dest[i] = src[i] * sum;
///     } else if (dest != src) {
///         memset(dest, 0, size * sizeof(float));
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    let mut sum: f32 = 0.0f32;

    // for (i = 0; i < size; i++) sum += src[i] * src[i];
    let mut i: c_int = 0;
    while i < size {
        let v: f32 = unsafe { *src.offset(i as isize) };
        sum += v * v;
        i += 1;
    }

    if sum > 0.0f32 {
        // sum = 1.0f / sqrtf(sum);
        sum = 1.0f32 / sum.sqrt();

        // for (i = 0; i < size; i++) dest[i] = src[i] * sum;
        i = 0;
        while i < size {
            let v: f32 = unsafe { *src.offset(i as isize) };
            unsafe { *dest.offset(i as isize) = v * sum };
            i += 1;
        }
    } else if dest as *const f32 != src {
        // memset(dest, 0, size * sizeof(float));
        //
        // C converts `size` (int) to size_t before multiplying, so negative
        // sizes sign-extend and the product wraps -- mirror that here.
        let nbytes: usize = (size as usize).wrapping_mul(size_of::<f32>());
        unsafe { core::ptr::write_bytes(dest as *mut u8, 0u8, nbytes) };
    }
}
