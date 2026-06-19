use std::ffi::c_float;

#[allow(non_snake_case)]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct cb_rgb_255 {
    pub R: u8,
    pub G: u8,
    pub B: u8,
}

fn cb_luminance(mut r: c_float, mut g: c_float, mut b: c_float) -> c_float {
    r = if r > 0.04045_f32 {
        (((r as f64) + 0.055_f64) / 1.055_f64).powf(2.4_f64) as c_float
    } else {
        r / 12.92_f32
    };
    g = if g > 0.04045_f32 {
        (((g as f64) + 0.055_f64) / 1.055_f64).powf(2.4_f64) as c_float
    } else {
        g / 12.92_f32
    };
    b = if b > 0.04045_f32 {
        (((b as f64) + 0.055_f64) / 1.055_f64).powf(2.4_f64) as c_float
    } else {
        b / 12.92_f32
    };

    0.2126_f32 * r + 0.7152_f32 * g + 0.0722_f32 * b
}

fn cb_contrast_ratio(
    ra: c_float,
    ga: c_float,
    ba: c_float,
    rb: c_float,
    gb: c_float,
    bb: c_float,
) -> c_float {
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
pub extern "C" fn contrast_ratio(a: cb_rgb_255, b: cb_rgb_255) -> c_float {
    cb_contrast_ratio(
        (a.R as c_float) / 255.0_f32,
        (a.G as c_float) / 255.0_f32,
        (a.B as c_float) / 255.0_f32,
        (b.R as c_float) / 255.0_f32,
        (b.G as c_float) / 255.0_f32,
        (b.B as c_float) / 255.0_f32,
    )
}
