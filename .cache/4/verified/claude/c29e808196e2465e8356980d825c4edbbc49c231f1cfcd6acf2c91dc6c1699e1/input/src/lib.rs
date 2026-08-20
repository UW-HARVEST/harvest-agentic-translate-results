//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `gaussian_kernel`
//!
//! The translation is intentionally literal: the original C code writes
//! `2 * (size / 2) + 1` elements into `dest` (which overruns the buffer by one
//! element for even `size`). That behaviour is reproduced exactly rather than
//! "fixed", so raw pointer writes are used instead of a slice.

use std::ffi::c_int;

extern "C" {
    /// Use libm's `expf` directly so results are bit-for-bit identical to the C
    /// build (which calls `expf` from `<math.h>`).
    fn expf(x: f32) -> f32;
}

/// void gaussian_kernel(float *dest, int size, float radius);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gaussian_kernel(dest: *mut f32, size: c_int, radius: f32) {
    let sigma: f32 = 1.6f32;
    let tetha: f32 = 2.25f32;
    let hsize: c_int = size / 2;

    let s2: f32 = 1.0f32 / expf(sigma * sigma * tetha);
    let rs: f32 = sigma / radius;

    let mut k: *mut f32 = dest;
    let mut sum: f32 = 0.0f32;

    let mut r: c_int = -hsize;
    while r <= hsize {
        let x: f32 = (r as f32) * rs;
        let mut v: f32 = (1.0f32 / expf(x * x)) - s2;
        // C: v = ((v) > (0)) ? (v) : (0);  -- integer 0 promotes to 0.0f
        v = if v > 0.0f32 { v } else { 0.0f32 };
        *k = v;
        sum += v;
        k = k.add(1);
        r += 1;
    }

    if sum > 0.0f32 {
        let isum: f32 = 1.0f32 / sum;
        let mut r: c_int = 0;
        while r < size {
            let p = dest.offset(r as isize);
            *p *= isum;
            r += 1;
        }
    }
}
