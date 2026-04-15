use std::ffi::c_uchar;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct cb_rgb_255 {
    pub R: c_uchar,
    pub G: c_uchar,
    pub B: c_uchar,
}

fn cb_luminance(mut r: f32, mut g: f32, mut b: f32) -> f32 {
    r = if r > 0.04045 {
        ((r + 0.055) / 1.055).powf(2.4)
    } else {
        r / 12.92
    };
    g = if g > 0.04045 {
        ((g + 0.055) / 1.055).powf(2.4)
    } else {
        g / 12.92
    };
    b = if b > 0.04045 {
        ((b + 0.055) / 1.055).powf(2.4)
    } else {
        b / 12.92
    };
    0.2126 * r + 0.7152 * g + 0.0722 * b
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
        a.R as f32 / 255.0,
        a.G as f32 / 255.0,
        a.B as f32 / 255.0,
        b.R as f32 / 255.0,
        b.G as f32 / 255.0,
        b.B as f32 / 255.0,
    )
}
