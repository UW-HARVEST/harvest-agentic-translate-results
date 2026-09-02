//! Rust translation of the C colour-blindness simulation library in `c_src/`.
//!
//! Public ABI surface (verified against `nm -D` on the C shared object):
//!   * `tritanopia`
//!
//! The translation is deliberately literal: every intermediate rounding step,
//! promotion between `float` and `double`, and the exact left-to-right
//! evaluation order of the original C expressions is preserved so that the
//! resulting bytes are identical for every possible input.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
// The float literals below are copied VERBATIM from the C source. Several carry
// more decimal digits than `f32`/`f64` can represent; that is deliberate — the C
// compiler rounds the same decimal text to the same nearest representable value,
// so keeping the digits identical is what guarantees identical bytes. Shortening
// them (as clippy suggests) would be a silent behaviour change.
#![allow(clippy::excessive_precision)]
// The explicit two-sided comparison in `c_float_to_uchar` is clearer than
// `Range::contains` about the fact that NaN must fall through to the `else` arm.
#![allow(clippy::manual_range_contains)]

use std::ffi::c_uchar;

unsafe extern "C" {
    /// The very same `pow` from libm that the C translation unit calls through
    /// the PLT. Using it directly (instead of `f64::powf`) removes any doubt
    /// about the two builds resolving to different implementations.
    fn pow(x: f64, y: f64) -> f64;
}

/// `typedef struct cb_rgb_255 { unsigned char R, G, B; } cb_rgb_255;`
///
/// Three bytes, alignment 1. Under the x86-64 SysV ABI this is a single
/// INTEGER-class eightbyte, so it travels in one register (`R` in the low
/// byte). `repr(C)` gives Rust the identical layout and passing convention.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct cb_rgb_255 {
    pub R: c_uchar,
    pub G: c_uchar,
    pub B: c_uchar,
}

/// `typedef struct cb_rgb { float R, G, B; } cb_rgb;` — file-local in the C.
#[derive(Clone, Copy)]
struct cb_rgb {
    R: f32,
    G: f32,
    B: f32,
}

/// Reproduces C's `(unsigned char)some_float` on x86-64.
///
/// The C standard leaves an out-of-range float-to-integer conversion
/// undefined, and the original code genuinely goes out of range: the
/// tritanopia matrix can push a channel above 1.0 or below 0.0, so
/// `value * 255.f + 0.5f` legitimately lands outside `0..=255`.
///
/// GCC emits `cvttss2si` into a 32-bit register followed by a read of the low
/// byte (confirmed by disassembling the reference `.so`), i.e. truncation
/// toward zero into `i32` and then a *wrapping* narrowing to 8 bits. Rust's
/// `as u8` would instead saturate, which would disagree with C, so the two
/// steps are spelled out explicitly.
#[inline]
fn c_float_to_uchar(value: f32) -> c_uchar {
    // `cvttss2si` yields the "integer indefinite" value 0x8000_0000 for NaN
    // and for anything outside the signed 32-bit range; its low byte is 0.
    if value >= -2147483648.0f32 && value < 2147483648.0f32 {
        (value as i32) as c_uchar
    } else {
        0
    }
}

/// ```c
/// static cb_rgb cbRemoveGammaRGB(cb_rgb RGB);
/// ```
///
/// Each channel is promoted to `double` for the comparison against the
/// `0.04045` threshold and for the whole computation, then narrowed back to
/// `float` by the explicit cast in the initialiser.
fn cbRemoveGammaRGB(RGB: cb_rgb) -> cb_rgb {
    #[inline]
    fn channel(c: f32) -> f32 {
        let c = c as f64; // C's usual arithmetic conversions
        if c > 0.04045 {
            unsafe { pow((c + 0.055) / 1.055, 2.4) as f32 }
        } else {
            (c / 12.92) as f32
        }
    }

    cb_rgb {
        R: channel(RGB.R),
        G: channel(RGB.G),
        B: channel(RGB.B),
    }
}

