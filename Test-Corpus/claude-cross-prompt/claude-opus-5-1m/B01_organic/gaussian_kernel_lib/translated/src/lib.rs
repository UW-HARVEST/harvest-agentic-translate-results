//! Translation of c_src/src/lib.c — a single function library.
//!
//! The original C exposes only `gaussian_kernel`, with no `main` and no I/O.
//! We mirror its behavior here in safe Rust.

/// Fill `dest` with a Gaussian-style kernel of `size` samples spanning `[-hsize, hsize]`,
/// where `hsize = size / 2`. Mirrors the original C bit-for-bit using `f32` arithmetic.
pub fn gaussian_kernel(dest: &mut [f32], size: i32, radius: f32) {
    let sigma: f32 = 1.6_f32;
    let tetha: f32 = 2.25_f32;
    let hsize: i32 = size / 2;
    let s2: f32 = 1.0_f32 / (sigma * sigma * tetha).exp();
    let rs: f32 = sigma / radius;
    let mut sum: f32 = 0.0_f32;

    let mut idx: usize = 0;
    let mut r: i32 = -hsize;
    while r <= hsize {
        let x: f32 = (r as f32) * rs;
        let mut v: f32 = (1.0_f32 / (x * x).exp()) - s2;
        // C ternary: ((v) > 0) ? v : 0
        if !(v > 0.0_f32) {
            v = 0.0_f32;
        }
        dest[idx] = v;
        sum += v;
        idx += 1;
        r += 1;
    }

    if sum > 0.0_f32 {
        let isum: f32 = 1.0_f32 / sum;
        let n = size as usize;
        for i in 0..n {
            dest[i] *= isum;
        }
    }
}
