//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) that
//! exports exactly one public symbol: `tritanopia`.  The remaining functions
//! (`cbRemoveGammaRGB`, `cbNorm`, `cbDenorm`, `cbApplyGammaRGB`, `Tritanopia`)
//! are `static` in C and therefore have internal linkage; they are translated
//! as private Rust functions.
//!
//! Great care is taken to reproduce the exact floating point semantics of the
//! C code:
//!   * `float` expressions stay in `f32`.
//!   * Expressions that mix a `float` with a *double* literal (e.g. `0.04045`,
//!     `12.92`, `0.055`, `1.055`) are promoted to `f64` in C, so they are
//!     computed in `f64` here and only then narrowed back to `f32`.
//!   * `pow()` is `f64::powf`, which lowers to a call to the very same libm
//!     `pow` used by the C build.
//!   * The order of the additions/multiplications in `Tritanopia` is preserved
//!     literally (C evaluates `a + b + c` as `(a + b) + c`).

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// Public types (from c_src/include/lib.h)
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct cb_rgb_255 {
///     unsigned char R;
///     unsigned char G;
///     unsigned char B;
/// } cb_rgb_255;
/// ```
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct cb_rgb_255 {
    pub R: ::std::os::raw::c_uchar,
    pub G: ::std::os::raw::c_uchar,
    pub B: ::std::os::raw::c_uchar,
}

// ---------------------------------------------------------------------------
// Private types (from c_src/src/lib.c)
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct cb_rgb {
///     float R;
///     float G;
///     float B;
/// } cb_rgb;
/// ```
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct cb_rgb {
    R: f32,
    G: f32,
    B: f32,
}

// ---------------------------------------------------------------------------
// Helpers reproducing C conversion semantics
// ---------------------------------------------------------------------------

/// Reproduces the C cast `(unsigned char)some_float`.
///
/// On x86-64 (and every other target the C library is built for in practice)
/// the compiler emits a float -> int32 truncating conversion (`cvttss2si`)
/// followed by a truncation of the low 8 bits.  Values outside the `int`
/// range produce the "integer indefinite" value `0x80000000` (low byte `0`).
/// Rust's `as` casts saturate instead of wrapping, so the conversion is
/// spelled out explicitly to keep the exact same (technically
/// implementation-defined) behaviour as the C code.
#[inline]
fn f32_to_uchar(value: f32) -> ::std::os::raw::c_uchar {
    // `cvttss2si` returns 0x80000000 for NaN and for anything that does not
    // fit into a 32-bit signed integer after truncation towards zero.
    let truncated = value.trunc();
    let as_i32 = if truncated.is_nan()
        || truncated < -2147483648.0f32
        || truncated > 2147483647.0f32
    {
        -2147483648i32
    } else {
        truncated as i32
    };
    (as_i32 as u32 & 0xff) as ::std::os::raw::c_uchar
}

// ---------------------------------------------------------------------------
// static cb_rgb cbRemoveGammaRGB(cb_rgb RGB)
// ---------------------------------------------------------------------------

/// ```c
/// static cb_rgb cbRemoveGammaRGB(cb_rgb RGB) {
///     cb_rgb Result = {
///         ((float)(RGB.R > 0.04045 ? pow((RGB.R + 0.055) / 1.055, 2.4)
///                                  : RGB.R / 12.92)),
///         ...
/// ```
///
/// Note that every sub-expression here is evaluated in `double`: the `float`
/// member is promoted because it is combined with `double` literals.
#[inline]
fn cb_remove_gamma_channel(channel: f32) -> f32 {
    let c = channel as f64;
    let value = if c > 0.04045 {
        ((c + 0.055) / 1.055).powf(2.4)
    } else {
        c / 12.92
    };
    value as f32
}

fn cbRemoveGammaRGB(RGB: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: cb_remove_gamma_channel(RGB.R),
        G: cb_remove_gamma_channel(RGB.G),
        B: cb_remove_gamma_channel(RGB.B),
    }
}

// ---------------------------------------------------------------------------
// static cb_rgb cbNorm(cb_rgb_255 RGB)
// ---------------------------------------------------------------------------

/// ```c
/// static cb_rgb cbNorm(cb_rgb_255 RGB) {
///     cb_rgb Result = {((float)(RGB.R) / 255.f), ((float)(RGB.G) / 255.f),
///                      ((float)(RGB.B) / 255.f)};
///     return Result;
/// }
/// ```
fn cbNorm(RGB: cb_rgb_255) -> cb_rgb {
    cb_rgb {
        R: (RGB.R as f32) / 255.0f32,
        G: (RGB.G as f32) / 255.0f32,
        B: (RGB.B as f32) / 255.0f32,
    }
}

// ---------------------------------------------------------------------------
// static cb_rgb_255 cbDenorm(cb_rgb RGB)
// ---------------------------------------------------------------------------

/// ```c
/// static cb_rgb_255 cbDenorm(cb_rgb RGB) {
///     cb_rgb_255 Result = {((unsigned char)((RGB.R) * 255.f + 0.5f)),
///                          ((unsigned char)((RGB.G) * 255.f + 0.5f)),
///                          ((unsigned char)((RGB.B) * 255.f + 0.5f))};
///     return Result;
/// }
/// ```
///
/// All of the arithmetic is done in `float` because the literals are `float`
/// literals (`255.f`, `0.5f`).
fn cbDenorm(RGB: cb_rgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: f32_to_uchar(RGB.R * 255.0f32 + 0.5f32),
        G: f32_to_uchar(RGB.G * 255.0f32 + 0.5f32),
        B: f32_to_uchar(RGB.B * 255.0f32 + 0.5f32),
    }
}