/// ```c
/// static cb_rgb cbNorm(cb_rgb_255 RGB);
/// ```
///
/// Single-precision throughout: `(float)(RGB.R) / 255.f`.
fn cbNorm(RGB: cb_rgb_255) -> cb_rgb {
    cb_rgb {
        R: (RGB.R as f32) / 255.0f32,
        G: (RGB.G as f32) / 255.0f32,
        B: (RGB.B as f32) / 255.0f32,
    }
}

/// ```c
/// static cb_rgb_255 cbDenorm(cb_rgb RGB);
/// ```
///
/// Single-precision scale and bias, then the C truncating cast to
/// `unsigned char` (see [`c_float_to_uchar`] — no clamping happens here, and
/// none is added).
fn cbDenorm(RGB: cb_rgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: c_float_to_uchar(RGB.R * 255.0f32 + 0.5f32),
        G: c_float_to_uchar(RGB.G * 255.0f32 + 0.5f32),
        B: c_float_to_uchar(RGB.B * 255.0f32 + 0.5f32),
    }
}

/// ```c
/// static cb_rgb cbApplyGammaRGB(cb_rgb RGB);
/// ```
///
/// The inverse of [`cbRemoveGammaRGB`]. Note the threshold is the long
/// literal `0.00313080495356037151702786377709` and the exponent is the
/// truncated `0.4166666666` (not `1.0 / 2.4`); both are kept verbatim.
///
/// For a channel that is `<= threshold` the linear branch is taken, so `pow`
/// is never handed a negative base here — but where the C would produce a
/// negative or greater-than-one result, so does this.
fn cbApplyGammaRGB(RGB: cb_rgb) -> cb_rgb {
    #[inline]
    fn channel(c: f32) -> f32 {
        let c = c as f64; // C's usual arithmetic conversions
        if c > 0.003_130_804_953_560_371_517_027_863_777_09 {
            unsafe { (1.055 * pow(c, 0.4166666666) - 0.055) as f32 }
        } else {
            (c * 12.92) as f32
        }
    }

    cb_rgb {
        R: channel(RGB.R),
        G: channel(RGB.G),
        B: channel(RGB.B),
    }
}

/// ```c
/// static void Tritanopia(float *Red, float *Green, float *Blue);
/// ```
///
/// The C reads all three inputs into locals up front and then overwrites the
/// pointees, so the later rows see the *original* values even though `*Red`
/// has already been stored. Taking `&mut cb_rgb` and snapshotting the fields
/// reproduces that exactly.
///
/// Every coefficient stays `f32` and each row keeps C's left-to-right
/// association: `((a) + (b)) - (c)` for the red row and `((a) + (b)) + (c)`
/// for the other two.
fn Tritanopia(rgb: &mut cb_rgb) {
    let R = rgb.R;
    let G = rgb.G;
    let B = rgb.B;

    rgb.R = R + 0.127_398_863_108_80f32 * G - 0.127_398_863_410_72f32 * B;
    rgb.G = -4.486E-11f32 * R + 0.873_909_299_283_61f32 * G + 0.126_090_701_015_23f32 * B;
    rgb.B = 3.1113E-10f32 * R + 0.873_909_297_258_48f32 * G + 0.126_090_700_671_15f32 * B;
}

/// ```c
/// cb_rgb_255 tritanopia(cb_rgb_255 RGB);
/// ```
///
/// The library's sole exported symbol: normalise to 0..1, linearise, apply the
/// tritanopia simulation matrix, re-apply gamma, and quantise back to bytes.
#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(RGB: cb_rgb_255) -> cb_rgb_255 {
    let mut RGBNorm = cbRemoveGammaRGB(cbNorm(RGB));
    Tritanopia(&mut RGBNorm);
    cbDenorm(cbApplyGammaRGB(RGBNorm))
}
