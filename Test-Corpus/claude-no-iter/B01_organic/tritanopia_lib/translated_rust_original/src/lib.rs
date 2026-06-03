#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cb_rgb_255 {
    pub R: u8,
    pub G: u8,
    pub B: u8,
}

#[derive(Copy, Clone)]
struct cb_rgb {
    R: f32,
    G: f32,
    B: f32,
}

fn cb_remove_gamma_rgb(rgb: cb_rgb) -> cb_rgb {
    // In C, the float operands are promoted to double when combined with
    // double constants (0.04045, 0.055, 1.055, 2.4, 12.92), and pow() is the
    // double-precision function. The result is then cast back to float.
    let r_r = if (rgb.R as f64) > 0.04045 {
        (((rgb.R as f64) + 0.055) / 1.055).powf(2.4) as f32
    } else {
        ((rgb.R as f64) / 12.92) as f32
    };
    let r_g = if (rgb.G as f64) > 0.04045 {
        (((rgb.G as f64) + 0.055) / 1.055).powf(2.4) as f32
    } else {
        ((rgb.G as f64) / 12.92) as f32
    };
    let r_b = if (rgb.B as f64) > 0.04045 {
        (((rgb.B as f64) + 0.055) / 1.055).powf(2.4) as f32
    } else {
        ((rgb.B as f64) / 12.92) as f32
    };
    cb_rgb {
        R: r_r,
        G: r_g,
        B: r_b,
    }
}

fn cb_norm(rgb: cb_rgb_255) -> cb_rgb {
    // In C: ((float)(RGB.R) / 255.f) — single-precision float division.
    cb_rgb {
        R: (rgb.R as f32) / 255.0_f32,
        G: (rgb.G as f32) / 255.0_f32,
        B: (rgb.B as f32) / 255.0_f32,
    }
}

fn cb_denorm(rgb: cb_rgb) -> cb_rgb_255 {
    // In C: ((unsigned char)((RGB.R) * 255.f + 0.5f)) — single-precision
    // float arithmetic, then truncating cast to unsigned char.
    cb_rgb_255 {
        R: (rgb.R * 255.0_f32 + 0.5_f32) as u8,
        G: (rgb.G * 255.0_f32 + 0.5_f32) as u8,
        B: (rgb.B * 255.0_f32 + 0.5_f32) as u8,
    }
}

fn cb_apply_gamma_rgb(rgb: cb_rgb) -> cb_rgb {
    // In C, the float operands are promoted to double when combined with
    // double constants, and pow() is the double-precision function. The
    // result is then cast back to float.
    const THRESHOLD: f64 = 0.00313080495356037151702786377709;
    let r_r = if (rgb.R as f64) > THRESHOLD {
        (1.055 * (rgb.R as f64).powf(0.4166666666) - 0.055) as f32
    } else {
        ((rgb.R as f64) * 12.92) as f32
    };
    let r_g = if (rgb.G as f64) > THRESHOLD {
        (1.055 * (rgb.G as f64).powf(0.4166666666) - 0.055) as f32
    } else {
        ((rgb.G as f64) * 12.92) as f32
    };
    let r_b = if (rgb.B as f64) > THRESHOLD {
        (1.055 * (rgb.B as f64).powf(0.4166666666) - 0.055) as f32
    } else {
        ((rgb.B as f64) * 12.92) as f32
    };
    cb_rgb {
        R: r_r,
        G: r_g,
        B: r_b,
    }
}

fn tritanopia_transform(red: &mut f32, green: &mut f32, blue: &mut f32) {
    // All literals in C have the `f` suffix => single-precision float math.
    let r = *red;
    let g = *green;
    let b = *blue;
    *red = r + 0.12739886310880_f32 * g - 0.12739886341072_f32 * b;
    *green = -4.486E-11_f32 * r + 0.87390929928361_f32 * g + 0.12609070101523_f32 * b;
    *blue = 3.1113E-10_f32 * r + 0.87390929725848_f32 * g + 0.12609070067115_f32 * b;
}

#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(RGB: cb_rgb_255) -> cb_rgb_255 {
    let mut rgb_norm = cb_remove_gamma_rgb(cb_norm(RGB));
    tritanopia_transform(&mut rgb_norm.R, &mut rgb_norm.G, &mut rgb_norm.B);
    cb_denorm(cb_apply_gamma_rgb(rgb_norm))
}
