//! Rust translation of `c_src/src/lib.c`.
//!
//! Behaviour is a faithful, byte-for-byte reproduction of the original C,
//! including the bug in the third hue branch (the C source tests
//! `h < 120.0f && h < 180.0f` where `h >= 120.0f && h < 180.0f` was clearly
//! intended). That branch is therefore only reachable for `h < 0.0f`, which is
//! preserved here deliberately.

use std::ffi::c_float;

/// Pure computation kernel: HSL triple in, RGB triple out.
///
/// Mirrors the C statement-for-statement so float rounding matches exactly.
fn hsl_to_rgb_impl(src: [c_float; 3]) -> [c_float; 3] {
    let h = src[0];
    let s = src[1];
    let l = src[2];

    if s == 0.0 {
        return [l, l, l];
    }

    // c = (1.0f - fabsf(2.0f * l - 1.0f)) * s;
    let c = (1.0f32 - (2.0f32 * l - 1.0f32).abs()) * s;
    // m = 1.0f * (l - 0.5f * c);
    let m = 1.0f32 * (l - 0.5f32 * c);
    // x = c * (1.0f - fabsf(fmodf(h / 60.0f, 2) - 1.0f));
    // Rust's `%` on f32 is truncated remainder, i.e. identical to fmodf.
    let x = c * (1.0f32 - ((h / 60.0f32) % 2.0f32 - 1.0f32).abs());

    if h >= 0.0f32 && h < 60.0f32 {
        [c + m, x + m, m]
    } else if h >= 60.0f32 && h < 120.0f32 {
        [x + m, c + m, m]
    } else if h < 120.0f32 && h < 180.0f32 {
        // Faithful reproduction of the original C condition (see module docs).
        [m, c + m, x + m]
    } else if h >= 180.0f32 && h < 240.0f32 {
        [m, x + m, c + m]
    } else if h >= 240.0f32 && h < 300.0f32 {
        [x + m, m, c + m]
    } else if h >= 300.0f32 && h < 360.0f32 {
        [c + m, m, x + m]
    } else {
        [m, m, m]
    }
}

/// C ABI entry point: `void hsl_to_rgb(float *dest, const float *src)`.
///
/// # Safety
///
/// `src` must point to at least 3 readable `float`s and `dest` to at least 3
/// writable `float`s. As in the C original, `dest` and `src` may overlap: the
/// source values are fully consumed before anything is written back.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsl_to_rgb(dest: *mut c_float, src: *const c_float) {
    // Read the inputs up front (as the C does into locals) so that overlapping
    // `dest`/`src` buffers behave identically and no aliasing slices coexist.
    let input = [
        unsafe { *src.add(0) },
        unsafe { *src.add(1) },
        unsafe { *src.add(2) },
    ];

    let out = hsl_to_rgb_impl(input);

    let dest = unsafe { std::slice::from_raw_parts_mut(dest, 3) };
    dest.copy_from_slice(&out);
}
