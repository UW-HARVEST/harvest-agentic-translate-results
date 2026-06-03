use std::ffi::c_uchar;

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types, non_snake_case)]
pub struct cb_rgb_255 {
    pub R: c_uchar,
    pub G: c_uchar,
    pub B: c_uchar,
}

fn cb_luminance(r: f32, g: f32, b: f32) -> f32 {
    // In C, the comparison `R > 0.04045` promotes R (float) to double.
    // The arithmetic in the ternary is done in double, then cast to float.
    let r = if (r as f64) > 0.04045_f64 {
        (((r as f64) + 0.055_f64) / 1.055_f64).powf(2.4_f64) as f32
    } else {
        ((r as f64) / 12.92_f64) as f32
    };
    let g = if (g as f64) > 0.04045_f64 {
        (((g as f64) + 0.055_f64) / 1.055_f64).powf(2.4_f64) as f32
    } else {
        ((g as f64) / 12.92_f64) as f32
    };
    let b = if (b as f64) > 0.04045_f64 {
        (((b as f64) + 0.055_f64) / 1.055_f64).powf(2.4_f64) as f32
    } else {
        ((b as f64) / 12.92_f64) as f32
    };
    let result: f32 = 0.2126_f32 * r + 0.7152_f32 * g + 0.0722_f32 * b;
    result
}

fn cb_contrast_ratio(ra: f32, ga: f32, ba: f32, rb: f32, gb: f32, bb: f32) -> f32 {
    let lum_a = cb_luminance(ra, ga, ba);
    let lum_b = cb_luminance(rb, gb, bb);
    let mut high = lum_a;
    let mut low = lum_b;
    if high < low {
        high = lum_b;
        low = lum_a;
    }
    let ratio = high / low;
    ratio
}

#[unsafe(no_mangle)]
pub extern "C" fn contrast_ratio(a: cb_rgb_255, b: cb_rgb_255) -> f32 {
    cb_contrast_ratio(
        (a.R as f32) / 255.0_f32,
        (a.G as f32) / 255.0_f32,
        (a.B as f32) / 255.0_f32,
        (b.R as f32) / 255.0_f32,
        (b.G as f32) / 255.0_f32,
        (b.B as f32) / 255.0_f32,
    )
}
