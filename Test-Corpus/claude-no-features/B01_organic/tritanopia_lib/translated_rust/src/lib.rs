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

#[inline]
fn remove_gamma_component(c: f32) -> f32 {
    // C: (float)(c > 0.04045 ? pow((c + 0.055) / 1.055, 2.4) : c / 12.92)
    // The comparison and arithmetic are done in double precision in C because
    // the literals are doubles, so c gets promoted to double.
    let cd = c as f64;
    if cd > 0.04045_f64 {
        (((cd + 0.055_f64) / 1.055_f64).powf(2.4_f64)) as f32
    } else {
        (cd / 12.92_f64) as f32
    }
}

fn cbRemoveGammaRGB(rgb: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: remove_gamma_component(rgb.R),
        G: remove_gamma_component(rgb.G),
        B: remove_gamma_component(rgb.B),
    }
}

fn cbNorm(rgb: cb_rgb_255) -> cb_rgb {
    cb_rgb {
        R: (rgb.R as f32) / 255.0_f32,
        G: (rgb.G as f32) / 255.0_f32,
        B: (rgb.B as f32) / 255.0_f32,
    }
}

#[inline]
fn float_to_uchar_c(f: f32) -> u8 {
    // Mimic C's `(unsigned char)floatvalue` on x86_64 GCC: convert to int via
    // truncation toward zero, then narrow to unsigned char (low 8 bits).
    // For values within i32 range (all expected values here), Rust's `as i32`
    // performs the same truncation; `as u8` then narrows to low 8 bits.
    (f as i32) as u8
}

fn cbDenorm(rgb: cb_rgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: float_to_uchar_c(rgb.R * 255.0_f32 + 0.5_f32),
        G: float_to_uchar_c(rgb.G * 255.0_f32 + 0.5_f32),
        B: float_to_uchar_c(rgb.B * 255.0_f32 + 0.5_f32),
    }
}

#[inline]
fn apply_gamma_component(c: f32) -> f32 {
    // C: (float)(c > 0.00313080... ? 1.055 * pow(c, 0.4166666666) - 0.055 : c * 12.92)
    let cd = c as f64;
    if cd > 0.00313080495356037151702786377709_f64 {
        (1.055_f64 * cd.powf(0.4166666666_f64) - 0.055_f64) as f32
    } else {
        (cd * 12.92_f64) as f32
    }
}

fn cbApplyGammaRGB(rgb: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: apply_gamma_component(rgb.R),
        G: apply_gamma_component(rgb.G),
        B: apply_gamma_component(rgb.B),
    }
}

fn Tritanopia(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let R = *red;
    let G = *green;
    let B = *blue;
    *red = R + 0.12739886310880_f32 * G - 0.12739886341072_f32 * B;
    *green = -4.486E-11_f32 * R + 0.87390929928361_f32 * G + 0.12609070101523_f32 * B;
    *blue = 3.1113E-10_f32 * R + 0.87390929725848_f32 * G + 0.12609070067115_f32 * B;
}

#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(RGB: cb_rgb_255) -> cb_rgb_255 {
    let mut rgb_norm = cbRemoveGammaRGB(cbNorm(RGB));
    Tritanopia(&mut rgb_norm.R, &mut rgb_norm.G, &mut rgb_norm.B);
    cbDenorm(cbApplyGammaRGB(rgb_norm))
}
