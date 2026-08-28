//! Rust translation of `c_src/src/lib.c`.
//!
//! Exposes `normalize`, which scales a float vector to unit length in-place or
//! into a separate destination buffer. Behaviour (including edge cases such as
//! a non-positive sum of squares, aliasing buffers and negative sizes) mirrors
//! the original C exactly.

use std::ffi::c_int;

/// Normalize `src` into `dest`.
///
/// Mirrors:
/// ```c
/// void normalize(float *dest, const float *src, int size);
/// ```
///
/// * Accumulates `sum = Σ src[i]²` in `f32`, in ascending index order.
/// * If `sum > 0`, writes `src[i] * (1 / sqrtf(sum))` into `dest[i]`.
/// * Otherwise, if `dest != src`, zeroes `size * sizeof(float)` bytes of `dest`.
/// * Otherwise leaves `dest` untouched.
///
/// # Safety
///
/// `dest` and `src` must be valid for `size` `f32` elements (writes and reads
/// respectively) whenever `size > 0`. As in C, the two ranges may alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    let mut sum: f32 = 0.0f32;

    let mut i: c_int = 0;
    while i < size {
        // SAFETY: caller guarantees `src` holds at least `size` elements.
        let v = unsafe { *src.offset(i as isize) };
        sum += v * v;
        i += 1;
    }

    if sum > 0.0f32 {
        sum = 1.0f32 / sum.sqrt();
        let mut i: c_int = 0;
        while i < size {
            // SAFETY: caller guarantees both buffers hold at least `size`
            // elements; overlapping ranges are handled element-wise just as
            // the C loop does.
            unsafe {
                let v = *src.offset(i as isize);
                *dest.offset(i as isize) = v * sum;
            }
            i += 1;
        }
    } else if dest as *const f32 != src {
        // `size * sizeof(float)` in C: the `int` is converted to `size_t`, so a
        // negative `size` sign-extends into a huge length. Reproduced verbatim
        // rather than guarded against.
        let bytes = (size as isize as usize).wrapping_mul(core::mem::size_of::<f32>());
        // SAFETY: same contract as the C `memset` call.
        unsafe {
            core::ptr::write_bytes(dest as *mut u8, 0u8, bytes);
        }
    }
}
