#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::c_uchar;

#[repr(C)]
pub struct cb_rgb_255 {
    pub R: c_uchar,
    pub G: c_uchar,
    pub B: c_uchar,
}

struct cb_rgb {
    R: f32,
    G: f32,
    B: f32,
}

#[inline]
fn cb_remove_gamma_rgb(rgb: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: if (rgb.R as f64) > 0.04045_f64 {
            (((rgb.R as f64) + 0.055_f64) / 1.055_f64).powf(2.4_f64) as f32
        } else {
            ((rgb.R as f64) / 12.92_f64) as f32
        },
        G: if (rgb.G as f64) > 0.04045_f64 {
            (((rgb.G as f64) + 0.055_f64) / 1.055_f64).powf(2.4_f64) as f32
        } else {
            ((rgb.G as f64) / 12.92_f64) as f32
        },
        B: if (rgb.B as f64) > 0.04045_f64 {
            (((rgb.B as f64) + 0.055_f64) / 1.055_f64).powf(2.4_f64) as f32
        } else {
            ((rgb.B as f64) / 12.92_f64) as f32
        },
    }
}

#[inline]
fn cb_norm(rgb: &cb_rgb_255) -> cb_rgb {
    cb_rgb {
        R: (rgb.R as f32) / 255.0_f32,
        G: (rgb.G as f32) / 255.0_f32,
        B: (rgb.B as f32) / 255.0_f32,
    }
}

#[inline]
fn float_to_uchar(x: f32) -> c_uchar {
    // Replicates C's (unsigned char)(float) which on x86_64 typically uses
    // cvttss2si (truncation toward zero) followed by taking the low 8 bits.
    // Rust's `as i32` from f32 saturates on out-of-range, but for values that
    // fit in i32 range (which all our intermediate values do), it truncates
    // toward zero just like cvttss2si. The subsequent `as u8` then takes the
    // low 8 bits, matching C's behavior for typical out-of-range RGB floats.
    (x as i32) as u8
}

#[inline]
fn cb_denorm(rgb: cb_rgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: float_to_uchar(rgb.R * 255.0_f32 + 0.5_f32),
        G: float_to_uchar(rgb.G * 255.0_f32 + 0.5_f32),
        B: float_to_uchar(rgb.B * 255.0_f32 + 0.5_f32),
    }
}

#[inline]
fn cb_apply_gamma_rgb(rgb: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: if (rgb.R as f64) > 0.00313080495356037151702786377709_f64 {
            (1.055_f64 * (rgb.R as f64).powf(0.4166666666_f64) - 0.055_f64) as f32
        } else {
            ((rgb.R as f64) * 12.92_f64) as f32
        },
        G: if (rgb.G as f64) > 0.00313080495356037151702786377709_f64 {
            (1.055_f64 * (rgb.G as f64).powf(0.4166666666_f64) - 0.055_f64) as f32
        } else {
            ((rgb.G as f64) * 12.92_f64) as f32
        },
        B: if (rgb.B as f64) > 0.00313080495356037151702786377709_f64 {
            (1.055_f64 * (rgb.B as f64).powf(0.4166666666_f64) - 0.055_f64) as f32
        } else {
            ((rgb.B as f64) * 12.92_f64) as f32
        },
    }
}

#[inline]
fn tritanopia_inner(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let r = *red;
    let g = *green;
    let b = *blue;
    *red = r + 0.12739886310880_f32 * g - 0.12739886341072_f32 * b;
    *green = -4.486E-11_f32 * r + 0.87390929928361_f32 * g + 0.12609070101523_f32 * b;
    *blue = 3.1113E-10_f32 * r + 0.87390929725848_f32 * g + 0.12609070067115_f32 * b;
}

#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(RGB: cb_rgb_255) -> cb_rgb_255 {
    let mut rgb_norm = cb_remove_gamma_rgb(cb_norm(&RGB));
    tritanopia_inner(&mut rgb_norm.R, &mut rgb_norm.G, &mut rgb_norm.B);
    cb_denorm(cb_apply_gamma_rgb(rgb_norm))
}
