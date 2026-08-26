use crate::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gamma_fixed(
    png_ptr: png_structrp,
    screen_gamma: png_fixed_point,
    override_file_gamma: png_fixed_point,
) {
    /* C locals: 'scrn_gamma' and 'file_gamma' are the (modifiable) parameters. */
    let mut scrn_gamma: png_fixed_point = screen_gamma;
    let mut file_gamma: png_fixed_point = override_file_gamma;

    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    /* New in libpng-1.5.4 - reserve particular negative values as flags. */
    scrn_gamma = translate_gamma_flags(scrn_gamma, 1 /*screen*/);
    file_gamma = translate_gamma_flags(file_gamma, 0 /*file*/);

    /* Checking the gamma values for being >0 was added in 1.5.4 along with the
     * premultiplied alpha support; this actually hides an undocumented feature
     * of the previous implementation which allowed gamma processing to be
     * disabled in background handling.  There is no evidence (so far) that this
     * was being used; however, png_set_background itself accepted and must still
     * accept '0' for the gamma value it takes, because it isn't always used.
     *
     * Since this is an API change (albeit a very minor one that removes an
     * undocumented API feature) the following checks were only enabled in
     * libpng-1.6.0.
     */
    if file_gamma <= 0 {
        png_app_error(png_ptr, cstr!("invalid file gamma in png_set_gamma"));
    }
    if scrn_gamma <= 0 {
        png_app_error(png_ptr, cstr!("invalid screen gamma in png_set_gamma"));
    }

    if unsupported_gamma(png_ptr, file_gamma, 1 /*warn*/) != 0
        || unsupported_gamma(png_ptr, scrn_gamma, 1 /*warn*/) != 0
    {
        return;
    }

    /* 1.6.47: png_struct::file_gamma and png_struct::screen_gamma are now only
     * written by this API.  This removes dependencies on the order of API calls
     * and allows the complex gamma checks to be delayed until needed.
     */
    (*png_ptr).file_gamma = file_gamma;
    (*png_ptr).screen_gamma = scrn_gamma;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gamma(
    png_ptr: png_structrp,
    screen_gamma: f64,
    override_file_gamma: f64,
) {
    png_set_gamma_fixed(
        png_ptr,
        convert_gamma_value(png_ptr, screen_gamma),
        convert_gamma_value(png_ptr, override_file_gamma),
    );
}

/* Expand paletted images to RGB, expand grayscale images of
 * less than 8-bit depth to 8-bit depth, and expand tRNS chunks
 * to alpha channels.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_expand(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_EXPAND | PNG_EXPAND_tRNS;
}

/* GRR 19990627:  the following three functions currently are identical
 *  to png_set_expand().  However, it is entirely reasonable that someone
 *  might wish to expand an indexed image to RGB but *not* expand a single,
 *  fully transparent palette entry to a full alpha channel--perhaps instead
 *  convert tRNS to the grayscale/RGB format (16-bit RGB value), or replace
 *  the transparent color with a particular RGB value, or drop tRNS entirely.
 *  IOW, a future version of the library may make the transformations flag
 *  a bit more fine-grained, with separate bits for each of these three
 *  functions.
 *
 *  More to the point, these functions make it obvious what libpng will be
 *  doing, whereas "expand" can (and does) mean any number of things.
 *
 *  GRP 20060307: In libpng-1.2.9, png_set_gray_1_2_4_to_8() was modified
 *  to expand only the sample depth but not to expand the tRNS to alpha
 *  and its name was changed to png_set_expand_gray_1_2_4_to_8().
 */

/* Expand paletted images to RGB. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_palette_to_rgb(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_EXPAND | PNG_EXPAND_tRNS;
}

/* Expand grayscale images of less than 8-bit depth to 8 bits. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_expand_gray_1_2_4_to_8(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_EXPAND;
}

/* Expand tRNS chunks to alpha channels. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_tRNS_to_alpha(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_EXPAND | PNG_EXPAND_tRNS;
}

/* Expand to 16-bit channels, expand the tRNS chunk too (because otherwise
 * it may not work correctly.)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_expand_16(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_EXPAND_16 | PNG_EXPAND | PNG_EXPAND_tRNS;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gray_to_rgb(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    /* Because rgb must be 8 bits or more: */
    png_set_expand_gray_1_2_4_to_8(png_ptr);
    (*png_ptr).transformations |= PNG_GRAY_TO_RGB;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_rgb_to_gray_fixed(
    png_ptr: png_structrp,
    error_action: c_int,
    red: png_fixed_point,
    green: png_fixed_point,
) {
    /* Need the IHDR here because of the check on color_type below. */
    /* TODO: fix this */
    if png_rtran_ok(png_ptr, 1) == 0 {
        return;
    }

    match error_action {
        PNG_ERROR_ACTION_NONE => {
            (*png_ptr).transformations |= PNG_RGB_TO_GRAY;
        }

        PNG_ERROR_ACTION_WARN => {
            (*png_ptr).transformations |= PNG_RGB_TO_GRAY_WARN;
        }

        PNG_ERROR_ACTION_ERROR => {
            (*png_ptr).transformations |= PNG_RGB_TO_GRAY_ERR;
        }

        _ => png_error(png_ptr, cstr!("invalid error action to rgb_to_gray")),
    }

    /* NOTE (a libpng quirk faithfully retained here): the 'if' below only
     * controls the single following statement; the block after it is always
     * executed.
     */
    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        (*png_ptr).transformations |= PNG_EXPAND;
    }
    {
        if red >= 0 && green >= 0 && red.wrapping_add(green) <= PNG_FP_1 {
            let red_int: png_uint_16;
            let green_int: png_uint_16;

            /* NOTE: this calculation does not round, but this behavior is retained
             * for consistency; the inaccuracy is very small.  The code here always
             * overwrites the coefficients, regardless of whether they have been
             * defaulted or set already.
             */
            red_int = (((red as png_uint_32).wrapping_mul(32768)) / 100000) as png_uint_16;
            green_int = (((green as png_uint_32).wrapping_mul(32768)) / 100000) as png_uint_16;

            (*png_ptr).rgb_to_gray_red_coeff = red_int;
            (*png_ptr).rgb_to_gray_green_coeff = green_int;
            (*png_ptr).rgb_to_gray_coefficients_set = 1;
        } else if red >= 0 && green >= 0 {
            png_app_warning(
                png_ptr,
                cstr!("ignoring out of range rgb_to_gray coefficients"),
            );
        }
    }
}

