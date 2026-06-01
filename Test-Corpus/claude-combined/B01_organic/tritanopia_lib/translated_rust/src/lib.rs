#![allow(non_camel_case_types, non_snake_case)]

use std::os::raw::c_uchar;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cb_rgb_255 {
    pub R: c_uchar,
    pub G: c_uchar,
    pub B: c_uchar,
}

#[derive(Copy, Clone)]
struct cb_rgb {
    R: f32,
    G: f32,
    B: f32,
}

#[inline]
fn remove_gamma_component(c: f32) -> f32 {
    let cd = c as f64;
    if cd > 0.04045 {
        ((cd + 0.055) / 1.055).powf(2.4) as f32
    } else {
        (cd / 12.92) as f32
    }
}

fn cb_remove_gamma_rgb(rgb: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: remove_gamma_component(rgb.R),
        G: remove_gamma_component(rgb.G),
        B: remove_gamma_component(rgb.B),
    }
}

fn cb_norm(rgb: cb_rgb_255) -> cb_rgb {
    cb_rgb {
        R: (rgb.R as f32) / 255.0_f32,
        G: (rgb.G as f32) / 255.0_f32,
        B: (rgb.B as f32) / 255.0_f32,
    }
}

// Replicate C's `(unsigned char)(float)` conversion semantics on x86_64 GCC:
// the compiler emits CVTTSS2SI to truncate toward zero into a signed 32-bit
// integer (out-of-range yields 0x80000000), then narrows to `unsigned char`
// by taking the low 8 bits (wrapping modulo 256). Rust's plain `as u8`
// from f32 saturates instead, so we must do this in two steps.
#[inline]
fn float_to_uchar_c_cast(v: f32) -> c_uchar {
    // Match CVTTSS2SI: out-of-range values produce i32::MIN (0x80000000).
    let i: i32 = if v.is_nan() || v < (i32::MIN as f32) || v >= -((i32::MIN as f32)) {
        i32::MIN
    } else {
        // Truncation toward zero, in range.
        v as i32
    };
    i as u32 as u8
}

fn cb_denorm(rgb: cb_rgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: float_to_uchar_c_cast(rgb.R * 255.0_f32 + 0.5_f32),
        G: float_to_uchar_c_cast(rgb.G * 255.0_f32 + 0.5_f32),
        B: float_to_uchar_c_cast(rgb.B * 255.0_f32 + 0.5_f32),
    }
}

#[inline]
fn apply_gamma_component(c: f32) -> f32 {
    let cd = c as f64;
    if cd > 0.00313080495356037151702786377709_f64 {
        (1.055_f64 * cd.powf(0.4166666666_f64) - 0.055_f64) as f32
    } else {
        (cd * 12.92_f64) as f32
    }
}

fn cb_apply_gamma_rgb(rgb: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: apply_gamma_component(rgb.R),
        G: apply_gamma_component(rgb.G),
        B: apply_gamma_component(rgb.B),
    }
}

fn tritanopia_inner(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let r = *red;
    let g = *green;
    let b = *blue;
    *red = r + 0.12739886310880_f32 * g - 0.12739886341072_f32 * b;
    *green = -4.486E-11_f32 * r + 0.87390929928361_f32 * g + 0.12609070101523_f32 * b;
    *blue = 3.1113E-10_f32 * r + 0.87390929725848_f32 * g + 0.12609070067115_f32 * b;
}

#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(rgb: cb_rgb_255) -> cb_rgb_255 {
    let mut rgb_norm = cb_remove_gamma_rgb(cb_norm(rgb));
    tritanopia_inner(&mut rgb_norm.R, &mut rgb_norm.G, &mut rgb_norm.B);
    cb_denorm(cb_apply_gamma_rgb(rgb_norm))
}
