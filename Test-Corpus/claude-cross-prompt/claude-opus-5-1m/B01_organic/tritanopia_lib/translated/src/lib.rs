// Translation of c_src/src/lib.c to Rust.
// Preserves the exact computational behavior including float precision and
// quirks (e.g. casts back to f32 after f64 pow operations).

#[derive(Copy, Clone, Debug)]
pub struct CbRgb255 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Copy, Clone, Debug)]
struct CbRgb {
    r: f32,
    g: f32,
    b: f32,
}

fn cb_remove_gamma_rgb(rgb: CbRgb) -> CbRgb {
    let r = if (rgb.r as f64) > 0.04045 {
        f64::powf(((rgb.r as f64) + 0.055) / 1.055, 2.4) as f32
    } else {
        ((rgb.r as f64) / 12.92) as f32
    };
    let g = if (rgb.g as f64) > 0.04045 {
        f64::powf(((rgb.g as f64) + 0.055) / 1.055, 2.4) as f32
    } else {
        ((rgb.g as f64) / 12.92) as f32
    };
    let b = if (rgb.b as f64) > 0.04045 {
        f64::powf(((rgb.b as f64) + 0.055) / 1.055, 2.4) as f32
    } else {
        ((rgb.b as f64) / 12.92) as f32
    };
    CbRgb { r, g, b }
}

fn cb_norm(rgb: CbRgb255) -> CbRgb {
    CbRgb {
        r: (rgb.r as f32) / 255.0_f32,
        g: (rgb.g as f32) / 255.0_f32,
        b: (rgb.b as f32) / 255.0_f32,
    }
}

fn cb_denorm(rgb: CbRgb) -> CbRgb255 {
    // Match C cast-to-unsigned-char behavior: truncate toward zero.
    // Values are expected in [0, 1] roughly; we mimic the C unsigned char cast.
    let r = (rgb.r * 255.0_f32 + 0.5_f32) as u8;
    let g = (rgb.g * 255.0_f32 + 0.5_f32) as u8;
    let b = (rgb.b * 255.0_f32 + 0.5_f32) as u8;
    CbRgb255 { r, g, b }
}

fn cb_apply_gamma_rgb(rgb: CbRgb) -> CbRgb {
    let threshold = 0.00313080495356037151702786377709_f64;
    let exp = 0.4166666666_f64;

    let r = if (rgb.r as f64) > threshold {
        (1.055 * f64::powf(rgb.r as f64, exp) - 0.055) as f32
    } else {
        ((rgb.r as f64) * 12.92) as f32
    };
    let g = if (rgb.g as f64) > threshold {
        (1.055 * f64::powf(rgb.g as f64, exp) - 0.055) as f32
    } else {
        ((rgb.g as f64) * 12.92) as f32
    };
    let b = if (rgb.b as f64) > threshold {
        (1.055 * f64::powf(rgb.b as f64, exp) - 0.055) as f32
    } else {
        ((rgb.b as f64) * 12.92) as f32
    };
    CbRgb { r, g, b }
}

fn tritanopia_inner(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let r = *red;
    let g = *green;
    let b = *blue;
    *red = r + 0.12739886310880_f32 * g - 0.12739886341072_f32 * b;
    *green = -4.486E-11_f32 * r + 0.87390929928361_f32 * g + 0.12609070101523_f32 * b;
    *blue = 3.1113E-10_f32 * r + 0.87390929725848_f32 * g + 0.12609070067115_f32 * b;
}

pub fn tritanopia(rgb: CbRgb255) -> CbRgb255 {
    let mut rgb_norm = cb_remove_gamma_rgb(cb_norm(rgb));
    tritanopia_inner(&mut rgb_norm.r, &mut rgb_norm.g, &mut rgb_norm.b);
    cb_denorm(cb_apply_gamma_rgb(rgb_norm))
}
