//! Rust translation of c_src/src/lib.c
//!
//! Public ABI (from `nm -D` on the C shared library):
//!   - gaussian_kernel
//!
//! The translation is a literal one: the same arithmetic in the same order,
//! the same libm `expf`, and the same (out of bounds for even `size`) memory
//! writes performed by the original loop `for (r = -hsize; r <= hsize; r++)`.

use core::ffi::c_int;

unsafe extern "C" {
    /// Use the platform libm `expf` so results match the C build bit for bit.
    fn expf(x: f32) -> f32;
}

/// void gaussian_kernel(float *dest, int size, float radius);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gaussian_kernel(dest: *mut f32, size: c_int, radius: f32) {
    let sigma: f32 = 1.6f32;
    let tetha: f32 = 2.25f32;
    // int hsize = size / 2;  (C integer division truncates toward zero)
    let hsize: c_int = size / 2;

    let s2: f32 = 1.0f32 / unsafe { expf(sigma * sigma * tetha) };
    let rs: f32 = sigma / radius;

    let mut k: *mut f32 = dest;
    let mut sum: f32 = 0.0f32;

    let mut r: c_int = -hsize;
    while r <= hsize {
        let x: f32 = (r as f32) * rs;
        let mut v: f32 = (1.0f32 / unsafe { expf(x * x) }) - s2;
        // v = ((v) > (0)) ? (v) : (0);  -- NaN compares false, so yields 0.0
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
