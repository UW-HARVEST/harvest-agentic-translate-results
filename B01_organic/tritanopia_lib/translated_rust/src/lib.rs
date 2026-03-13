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

fn cb_remove_gamma_component(v: f32) -> f32 {
    if v > 0.04045 {
        ((v as f64 + 0.055) / 1.055).powf(2.4) as f32
    } else {
        v / 12.92
    }
}

fn cb_remove_gamma_rgb(rgb: CbRgb) -> CbRgb {
    CbRgb {
        r: cb_remove_gamma_component(rgb.r),
        g: cb_remove_gamma_component(rgb.g),
        b: cb_remove_gamma_component(rgb.b),
    }
}

fn cb_norm(rgb: cb_rgb_255) -> CbRgb {
    CbRgb {
        r: rgb.R as f32 / 255.0f32,
        g: rgb.G as f32 / 255.0f32,
        b: rgb.B as f32 / 255.0f32,
    }
}

fn cb_denorm(rgb: CbRgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: (rgb.r * 255.0f32 + 0.5f32) as u8,
        G: (rgb.g * 255.0f32 + 0.5f32) as u8,
        B: (rgb.b * 255.0f32 + 0.5f32) as u8,
    }
}

fn cb_apply_gamma_component(v: f32) -> f32 {
    if v > 0.00313080495356037151702786377709 {
        (1.055 * (v as f64).powf(0.4166666666) - 0.055) as f32
    } else {
        v * 12.92
    }
}

fn cb_apply_gamma_rgb(rgb: CbRgb) -> CbRgb {
    CbRgb {
        r: cb_apply_gamma_component(rgb.r),
        g: cb_apply_gamma_component(rgb.g),
        b: cb_apply_gamma_component(rgb.b),
    }
}

fn tritanopia_transform(r: &mut f32, g: &mut f32, b: &mut f32) {
    let (ri, gi, bi) = (*r, *g, *b);
    *r = ri + 0.12739886310880f32 * gi - 0.12739886341072f32 * bi;
    *g = -4.486E-11f32 * ri + 0.87390929928361f32 * gi + 0.12609070101523f32 * bi;
    *b = 3.1113E-10f32 * ri + 0.87390929725848f32 * gi + 0.12609070067115f32 * bi;
}

#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(rgb: cb_rgb_255) -> cb_rgb_255 {
    let mut rgb_norm = cb_remove_gamma_rgb(cb_norm(rgb));
    tritanopia_transform(&mut rgb_norm.r, &mut rgb_norm.g, &mut rgb_norm.b);
    cb_denorm(cb_apply_gamma_rgb(rgb_norm))
}
