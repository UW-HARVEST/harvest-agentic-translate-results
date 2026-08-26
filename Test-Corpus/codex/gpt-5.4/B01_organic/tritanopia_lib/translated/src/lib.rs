#![allow(non_snake_case)]

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cb_rgb_255 {
    pub R: u8,
    pub G: u8,
    pub B: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct cb_rgb {
    R: f32,
    G: f32,
    B: f32,
}

#[link(name = "m")]
unsafe extern "C" {
    fn pow(x: f64, y: f64) -> f64;
}

fn cb_remove_gamma_rgb(rgb: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: if (rgb.R as f64) > 0.04045_f64 {
            unsafe { pow(((rgb.R as f64) + 0.055_f64) / 1.055_f64, 2.4_f64) as f32 }
        } else {
            ((rgb.R as f64) / 12.92_f64) as f32
        },
        G: if (rgb.G as f64) > 0.04045_f64 {
            unsafe { pow(((rgb.G as f64) + 0.055_f64) / 1.055_f64, 2.4_f64) as f32 }
        } else {
            ((rgb.G as f64) / 12.92_f64) as f32
        },
        B: if (rgb.B as f64) > 0.04045_f64 {
            unsafe { pow(((rgb.B as f64) + 0.055_f64) / 1.055_f64, 2.4_f64) as f32 }
        } else {
            ((rgb.B as f64) / 12.92_f64) as f32
        },
    }
}

fn cb_norm(rgb: cb_rgb_255) -> cb_rgb {
    cb_rgb {
        R: (rgb.R as f32) / 255.0_f32,
        G: (rgb.G as f32) / 255.0_f32,
        B: (rgb.B as f32) / 255.0_f32,
    }
}

fn cb_denorm(rgb: cb_rgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: c_float_to_u8(rgb.R * 255.0_f32 + 0.5_f32),
        G: c_float_to_u8(rgb.G * 255.0_f32 + 0.5_f32),
        B: c_float_to_u8(rgb.B * 255.0_f32 + 0.5_f32),
    }
}

fn c_float_to_u8(value: f32) -> u8 {
    value.trunc() as i64 as u8
}

fn cb_apply_gamma_rgb(rgb: cb_rgb) -> cb_rgb {
    cb_rgb {
        R: if (rgb.R as f64) > 0.00313080495356037151702786377709_f64 {
            unsafe { (1.055_f64 * pow(rgb.R as f64, 0.4166666666_f64) - 0.055_f64) as f32 }
        } else {
            ((rgb.R as f64) * 12.92_f64) as f32
        },
        G: if (rgb.G as f64) > 0.00313080495356037151702786377709_f64 {
            unsafe { (1.055_f64 * pow(rgb.G as f64, 0.4166666666_f64) - 0.055_f64) as f32 }
        } else {
            ((rgb.G as f64) * 12.92_f64) as f32
        },
        B: if (rgb.B as f64) > 0.00313080495356037151702786377709_f64 {
            unsafe { (1.055_f64 * pow(rgb.B as f64, 0.4166666666_f64) - 0.055_f64) as f32 }
        } else {
            ((rgb.B as f64) * 12.92_f64) as f32
        },
    }
}

fn tritanopia_impl(red: &mut f32, green: &mut f32, blue: &mut f32) {
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
    tritanopia_impl(&mut rgb_norm.R, &mut rgb_norm.G, &mut rgb_norm.B);
    cb_denorm(cb_apply_gamma_rgb(rgb_norm))
}
