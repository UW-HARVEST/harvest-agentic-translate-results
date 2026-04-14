#[repr(C)]
#[derive(Copy, Clone)]
pub struct cb_rgb_255 {
    pub R: u8,
    pub G: u8,
    pub B: u8,
}

#[derive(Copy, Clone)]
struct CbRgb {
    R: f32,
    G: f32,
    B: f32,
}

fn cb_remove_gamma_rgb(rgb: CbRgb) -> CbRgb {
    CbRgb {
        R: if rgb.R > 0.04045 {
            ((rgb.R + 0.055) / 1.055).powf(2.4)
        } else {
            rgb.R / 12.92
        },
        G: if rgb.G > 0.04045 {
            ((rgb.G + 0.055) / 1.055).powf(2.4)
        } else {
            rgb.G / 12.92
        },
        B: if rgb.B > 0.04045 {
            ((rgb.B + 0.055) / 1.055).powf(2.4)
        } else {
            rgb.B / 12.92
        },
    }
}

fn cb_norm(rgb: cb_rgb_255) -> CbRgb {
    CbRgb {
        R: rgb.R as f32 / 255.0,
        G: rgb.G as f32 / 255.0,
        B: rgb.B as f32 / 255.0,
    }
}

fn cb_denorm(rgb: CbRgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: (rgb.R * 255.0 + 0.5) as u8,
        G: (rgb.G * 255.0 + 0.5) as u8,
        B: (rgb.B * 255.0 + 0.5) as u8,
    }
}

fn cb_apply_gamma_rgb(rgb: CbRgb) -> CbRgb {
    CbRgb {
        R: if rgb.R > 0.0031308049535603715 {
            1.055 * rgb.R.powf(0.4166666666) - 0.055
        } else {
            rgb.R * 12.92
        },
        G: if rgb.G > 0.0031308049535603715 {
            1.055 * rgb.G.powf(0.4166666666) - 0.055
        } else {
            rgb.G * 12.92
        },
        B: if rgb.B > 0.0031308049535603715 {
            1.055 * rgb.B.powf(0.4166666666) - 0.055
        } else {
            rgb.B * 12.92
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
    let mut rgb_norm = cb_remove_gamma_rgb(cb_norm(rgb));
    tritanopia_transform(&mut rgb_norm.R, &mut rgb_norm.G, &mut rgb_norm.B);
    cb_denorm(cb_apply_gamma_rgb(rgb_norm))
}
