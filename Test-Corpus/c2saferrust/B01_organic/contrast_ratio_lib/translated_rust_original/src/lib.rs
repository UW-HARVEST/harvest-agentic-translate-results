


extern "C" {
    fn pow(__x: ::core::ffi::c_double, __y: ::core::ffi::c_double) -> ::core::ffi::c_double;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cb_rgb_255 {
    pub R: ::core::ffi::c_uchar,
    pub G: ::core::ffi::c_uchar,
    pub B: ::core::ffi::c_uchar,
}
fn cbLuminance(r: f32, g: f32, b: f32) -> f32 {
    fn linearize(c: f32) -> f32 {
        if c > 0.04045 {
            ((c + 0.055) / 1.055).powf(2.4)
        } else {
            c / 12.92
        }
    }

    let r = linearize(r);
    let g = linearize(g);
    let b = linearize(b);

    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn cbContrastRatio(
    ra: f32,
    ga: f32,
    ba: f32,
    rb: f32,
    gb: f32,
    bb: f32,
) -> f32 {
    let lum_a = cbLuminance(ra, ga, ba);
    let lum_b = cbLuminance(rb, gb, bb);
    let (high, low) = if lum_a < lum_b {
        (lum_b, lum_a)
    } else {
        (lum_a, lum_b)
    };
    high / low
}

#[no_mangle]
pub fn contrast_ratio(a: cb_rgb_255, b: cb_rgb_255) -> f32 {
    cbContrastRatio(
        a.R as f32 / 255.0,
        a.G as f32 / 255.0,
        a.B as f32 / 255.0,
        b.R as f32 / 255.0,
        b.G as f32 / 255.0,
        b.B as f32 / 255.0,
    )
}

