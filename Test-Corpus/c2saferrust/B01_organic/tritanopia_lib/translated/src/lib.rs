





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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cb_rgb {
    pub R: ::core::ffi::c_float,
    pub G: ::core::ffi::c_float,
    pub B: ::core::ffi::c_float,
}
fn cbRemoveGammaRGB(rgb: cb_rgb) -> cb_rgb {
    fn remove_gamma(channel: f32) -> f32 {
        if channel > 0.04045 {
            (((channel as f64 + 0.055) / 1.055).powf(2.4)) as f32
        } else {
            channel / 12.92
        }
    }

    cb_rgb {
        R: remove_gamma(rgb.R),
        G: remove_gamma(rgb.G),
        B: remove_gamma(rgb.B),
    }
}

fn cbNorm(rgb: cb_rgb_255) -> cb_rgb {
    cb_rgb {
        R: rgb.R as f32 / 255.0,
        G: rgb.G as f32 / 255.0,
        B: rgb.B as f32 / 255.0,
    }
}

fn cbDenorm(rgb: cb_rgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: (rgb.R * 255.0 + 0.5) as _,
        G: (rgb.G * 255.0 + 0.5) as _,
        B: (rgb.B * 255.0 + 0.5) as _,
    }
}

fn cbApplyGammaRGB(rgb: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: if (rgb.R as f64) > 0.0031308049535603715_f64 {
            (1.055_f64 * (rgb.R as f64).powf(0.4166666666_f64) - 0.055_f64) as f32
        } else {
            ((rgb.R as f64) * 12.92_f64) as f32
        },
        G: if (rgb.G as f64) > 0.0031308049535603715_f64 {
            (1.055_f64 * (rgb.G as f64).powf(0.4166666666_f64) - 0.055_f64) as f32
        } else {
            ((rgb.G as f64) * 12.92_f64) as f32
        },
        B: if (rgb.B as f64) > 0.0031308049535603715_f64 {
            (1.055_f64 * (rgb.B as f64).powf(0.4166666666_f64) - 0.055_f64) as f32
        } else {
            ((rgb.B as f64) * 12.92_f64) as f32
        },
    }
}

fn Tritanopia(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let r = *red;
    let g = *green;
    let b = *blue;
    *red = r + 0.12739886310880f32 * g - 0.12739886341072f32 * b;
    *green = -4.486E-11f32 * r + 0.87390929928361f32 * g + 0.12609070101523f32 * b;
    *blue = 3.1113E-10f32 * r + 0.87390929725848f32 * g + 0.12609070067115f32 * b;
}

#[no_mangle]
pub fn tritanopia(rgb: cb_rgb_255) -> cb_rgb_255 {
    let mut rgb_norm: cb_rgb = cbRemoveGammaRGB(cbNorm(rgb));
    Tritanopia(&mut rgb_norm.R, &mut rgb_norm.G, &mut rgb_norm.B);
    cbDenorm(cbApplyGammaRGB(rgb_norm))
}

