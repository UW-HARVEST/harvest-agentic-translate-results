#[repr(C)]
#[derive(Clone, Copy)]
pub struct CbRgb255 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy)]
struct CbRgb {
    r: f32,
    g: f32,
    b: f32,
}

#[link(name = "m")]
unsafe extern "C" {
    fn pow(base: f64, exponent: f64) -> f64;
}

fn remove_gamma(value: f32) -> f32 {
    if value > 0.04045 {
        unsafe { pow((f64::from(value) + 0.055) / 1.055, 2.4) as f32 }
    } else {
        (f64::from(value) / 12.92) as f32
    }
}

fn apply_gamma(value: f32) -> f32 {
    if f64::from(value) > 0.00313080495356037151702786377709 {
        (1.055 * unsafe { pow(f64::from(value), 0.4166666666) } - 0.055) as f32
    } else {
        (f64::from(value) * 12.92) as f32
    }
}

fn denormalize(value: f32) -> u8 {
    let scaled = value * 255.0;
    ((scaled + 0.5) as i32) as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(rgb: CbRgb255) -> CbRgb255 {
    let rgb = CbRgb {
        r: remove_gamma(f32::from(rgb.r) / 255.0),
        g: remove_gamma(f32::from(rgb.g) / 255.0),
        b: remove_gamma(f32::from(rgb.b) / 255.0),
    };

    let r = rgb.r;
    let g = rgb.g;
    let b = rgb.b;
    let transformed = CbRgb {
        r: r + 0.12739886310880_f32 * g - 0.12739886341072_f32 * b,
        g: -4.486E-11_f32 * r + 0.87390929928361_f32 * g + 0.12609070101523_f32 * b,
        b: 3.1113E-10_f32 * r + 0.87390929725848_f32 * g + 0.12609070067115_f32 * b,
    };

    CbRgb255 {
        r: denormalize(apply_gamma(transformed.r)),
        g: denormalize(apply_gamma(transformed.g)),
        b: denormalize(apply_gamma(transformed.b)),
    }
}
