/* png.c lines 2726..2929 */

/* png_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_fixed(
    png_ptr: png_const_structrp,
    fp: f64,
    text: png_const_charp,
) -> png_fixed_point {
    let r: f64 = floor(100000.0 * fp + 0.5);

    if r > 2147483647. || r < -2147483648. {
        png_fixed_error(png_ptr, text);
    }

    r as png_fixed_point
}

/* png_fixed_ITU */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_fixed_ITU(
    png_ptr: png_const_structrp,
    fp: f64,
    text: png_const_charp,
) -> png_uint_32 {
    let r: f64 = floor(10000.0 * fp + 0.5);

    if r > 2147483647. || r < 0.0 {
        png_fixed_error(png_ptr, text);
    }

    r as png_uint_32
}

/* muldiv functions */
/* This API takes signed arguments and rounds the result to the nearest
 * integer (or, for a fixed point number - the standard argument - to
 * the nearest .00001).  Overflow and divide by zero are signalled in
 * the result, a boolean - true on success, false on overflow.
 */
/* png_muldiv */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_muldiv(
    res: png_fixed_point_p,
    a: png_fixed_point,
    times: png_int_32,
    divisor: png_int_32,
) -> c_int {
    /* Return a * times / divisor, rounded. */
    if divisor != 0 {
        if a == 0 || times == 0 {
            *res = 0;
            return 1;
        } else {
            let mut r: f64 = a as f64;
            r *= times as f64;
            r /= divisor as f64;
            r = floor(r + 0.5);

            /* A png_fixed_point is a 32-bit integer. */
            if r <= 2147483647. && r >= -2147483648. {
                *res = r as png_fixed_point;
                return 1;
            }
        }
    }

    0
}

/* Calculate a reciprocal, return 0 on div-by-zero or overflow. */
/* png_reciprocal */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reciprocal(a: png_fixed_point) -> png_fixed_point {
    let r: f64 = floor(1E10 / (a as f64) + 0.5);

    if r <= 2147483647. && r >= -2147483648. {
        return r as png_fixed_point;
    }

    0 /* error/overflow */
}

/* This is the shared test on whether a gamma value is 'significant' - whether
 * it is worth doing gamma correction.
 */
/* png_gamma_significant */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_significant(gamma_val: png_fixed_point) -> c_int {
    /* sRGB:       1/2.2 == 0.4545(45)
     * AdobeRGB:   1/(2+51/256) ~= 0.45471 5dp
     *
     * So the correction from AdobeRGB to sRGB (output) is:
     *
     *    2.2/(2+51/256) == 1.00035524
     *
     * I.e. vanishingly small (<4E-4) but still detectable in 16-bit linear (+/-
     * 23).  Note that the Adobe choice seems to be something intended to give an
     * exact number with 8 binary fractional digits - it is the closest to 2.2
     * that is possible a base 2 .8p representation.
     */
    (gamma_val < PNG_FP_1 - PNG_GAMMA_THRESHOLD_FIXED
        || gamma_val > PNG_FP_1 + PNG_GAMMA_THRESHOLD_FIXED) as c_int
}
