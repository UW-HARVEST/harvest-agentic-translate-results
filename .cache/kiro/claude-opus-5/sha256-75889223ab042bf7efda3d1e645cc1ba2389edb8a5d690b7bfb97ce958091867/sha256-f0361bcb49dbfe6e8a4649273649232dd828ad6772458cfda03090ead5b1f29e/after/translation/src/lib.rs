//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (from `nm -D` on the C shared object):
//!   * `normalize`
//!
//! The header declares no namespace-renaming macros, so the linker symbol
//! matches the source-level name exactly.

use std::ffi::c_int;

/// Translation of:
///
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
///
/// Behaviour notes preserved verbatim from the C:
///   * The accumulation is performed in `f32` (single precision) in ascending
///     index order, so rounding matches the C compiled without fast-math.
///   * The reciprocal square root is computed as `1.0f / sqrtf(sum)` — a
///     division of the correctly-rounded square root, *not* an approximate
///     `rsqrt`.
///   * `sum > 0.0f` is false for `sum == 0.0` and for `NaN`, in which case the
///     `else if` branch is taken (zero-fill when `dest != src`).
///   * The `memset` length is `size * sizeof(float)` where `size` is an `int`
///     converted to `size_t` by the usual arithmetic conversions. A negative
///     `size` therefore wraps to a huge length, exactly as in C.
///   * When `sum <= 0.0f` and `dest == src`, nothing is written at all.
///
/// # Safety
///
/// Same contract as the C function: `src` must be readable for `size`
/// elements, and `dest` writable for `size` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    let mut sum: f32 = 0.0f32;

    let mut i: c_int = 0;
    while i < size {
        let v = unsafe { *src.offset(i as isize) };
        sum += v * v;
        i += 1;
    }

    if sum > 0.0f32 {
        sum = 1.0f32 / sum.sqrt();
        i = 0;
        while i < size {
            let v = unsafe { *src.offset(i as isize) };
            unsafe { *dest.offset(i as isize) = v * sum };
            i += 1;
        }
    } else if dest as *const f32 != src {
        // Reproduce C's `size * sizeof(float)` size_t arithmetic, including
        // the wraparound for negative `size`.
        let len = (size as i64 as u64).wrapping_mul(std::mem::size_of::<f32>() as u64) as usize;
        unsafe { std::ptr::write_bytes(dest as *mut u8, 0u8, len) };
    }
}
