//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `c_src/include/lib.h`):
//!     void normalize(float *dest, const float *src, int size);

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
/// Behaviour notes preserved verbatim from the C original:
/// * The accumulation is performed strictly in order with `f32` precision, so
///   the resulting rounding (and therefore the emitted bytes) matches the C
///   build, which is compiled without fast-math relaxations.
/// * `sum > 0.0f` is false for `0.0`, negative zero and NaN; in those cases the
///   `else if` branch is taken.
/// * The zero-fill branch is only taken when `dest != src` (pointer compare),
///   exactly as in C, and the byte count is computed as
///   `size * sizeof(float)` with C's implicit conversion of the (possibly
///   negative) `int` to `size_t`, i.e. sign extension followed by wrapping
///   multiplication.
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
        let mut i: c_int = 0;
        while i < size {
            let v = unsafe { *src.offset(i as isize) };
            unsafe { *dest.offset(i as isize) = v * sum };
            i += 1;
        }
    } else if dest as *const f32 != src {
        // memset(dest, 0, size * sizeof(float))
        let nbytes = (size as usize).wrapping_mul(core::mem::size_of::<f32>());
        unsafe { core::ptr::write_bytes(dest as *mut u8, 0u8, nbytes) };
    }
}
