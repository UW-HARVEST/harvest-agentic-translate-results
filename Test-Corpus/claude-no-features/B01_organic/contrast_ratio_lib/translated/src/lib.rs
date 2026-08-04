#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cb_rgb_255 {
    pub R: core::ffi::c_uchar,
    pub G: core::ffi::c_uchar,
    pub B: core::ffi::c_uchar,
}

fn cb_luminance(R: f32, G: f32, B: f32) -> f32 {
    // Reproduce C semantics exactly: comparisons and arithmetic against double
    // literals promote the float operand to double, then result is cast back
    // to float.
    let R: f32 = {
        let Rd = R as f64;
        if Rd > 0.04045_f64 {
            ((Rd + 0.055_f64) / 1.055_f64).powf(2.4_f64) as f32
        } else {
            (Rd / 12.92_f64) as f32
        }
    };
    let G: f32 = {
        let Gd = G as f64;
        if Gd > 0.04045_f64 {
            ((Gd + 0.055_f64) / 1.055_f64).powf(2.4_f64) as f32
        } else {
            (Gd / 12.92_f64) as f32
        }
    };
    let B: f32 = {
        let Bd = B as f64;
        if Bd > 0.04045_f64 {
            ((Bd + 0.055_f64) / 1.055_f64).powf(2.4_f64) as f32
        } else {
            (Bd / 12.92_f64) as f32
        }
    };
    let result: f32 = 0.2126_f32 * R + 0.7152_f32 * G + 0.0722_f32 * B;
    result
}

fn cb_contrast_ratio(RA: f32, GA: f32, BA: f32, RB: f32, GB: f32, BB: f32) -> f32 {
    let lum_a = cb_luminance(RA, GA, BA);
    let lum_b = cb_luminance(RB, GB, BB);
    let mut high = lum_a;
    let mut low = lum_b;
    if high < low {
        high = lum_b;
        low = lum_a;
    }
    high / low
}

#[unsafe(no_mangle)]
pub extern "C" fn contrast_ratio(A: cb_rgb_255, B: cb_rgb_255) -> f32 {
    cb_contrast_ratio(
        (A.R as f32) / 255.0_f32,
        (A.G as f32) / 255.0_f32,
        (A.B as f32) / 255.0_f32,
        (B.R as f32) / 255.0_f32,
        (B.G as f32) / 255.0_f32,
        (B.B as f32) / 255.0_f32,
    )
}
