/* png.c lines 1137..1501 */

/* Added at libpng-1.5.5 to support read and write of true CIEXYZ values for
 * cHRM, as opposed to using chromaticities.  These internal APIs return
 * non-zero on a parameter error.  The X, Y and Z values are required to be
 * positive and less than 1.0.
 */
/* png_xy_from_XYZ */
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
    if png_safe_add(
        core::ptr::addr_of_mut!(d),
        (*XYZ).red_Y,
        (*XYZ).red_Z,
    ) != 0
    {
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
    if png_safe_add(
        core::ptr::addr_of_mut!(d),
        (*XYZ).green_Y,
        (*XYZ).green_Z,
    ) != 0
    {
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
    if png_safe_add(
        core::ptr::addr_of_mut!(d),
        (*XYZ).blue_Y,
        (*XYZ).blue_Z,
    ) != 0
    {
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
    if png_safe_add(
        core::ptr::addr_of_mut!(d),
        (*XYZ).green_X,
        (*XYZ).blue_X,
    ) != 0
    {
        return 1;
    }
    whiteX = d;

    d = (*XYZ).red_Y;
    if png_safe_add(
        core::ptr::addr_of_mut!(d),
        (*XYZ).green_Y,
        (*XYZ).blue_Y,
    ) != 0
    {
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

/* png_XYZ_from_xy */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_XYZ_from_xy(XYZ: *mut png_XYZ, xy: *const png_xy) -> c_int {
    /* NOTE: returns 0 on success, 1 means error. */
    let mut red_inverse: png_fixed_point = 0;
    let mut green_inverse: png_fixed_point = 0;
    let blue_scale: png_fixed_point;
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
