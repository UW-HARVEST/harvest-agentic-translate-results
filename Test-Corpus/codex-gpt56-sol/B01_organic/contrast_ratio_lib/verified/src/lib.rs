#![allow(non_snake_case)]

#[repr(C)]
pub struct cb_rgb_255 {
    pub R: u8,
    pub G: u8,
    pub B: u8,
}

#[link(name = "m")]
unsafe extern "C" {
    fn pow(base: f64, exponent: f64) -> f64;
}

fn linearize(channel: f32) -> f32 {
    let channel = f64::from(channel);
    if channel > 0.04045 {
        unsafe { pow((channel + 0.055) / 1.055, 2.4) as f32 }
    } else {
        (channel / 12.92) as f32
    }
}

fn luminance(R: f32, G: f32, B: f32) -> f32 {
    let R = linearize(R);
    let G = linearize(G);
    let B = linearize(B);
    0.2126_f32 * R + 0.7152_f32 * G + 0.0722_f32 * B
}

#[unsafe(no_mangle)]
pub extern "C" fn contrast_ratio(A: cb_rgb_255, B: cb_rgb_255) -> f32 {
    let lum_a = luminance(
        f32::from(A.R) / 255.0_f32,
        f32::from(A.G) / 255.0_f32,
        f32::from(A.B) / 255.0_f32,
    );
    let lum_b = luminance(
        f32::from(B.R) / 255.0_f32,
        f32::from(B.G) / 255.0_f32,
        f32::from(B.B) / 255.0_f32,
    );
    let (high, low) = if lum_a < lum_b {
        (lum_b, lum_a)
    } else {
        (lum_a, lum_b)
    };
    high / low
}