/* Convert a RGB image to a grayscale of the same width.  This allows us,
 * for example, to convert a 24 bpp RGB image into an 8 bpp grayscale image.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_rgb_to_gray(
    png_ptr: png_structrp,
    error_action: c_int,
    red: f64,
    green: f64,
) {
    png_set_rgb_to_gray_fixed(
        png_ptr,
        error_action,
        png_fixed(png_ptr, red, cstr!("rgb to gray red coefficient")),
        png_fixed(png_ptr, green, cstr!("rgb to gray green coefficient")),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_read_user_transform_fn(
    png_ptr: png_structrp,
    read_user_transform_fn: png_user_transform_ptr,
) {
    (*png_ptr).transformations |= PNG_USER_TRANSFORM;
    (*png_ptr).read_user_transform_fn = read_user_transform_fn;
}

/* In the case of gamma transformations only do transformations on images where
 * the [file] gamma and screen_gamma are not close reciprocals, otherwise it
 * slows things down slightly, and also needlessly introduces small errors.
 */
unsafe fn png_gamma_threshold(screen_gamma: png_fixed_point, file_gamma: png_fixed_point) -> c_int {
    /* PNG_GAMMA_THRESHOLD is the threshold for performing gamma
     * correction as a difference of the overall transform from 1.0
     *
     * We want to compare the threshold with s*f - 1, if we get
     * overflow here it is because of wacky gamma values so we
     * turn on processing anyway.
     */
    let mut gtest: png_fixed_point = 0;
    (png_muldiv(
        &mut gtest as png_fixed_point_p,
        screen_gamma,
        file_gamma,
        PNG_FP_1,
    ) == 0
        || png_gamma_significant(gtest) != 0) as c_int
}
