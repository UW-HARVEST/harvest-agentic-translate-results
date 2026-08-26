// png.c (part 2): colorspace / cHRM fixed point arithmetic and ICC profile
// checking.

use crate::*;

unsafe fn png_fp_add(addend0: png_int_32, addend1: png_int_32, error: *mut c_int) -> png_int_32 {
    /* Safely add two fixed point values setting an error flag and returning 0.5
     * on overflow.
     * IMPLEMENTATION NOTE: ANSI requires signed overflow not to occur, therefore
     * relying on addition of two positive values producing a negative one is not
     * safe.
     */
    if addend0 > 0 {
        if 0x7fffffff - addend0 >= addend1 {
            return addend0.wrapping_add(addend1);
        }
    } else if addend0 < 0 {
        if -0x7fffffff - addend0 <= addend1 {
            return addend0.wrapping_add(addend1);
        }
    } else {
        return addend1;
    }

    *error = 1;
    PNG_FP_1 / 2
}

unsafe fn png_fp_sub(addend0: png_int_32, addend1: png_int_32, error: *mut c_int) -> png_int_32 {
    /* As above but calculate addend0-addend1. */
    if addend1 > 0 {
        if -0x7fffffff + addend1 <= addend0 {
            return addend0.wrapping_sub(addend1);
        }
    } else if addend1 < 0 {
        if 0x7fffffff + addend1 >= addend0 {
            return addend0.wrapping_sub(addend1);
        }
    } else {
        return addend0;
    }

    *error = 1;
    PNG_FP_1 / 2
}

unsafe fn png_safe_add(
    addend0_and_result: *mut png_int_32,
    addend1: png_int_32,
    addend2: png_int_32,
) -> c_int {
    /* Safely add three integers.  Returns 0 on success, 1 on overflow.  Does not
     * set the result on overflow.
     */
    let mut error: c_int = 0;
    let result: c_int = png_fp_add(
        *addend0_and_result,
        png_fp_add(addend1, addend2, core::ptr::addr_of_mut!(error)),
        core::ptr::addr_of_mut!(error),
    );
    if error == 0 {
        *addend0_and_result = result;
    }
    error
}

