#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct cb_rgb_255 {
    pub R: std::os::raw::c_uchar,
    pub G: std::os::raw::c_uchar,
    pub B: std::os::raw::c_uchar,
}

#[derive(Copy, Clone, Debug)]
struct cb_rgb {
    R: f32,
    G: f32,
    B: f32,
}

fn cbRemoveGammaRGB(RGB: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: if (RGB.R as f64) > 0.04045 {
            (((RGB.R as f64 + 0.055) / 1.055).powf(2.4)) as f32
        } else {
            (RGB.R as f64 / 12.92) as f32
        },
        G: if (RGB.G as f64) > 0.04045 {
            (((RGB.G as f64 + 0.055) / 1.055).powf(2.4)) as f32
        } else {
            (RGB.G as f64 / 12.92) as f32
        },
        B: if (RGB.B as f64) > 0.04045 {
            (((RGB.B as f64 + 0.055) / 1.055).powf(2.4)) as f32
        } else {
            (RGB.B as f64 / 12.92) as f32
        },
    }
}

fn cbNorm(RGB: cb_rgb_255) -> cb_rgb {
    cb_rgb {
        R: (RGB.R as f32) / 255.0_f32,
        G: (RGB.G as f32) / 255.0_f32,
        B: (RGB.B as f32) / 255.0_f32,
    }
}

fn cbDenorm(RGB: cb_rgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: (RGB.R * 255.0_f32 + 0.5_f32) as u8,
        G: (RGB.G * 255.0_f32 + 0.5_f32) as u8,
        B: (RGB.B * 255.0_f32 + 0.5_f32) as u8,
    }
}

fn cbApplyGammaRGB(RGB: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: if (RGB.R as f64) > 0.00313080495356037151702786377709 {
            (1.055 * (RGB.R as f64).powf(0.4166666666) - 0.055) as f32
        } else {
            (RGB.R as f64 * 12.92) as f32
        },
        G: if (RGB.G as f64) > 0.00313080495356037151702786377709 {
            (1.055 * (RGB.G as f64).powf(0.4166666666) - 0.055) as f32
        } else {
            (RGB.G as f64 * 12.92) as f32
        },
        B: if (RGB.B as f64) > 0.00313080495356037151702786377709 {
            (1.055 * (RGB.B as f64).powf(0.4166666666) - 0.055) as f32
        } else {
            (RGB.B as f64 * 12.92) as f32
        },
    }
}

fn Tritanopia(Red: &mut f32, Green: &mut f32, Blue: &mut f32) {
    let R = *Red;
    let G = *Green;
    let B = *Blue;
    *Red = R + 0.12739886310880_f32 * G - 0.12739886341072_f32 * B;
    *Green = -4.486E-11_f32 * R + 0.87390929928361_f32 * G + 0.12609070101523_f32 * B;
    *Blue = 3.1113E-10_f32 * R + 0.87390929725848_f32 * G + 0.12609070067115_f32 * B;
}

#[no_mangle]
pub extern "C" fn tritanopia(RGB: cb_rgb_255) -> cb_rgb_255 {
    let mut RGBNorm = cbRemoveGammaRGB(cbNorm(RGB));
    Tritanopia(&mut RGBNorm.R, &mut RGBNorm.G, &mut RGBNorm.B);
    cbDenorm(cbApplyGammaRGB(RGBNorm))
}
