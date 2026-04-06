extern "C" {
    fn pow(__x: ::core::ffi::c_double, __y: ::core::ffi::c_double) -> ::core::ffi::c_double;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cb_rgb_255 {
    pub R: ::core::ffi::c_uchar,
    pub G: ::core::ffi::c_uchar,
    pub B: ::core::ffi::c_uchar,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cb_rgb {
    pub R: ::core::ffi::c_float,
    pub G: ::core::ffi::c_float,
    pub B: ::core::ffi::c_float,
}
unsafe extern "C" fn cbRemoveGammaRGB(mut RGB: cb_rgb) -> cb_rgb {
    let mut Result: cb_rgb = cb_rgb {
        R: (if RGB.R as ::core::ffi::c_double > 0.04045f64 {
            pow(
                (RGB.R as ::core::ffi::c_double + 0.055f64) / 1.055f64,
                2.4f64,
            )
        } else {
            RGB.R as ::core::ffi::c_double / 12.92f64
        }) as ::core::ffi::c_float,
        G: (if RGB.G as ::core::ffi::c_double > 0.04045f64 {
            pow(
                (RGB.G as ::core::ffi::c_double + 0.055f64) / 1.055f64,
                2.4f64,
            )
        } else {
            RGB.G as ::core::ffi::c_double / 12.92f64
        }) as ::core::ffi::c_float,
        B: (if RGB.B as ::core::ffi::c_double > 0.04045f64 {
            pow(
                (RGB.B as ::core::ffi::c_double + 0.055f64) / 1.055f64,
                2.4f64,
            )
        } else {
            RGB.B as ::core::ffi::c_double / 12.92f64
        }) as ::core::ffi::c_float,
    };
    return Result;
}
unsafe extern "C" fn cbNorm(mut RGB: cb_rgb_255) -> cb_rgb {
    let mut Result: cb_rgb = cb_rgb {
        R: RGB.R as ::core::ffi::c_float / 255.0f32,
        G: RGB.G as ::core::ffi::c_float / 255.0f32,
        B: RGB.B as ::core::ffi::c_float / 255.0f32,
    };
    return Result;
}
unsafe extern "C" fn cbDenorm(mut RGB: cb_rgb) -> cb_rgb_255 {
    let mut Result: cb_rgb_255 = cb_rgb_255 {
        R: (RGB.R * 255.0f32 + 0.5f32) as ::core::ffi::c_uchar,
        G: (RGB.G * 255.0f32 + 0.5f32) as ::core::ffi::c_uchar,
        B: (RGB.B * 255.0f32 + 0.5f32) as ::core::ffi::c_uchar,
    };
    return Result;
}
unsafe extern "C" fn cbApplyGammaRGB(mut RGB: cb_rgb) -> cb_rgb {
    let mut Result: cb_rgb = cb_rgb {
        R: (if RGB.R as ::core::ffi::c_double > 0.00313080495356037151702786377709f64 {
            1.055f64 * pow(RGB.R as ::core::ffi::c_double, 0.4166666666f64) - 0.055f64
        } else {
            RGB.R as ::core::ffi::c_double * 12.92f64
        }) as ::core::ffi::c_float,
        G: (if RGB.G as ::core::ffi::c_double > 0.00313080495356037151702786377709f64 {
            1.055f64 * pow(RGB.G as ::core::ffi::c_double, 0.4166666666f64) - 0.055f64
        } else {
            RGB.G as ::core::ffi::c_double * 12.92f64
        }) as ::core::ffi::c_float,
        B: (if RGB.B as ::core::ffi::c_double > 0.00313080495356037151702786377709f64 {
            1.055f64 * pow(RGB.B as ::core::ffi::c_double, 0.4166666666f64) - 0.055f64
        } else {
            RGB.B as ::core::ffi::c_double * 12.92f64
        }) as ::core::ffi::c_float,
    };
    return Result;
}
unsafe extern "C" fn Tritanopia(
    mut Red: *mut ::core::ffi::c_float,
    mut Green: *mut ::core::ffi::c_float,
    mut Blue: *mut ::core::ffi::c_float,
) {
    let mut R: ::core::ffi::c_float = *Red;
    let mut G: ::core::ffi::c_float = *Green;
    let mut B: ::core::ffi::c_float = *Blue;
    *Red = R + 0.12739886310880f32 * G - 0.12739886341072f32 * B;
    *Green = -4.486E-11f32 * R + 0.87390929928361f32 * G + 0.12609070101523f32 * B;
    *Blue = 3.1113E-10f32 * R + 0.87390929725848f32 * G + 0.12609070067115f32 * B;
}
#[no_mangle]
pub unsafe extern "C" fn tritanopia(mut RGB: cb_rgb_255) -> cb_rgb_255 {
    let mut RGBNorm: cb_rgb = cbRemoveGammaRGB(cbNorm(RGB));
    Tritanopia(&raw mut RGBNorm.R, &raw mut RGBNorm.G, &raw mut RGBNorm.B);
    let mut Result: cb_rgb_255 = cbDenorm(cbApplyGammaRGB(RGBNorm));
    return Result;
}
