#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::c_uchar;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cb_rgb_255 {
    pub R: c_uchar,
    pub G: c_uchar,
    pub B: c_uchar,
}

#[derive(Copy, Clone)]
struct CbRgb {
    r: f32,
    g: f32,
    b: f32,
}

#[inline]
fn float_to_uchar(v: f32) -> u8 {
    // Mimic the C `(unsigned char)` cast from a float on x86_64 (cvttss2si + truncation),
    // which truncates the fractional part toward zero, converts to int, then takes the
    // low byte. This matches gcc/clang behavior on x86 for the range we operate in.
    // For non-finite or out-of-i32-range values, x86 returns 0x80000000 whose low byte
    // is 0, so we return 0 in that case.
    if v.is_finite() && v > -2147483649.0_f32 && v < 2147483648.0_f32 {
        // Safety: value is finite and within i32 range, so truncation is well-defined.
        unsafe { v.to_int_unchecked::<i32>() as u8 }
    } else {
        0
    }
}

fn cbRemoveGammaRGB(RGB: CbRgb) -> CbRgb {
    let process = |v: f32| -> f32 {
        let v_d = v as f64;
        if v_d > 0.04045_f64 {
            ((v_d + 0.055_f64) / 1.055_f64).powf(2.4_f64) as f32
        } else {
            (v_d / 12.92_f64) as f32
        }
    };
    CbRgb {
        r: process(RGB.r),
        g: process(RGB.g),
        b: process(RGB.b),
    }
}

fn cbNorm(RGB: cb_rgb_255) -> CbRgb {
    CbRgb {
        r: (RGB.R as f32) / 255.0_f32,
        g: (RGB.G as f32) / 255.0_f32,
        b: (RGB.B as f32) / 255.0_f32,
    }
}

fn cbDenorm(RGB: CbRgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: float_to_uchar(RGB.r * 255.0_f32 + 0.5_f32),
        G: float_to_uchar(RGB.g * 255.0_f32 + 0.5_f32),
        B: float_to_uchar(RGB.b * 255.0_f32 + 0.5_f32),
    }
}

fn cbApplyGammaRGB(RGB: CbRgb) -> CbRgb {
    let process = |v: f32| -> f32 {
        let v_d = v as f64;
        if v_d > 0.00313080495356037151702786377709_f64 {
            (1.055_f64 * v_d.powf(0.4166666666_f64) - 0.055_f64) as f32
        } else {
            (v_d * 12.92_f64) as f32
        }
    };
    CbRgb {
        r: process(RGB.r),
        g: process(RGB.g),
        b: process(RGB.b),
    }
}

fn Tritanopia(Red: &mut f32, Green: &mut f32, Blue: &mut f32) {
    let R = *Red;
    let G = *Green;
    let B = *Blue;
    *Red = R + 0.12739886310880_f32 * G - 0.12739886341072_f32 * B;
    *Green = -4.486E-11_f32 * R + 0.87390929928361_f32 * G + 0.12609070101523_f32 * B;
    *Blue = 3.1113E-10_f32 * R + 0.87390929725848_f32 * G + 0.12609070067115_f32 * B;
}

#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(RGB: cb_rgb_255) -> cb_rgb_255 {
    let mut RGBNorm = cbRemoveGammaRGB(cbNorm(RGB));
    Tritanopia(&mut RGBNorm.r, &mut RGBNorm.g, &mut RGBNorm.b);
    cbDenorm(cbApplyGammaRGB(RGBNorm))
}