/* Added at libpng-1.5.5 to support read and write of true CIEXYZ values for
 * cHRM, as opposed to using chromaticities.  These internal APIs return
 * non-zero on a parameter error.  The X, Y and Z values are required to be
 * positive and less than 1.0.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_xy_from_XYZ(xy: *mut png_xy, XYZ: *const png_XYZ) -> c_int {
    /* NOTE: returns 0 on success, 1 means error. */
    let mut d: png_int_32;
    let dred: png_int_32;
    let dgreen: png_int_32;
    let dblue: png_int_32;
    let dwhite: png_int_32;
    let whiteX: png_int_32;
    let whiteY: png_int_32;

    /* 'd' in each of the blocks below is just X+Y+Z for each component,
     * x, y and z are X,Y,Z/(X+Y+Z).
     */
    d = (*XYZ).red_X;
    if png_safe_add(core::ptr::addr_of_mut!(d), (*XYZ).red_Y, (*XYZ).red_Z) != 0 {
        return 1;
    }
    dred = d;
    if png_muldiv(
        core::ptr::addr_of_mut!((*xy).redx),
        (*XYZ).red_X,
        PNG_FP_1,
        dred,
    ) == 0
    {
        return 1;
    }
    if png_muldiv(
        core::ptr::addr_of_mut!((*xy).redy),
        (*XYZ).red_Y,
        PNG_FP_1,
        dred,
    ) == 0
    {
        return 1;
    }

    d = (*XYZ).green_X;
    if png_safe_add(core::ptr::addr_of_mut!(d), (*XYZ).green_Y, (*XYZ).green_Z) != 0 {
        return 1;
    }
    dgreen = d;
    if png_muldiv(
        core::ptr::addr_of_mut!((*xy).greenx),
        (*XYZ).green_X,
        PNG_FP_1,
        dgreen,
    ) == 0
    {
        return 1;
    }
    if png_muldiv(
        core::ptr::addr_of_mut!((*xy).greeny),
        (*XYZ).green_Y,
        PNG_FP_1,
        dgreen,
    ) == 0
    {
        return 1;
    }

    d = (*XYZ).blue_X;
    if png_safe_add(core::ptr::addr_of_mut!(d), (*XYZ).blue_Y, (*XYZ).blue_Z) != 0 {
        return 1;
    }
    dblue = d;
    if png_muldiv(
        core::ptr::addr_of_mut!((*xy).bluex),
        (*XYZ).blue_X,
        PNG_FP_1,
        dblue,
    ) == 0
    {
        return 1;
    }
    if png_muldiv(
        core::ptr::addr_of_mut!((*xy).bluey),
        (*XYZ).blue_Y,
        PNG_FP_1,
        dblue,
    ) == 0
    {
        return 1;
    }

    /* The reference white is simply the sum of the end-point (X,Y,Z) vectors so
     * the following calculates (X+Y+Z) of the reference white (media white,
     * encoding white) itself:
     */
    d = dblue;
    if png_safe_add(core::ptr::addr_of_mut!(d), dred, dgreen) != 0 {
        return 1;
    }
    dwhite = d;

    /* Find the white X,Y values from the sum of the red, green and blue X,Y
     * values.
     */
    d = (*XYZ).red_X;
    if png_safe_add(core::ptr::addr_of_mut!(d), (*XYZ).green_X, (*XYZ).blue_X) != 0 {
        return 1;
    }
    whiteX = d;

    d = (*XYZ).red_Y;
    if png_safe_add(core::ptr::addr_of_mut!(d), (*XYZ).green_Y, (*XYZ).blue_Y) != 0 {
        return 1;
    }
    whiteY = d;

    if png_muldiv(
        core::ptr::addr_of_mut!((*xy).whitex),
        whiteX,
        PNG_FP_1,
        dwhite,
    ) == 0
    {
        return 1;
    }
    if png_muldiv(
        core::ptr::addr_of_mut!((*xy).whitey),
        whiteY,
        PNG_FP_1,
        dwhite,
    ) == 0
    {
        return 1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_XYZ_from_xy(XYZ: *mut png_XYZ, xy: *const png_xy) -> c_int {
    /* NOTE: returns 0 on success, 1 means error. */
    let mut red_inverse: png_fixed_point = 0;
    let mut green_inverse: png_fixed_point = 0;
    let mut blue_scale: png_fixed_point;
    let mut left: png_fixed_point = 0;
    let mut right: png_fixed_point = 0;
    let denominator: png_fixed_point;

    /* Check xy and, implicitly, z.  Note that wide gamut color spaces typically
     * have end points with 0 tristimulus values (these are impossible end
     * points, but they are used to cover the possible colors).  We check
     * xy->whitey against 5, not 0, to avoid a possible integer overflow.
     *
     * The limits here will *not* accept ACES AP0, where bluey is -7700
     * (-0.0770) because the PNG spec itself requires the xy values to be
     * unsigned.  whitey is also required to be 5 or more to avoid overflow.
     *
     * Instead the upper limits have been relaxed to accommodate ACES AP1 where
     * redz ends up as -600 (-0.006).  ProPhotoRGB was already "in range."
     * The new limit accommodates the AP0 and AP1 ranges for z but not AP0 redy.
     */
    let fpLimit: png_fixed_point = PNG_FP_1 + (PNG_FP_1 / 10);
    if (*xy).redx < 0 || (*xy).redx > fpLimit {
        return 1;
    }
    if (*xy).redy < 0 || (*xy).redy > fpLimit - (*xy).redx {
        return 1;
    }
    if (*xy).greenx < 0 || (*xy).greenx > fpLimit {
        return 1;
    }
    if (*xy).greeny < 0 || (*xy).greeny > fpLimit - (*xy).greenx {
        return 1;
    }
    if (*xy).bluex < 0 || (*xy).bluex > fpLimit {
        return 1;
    }
    if (*xy).bluey < 0 || (*xy).bluey > fpLimit - (*xy).bluex {
        return 1;
    }
    if (*xy).whitex < 0 || (*xy).whitex > fpLimit {
        return 1;
    }
    if (*xy).whitey < 5 || (*xy).whitey > fpLimit - (*xy).whitex {
        return 1;
    }

    /* The reverse calculation is more difficult because the original tristimulus
     * value had 9 independent values (red,green,blue)x(X,Y,Z) however only 8
     * derived values were recorded in the cHRM chunk;
     * (red,green,blue,white)x(x,y).  This loses one degree of freedom and
     * therefore an arbitrary ninth value has to be introduced to undo the
     * original transformations.
     *
     * Think of the original end-points as points in (X,Y,Z) space.  The
     * chromaticity values (c) have the property:
     *
     *           C
     *   c = ---------
     *       X + Y + Z
     *
     * For each c (x,y,z) from the corresponding original C (X,Y,Z).  Thus the
     * three chromaticity values (x,y,z) for each end-point obey the
     * relationship:
     *
     *   x + y + z = 1
     *
     * This describes the plane in (X,Y,Z) space that intersects each axis at the
     * value 1.0; call this the chromaticity plane.  Thus the chromaticity
     * calculation has scaled each end-point so that it is on the x+y+z=1 plane
     * and chromaticity is the intersection of the vector from the origin to the
     * (X,Y,Z) value with the chromaticity plane.
     *
     * To fully invert the chromaticity calculation we would need the three
     * end-point scale factors, (red-scale, green-scale, blue-scale), but these
     * were not recorded.  Instead we calculated the reference white (X,Y,Z) and
     * recorded the chromaticity of this.  The reference white (X,Y,Z) would have
     * given all three of the scale factors since:
     *
     *    color-C = color-c * color-scale
     *    white-C = red-C + green-C + blue-C
     *            = red-c*red-scale + green-c*green-scale + blue-c*blue-scale
     *
     * But cHRM records only white-x and white-y, so we have lost the white scale
     * factor:
     *
     *    white-C = white-c*white-scale
     *
     * To handle this the inverse transformation makes an arbitrary assumption
     * about white-scale:
     *
     *    Assume: white-Y = 1.0
     *    Hence:  white-scale = 1/white-y
     *    Or:     red-Y + green-Y + blue-Y = 1.0
     *
     * Notice the last statement of the assumption gives an equation in three of
     * the nine values we want to calculate.  8 more equations come from the
     * above routine as summarised at the top above (the chromaticity
     * calculation):
     *
     *    Given: color-x = color-X / (color-X + color-Y + color-Z)
     *    Hence: (color-x - 1)*color-X + color.x*color-Y + color.x*color-Z = 0
     *
     * This is 9 simultaneous equations in the 9 variables "color-C" and can be
     * solved by Cramer's rule.  Cramer's rule requires calculating 10 9x9 matrix
     * determinants, however this is not as bad as it seems because only 28 of
     * the total of 90 terms in the various matrices are non-zero.  Nevertheless
     * Cramer's rule is notoriously numerically unstable because the determinant
     * calculation involves the difference of large, but similar, numbers.  It is
     * difficult to be sure that the calculation is stable for real world values
     * and it is certain that it becomes unstable where the end points are close
     * together.
     *
     * So this code uses the perhaps slightly less optimal but more
     * understandable and totally obvious approach of calculating color-scale.
     *
     * This algorithm depends on the precision in white-scale and that is
     * (1/white-y), so we can immediately see that as white-y approaches 0 the
     * accuracy inherent in the cHRM chunk drops off substantially.
     *
     * libpng arithmetic: a simple inversion of the above equations
     * ------------------------------------------------------------
     *
     *    white_scale = 1/white-y
     *    white-X = white-x * white-scale
     *    white-Y = 1.0
     *    white-Z = (1 - white-x - white-y) * white_scale
     *
     *    white-C = red-C + green-C + blue-C
     *            = red-c*red-scale + green-c*green-scale + blue-c*blue-scale
     *
     * This gives us three equations in (red-scale,green-scale,blue-scale) where
     * all the coefficients are now known:
     *
     *    red-x*red-scale + green-x*green-scale + blue-x*blue-scale
     *       = white-x/white-y
     *    red-y*red-scale + green-y*green-scale + blue-y*blue-scale = 1
     *    red-z*red-scale + green-z*green-scale + blue-z*blue-scale
     *       = (1 - white-x - white-y)/white-y
     *
     * In the last equation color-z is (1 - color-x - color-y) so we can add all
     * three equations together to get an alternative third:
     *
     *    red-scale + green-scale + blue-scale = 1/white-y = white-scale
     *
     * So now we have a Cramer's rule solution where the determinants are just
     * 3x3 - far more tractable.  Unfortunately 3x3 determinants still involve
     * multiplication of three coefficients so we can't guarantee to avoid
     * overflow in the libpng fixed point representation.  Using Cramer's rule in
     * floating point is probably a good choice here, but it's not an option for
     * fixed point.  Instead proceed to simplify the first two equations by
     * eliminating what is likely to be the largest value, blue-scale:
     *
     *    blue-scale = white-scale - red-scale - green-scale
     *
     * Hence:
     *
     *    (red-x - blue-x)*red-scale + (green-x - blue-x)*green-scale =
     *                (white-x - blue-x)*white-scale
     *
     *    (red-y - blue-y)*red-scale + (green-y - blue-y)*green-scale =
     *                1 - blue-y*white-scale
     *
     * And now we can trivially solve for (red-scale,green-scale):
     *
     *    green-scale =
     *                (white-x - blue-x)*white-scale - (red-x - blue-x)*red-scale
     *                -----------------------------------------------------------
     *                                  green-x - blue-x
     *
     *    red-scale =
     *                1 - blue-y*white-scale - (green-y - blue-y) * green-scale
     *                ---------------------------------------------------------
     *                                  red-y - blue-y
     *
     * Hence:
     *
     *    red-scale =
     *          ( (green-x - blue-x) * (white-y - blue-y) -
     *            (green-y - blue-y) * (white-x - blue-x) ) / white-y
     * -------------------------------------------------------------------------
     *  (green-x - blue-x)*(red-y - blue-y)-(green-y - blue-y)*(red-x - blue-x)
     *
     *    green-scale =
     *          ( (red-y - blue-y) * (white-x - blue-x) -
     *            (red-x - blue-x) * (white-y - blue-y) ) / white-y
     * -------------------------------------------------------------------------
     *  (green-x - blue-x)*(red-y - blue-y)-(green-y - blue-y)*(red-x - blue-x)
     *
     * Accuracy:
     * The input values have 5 decimal digits of accuracy.
     *
     * In the previous implementation the values were all in the range 0 < value
     * < 1, so simple products are in the same range but may need up to 10
     * decimal digits to preserve the original precision and avoid underflow.
     * Because we are using a 32-bit signed representation we cannot match this;
     * the best is a little over 9 decimal digits, less than 10.
     *
     * This range has now been extended to allow values up to 1.1, or 110,000 in
     * fixed point.
     *
     * The approach used here is to preserve the maximum precision within the
     * signed representation.  Because the red-scale calculation above uses the
     * difference between two products of values that must be in the range
     * -1.1..+1.1 it is sufficient to divide the product by 8;
     * ceil(121,000/32767*2).  The factor is irrelevant in the calculation
     * because it is applied to both numerator and denominator.
     *
     * Note that the values of the differences of the products of the
     * chromaticities in the above equations tend to be small, for example for
     * the sRGB chromaticities they are:
     *
     * red numerator:    -0.04751
     * green numerator:  -0.08788
     * denominator:      -0.2241 (without white-y multiplication)
     *
     *  The resultant Y coefficients from the chromaticities of some widely used
     *  color space definitions are (to 15 decimal places):
     *
     *  sRGB
     *    0.212639005871510 0.715168678767756 0.072192315360734
     *  Kodak ProPhoto
     *    0.288071128229293 0.711843217810102 0.000085653960605
     *  Adobe RGB
     *    0.297344975250536 0.627363566255466 0.075291458493998
     *  Adobe Wide Gamut RGB
     *    0.258728243040113 0.724682314948566 0.016589442011321
     */
    {
        let mut error: c_int = 0;

        /* By the argument above overflow should be impossible here, however the
         * code now simply returns a failure code.  The xy subtracts in the
         * arguments to png_muldiv are *not* checked for overflow because the
         * checks at the start guarantee they are in the range 0..110000 and
         * png_fixed_point is a 32-bit signed number.
         */
        if png_muldiv(
            core::ptr::addr_of_mut!(left),
            (*xy).greenx - (*xy).bluex,
            (*xy).redy - (*xy).bluey,
            8,
        ) == 0
        {
            return 1;
        }
        if png_muldiv(
            core::ptr::addr_of_mut!(right),
            (*xy).greeny - (*xy).bluey,
            (*xy).redx - (*xy).bluex,
            8,
        ) == 0
        {
            return 1;
        }
        denominator = png_fp_sub(left, right, core::ptr::addr_of_mut!(error));
        if error != 0 {
            return 1;
        }

        /* Now find the red numerator. */
        if png_muldiv(
            core::ptr::addr_of_mut!(left),
            (*xy).greenx - (*xy).bluex,
            (*xy).whitey - (*xy).bluey,
            8,
        ) == 0
        {
            return 1;
        }
        if png_muldiv(
            core::ptr::addr_of_mut!(right),
            (*xy).greeny - (*xy).bluey,
            (*xy).whitex - (*xy).bluex,
            8,
        ) == 0
        {
            return 1;
        }

        /* Overflow is possible here and it indicates an extreme set of PNG cHRM
         * chunk values.  This calculation actually returns the reciprocal of the
         * scale value because this allows us to delay the multiplication of
         * white-y into the denominator, which tends to produce a small number.
         */
        if png_muldiv(
            core::ptr::addr_of_mut!(red_inverse),
            (*xy).whitey,
            denominator,
            png_fp_sub(left, right, core::ptr::addr_of_mut!(error)),
        ) == 0
            || error != 0
            || red_inverse <= (*xy).whitey
        /* r+g+b scales = white scale */
        {
            return 1;
        }

        /* Similarly for green_inverse: */
        if png_muldiv(
            core::ptr::addr_of_mut!(left),
            (*xy).redy - (*xy).bluey,
            (*xy).whitex - (*xy).bluex,
            8,
        ) == 0
        {
            return 1;
        }
        if png_muldiv(
            core::ptr::addr_of_mut!(right),
            (*xy).redx - (*xy).bluex,
            (*xy).whitey - (*xy).bluey,
            8,
        ) == 0
        {
            return 1;
        }
        if png_muldiv(
            core::ptr::addr_of_mut!(green_inverse),
            (*xy).whitey,
            denominator,
            png_fp_sub(left, right, core::ptr::addr_of_mut!(error)),
        ) == 0
            || error != 0
            || green_inverse <= (*xy).whitey
        {
            return 1;
        }

        /* And the blue scale, the checks above guarantee this can't overflow but
         * it can still produce 0 for extreme cHRM values.
         */
        blue_scale = png_fp_sub(
            png_fp_sub(
                png_reciprocal((*xy).whitey),
                png_reciprocal(red_inverse),
                core::ptr::addr_of_mut!(error),
            ),
            png_reciprocal(green_inverse),
            core::ptr::addr_of_mut!(error),
        );
        if error != 0 || blue_scale <= 0 {
            return 1;
        }
    }

    /* And fill in the png_XYZ.  Again the subtracts are safe because of the
     * checks on the xy values at the start (the subtracts just calculate the
     * corresponding z values.)
     */
    if png_muldiv(
        core::ptr::addr_of_mut!((*XYZ).red_X),
        (*xy).redx,
        PNG_FP_1,
        red_inverse,
    ) == 0
    {
        return 1;
    }
    if png_muldiv(
        core::ptr::addr_of_mut!((*XYZ).red_Y),
        (*xy).redy,
        PNG_FP_1,
        red_inverse,
    ) == 0
    {
        return 1;
    }
    if png_muldiv(
        core::ptr::addr_of_mut!((*XYZ).red_Z),
        PNG_FP_1 - (*xy).redx - (*xy).redy,
        PNG_FP_1,
        red_inverse,
    ) == 0
    {
        return 1;
    }

    if png_muldiv(
        core::ptr::addr_of_mut!((*XYZ).green_X),
        (*xy).greenx,
        PNG_FP_1,
        green_inverse,
    ) == 0
    {
        return 1;
    }
    if png_muldiv(
        core::ptr::addr_of_mut!((*XYZ).green_Y),
        (*xy).greeny,
        PNG_FP_1,
        green_inverse,
    ) == 0
    {
        return 1;
    }
    if png_muldiv(
        core::ptr::addr_of_mut!((*XYZ).green_Z),
        PNG_FP_1 - (*xy).greenx - (*xy).greeny,
        PNG_FP_1,
        green_inverse,
    ) == 0
    {
        return 1;
    }

    if png_muldiv(
        core::ptr::addr_of_mut!((*XYZ).blue_X),
        (*xy).bluex,
        blue_scale,
        PNG_FP_1,
    ) == 0
    {
        return 1;
    }
    if png_muldiv(
        core::ptr::addr_of_mut!((*XYZ).blue_Y),
        (*xy).bluey,
        blue_scale,
        PNG_FP_1,
    ) == 0
    {
        return 1;
    }
    if png_muldiv(
        core::ptr::addr_of_mut!((*XYZ).blue_Z),
        PNG_FP_1 - (*xy).bluex - (*xy).bluey,
        blue_scale,
        PNG_FP_1,
    ) == 0
    {
        return 1;
    }

    0 /*success*/
}

/* Error message generation */
fn png_icc_tag_char(byte: png_uint_32) -> c_char {
    let byte = byte & 0xff;
    if byte >= 32 && byte <= 126 {
        byte as c_char
    } else {
        b'?' as c_char
    }
}

unsafe fn png_icc_tag_name(name: *mut c_char, tag: png_uint_32) {
    *name.add(0) = b'\'' as c_char;
    *name.add(1) = png_icc_tag_char(tag >> 24);
    *name.add(2) = png_icc_tag_char(tag >> 16);
    *name.add(3) = png_icc_tag_char(tag >> 8);
    *name.add(4) = png_icc_tag_char(tag);
    *name.add(5) = b'\'' as c_char;
}

fn is_ICC_signature_char(it: png_alloc_size_t) -> c_int {
    (it == 32 || (it >= 48 && it <= 57) || (it >= 65 && it <= 90) || (it >= 97 && it <= 122))
        as c_int
}

fn is_ICC_signature(it: png_alloc_size_t) -> c_int {
    (is_ICC_signature_char(it >> 24) != 0 /* checks all the top bits */
        && is_ICC_signature_char((it >> 16) & 0xff) != 0
        && is_ICC_signature_char((it >> 8) & 0xff) != 0
        && is_ICC_signature_char(it & 0xff) != 0) as c_int
}

unsafe fn png_icc_profile_error(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    value: png_alloc_size_t,
    reason: png_const_charp,
) -> c_int {
    let mut pos: usize;
    let mut message: [c_char; 196] = [0; 196]; /* see below for calculation */
    let message_p: png_charp = message.as_mut_ptr();
    let sizeof_message: usize = core::mem::size_of::<[c_char; 196]>();

    pos = png_safecat(message_p, sizeof_message, 0, cstr!("profile '")); /* 9 chars */
    pos = png_safecat(message_p, pos + 79, pos, name); /* Truncate to 79 chars */
    pos = png_safecat(message_p, sizeof_message, pos, cstr!("': ")); /* +2 = 90 */
    if is_ICC_signature(value) != 0 {
        /* So 'value' is at most 4 bytes and the following cast is safe */
        png_icc_tag_name(message_p.add(pos), value as png_uint_32);
        pos += 6; /* total +8; less than the else clause */
        *message_p.add(pos) = b':' as c_char;
        pos += 1;
        *message_p.add(pos) = b' ' as c_char;
        pos += 1;
    } else {
        let mut number: [c_char; PNG_NUMBER_BUFFER_SIZE] = [0; PNG_NUMBER_BUFFER_SIZE]; /* +24 = 114 */
        let number_p: png_charp = number.as_mut_ptr();
        let sizeof_number: usize = core::mem::size_of::<[c_char; PNG_NUMBER_BUFFER_SIZE]>();

        pos = png_safecat(
            message_p,
            sizeof_message,
            pos,
            png_format_number(
                number_p as png_const_charp,
                number_p.add(sizeof_number),
                PNG_NUMBER_FORMAT_x,
                value,
            ),
        );
        pos = png_safecat(message_p, sizeof_message, pos, cstr!("h: ")); /* +2 = 116 */
    }

    /* The 'reason' is an arbitrary message, allow +79 maximum 195 */
    pos = png_safecat(message_p, sizeof_message, pos, reason);

    png_chunk_benign_error(png_ptr, message_p as png_const_charp);

    0
}

/* Encoded value of D50 as an ICC XYZNumber.  From the ICC 2010 spec the value
 * is XYZ(0.9642,1.0,0.8249), which scales to:
 *
 *    (63189.8112, 65536, 54060.6464)
 */
static D50_nCIEXYZ: [png_byte; 12] = [
    0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
];

/* bool */
unsafe fn icc_check_length(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
) -> c_int {
    if profile_length < 132 {
        return png_icc_profile_error(
            png_ptr,
            name,
            profile_length as png_alloc_size_t,
            cstr!("too short"),
        );
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_icc_check_length(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
) -> c_int {
    if icc_check_length(png_ptr, name, profile_length) == 0 {
        return 0;
    }

    /* This needs to be here because the 'normal' check is in
     * png_decompress_chunk, yet this happens after the attempt to
     * png_malloc_base the required data.  We only need this on read; on write
     * the caller supplies the profile buffer so libpng doesn't allocate it.  See
     * the call to icc_check_length below (the write case).
     */
    if profile_length as png_alloc_size_t > png_chunk_max(png_ptr) {
        return png_icc_profile_error(
            png_ptr,
            name,
            profile_length as png_alloc_size_t,
            cstr!("profile too long"),
        );
    }

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_icc_check_header(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
    profile: png_const_bytep, /* first 132 bytes only */
    color_type: c_int,
) -> c_int {
    let mut temp: png_uint_32;

    /* Length check; this cannot be ignored in this code because profile_length
     * is used later to check the tag table, so even if the profile seems over
     * long profile_length from the caller must be correct.  The caller can fix
     * this up on read or write by just passing in the profile header length.
     */
    temp = png_get_uint_32(profile);
    if temp != profile_length {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            cstr!("length does not match profile"),
        );
    }

    temp = *profile.add(8) as png_uint_32;
    if temp > 3 && (profile_length & 3) != 0 {
        return png_icc_profile_error(
            png_ptr,
            name,
            profile_length as png_alloc_size_t,
            cstr!("invalid length"),
        );
    }

    temp = png_get_uint_32(profile.add(128)); /* tag count: 12 bytes/tag */
    if temp > 357913930 /* (2^32-4-132)/12: maximum possible tag count */
        || profile_length < 132u32.wrapping_add(12u32.wrapping_mul(temp))
    /* truncated tag table */
    {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            cstr!("tag count too large"),
        );
    }

    /* The 'intent' must be valid or we can't store it, ICC limits the intent to
     * 16 bits.
     */
    temp = png_get_uint_32(profile.add(64));
    if temp >= 0xffff
    /* The ICC limit */
    {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            cstr!("invalid rendering intent"),
        );
    }

    /* This is just a warning because the profile may be valid in future
     * versions.
     */
    if temp >= PNG_sRGB_INTENT_LAST as png_uint_32 {
        png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            cstr!("intent outside defined range"),
        );
    }

    /* At this point the tag table can't be checked because it hasn't necessarily
     * been loaded; however, various header fields can be checked.  These checks
     * are for values permitted by the PNG spec in an ICC profile; the PNG spec
     * restricts the profiles that can be passed in an iCCP chunk (they must be
     * appropriate to processing PNG data!)
     */

    /* Data checks (could be skipped).  These checks must be independent of the
     * version number; however, the version number doesn't accommodate changes in
     * the header fields (just the known tags and the interpretation of the
     * data.)
     */
    temp = png_get_uint_32(profile.add(36)); /* signature 'ascp' */
    if temp != 0x61637370 {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            cstr!("invalid signature"),
        );
    }

    /* Currently the PCS illuminant/adopted white point (the computational
     * white point) are required to be D50,
     * however the profile contains a record of the illuminant so perhaps ICC
     * expects to be able to change this in the future (despite the rationale in
     * the introduction for using a fixed PCS adopted white.)  Consequently the
     * following is just a warning.
     */
    if memcmp(
        profile.add(68) as *const c_void,
        D50_nCIEXYZ.as_ptr() as *const c_void,
        12,
    ) != 0
    {
        png_icc_profile_error(
            png_ptr,
            name,
            0, /*no tag value*/
            cstr!("PCS illuminant is not D50"),
        );
    }

    /* The PNG spec requires this:
     * "If the iCCP chunk is present, the image samples conform to the colour
     * space represented by the embedded ICC profile as defined by the
     * International Color Consortium [ICC]. The colour space of the ICC profile
     * shall be an RGB colour space for colour images (PNG colour types 2, 3, and
     * 6), or a greyscale colour space for greyscale images (PNG colour types 0
     * and 4)."
     *
     * This checking code ensures the embedded profile (on either read or write)
     * conforms to the specification requirements.  Notice that an ICC 'gray'
     * color-space profile contains the information to transform the monochrome
     * data to XYZ or L*a*b (according to which PCS the profile uses) and this
     * should be used in preference to the standard libpng K channel replication
     * into R, G and B channels.
     *
     * Previously it was suggested that an RGB profile on grayscale data could be
     * handled.  However it is clear that using an RGB profile in this context
     * must be an error - there is no specification of what it means.  Thus it is
     * almost certainly more correct to ignore the profile.
     */
    temp = png_get_uint_32(profile.add(16)); /* data colour space field */
    match temp {
        0x52474220 => {
            /* 'RGB ' */
            if (color_type & PNG_COLOR_MASK_COLOR) == 0 {
                return png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    cstr!("RGB color space not permitted on grayscale PNG"),
                );
            }
        }

        0x47524159 => {
            /* 'GRAY' */
            if (color_type & PNG_COLOR_MASK_COLOR) != 0 {
                return png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    cstr!("Gray color space not permitted on RGB PNG"),
                );
            }
        }

        _ => {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                cstr!("invalid ICC profile color space"),
            );
        }
    }

    /* It is up to the application to check that the profile class matches the
     * application requirements; the spec provides no guidance, but it's pretty
     * weird if the profile is not scanner ('scnr'), monitor ('mntr'), printer
     * ('prtr') or 'spac' (for generic color spaces).  Issue a warning in these
     * cases.  Issue an error for device link or abstract profiles - these don't
     * contain the records necessary to transform the color-space to anything
     * other than the target device (and not even that for an abstract profile).
     * Profiles of these classes may not be embedded in images.
     */
    temp = png_get_uint_32(profile.add(12)); /* profile/device class */
    match temp {
        /* 'scnr' */
        /* 'mntr' */
        /* 'prtr' */
        /* 'spac' */
        0x73636e72 | 0x6d6e7472 | 0x70727472 | 0x73706163 => {
            /* All supported */
        }

        0x61627374 => {
            /* 'abst' */
            /* May not be embedded in an image */
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                cstr!("invalid embedded Abstract ICC profile"),
            );
        }

        0x6c696e6b => {
            /* 'link' */
            /* DeviceLink profiles cannot be interpreted in a non-device specific
             * fashion, if an app uses the AToB0Tag in the profile the results are
             * undefined unless the result is sent to the intended device,
             * therefore a DeviceLink profile should not be found embedded in a
             * PNG.
             */
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                cstr!("unexpected DeviceLink ICC profile class"),
            );
        }

        0x6e6d636c => {
            /* 'nmcl' */
            /* A NamedColor profile is also device specific, however it doesn't
             * contain an AToB0 tag that is open to misinterpretation.  Almost
             * certainly it will fail the tests below.
             */
            png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                cstr!("unexpected NamedColor ICC profile class"),
            );
        }

        _ => {
            /* To allow for future enhancements to the profile accept unrecognized
             * profile classes with a warning, these then hit the test below on the
             * tag content to ensure they are backward compatible with one of the
             * understood profiles.
             */
            png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                cstr!("unrecognized ICC profile class"),
            );
        }
    }

    /* For any profile other than a device link one the PCS must be encoded
     * either in XYZ or Lab.
     */
    temp = png_get_uint_32(profile.add(20));
    match temp {
        /* 'XYZ ' */
        /* 'Lab ' */
        0x58595a20 | 0x4c616220 => {}

        _ => {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                cstr!("unexpected ICC PCS encoding"),
            );
        }
    }

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_icc_check_tag_table(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
    profile: png_const_bytep, /* header plus whole tag table */
) -> c_int {
    let tag_count: png_uint_32 = png_get_uint_32(profile.add(128));
    let mut itag: png_uint_32;
    let mut tag: png_const_bytep = profile.add(132); /* The first tag */

    /* First scan all the tags in the table and add bits to the icc_info value
     * (temporarily in 'tags').
     */
    itag = 0;
    while itag < tag_count {
        let tag_id: png_uint_32 = png_get_uint_32(tag.add(0));
        let tag_start: png_uint_32 = png_get_uint_32(tag.add(4)); /* must be aligned */
        let tag_length: png_uint_32 = png_get_uint_32(tag.add(8)); /* not padded */

        /* The ICC specification does not exclude zero length tags, therefore the
         * start might actually be anywhere if there is no data, but this would be
         * a clear abuse of the intent of the standard so the start is checked for
         * being in range.  All defined tag types have an 8 byte header - a 4 byte
         * type signature then 0.
         */

        /* This is a hard error; potentially it can cause read outside the
         * profile.
         */
        if tag_start > profile_length || tag_length > profile_length - tag_start {
            return png_icc_profile_error(
                png_ptr,
                name,
                tag_id as png_alloc_size_t,
                cstr!("ICC profile tag outside profile"),
            );
        }

        if (tag_start & 3) != 0 {
            /* CNHP730S.icc shipped with Microsoft Windows 64 violates this; it is
             * only a warning here because libpng does not care about the
             * alignment.
             */
            png_icc_profile_error(
                png_ptr,
                name,
                tag_id as png_alloc_size_t,
                cstr!("ICC profile tag start not a multiple of 4"),
            );
        }

        itag += 1;
        tag = tag.add(12);
    }

    1 /* success, maybe with warnings */
}

