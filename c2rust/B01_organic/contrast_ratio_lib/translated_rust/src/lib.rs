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
unsafe extern "C" fn cbLuminance(
    mut R: ::core::ffi::c_float,
    mut G: ::core::ffi::c_float,
    mut B: ::core::ffi::c_float,
) -> ::core::ffi::c_float {
    R = (if R as ::core::ffi::c_double > 0.04045f64 {
        pow((R as ::core::ffi::c_double + 0.055f64) / 1.055f64, 2.4f64)
    } else {
        R as ::core::ffi::c_double / 12.92f64
    }) as ::core::ffi::c_float;
    G = (if G as ::core::ffi::c_double > 0.04045f64 {
        pow((G as ::core::ffi::c_double + 0.055f64) / 1.055f64, 2.4f64)
    } else {
        G as ::core::ffi::c_double / 12.92f64
    }) as ::core::ffi::c_float;
    B = (if B as ::core::ffi::c_double > 0.04045f64 {
        pow((B as ::core::ffi::c_double + 0.055f64) / 1.055f64, 2.4f64)
    } else {
        B as ::core::ffi::c_double / 12.92f64
    }) as ::core::ffi::c_float;
    let mut Result: ::core::ffi::c_float = 0.2126f32 * R + 0.7152f32 * G + 0.0722f32 * B;
    return Result;
}
unsafe extern "C" fn cbContrastRatio(
    mut RA: ::core::ffi::c_float,
    mut GA: ::core::ffi::c_float,
    mut BA: ::core::ffi::c_float,
    mut RB: ::core::ffi::c_float,
    mut GB: ::core::ffi::c_float,
    mut BB: ::core::ffi::c_float,
) -> ::core::ffi::c_float {
    let mut LumA: ::core::ffi::c_float = cbLuminance(RA, GA, BA);
    let mut LumB: ::core::ffi::c_float = cbLuminance(RB, GB, BB);
    let mut High: ::core::ffi::c_float = LumA;
    let mut Low: ::core::ffi::c_float = LumB;
    if High < Low {
        High = LumB;
        Low = LumA;
    }
    let mut Ratio: ::core::ffi::c_float = High / Low;
    return Ratio;
}
#[no_mangle]
pub unsafe extern "C" fn contrast_ratio(
    mut A: cb_rgb_255,
    mut B: cb_rgb_255,
) -> ::core::ffi::c_float {
    return cbContrastRatio(
        A.R as ::core::ffi::c_float / 255.0f32,
        A.G as ::core::ffi::c_float / 255.0f32,
        A.B as ::core::ffi::c_float / 255.0f32,
        B.R as ::core::ffi::c_float / 255.0f32,
        B.G as ::core::ffi::c_float / 255.0f32,
        B.B as ::core::ffi::c_float / 255.0f32,
    );
}
