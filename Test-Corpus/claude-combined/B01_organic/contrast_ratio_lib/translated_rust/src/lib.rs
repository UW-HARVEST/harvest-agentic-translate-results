use std::os::raw::c_uchar;

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types, non_snake_case)]
pub struct cb_rgb_255 {
    pub R: c_uchar,
    pub G: c_uchar,
    pub B: c_uchar,
}

fn cb_luminance(r: f32, g: f32, b: f32) -> f32 {
    // In C: R = (float)(R > 0.04045 ? pow((R + 0.055) / 1.055, 2.4) : R / 12.92)
    // The comparison and arithmetic are all done as doubles.
    let r = if (r as f64) > 0.04045f64 {
        (((r as f64) + 0.055f64) / 1.055f64).powf(2.4f64) as f32
    } else {
        ((r as f64) / 12.92f64) as f32
    };
    let g = if (g as f64) > 0.04045f64 {
        (((g as f64) + 0.055f64) / 1.055f64).powf(2.4f64) as f32
    } else {
        ((g as f64) / 12.92f64) as f32
    };
    let b = if (b as f64) > 0.04045f64 {
        (((b as f64) + 0.055f64) / 1.055f64).powf(2.4f64) as f32
    } else {
        ((b as f64) / 12.92f64) as f32
    };

    0.2126f32 * r + 0.7152f32 * g + 0.0722f32 * b
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
    high / low
}

#[unsafe(no_mangle)]
pub extern "C" fn contrast_ratio(a: cb_rgb_255, b: cb_rgb_255) -> f32 {
    cb_contrast_ratio(
        (a.R as f32) / 255.0f32,
        (a.G as f32) / 255.0f32,
        (a.B as f32) / 255.0f32,
        (b.R as f32) / 255.0f32,
        (b.G as f32) / 255.0f32,
        (b.B as f32) / 255.0f32,
    )
}
