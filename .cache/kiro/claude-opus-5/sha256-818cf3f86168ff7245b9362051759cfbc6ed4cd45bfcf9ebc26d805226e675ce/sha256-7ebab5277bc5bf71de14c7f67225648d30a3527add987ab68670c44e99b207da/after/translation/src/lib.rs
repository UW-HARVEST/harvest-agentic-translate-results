//! Rust translation of `c_src/src/lib.c` (WCAG-style contrast ratio).
//!
//! The C code is reproduced verbatim, including its quirks:
//!   * The linear-ization branches promote the `float` channel to `double`,
//!     do the work in `double`, then truncate the result back to `float`.
//!   * The luminance sum is evaluated purely in `float`, left to right.
//!   * The ratio is a bare `High / Low` with no `+ 0.05` offset and no guard
//!     against a zero denominator, so pure black inputs yield `inf`/`NaN`
//!     exactly as the original does.

// The C identifiers are kept verbatim so the FFI surface reads like the header.
#![allow(non_snake_case, non_camel_case_types)]

/// Mirrors `cb_rgb_255` from `include/lib.h`: three tightly packed bytes,
/// passed by value across the C ABI.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct cb_rgb_255 {
    pub R: core::ffi::c_uchar,
    pub G: core::ffi::c_uchar,
    pub B: core::ffi::c_uchar,
}

unsafe extern "C" {
    /// Use libm's `pow` directly so the double-precision results are
    /// bit-for-bit those of the C build.
    fn pow(x: f64, y: f64) -> f64;
}

/// `R = ((float)(R > 0.04045 ? pow((R + 0.055) / 1.055, 2.4) : R / 12.92));`
///
/// The comparison and both arithmetic branches happen in `double` because the
/// literals are `double`; only the final store narrows back to `float`.
#[inline]
fn linearize(channel: f32) -> f32 {
    let c = channel as f64;
    if c > 0.04045 {
        unsafe { pow((c + 0.055) / 1.055, 2.4) as f32 }
    } else {
        (c / 12.92) as f32
    }
}

/// Port of `static float cbLuminance(float, float, float)`.
fn cb_luminance(r: f32, g: f32, b: f32) -> f32 {
    let r = linearize(r);
    let g = linearize(g);
    let b = linearize(b);
    // Single-precision, left-to-right: ((0.2126*R) + (0.7152*G)) + (0.0722*B).
    0.2126f32 * r + 0.7152f32 * g + 0.0722f32 * b
}

/// Port of `static float cbContrastRatio(float, float, float, float, float, float)`.
fn cb_contrast_ratio(ra: f32, ga: f32, ba: f32, rb: f32, gb: f32, bb: f32) -> f32 {
    let lum_a = cb_luminance(ra, ga, ba);
    let lum_b = cb_luminance(rb, gb, bb);

    let mut high = lum_a;
    let mut low = lum_b;
    // A false (e.g. NaN) comparison leaves the initial assignment in place,
    // matching C.
    if high < low {
        high = lum_b;
        low = lum_a;
    }

    high / low
}

/// `float contrast_ratio(cb_rgb_255 A, cb_rgb_255 B);`
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
