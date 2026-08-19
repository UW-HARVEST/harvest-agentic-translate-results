//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (from `nm -D` on the C shared object):
//!   * `contrast_ratio`
//!
//! The translation mirrors the C floating-point evaluation exactly:
//!  - the per-channel `u8 -> float` conversion and `/ 255.f` are done in `f32`,
//!  - the sRGB linearization (`> 0.04045`, `pow(...)`, `/ 12.92`) happens in
//!    `f64` (C usual arithmetic conversions promote the `float` operand to
//!    `double` against the `double` literals), and only the final ternary
//!    result is truncated back to `f32`,
//!  - the luminance dot product and the final ratio are `f32` operations
//!    evaluated left-to-right without contraction.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

/// `typedef struct cb_rgb_255 { unsigned char R, G, B; } cb_rgb_255;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct cb_rgb_255 {
    pub R: core::ffi::c_uchar,
    pub G: core::ffi::c_uchar,
    pub B: core::ffi::c_uchar,
}

/// Linearize one sRGB channel, matching the C expression
/// `(float)(C > 0.04045 ? pow((C + 0.055) / 1.055, 2.4) : C / 12.92)`.
#[inline]
fn cb_linearize(c: f32) -> f32 {
    let c = c as f64;
    let v = if c > 0.04045 {
        ((c + 0.055) / 1.055).powf(2.4)
    } else {
        c / 12.92
    };
    v as f32
}

/// `static float cbLuminance(float R, float G, float B)`
fn cb_luminance(r: f32, g: f32, b: f32) -> f32 {
    let r = cb_linearize(r);
    let g = cb_linearize(g);
    let b = cb_linearize(b);
    // Result = 0.2126f * R + 0.7152f * G + 0.0722f * B;
    let result = 0.2126f32 * r + 0.7152f32 * g + 0.0722f32 * b;
    result
}

/// `static float cbContrastRatio(float RA, float GA, float BA,
///                              float RB, float GB, float BB)`
fn cb_contrast_ratio(ra: f32, ga: f32, ba: f32, rb: f32, gb: f32, bb: f32) -> f32 {
    let lum_a = cb_luminance(ra, ga, ba);
    let lum_b = cb_luminance(rb, gb, bb);

    let mut high = lum_a;
    let mut low = lum_b;
    if high < low {
        high = lum_b;
        low = lum_a;
    }

    // No division-by-zero guard in the C source: reproduce inf/NaN as-is.
    let ratio = high / low;
    ratio
}

/// `float contrast_ratio(cb_rgb_255 A, cb_rgb_255 B)`
#[unsafe(no_mangle)]
pub extern "C" fn contrast_ratio(A: cb_rgb_255, B: cb_rgb_255) -> f32 {
    cb_contrast_ratio(
        f32::from(A.R) / 255.0f32,
        f32::from(A.G) / 255.0f32,
        f32::from(A.B) / 255.0f32,
        f32::from(B.R) / 255.0f32,
        f32::from(B.G) / 255.0f32,
        f32::from(B.B) / 255.0f32,
    )
}
