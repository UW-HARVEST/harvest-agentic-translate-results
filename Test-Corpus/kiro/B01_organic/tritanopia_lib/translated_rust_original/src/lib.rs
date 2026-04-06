#[repr(C)]
#[allow(non_snake_case)]
pub struct cb_rgb_255 {
    pub R: u8,
    pub G: u8,
    pub B: u8,
}

struct CbRgb {
    r: f32,
    g: f32,
    b: f32,
}

fn cb_remove_gamma_rgb(rgb: CbRgb) -> CbRgb {
    CbRgb {
        r: if rgb.r > 0.04045 {
            ((rgb.r as f64 + 0.055) / 1.055).powf(2.4) as f32
        } else {
            rgb.r / 12.92
        },
        g: if rgb.g > 0.04045 {
            ((rgb.g as f64 + 0.055) / 1.055).powf(2.4) as f32
        } else {
            rgb.g / 12.92
        },
        b: if rgb.b > 0.04045 {
            ((rgb.b as f64 + 0.055) / 1.055).powf(2.4) as f32
        } else {
            rgb.b / 12.92
        },
    }
}

fn cb_norm(rgb: &cb_rgb_255) -> CbRgb {
    CbRgb {
        r: rgb.R as f32 / 255.0,
        g: rgb.G as f32 / 255.0,
        b: rgb.B as f32 / 255.0,
    }
}

fn cb_denorm(rgb: CbRgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: (rgb.r * 255.0 + 0.5) as u8,
        G: (rgb.g * 255.0 + 0.5) as u8,
        B: (rgb.b * 255.0 + 0.5) as u8,
    }
}

fn cb_apply_gamma_rgb(rgb: CbRgb) -> CbRgb {
    CbRgb {
        r: if rgb.r > 0.00313080495356037151702786377709 {
            (1.055 * (rgb.r as f64).powf(0.4166666666) - 0.055) as f32
        } else {
            rgb.r * 12.92
        },
        g: if rgb.g > 0.00313080495356037151702786377709 {
            (1.055 * (rgb.g as f64).powf(0.4166666666) - 0.055) as f32
        } else {
            rgb.g * 12.92
        },
        b: if rgb.b > 0.00313080495356037151702786377709 {
            (1.055 * (rgb.b as f64).powf(0.4166666666) - 0.055) as f32
        } else {
            rgb.b * 12.92
        },
    }
}

fn tritanopia_transform(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let r = *red;
    let g = *green;
    let b = *blue;
    *red = r + 0.12739886310880_f32 * g - 0.12739886341072_f32 * b;
    *green = -4.486e-11_f32 * r + 0.87390929928361_f32 * g + 0.12609070101523_f32 * b;
    *blue = 3.1113e-10_f32 * r + 0.87390929725848_f32 * g + 0.12609070067115_f32 * b;
}

#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(rgb: cb_rgb_255) -> cb_rgb_255 {
    let mut rgb_norm = cb_remove_gamma_rgb(cb_norm(&rgb));
    tritanopia_transform(&mut rgb_norm.r, &mut rgb_norm.g, &mut rgb_norm.b);
    cb_denorm(cb_apply_gamma_rgb(rgb_norm))
}
