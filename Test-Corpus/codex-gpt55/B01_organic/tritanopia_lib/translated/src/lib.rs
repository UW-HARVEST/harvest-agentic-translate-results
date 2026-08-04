#[repr(C)]
#[derive(Clone, Copy)]
#[allow(non_snake_case)]
pub struct cb_rgb_255 {
    pub R: u8,
    pub G: u8,
    pub B: u8,
}

#[derive(Clone, Copy)]
struct CbRgb {
    r: f32,
    g: f32,
    b: f32,
}

fn cb_remove_gamma_rgb(rgb: CbRgb) -> CbRgb {
    CbRgb {
        r: if rgb.r > 0.04045_f32 {
            (((rgb.r as f64 + 0.055) / 1.055).powf(2.4)) as f32
        } else {
            (rgb.r as f64 / 12.92) as f32
        },
        g: if rgb.g > 0.04045_f32 {
            (((rgb.g as f64 + 0.055) / 1.055).powf(2.4)) as f32
        } else {
            (rgb.g as f64 / 12.92) as f32
        },
        b: if rgb.b > 0.04045_f32 {
            (((rgb.b as f64 + 0.055) / 1.055).powf(2.4)) as f32
        } else {
            (rgb.b as f64 / 12.92) as f32
        },
    }
}

fn cb_norm(rgb: cb_rgb_255) -> CbRgb {
    CbRgb {
        r: rgb.R as f32 / 255.0_f32,
        g: rgb.G as f32 / 255.0_f32,
        b: rgb.B as f32 / 255.0_f32,
    }
}

fn cb_denorm(rgb: CbRgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: c_float_to_uchar(rgb.r * 255.0_f32 + 0.5_f32),
        G: c_float_to_uchar(rgb.g * 255.0_f32 + 0.5_f32),
        B: c_float_to_uchar(rgb.b * 255.0_f32 + 0.5_f32),
    }
}

fn c_float_to_uchar(value: f32) -> u8 {
    (value as i32) as u8
}

fn cb_apply_gamma_rgb(rgb: CbRgb) -> CbRgb {
    CbRgb {
        r: if rgb.r > 0.00313080495356037151702786377709_f32 {
            (1.055 * (rgb.r as f64).powf(0.4166666666) - 0.055) as f32
        } else {
            (rgb.r as f64 * 12.92) as f32
        },
        g: if rgb.g > 0.00313080495356037151702786377709_f32 {
            (1.055 * (rgb.g as f64).powf(0.4166666666) - 0.055) as f32
        } else {
            (rgb.g as f64 * 12.92) as f32
        },
        b: if rgb.b > 0.00313080495356037151702786377709_f32 {
            (1.055 * (rgb.b as f64).powf(0.4166666666) - 0.055) as f32
        } else {
            (rgb.b as f64 * 12.92) as f32
        },
    }
}

fn tritanopia_inner(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let r = *red;
    let g = *green;
    let b = *blue;

    *red = r + 0.12739886310880_f32 * g - 0.12739886341072_f32 * b;
    *green = -4.486E-11_f32 * r + 0.87390929928361_f32 * g + 0.12609070101523_f32 * b;
    *blue = 3.1113E-10_f32 * r + 0.87390929725848_f32 * g + 0.12609070067115_f32 * b;
}

#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(rgb: cb_rgb_255) -> cb_rgb_255 {
    let mut rgb_norm = cb_remove_gamma_rgb(cb_norm(rgb));
    tritanopia_inner(&mut rgb_norm.r, &mut rgb_norm.g, &mut rgb_norm.b);
    cb_denorm(cb_apply_gamma_rgb(rgb_norm))
}
