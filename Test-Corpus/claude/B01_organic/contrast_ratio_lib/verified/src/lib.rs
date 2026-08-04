use std::ffi::c_float;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cb_rgb_255 {
    pub R: u8,
    pub G: u8,
    pub B: u8,
}

fn cb_luminance(r: f32, g: f32, b: f32) -> f32 {
    // Match C semantics: comparisons and arithmetic with double constants
    // promote f32 to f64; pow() is double precision; final cast back to float.
    let r_d: f64 = r as f64;
    let g_d: f64 = g as f64;
    let b_d: f64 = b as f64;

    let r_f: f32 = if r_d > 0.04045 {
        ((r_d + 0.055) / 1.055).powf(2.4) as f32
    } else {
        (r_d / 12.92) as f32
    };
    let g_f: f32 = if g_d > 0.04045 {
        ((g_d + 0.055) / 1.055).powf(2.4) as f32
    } else {
        (g_d / 12.92) as f32
    };
    let b_f: f32 = if b_d > 0.04045 {
        ((b_d + 0.055) / 1.055).powf(2.4) as f32
    } else {
        (b_d / 12.92) as f32
    };

    let result: f32 = 0.2126_f32 * r_f + 0.7152_f32 * g_f + 0.0722_f32 * b_f;
    result
}

fn cb_contrast_ratio(ra: f32, ga: f32, ba: f32, rb: f32, gb: f32, bb: f32) -> f32 {
    let lum_a: f32 = cb_luminance(ra, ga, ba);
    let lum_b: f32 = cb_luminance(rb, gb, bb);
    let mut high: f32 = lum_a;
    let mut low: f32 = lum_b;
    if high < low {
        high = lum_b;
        low = lum_a;
    }
    let ratio: f32 = high / low;
    ratio
}

#[unsafe(no_mangle)]
pub extern "C" fn contrast_ratio(A: cb_rgb_255, B: cb_rgb_255) -> c_float {
    cb_contrast_ratio(
        (A.R as f32) / 255.0_f32,
        (A.G as f32) / 255.0_f32,
        (A.B as f32) / 255.0_f32,
        (B.R as f32) / 255.0_f32,
        (B.G as f32) / 255.0_f32,
        (B.B as f32) / 255.0_f32,
    )
}
