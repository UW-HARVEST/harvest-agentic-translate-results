#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct CbRgb255 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

fn cb_luminance(r: f32, g: f32, b: f32) -> f32 {
    // In C: R = (float)(R > 0.04045 ? pow((R + 0.055) / 1.055, 2.4) : R / 12.92);
    // The comparison and arithmetic happen in double precision because of the
    // double-typed literals; the result is then cast back to float.
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
    high / low
}

pub fn contrast_ratio(a: CbRgb255, b: CbRgb255) -> f32 {
    cb_contrast_ratio(
        (a.r as f32) / 255.0_f32,
        (a.g as f32) / 255.0_f32,
        (a.b as f32) / 255.0_f32,
        (b.r as f32) / 255.0_f32,
        (b.g as f32) / 255.0_f32,
        (b.b as f32) / 255.0_f32,
    )
}
