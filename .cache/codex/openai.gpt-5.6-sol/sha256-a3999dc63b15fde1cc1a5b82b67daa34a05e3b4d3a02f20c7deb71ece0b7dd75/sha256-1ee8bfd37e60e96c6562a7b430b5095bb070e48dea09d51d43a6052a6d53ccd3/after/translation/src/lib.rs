#![allow(non_snake_case)]

use std::ffi::c_uchar;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cb_rgb_255 {
    pub R: c_uchar,
    pub G: c_uchar,
    pub B: c_uchar,
}

#[derive(Clone, Copy)]
struct CbRgb {
    R: f32,
    G: f32,
    B: f32,
}

#[link(name = "m")]
unsafe extern "C" {
    fn pow(base: f64, exponent: f64) -> f64;
}

#[inline]
fn remove_gamma(value: f32) -> f32 {
    if f64::from(value) > 0.04045 {
        unsafe { pow((f64::from(value) + 0.055) / 1.055, 2.4) as f32 }
    } else {
        (f64::from(value) / 12.92) as f32
    }
}

#[inline]
fn apply_gamma(value: f32) -> f32 {
    if f64::from(value) > 0.00313080495356037151702786377709 {
        unsafe { (1.055 * pow(f64::from(value), 0.4166666666) - 0.055) as f32 }
    } else {
        (f64::from(value) * 12.92) as f32
    }
}

#[inline]
fn narrow_to_uchar(value: f32) -> c_uchar {
    (value as i32) as c_uchar
}

#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(RGB: cb_rgb_255) -> cb_rgb_255 {
    let mut rgb = CbRgb {
        R: remove_gamma(RGB.R as f32 / 255.0_f32),
        G: remove_gamma(RGB.G as f32 / 255.0_f32),
        B: remove_gamma(RGB.B as f32 / 255.0_f32),
    };

    let (r, g, b) = (rgb.R, rgb.G, rgb.B);
    rgb.R = r + 0.12739886310880_f32 * g - 0.12739886341072_f32 * b;
    rgb.G = -4.486E-11_f32 * r + 0.87390929928361_f32 * g + 0.12609070101523_f32 * b;
    rgb.B = 3.1113E-10_f32 * r + 0.87390929725848_f32 * g + 0.12609070067115_f32 * b;

    cb_rgb_255 {
        R: narrow_to_uchar(apply_gamma(rgb.R) * 255.0_f32 + 0.5_f32),
        G: narrow_to_uchar(apply_gamma(rgb.G) * 255.0_f32 + 0.5_f32),
        B: narrow_to_uchar(apply_gamma(rgb.B) * 255.0_f32 + 0.5_f32),
    }
}