unsafe fn have_chromaticities(png_ptr: png_const_structrp) -> c_int {
    /* Handle new PNGv3 chunks and the precedence rules to determine whether
     * png_struct::chromaticities must be processed.  Only required for RGB to
     * gray.
     *
     * mDCV: this is the mastering colour space and it is independent of the
     *       encoding so it needs to be used regardless of the encoded space.
     *
     * cICP: first in priority but not yet implemented - the chromaticities come
     *       from the 'primaries'.
     *
     * iCCP: not supported by libpng (so ignored)
     *
     * sRGB: the defaults match sRGB
     *
     * cHRM: calculate the coefficients
     */

    if png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
        return 1;
    }

    if png_file_has_chunk(png_ptr, PNG_INDEX_sRGB) {
        return 0;
    }

    if png_file_has_chunk(png_ptr, PNG_INDEX_cHRM) {
        return 1;
    }

    0 /* sRGB defaults */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_rgb_coefficients(png_ptr: png_structrp) {
    /* Set the rgb_to_gray coefficients from the colorspace if available.  Note
     * that '_set' means that png_rgb_to_gray was called **and** it successfully
     * set up the coefficients.
     */
    if (*png_ptr).rgb_to_gray_coefficients_set == 0 {
        let mut xyz: png_XYZ = core::mem::zeroed();

        if have_chromaticities(png_ptr as png_const_structrp) != 0
            && png_XYZ_from_xy(
                core::ptr::addr_of_mut!(xyz),
                core::ptr::addr_of!((*png_ptr).chromaticities),
            ) == 0
        {
            /* png_set_rgb_to_gray has not set the coefficients, get them from the
             * Y * values of the colorspace colorants.
             */
            let mut r: png_fixed_point = xyz.red_Y;
            let mut g: png_fixed_point = xyz.green_Y;
            let mut b: png_fixed_point = xyz.blue_Y;
            let total: png_fixed_point = r.wrapping_add(g).wrapping_add(b);

            if total > 0
                && r >= 0
                && png_muldiv(core::ptr::addr_of_mut!(r), r, 32768, total) != 0
                && r >= 0
                && r <= 32768
                && g >= 0
                && png_muldiv(core::ptr::addr_of_mut!(g), g, 32768, total) != 0
                && g >= 0
                && g <= 32768
                && b >= 0
                && png_muldiv(core::ptr::addr_of_mut!(b), b, 32768, total) != 0
                && b >= 0
                && b <= 32768
                && r + g + b <= 32769
            {
                /* We allow 0 coefficients here.  r+g+b may be 32769 if two or
                 * all of the coefficients were rounded up.  Handle this by
                 * reducing the *largest* coefficient by 1; this matches the
                 * approach used for the default coefficients in pngrtran.c
                 */
                let mut add: c_int = 0;

                if r + g + b > 32768 {
                    add = -1;
                } else if r + g + b < 32768 {
                    add = 1;
                }

                if add != 0 {
                    if g >= r && g >= b {
                        g += add;
                    } else if r >= g && r >= b {
                        r += add;
                    } else {
                        b += add;
                    }
                }

                /* Check for an internal error. */
                if r + g + b != 32768 {
                    png_error(
                        png_ptr as png_const_structrp,
                        cstr!("internal error handling cHRM coefficients"),
                    );
                } else {
                    (*png_ptr).rgb_to_gray_red_coeff = r as png_uint_16;
                    (*png_ptr).rgb_to_gray_green_coeff = g as png_uint_16;
                }
            }
        } else {
            /* Use the historical REC 709 (etc) values: */
            (*png_ptr).rgb_to_gray_red_coeff = 6968;
            (*png_ptr).rgb_to_gray_green_coeff = 23434;
            /* png_ptr->rgb_to_gray_blue_coeff  = 2366; */
        }
    }
}
