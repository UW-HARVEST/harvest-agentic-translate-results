use std::ffi::{c_float, c_uchar};

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(non_snake_case)]
pub struct cb_rgb_255 {
    pub R: c_uchar,
    pub G: c_uchar,
    pub B: c_uchar,
}

#[link(name = "m")]
unsafe extern "C" {
    fn pow(x: f64, y: f64) -> f64;
}

fn cb_luminance(mut r: c_float, mut g: c_float, mut b: c_float) -> c_float {
    r = if f64::from(r) > 0.04045 {
        unsafe { pow((f64::from(r) + 0.055) / 1.055, 2.4) as c_float }
    } else {
        (f64::from(r) / 12.92) as c_float
    };
    g = if f64::from(g) > 0.04045 {
        unsafe { pow((f64::from(g) + 0.055) / 1.055, 2.4) as c_float }
    } else {
        (f64::from(g) / 12.92) as c_float
    };
    b = if f64::from(b) > 0.04045 {
        unsafe { pow((f64::from(b) + 0.055) / 1.055, 2.4) as c_float }
    } else {
        (f64::from(b) / 12.92) as c_float
    };

    0.2126f32 * r + 0.7152f32 * g + 0.0722f32 * b
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
        c_float::from(a.R) / 255.0f32,
        c_float::from(a.G) / 255.0f32,
        c_float::from(a.B) / 255.0f32,
        c_float::from(b.R) / 255.0f32,
        c_float::from(b.G) / 255.0f32,
        c_float::from(b.B) / 255.0f32,
    )
}
