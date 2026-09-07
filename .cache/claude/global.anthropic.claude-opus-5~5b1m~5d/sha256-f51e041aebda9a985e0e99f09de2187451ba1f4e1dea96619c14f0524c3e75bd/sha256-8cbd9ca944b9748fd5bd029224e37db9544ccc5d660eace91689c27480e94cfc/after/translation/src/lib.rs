//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (matches `nm -D` on the C shared library exactly):
//!   * `gaussian_kernel`
//!
//! Original C source (`c_src/src/lib.c`):
//!
//! ```c
//! void gaussian_kernel(float *dest, int size, float radius) {
//!     float *k;
//!     float rs, s2, sum;
//!     float sigma = 1.6f;
//!     float tetha = 2.25f;
//!     int r, hsize = size / 2;
//!     s2 = 1.0f / expf(sigma * sigma * tetha);
//!     rs = sigma / radius;
//!     k = dest;
//!     sum = 0.0f;
//!     for (r = -hsize; r <= hsize; r++) {
//!         float x = r * rs;
//!         float v = (1.0f / expf(x * x)) - s2;
//!         v = (((v) > (0)) ? (v) : (0));
//!         *k = v;
//!         sum += v;
//!         k++;
//!     }
//!     if (sum > 0.0f) {
//!         float isum = 1.0f / sum;
//!         for (r = 0; r < size; r++)
//!             dest[r] *= isum;
//!     }
//! }
//! ```
//!
//! Behavioural notes (bugs are preserved verbatim, not fixed):
//!   * `hsize = size / 2` uses C integer division (truncation toward zero), and
//!     the kernel loop runs over the inclusive range `[-hsize, hsize]`, i.e.
//!     `2 * hsize + 1` iterations. For an even `size` this writes `size + 1`
//!     floats into `dest` -- one element past what the caller's `size` implies.
//!     That out-of-bounds store is part of the observable C behaviour and is
//!     reproduced here.
//!   * For a negative `size`, `-hsize > hsize`, so the loop body never runs,
//!     `sum` stays `0.0` and nothing is written or normalised.
//!   * `radius == 0.0` yields `rs == inf`; `x * x` is then `inf` (or `NaN` when
//!     `r == 0`, since `0 * inf == NaN`), matching the C exactly.
//!   * The clamp `v > 0 ? v : 0` compares against `0` promoted to `0.0f`, so a
//!     `NaN` `v` fails the comparison and becomes `+0.0f`.

#![allow(non_snake_case)]

use std::ffi::c_int;

extern "C" {
    /// The very same `expf` from the platform libm that the C build links
    /// against (`target_link_libraries(... m)`), so the results are
    /// bit-for-bit identical instead of merely close.
    fn expf(x: f32) -> f32;
}

/// Fill `dest` with a normalised, pedestal-subtracted Gaussian kernel.
///
/// # Safety
///
/// `dest` must be valid for writes of `2 * (size / 2) + 1` `f32` values (note
/// that this is `size + 1` when `size` is even -- see the module docs) and must
/// be valid for reads and writes of `size` `f32` values. Exactly the contract
/// the C function imposes on its callers.
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
