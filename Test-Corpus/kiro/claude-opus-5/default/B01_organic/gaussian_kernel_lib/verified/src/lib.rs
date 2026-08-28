//! Rust translation of `c_src/src/lib.c`.
//!
//! Behaviour is reproduced exactly, including the original's quirks:
//!  * the first loop always writes `2 * (size / 2) + 1` elements, which is one
//!    element past `size` when `size` is even (an out-of-bounds write in the C
//!    original — kept as-is on purpose),
//!  * the reciprocal-of-`expf` formulation (`1/expf(x*x)`) is preserved rather
//!    than being rewritten as `expf(-x*x)`, since the two differ in the last
//!    bits,
//!  * `float` (`f32`) precision is used for every intermediate value.

use std::ffi::c_int;

unsafe extern "C" {
    /// Single-precision exponential from the platform math library, so that the
    /// results match the C build bit-for-bit.
    fn expf(x: f32) -> f32;
}

/// `void gaussian_kernel(float *dest, int size, float radius);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gaussian_kernel(dest: *mut f32, size: c_int, radius: f32) {
    let sigma: f32 = 1.6f32;
    let tetha: f32 = 2.25f32;
    let hsize: c_int = size / 2;

    let s2: f32 = 1.0f32 / unsafe { expf(sigma * sigma * tetha) };
    let rs: f32 = sigma / radius;

    let mut k: *mut f32 = dest;
    let mut sum: f32 = 0.0f32;

    let mut r: c_int = -hsize;
    while r <= hsize {
        let x: f32 = (r as f32) * rs;
        let mut v: f32 = (1.0f32 / unsafe { expf(x * x) }) - s2;
        v = if v > 0.0f32 { v } else { 0.0f32 };
        unsafe {
            *k = v;
        }
        sum += v;
        k = unsafe { k.add(1) };
        r += 1;
    }

    if sum > 0.0f32 {
        let isum: f32 = 1.0f32 / sum;
        let mut r: c_int = 0;
        while r < size {
            unsafe {
                let p = dest.offset(r as isize);
                *p *= isum;
            }
            r += 1;
        }
    }
}
