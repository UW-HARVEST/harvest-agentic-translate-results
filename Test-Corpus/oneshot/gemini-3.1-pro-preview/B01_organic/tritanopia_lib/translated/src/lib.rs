#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct cb_rgb_255 {
    pub R: u8,
    pub G: u8,
    pub B: u8,
}

#[derive(Debug, Copy, Clone)]
struct cb_rgb {
    R: f32,
    G: f32,
    B: f32,
}

fn cb_remove_gamma_rgb(rgb: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: if rgb.R > 0.04045_f32 {
            ((rgb.R + 0.055_f32) / 1.055_f32).powf(2.4_f32)
        } else {
            rgb.R / 12.92_f32
        },
        G: if rgb.G > 0.04045_f32 {
            ((rgb.G + 0.055_f32) / 1.055_f32).powf(2.4_f32)
        } else {
            rgb.G / 12.92_f32
        },
        B: if rgb.B > 0.04045_f32 {
            ((rgb.B + 0.055_f32) / 1.055_f32).powf(2.4_f32)
        } else {
            rgb.B / 12.92_f32
        },
    }
}

fn cb_norm(rgb: cb_rgb_255) -> cb_rgb {
    cb_rgb {
        R: rgb.R as f32 / 255.0_f32,
        G: rgb.G as f32 / 255.0_f32,
        B: rgb.B as f32 / 255.0_f32,
    }
}

fn cb_denorm(rgb: cb_rgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: (rgb.R * 255.0_f32 + 0.5_f32) as u8,
        G: (rgb.G * 255.0_f32 + 0.5_f32) as u8,
        B: (rgb.B * 255.0_f32 + 0.5_f32) as u8,
    }
}

fn cb_apply_gamma_rgb(rgb: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: if rgb.R > 0.00313080495356037151702786377709_f32 {
            1.055_f32 * rgb.R.powf(0.4166666666_f32) - 0.055_f32
        } else {
            rgb.R * 12.92_f32
        },
        G: if rgb.G > 0.00313080495356037151702786377709_f32 {
            1.055_f32 * rgb.G.powf(0.4166666666_f32) - 0.055_f32
        } else {
            rgb.G * 12.92_f32
        },
        B: if rgb.B > 0.00313080495356037151702786377709_f32 {
            1.055_f32 * rgb.B.powf(0.4166666666_f32) - 0.055_f32
        } else {
            rgb.B * 12.92_f32
        },
    }
}

fn tritanopia_internal(red: &mut f32, green: &mut f32, blue: &mut f32) {
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
    tritanopia_internal(&mut rgb_norm.R, &mut rgb_norm.G, &mut rgb_norm.B);
    cb_denorm(cb_apply_gamma_rgb(rgb_norm))
}
