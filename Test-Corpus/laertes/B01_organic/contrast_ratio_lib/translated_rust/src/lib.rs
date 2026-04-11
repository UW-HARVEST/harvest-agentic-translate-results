extern "C" {
    fn pow(__x: libc::c_double, __y: libc::c_double) -> libc::c_double;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cb_rgb_255 {
    pub R: libc::c_uchar,
    pub G: libc::c_uchar,
    pub B: libc::c_uchar,
}
unsafe extern "C" fn cbLuminance(
    mut R: libc::c_float,
    mut G: libc::c_float,
    mut B: libc::c_float,
) -> libc::c_float {
    R = (if R as libc::c_double > 0.04045f64 {
        pow((R as libc::c_double + 0.055f64) / 1.055f64, 2.4f64)
    } else {
        R as libc::c_double / 12.92f64
    }) as libc::c_float;
    G = (if G as libc::c_double > 0.04045f64 {
        pow((G as libc::c_double + 0.055f64) / 1.055f64, 2.4f64)
    } else {
        G as libc::c_double / 12.92f64
    }) as libc::c_float;
    B = (if B as libc::c_double > 0.04045f64 {
        pow((B as libc::c_double + 0.055f64) / 1.055f64, 2.4f64)
    } else {
        B as libc::c_double / 12.92f64
    }) as libc::c_float;
    let mut Result: libc::c_float = 0.2126f32 * R + 0.7152f32 * G + 0.0722f32 * B;
    return Result;
}
unsafe extern "C" fn cbContrastRatio(
    mut RA: libc::c_float,
    mut GA: libc::c_float,
    mut BA: libc::c_float,
    mut RB: libc::c_float,
    mut GB: libc::c_float,
    mut BB: libc::c_float,
) -> libc::c_float {
    let mut LumA: libc::c_float = cbLuminance(RA, GA, BA);
    let mut LumB: libc::c_float = cbLuminance(RB, GB, BB);
    let mut High: libc::c_float = LumA;
    let mut Low: libc::c_float = LumB;
    if High < Low {
        High = LumB;
        Low = LumA;
    }
    let mut Ratio: libc::c_float = High / Low;
    return Ratio;
}
#[no_mangle]
pub unsafe extern "C" fn contrast_ratio(
    mut A: cb_rgb_255,
    mut B: cb_rgb_255,
) -> libc::c_float {
    return cbContrastRatio(
        A.R as libc::c_float / 255.0f32,
        A.G as libc::c_float / 255.0f32,
        A.B as libc::c_float / 255.0f32,
        B.R as libc::c_float / 255.0f32,
        B.G as libc::c_float / 255.0f32,
        B.B as libc::c_float / 255.0f32,
    );
}
