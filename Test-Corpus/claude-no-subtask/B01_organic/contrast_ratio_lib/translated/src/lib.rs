#![allow(non_snake_case)]

use std::os::raw::c_uchar;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cb_rgb_255 {
    pub R: c_uchar,
    pub G: c_uchar,
    pub B: c_uchar,
}

fn cb_luminance(R: f32, G: f32, B: f32) -> f32 {
    // In C: R > 0.04045 promotes R to double; pow takes/returns double; cast back to float.
    let R = if (R as f64) > 0.04045f64 {
        ((R as f64 + 0.055f64) / 1.055f64).powf(2.4f64) as f32
    } else {
        (R as f64 / 12.92f64) as f32
    };
    let G = if (G as f64) > 0.04045f64 {
        ((G as f64 + 0.055f64) / 1.055f64).powf(2.4f64) as f32
    } else {
        (G as f64 / 12.92f64) as f32
    };
    let B = if (B as f64) > 0.04045f64 {
        ((B as f64 + 0.055f64) / 1.055f64).powf(2.4f64) as f32
    } else {
        (B as f64 / 12.92f64) as f32
    };
    let Result: f32 = 0.2126f32 * R + 0.7152f32 * G + 0.0722f32 * B;
    Result
}

fn cb_contrast_ratio(RA: f32, GA: f32, BA: f32, RB: f32, GB: f32, BB: f32) -> f32 {
    let LumA = cb_luminance(RA, GA, BA);
    let LumB = cb_luminance(RB, GB, BB);
    let mut High = LumA;
    let mut Low = LumB;
    if High < Low {
        High = LumB;
        Low = LumA;
    }
    let Ratio = High / Low;
    Ratio
}

#[unsafe(no_mangle)]
pub extern "C" fn contrast_ratio(A: cb_rgb_255, B: cb_rgb_255) -> f32 {
    cb_contrast_ratio(
        A.R as f32 / 255.0f32,
        A.G as f32 / 255.0f32,
        A.B as f32 / 255.0f32,
        B.R as f32 / 255.0f32,
        B.G as f32 / 255.0f32,
        B.B as f32 / 255.0f32,
    )
}
