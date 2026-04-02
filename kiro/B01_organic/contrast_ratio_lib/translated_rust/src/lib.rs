#[repr(C)]
#[allow(non_snake_case)]
pub struct cb_rgb_255 {
    pub R: u8,
    pub G: u8,
    pub B: u8,
}

fn cb_luminance(mut r: f32, mut g: f32, mut b: f32) -> f32 {
    r = if r > 0.04045 {
        ((r as f64 + 0.055) / 1.055).powf(2.4) as f32
    } else {
        r / 12.92
    };
    g = if g > 0.04045 {
        ((g as f64 + 0.055) / 1.055).powf(2.4) as f32
    } else {
        g / 12.92
    };
    b = if b > 0.04045 {
        ((b as f64 + 0.055) / 1.055).powf(2.4) as f32
    } else {
        b / 12.92
    };
    0.2126_f32 * r + 0.7152_f32 * g + 0.0722_f32 * b
}

fn cb_contrast_ratio(ra: f32, ga: f32, ba: f32, rb: f32, gb: f32, bb: f32) -> f32 {
    let lum_a = cb_luminance(ra, ga, ba);
    let lum_b = cb_luminance(rb, gb, bb);
    let (high, low) = if lum_a < lum_b {
        (lum_b, lum_a)
    } else {
        (lum_a, lum_b)
    };
    high / low
}

#[unsafe(no_mangle)]
pub extern "C" fn contrast_ratio(a: cb_rgb_255, b: cb_rgb_255) -> f32 {
    cb_contrast_ratio(
        a.R as f32 / 255.0_f32,
        a.G as f32 / 255.0_f32,
        a.B as f32 / 255.0_f32,
        b.R as f32 / 255.0_f32,
        b.G as f32 / 255.0_f32,
        b.B as f32 / 255.0_f32,
    )
}
