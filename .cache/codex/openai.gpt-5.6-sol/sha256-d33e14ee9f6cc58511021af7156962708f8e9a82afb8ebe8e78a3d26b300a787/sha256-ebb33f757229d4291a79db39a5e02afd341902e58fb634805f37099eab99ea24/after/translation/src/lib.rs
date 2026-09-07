use std::ffi::c_uchar;

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Clone, Copy)]
pub struct cb_rgb_255 {
    pub R: c_uchar,
    pub G: c_uchar,
    pub B: c_uchar,
}

#[link(name = "m")]
unsafe extern "C" {
    fn pow(base: f64, exponent: f64) -> f64;
}

fn linearize(channel: f32) -> f32 {
    let channel = f64::from(channel);

    if channel > 0.04045 {
        // The C expression promotes its float input and constants to double.
        unsafe { pow((channel + 0.055) / 1.055, 2.4) as f32 }
    } else {
        (channel / 12.92) as f32
    }
}

fn luminance(red: f32, green: f32, blue: f32) -> f32 {
    let red = linearize(red);
    let green = linearize(green);
    let blue = linearize(blue);

    0.2126_f32 * red + 0.7152_f32 * green + 0.0722_f32 * blue
}

#[unsafe(no_mangle)]
pub extern "C" fn contrast_ratio(a: cb_rgb_255, b: cb_rgb_255) -> f32 {
    let luminance_a = luminance(
        f32::from(a.R) / 255.0_f32,
        f32::from(a.G) / 255.0_f32,
        f32::from(a.B) / 255.0_f32,
    );
    let luminance_b = luminance(
        f32::from(b.R) / 255.0_f32,
        f32::from(b.G) / 255.0_f32,
        f32::from(b.B) / 255.0_f32,
    );

    let (high, low) = if luminance_a < luminance_b {
        (luminance_b, luminance_a)
    } else {
        (luminance_a, luminance_b)
    };

    high / low
}