// ---------------------------------------------------------------------------
// static cb_rgb cbApplyGammaRGB(cb_rgb RGB)
// ---------------------------------------------------------------------------

/// ```c
/// static cb_rgb cbApplyGammaRGB(cb_rgb RGB) {
///     cb_rgb Result = {((float)(RGB.R > 0.00313080495356037151702786377709
///                                   ? 1.055 * pow((RGB.R), 0.4166666666) - 0.055
///                                   : RGB.R * 12.92)),
///                      ...
/// ```
///
/// Again the whole conditional expression is computed in `double` and only the
/// final result is narrowed to `float`.
#[inline]
fn cb_apply_gamma_channel(channel: f32) -> f32 {
    let c = channel as f64;
    let value = if c > 0.003_130_804_953_560_371_517_027_863_777_09 {
        1.055 * c.powf(0.4166666666) - 0.055
    } else {
        c * 12.92
    };
    value as f32
}

fn cbApplyGammaRGB(RGB: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: cb_apply_gamma_channel(RGB.R),
        G: cb_apply_gamma_channel(RGB.G),
        B: cb_apply_gamma_channel(RGB.B),
    }
}

// ---------------------------------------------------------------------------
// static void Tritanopia(float *Red, float *Green, float *Blue)
// ---------------------------------------------------------------------------

/// ```c
/// static void Tritanopia(float *Red, float *Green, float *Blue) {
///     float R = *Red, G = *Green, B = *Blue;
///     *Red = R + 0.12739886310880f * G - 0.12739886341072f * B;
///     *Green = -4.486E-11f * R + 0.87390929928361f * G + 0.12609070101523f * B;
///     *Blue = 3.1113E-10f * R + 0.87390929725848f * G + 0.12609070067115f * B;
/// }
/// ```
///
/// The operand grouping of the C source is preserved exactly:
/// `a + b - c` == `(a + b) - c` and `a + b + c` == `(a + b) + c`.
fn Tritanopia(Red: &mut f32, Green: &mut f32, Blue: &mut f32) {
    let R: f32 = *Red;
    let G: f32 = *Green;
    let B: f32 = *Blue;

    *Red = (R + 0.127_398_863_108_80_f32 * G) - 0.127_398_863_410_72_f32 * B;
    *Green = ((-4.486E-11_f32) * R + 0.873_909_299_283_61_f32 * G)
        + 0.126_090_701_015_23_f32 * B;
    *Blue = (3.1113E-10_f32 * R + 0.873_909_297_258_48_f32 * G)
        + 0.126_090_700_671_15_f32 * B;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// ```c
/// cb_rgb_255 tritanopia(cb_rgb_255 RGB) {
///     cb_rgb RGBNorm = cbRemoveGammaRGB(cbNorm(RGB));
///     Tritanopia(&RGBNorm.R, &RGBNorm.G, &RGBNorm.B);
///     cb_rgb_255 Result = cbDenorm(cbApplyGammaRGB(RGBNorm));
///     return Result;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(RGB: cb_rgb_255) -> cb_rgb_255 {
    let mut RGBNorm: cb_rgb = cbRemoveGammaRGB(cbNorm(RGB));
    // Split the struct into three independent bindings so that the
    // `&mut` borrows do not overlap, then write the values back.
    let mut r = RGBNorm.R;
    let mut g = RGBNorm.G;
    let mut b = RGBNorm.B;
    Tritanopia(&mut r, &mut g, &mut b);
    RGBNorm.R = r;
    RGBNorm.G = g;
    RGBNorm.B = b;

    cbDenorm(cbApplyGammaRGB(RGBNorm))
}
