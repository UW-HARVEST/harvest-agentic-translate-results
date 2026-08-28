//! Rust translation of `c_src/src/lib.c` (colour-blindness simulation:
//! tritanopia).
//!
//! The goal is byte-identical output with the original C for every possible
//! input. That drives a few deliberate choices:
//!
//! * The C code mixes `float` storage with `double` arithmetic (the literals
//!   `0.04045`, `1.055`, `12.92`, ... are all `double`, and `pow` is the
//!   `double` version). Every intermediate rounding step is reproduced here
//!   explicitly with `as f64` / `as f32` casts.
//! * `Tritanopia()` is done entirely in `f32`, matching the `f`-suffixed
//!   literals in the C source.
//! * `cbDenorm` casts a `float` straight to `unsigned char`. That is undefined
//!   behaviour in C for out-of-range values, and out-of-range values really do
//!   occur here (the tritanopia matrix pushes channels outside `[0, 1]`). We
//!   reproduce what the C compiler actually emits on x86-64: truncate towards
//!   zero into a 32-bit integer, then keep the low byte.
//! * `pow` is taken from libm rather than `f64::powf` so the exact same
//!   implementation is used as by the C build.

#![allow(non_snake_case, non_camel_case_types)]

use std::ffi::c_uchar;

unsafe extern "C" {
    fn pow(x: f64, y: f64) -> f64;
}

/// Mirrors `cb_rgb_255` from `include/lib.h`.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct cb_rgb_255 {
    pub R: c_uchar,
    pub G: c_uchar,
    pub B: c_uchar,
}

/// Mirrors the private `cb_rgb` struct from `src/lib.c`.
#[derive(Copy, Clone)]
struct CbRgb {
    r: f32,
    g: f32,
    b: f32,
}

/// `(float)(c > 0.04045 ? pow((c + 0.055) / 1.055, 2.4) : c / 12.92)`
#[inline]
fn remove_gamma_channel(c: f32) -> f32 {
    let c = c as f64;
    if c > 0.04045 {
        (unsafe { pow((c + 0.055) / 1.055, 2.4) }) as f32
    } else {
        (c / 12.92) as f32
    }
}

fn cb_remove_gamma_rgb(rgb: CbRgb) -> CbRgb {
    CbRgb {
        r: remove_gamma_channel(rgb.r),
        g: remove_gamma_channel(rgb.g),
        b: remove_gamma_channel(rgb.b),
    }
}

/// `(float)(RGB.x) / 255.f`
fn cb_norm(rgb: cb_rgb_255) -> CbRgb {
    CbRgb {
        r: f32::from(rgb.R) / 255.0f32,
        g: f32::from(rgb.G) / 255.0f32,
        b: f32::from(rgb.B) / 255.0f32,
    }
}

/// `(unsigned char)(c * 255.f + 0.5f)`
///
/// The C cast is only defined for values in `[0, UCHAR_MAX + 1)`; for anything
/// else x86-64 compilers emit `cvttss2si` into a 32-bit register and then read
/// the low byte, which is what we replicate. `as i32` on a NaN yields `0` in
/// Rust, whose low byte matches the `0x80000000` produced by `cvttss2si`.
#[inline]
fn denorm_channel(c: f32) -> c_uchar {
    let scaled = c * 255.0f32 + 0.5f32;
    (scaled.trunc() as i32) as c_uchar
}

fn cb_denorm(rgb: CbRgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: denorm_channel(rgb.r),
        G: denorm_channel(rgb.g),
        B: denorm_channel(rgb.b),
    }
}

/// `(float)(c > 0.0031308... ? 1.055 * pow(c, 0.4166666666) - 0.055 : c * 12.92)`
#[inline]
fn apply_gamma_channel(c: f32) -> f32 {
    let c = c as f64;
    if c > 0.00313080495356037151702786377709 {
        (1.055 * unsafe { pow(c, 0.4166666666) } - 0.055) as f32
    } else {
        (c * 12.92) as f32
    }
}

fn cb_apply_gamma_rgb(rgb: CbRgb) -> CbRgb {
    CbRgb {
        r: apply_gamma_channel(rgb.r),
        g: apply_gamma_channel(rgb.g),
        b: apply_gamma_channel(rgb.b),
    }
}

/// The tritanopia simulation matrix, applied in single precision exactly as in
/// the C source. Note that the two coefficients on the red row, and the pairs
/// on the green/blue rows, are *not* the same constants in the C source even
/// though they are numerically very close; they are transcribed verbatim.
///
/// All three channels are read before any is written, matching the C code's
/// copy into locals.
fn tritanopia_matrix(rgb: CbRgb) -> CbRgb {
    let (r, g, b) = (rgb.r, rgb.g, rgb.b);
    CbRgb {
        r: r + 0.12739886310880f32 * g - 0.12739886341072f32 * b,
        g: -4.486E-11f32 * r + 0.87390929928361f32 * g + 0.12609070101523f32 * b,
        b: 3.1113E-10f32 * r + 0.87390929725848f32 * g + 0.12609070067115f32 * b,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(RGB: cb_rgb_255) -> cb_rgb_255 {
    let rgb_norm = cb_remove_gamma_rgb(cb_norm(RGB));
    let rgb_norm = tritanopia_matrix(rgb_norm);
    cb_denorm(cb_apply_gamma_rgb(rgb_norm))
}
