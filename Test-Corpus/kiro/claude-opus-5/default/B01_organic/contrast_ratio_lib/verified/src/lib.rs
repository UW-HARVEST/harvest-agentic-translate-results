//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (as reported by `nm -D` on the C shared object):
//!   * `contrast_ratio`
//!
//! The floating point evaluation order and precision of the original C is
//! reproduced exactly:
//!   * The linearization step promotes the `float` channel to `double`
//!     (C usual arithmetic conversions against the `double` literals
//!     `0.04045`, `0.055`, `1.055`, `2.4`, `12.92`), performs the work in
//!     double precision, then narrows back to `float`.
//!   * The luminance weighting is done entirely in single precision, left to
//!     right, matching `0.2126f * R + 0.7152f * G + 0.0722f * B`.
//!   * The high/low selection uses a strict `<` comparison, so NaN inputs take
//!     the no-swap path exactly as the C does.

#![allow(non_snake_case)]

/// Mirror of the C `cb_rgb_255` struct (three `unsigned char` channels).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct cb_rgb_255 {
    pub R: u8,
    pub G: u8,
    pub B: u8,
}

/// sRGB -> linear transfer function for one channel.
///
/// Translation of the ternary expression applied to each channel in
/// `cbLuminance`:
/// `(float)(C > 0.04045 ? pow((C + 0.055) / 1.055, 2.4) : C / 12.92)`
#[inline]
fn cb_linearize(channel: f32) -> f32 {
    // `channel` is promoted to `double` for the comparison and the arithmetic,
    // then the `double` result is truncated back to `float`.
    let c = channel as f64;
    let linear = if c > 0.04045 {
        ((c + 0.055) / 1.055).powf(2.4)
    } else {
        c / 12.92
    };
    linear as f32
}

/// Translation of the static C helper `cbLuminance`.
fn cbLuminance(R: f32, G: f32, B: f32) -> f32 {
    let R = cb_linearize(R);
    let G = cb_linearize(G);
    let B = cb_linearize(B);

    // Single precision, left-to-right accumulation, as in the C source.
    let Result = 0.2126f32 * R + 0.7152f32 * G + 0.0722f32 * B;
    Result
}

/// Translation of the static C helper `cbContrastRatio`.
fn cbContrastRatio(RA: f32, GA: f32, BA: f32, RB: f32, GB: f32, BB: f32) -> f32 {
    let LumA = cbLuminance(RA, GA, BA);
    let LumB = cbLuminance(RB, GB, BB);

    let mut High = LumA;
    let mut Low = LumB;
    if High < Low {
        High = LumB;
        Low = LumA;
    }

    // No division-by-zero guard in the C original; the IEEE-754 result
    // (+/-inf or NaN) is propagated verbatim.
    let Ratio = High / Low;
    Ratio
}

/// Public C entry point: WCAG-style contrast ratio between two 8-bit sRGB
/// colors.
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
