//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) and a
//! single public header (`include/lib.h`). Its complete exported ABI is:
//!
//! ```text
//! float contrast_ratio(cb_rgb_255 A, cb_rgb_255 B);
//! ```
//!
//! `cbLuminance` and `cbContrastRatio` are `static` in the C source and are
//! therefore *not* part of the exported ABI; they are reproduced here as
//! private helpers.
//!
//! Every arithmetic step below mirrors the exact C evaluation order, the exact
//! C usual-arithmetic-conversions (`float` operands promoted to `double`
//! whenever they meet a `double` literal such as `0.04045`, `0.055`, `1.055`
//! or `12.92`), and the exact `float`-precision accumulation used for the
//! luminance dot product. This is required for byte-identical output.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::c_uchar;

extern "C" {
    /// Bind directly to the platform `pow` (`pow@GLIBC_2.29` in the C build)
    /// so the translated code produces bit-identical results to the C library
    /// rather than relying on any Rust-side reimplementation.
    fn pow(x: f64, y: f64) -> f64;
}

/// ```c
/// typedef struct cb_rgb_255 {
///     unsigned char R;
///     unsigned char G;
///     unsigned char B;
/// } cb_rgb_255;
/// ```
///
/// `#[repr(C)]` gives this size 3 / align 1, so it is classified INTEGER and
/// passed packed in a single general-purpose register, matching the C ABI.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct cb_rgb_255 {
    pub R: c_uchar,
    pub G: c_uchar,
    pub B: c_uchar,
}

/// Faithful translation of the C expression
///
/// ```c
/// (float)(X > 0.04045 ? pow((X + 0.055) / 1.055, 2.4) : X / 12.92)
/// ```
///
/// The comparison and both arms of the conditional are evaluated in `double`
/// (the `float` argument is promoted because the literals are `double`), the
/// conditional expression therefore has type `double`, and the result is then
/// converted back to `float` by the cast.
#[inline]
fn cb_srgb_to_linear(x: f32) -> f32 {
    let x = x as f64;
    let linear: f64 = if x > 0.04045 {
        unsafe { pow((x + 0.055) / 1.055, 2.4) }
    } else {
        x / 12.92
    };
    linear as f32
}

/// ```c
/// static float cbLuminance(float R, float G, float B) {
///     R = ((float)(R > 0.04045 ? pow((R + 0.055) / 1.055, 2.4) : R / 12.92));
///     G = ((float)(G > 0.04045 ? pow((G + 0.055) / 1.055, 2.4) : G / 12.92));
///     B = ((float)(B > 0.04045 ? pow((B + 0.055) / 1.055, 2.4) : B / 12.92));
///     float Result = 0.2126f * R + 0.7152f * G + 0.0722f * B;
///     return Result;
/// }
/// ```
fn cbLuminance(R: f32, G: f32, B: f32) -> f32 {
    let R = cb_srgb_to_linear(R);
    let G = cb_srgb_to_linear(G);
    let B = cb_srgb_to_linear(B);

    // Single-precision throughout, left-to-right association exactly as C
    // groups it: ((0.2126f * R) + (0.7152f * G)) + (0.0722f * B).
    let Result: f32 = 0.2126f32 * R + 0.7152f32 * G + 0.0722f32 * B;
    Result
}

/// ```c
/// static float cbContrastRatio(float RA, float GA, float BA,
///                              float RB, float GB, float BB) {
///     float LumA = cbLuminance(RA, GA, BA);
///     float LumB = cbLuminance(RB, GB, BB);
///     float High = LumA, Low = LumB;
///     if (High < Low) {
///         High = LumB, Low = LumA;
///     }
///     float Ratio = High / Low;
///     return Ratio;
/// }
/// ```
///
/// Reproduced verbatim, including the original's behaviour for the degenerate
/// cases: a zero `Low` yields an infinity (or NaN for 0/0) instead of being
/// guarded against, and the C code applies no WCAG `+0.05` offset. NaN inputs
/// make `High < Low` false, so no swap occurs -- identical to C.
fn cbContrastRatio(RA: f32, GA: f32, BA: f32, RB: f32, GB: f32, BB: f32) -> f32 {
    let LumA = cbLuminance(RA, GA, BA);
    let LumB = cbLuminance(RB, GB, BB);

    let mut High = LumA;
    let mut Low = LumB;
    if High < Low {
        High = LumB;
        Low = LumA;
    }

    let Ratio = High / Low;
    Ratio
}

/// ```c
/// float contrast_ratio(cb_rgb_255 A, cb_rgb_255 B) {
///     return cbContrastRatio(((float)(A.R) / 255.f), ((float)(A.G) / 255.f),
///                            ((float)(A.B) / 255.f), ((float)(B.R) / 255.f),
///                            ((float)(B.G) / 255.f), ((float)(B.B) / 255.f));
/// }
/// ```
///
/// Each channel is converted `unsigned char` -> `float` and divided by the
/// `float` literal `255.f`, i.e. the division happens in single precision.
#[unsafe(no_mangle)]
pub extern "C" fn contrast_ratio(A: cb_rgb_255, B: cb_rgb_255) -> f32 {
    cbContrastRatio(
        A.R as f32 / 255.0f32,
        A.G as f32 / 255.0f32,
        A.B as f32 / 255.0f32,
        B.R as f32 / 255.0f32,
        B.G as f32 / 255.0f32,
        B.B as f32 / 255.0f32,
    )
}
