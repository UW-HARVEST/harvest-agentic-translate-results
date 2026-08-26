use crate::*;

/* Set the action on getting a CRC error for an ancillary or critical chunk. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_crc_action(
    png_ptr: png_structrp,
    crit_action: c_int,
    ancil_action: c_int,
) {
    if png_ptr.is_null() {
        return;
    }

    /* Tell libpng how we react to CRC errors in critical chunks */
    match crit_action {
        PNG_CRC_NO_CHANGE => {
            /* Leave setting as is */
        }

        PNG_CRC_WARN_USE => {
            /* Warn/use data */
            (*png_ptr).flags &= !PNG_FLAG_CRC_CRITICAL_MASK;
            (*png_ptr).flags |= PNG_FLAG_CRC_CRITICAL_USE;
        }

        PNG_CRC_QUIET_USE => {
            /* Quiet/use data */
            (*png_ptr).flags &= !PNG_FLAG_CRC_CRITICAL_MASK;
            (*png_ptr).flags |= PNG_FLAG_CRC_CRITICAL_USE | PNG_FLAG_CRC_CRITICAL_IGNORE;
        }

        PNG_CRC_WARN_DISCARD => {
            /* Not a valid action for critical data */
            png_warning(png_ptr, cstr!("Can't discard critical data on CRC error"));
            /* FALLTHROUGH */
            (*png_ptr).flags &= !PNG_FLAG_CRC_CRITICAL_MASK;
        }

        /* PNG_CRC_ERROR_QUIT:                                   Error/quit
         * PNG_CRC_DEFAULT:
         * default:
         */
        _ => {
            (*png_ptr).flags &= !PNG_FLAG_CRC_CRITICAL_MASK;
        }
    }

    /* Tell libpng how we react to CRC errors in ancillary chunks */
    match ancil_action {
        PNG_CRC_NO_CHANGE => {
            /* Leave setting as is */
        }

        PNG_CRC_WARN_USE => {
            /* Warn/use data */
            (*png_ptr).flags &= !PNG_FLAG_CRC_ANCILLARY_MASK;
            (*png_ptr).flags |= PNG_FLAG_CRC_ANCILLARY_USE;
        }

        PNG_CRC_QUIET_USE => {
            /* Quiet/use data */
            (*png_ptr).flags &= !PNG_FLAG_CRC_ANCILLARY_MASK;
            (*png_ptr).flags |= PNG_FLAG_CRC_ANCILLARY_USE | PNG_FLAG_CRC_ANCILLARY_NOWARN;
        }

        PNG_CRC_ERROR_QUIT => {
            /* Error/quit */
            (*png_ptr).flags &= !PNG_FLAG_CRC_ANCILLARY_MASK;
            (*png_ptr).flags |= PNG_FLAG_CRC_ANCILLARY_NOWARN;
        }

        /* PNG_CRC_WARN_DISCARD:                           Warn/discard data
         * PNG_CRC_DEFAULT:
         * default:
         */
        _ => {
            (*png_ptr).flags &= !PNG_FLAG_CRC_ANCILLARY_MASK;
        }
    }
}

/* Is it OK to set a transformation now?  Only if png_start_read_image or
 * png_read_update_info have not been called.  It is not necessary for the IHDR
 * to have been read in all cases; the need_IHDR parameter allows for this
 * check too.
 */
unsafe fn png_rtran_ok(png_ptr: png_structrp, need_IHDR: c_int) -> c_int {
    if !png_ptr.is_null() {
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) != 0 {
            png_app_error(
                png_ptr,
                cstr!("invalid after png_start_read_image or png_read_update_info"),
            );
        } else if need_IHDR != 0 && ((*png_ptr).mode & PNG_HAVE_IHDR) == 0 {
            png_app_error(png_ptr, cstr!("invalid before the PNG header has been read"));
        } else {
            /* Turn on failure to initialize correctly for all transforms. */
            (*png_ptr).flags |= PNG_FLAG_DETECT_UNINITIALIZED;

            return 1; /* Ok */
        }
    }

    0 /* no png_error possible! */
}

/* Handle alpha and tRNS via a background color */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_background_fixed(
    png_ptr: png_structrp,
    background_color: png_const_color_16p,
    background_gamma_code: c_int,
    need_expand: c_int,
    background_gamma: png_fixed_point,
) {
    if png_rtran_ok(png_ptr, 0) == 0 || background_color.is_null() {
        return;
    }

    if background_gamma_code == PNG_BACKGROUND_GAMMA_UNKNOWN {
        png_warning(
            png_ptr,
            cstr!("Application must supply a known background gamma"),
        );
        return;
    }

    (*png_ptr).transformations |= PNG_COMPOSE | PNG_STRIP_ALPHA;
    (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
    (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;

    (*png_ptr).background = *background_color;
    (*png_ptr).background_gamma = background_gamma;
    (*png_ptr).background_gamma_type = background_gamma_code as png_byte;
    if need_expand != 0 {
        (*png_ptr).transformations |= PNG_BACKGROUND_EXPAND;
    } else {
        (*png_ptr).transformations &= !PNG_BACKGROUND_EXPAND;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_background(
    png_ptr: png_structrp,
    background_color: png_const_color_16p,
    background_gamma_code: c_int,
    need_expand: c_int,
    background_gamma: f64,
) {
    png_set_background_fixed(
        png_ptr,
        background_color,
        background_gamma_code,
        need_expand,
        png_fixed(png_ptr, background_gamma, cstr!("png_set_background")),
    );
}

/* Scale 16-bit depth files to 8-bit depth.  If both of these are set then the
 * one that pngrtran does first (scale) happens.  This is necessary to allow the
 * TRANSFORM and API behavior to be somewhat consistent, and it's simpler.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_scale_16(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_SCALE_16_TO_8;
}

/* Chop 16-bit depth files to 8-bit depth */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_strip_16(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_16_TO_8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_strip_alpha(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_STRIP_ALPHA;
}

/* PNGv3 conformance: this private API exists to resolve the now mandatory error
 * resolution when multiple conflicting sources of gamma or colour space
 * information are available.
 *
 * Terminology (assuming power law, "gamma", encodings):
 *    "screen" gamma: a power law imposed by the output device when digital
 *    samples are converted to visible light output.  The EOTF - voltage to
 *    luminance on output.
 *
 *    "file" gamma: a power law used to encode luminance levels from the input
 *    data (the scene or the mastering display system) into digital voltages.
 *    The OETF - luminance to voltage on input.
 *
 *    gamma "correction": a power law matching the **inverse** of the overall
 *    transfer function from input luminance levels to output levels.  The
 *    **inverse** of the OOTF; the correction "corrects" for the OOTF by aiming
 *    to make the overall OOTF (including the correction) linear.
 *
 * It is important to understand this terminology because the defined terms are
 * scattered throughout the libpng code and it is very easy to end up with the
 * inverse of the power law required.
 *
 * Variable and struct::member names:
 *    file_gamma        OETF  how the PNG data was encoded
 *
 *    screen_gamma      EOTF  how the screen will decode digital levels
 *
 *    -- not used --    OOTF  the net effect OETF x EOTF
 *    gamma_correction        the inverse of OOTF to make the result linear
 *
 * All versions of libpng require a call to "png_set_gamma" to establish the
 * "screen" gamma, the power law representing the EOTF.  png_set_gamma may also
 * set or default the "file" gamma; the OETF.  gamma_correction is calculated
 * internally.
 *
 * The earliest libpng versions required file_gamma to be supplied to set_gamma.
 * Later versions started allowing png_set_gamma and, later, png_set_alpha_mode,
 * to cause defaulting from the file data.
 *
 * PNGv3 mandated a particular form for this defaulting, one that is compatible
 * with what libpng did except that if libpng detected inconsistencies it marked
 * all the chunks as "invalid".  PNGv3 effectively invalidates this prior code.
 *
 * Behaviour implemented below:
 *    translate_gamma_flags(gamma, is_screen)
 *       The libpng-1.6 API for the gamma parameters to libpng APIs
 *       (png_set_gamma and png_set_alpha_mode at present).  This allows the
 *       'gamma' value to be passed as a png_fixed_point number or as one of a
 *       set of integral values for specific "well known" examples of transfer
 *       functions.  This is compatible with PNGv3.
 */
unsafe fn translate_gamma_flags(
    mut output_gamma: png_fixed_point,
    is_screen: c_int,
) -> png_fixed_point {
    /* Check for flag values.  The main reason for having the old Mac value as a
     * flag is that it is pretty near impossible to work out what the correct
     * value is from Apple documentation - a working Mac system is needed to
     * discover the value!
     */
    if output_gamma == PNG_DEFAULT_sRGB || output_gamma == PNG_FP_1 / PNG_DEFAULT_sRGB {
        if is_screen != 0 {
            output_gamma = PNG_GAMMA_sRGB;
        } else {
            output_gamma = PNG_GAMMA_sRGB_INVERSE;
        }
    } else if output_gamma == PNG_GAMMA_MAC_18 || output_gamma == PNG_FP_1 / PNG_GAMMA_MAC_18 {
        if is_screen != 0 {
            output_gamma = PNG_GAMMA_MAC_OLD;
        } else {
            output_gamma = PNG_GAMMA_MAC_INVERSE;
        }
    }

    output_gamma
}

unsafe fn convert_gamma_value(png_ptr: png_structrp, mut output_gamma: f64) -> png_fixed_point {
    /* The following silently ignores cases where fixed point (times 100,000)
     * gamma values are passed to the floating point API.  This is safe and it
     * means the fixed point constants work just fine with the floating point
     * API.  The alternative would just lead to undetected errors and spurious
     * bug reports.  Negative values fail inside the _fixed API unless they
     * correspond to the flag values.
     */
    if output_gamma > 0.0 && output_gamma < 128.0 {
        output_gamma *= PNG_FP_1 as f64;
    }

    /* This preserves -1 and -2 exactly: */
    output_gamma = floor(output_gamma + 0.5);

    if output_gamma > PNG_FP_MAX as f64 || output_gamma < PNG_FP_MIN as f64 {
        png_fixed_error(png_ptr, cstr!("gamma value"));
    }

    output_gamma as png_fixed_point
}

unsafe fn unsupported_gamma(png_ptr: png_structrp, gamma: png_fixed_point, warn: c_int) -> c_int {
    /* Validate a gamma value to ensure it is in a reasonable range.  The value
     * is expected to be 1 or greater, but this range test allows for some
     * viewing correction values.  The intent is to weed out the API users
     * who might use the inverse of the gamma value accidentally!
     *
     * 1.6.47: apply the test in png_set_gamma as well but only warn and return
     * false if it fires.
     *
     * TODO: 1.8: make this an app_error in png_set_gamma as well.
     */
    if gamma < PNG_LIB_GAMMA_MIN || gamma > PNG_LIB_GAMMA_MAX {
        /* msg: "gamma out of supported range" */
        if warn != 0 {
            png_app_warning(png_ptr, cstr!("gamma out of supported range"));
        } else {
            png_app_error(png_ptr, cstr!("gamma out of supported range"));
        }
        return 1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_alpha_mode_fixed(
    png_ptr: png_structrp,
    mode: c_int,
    mut output_gamma: png_fixed_point,
) {
    let mut file_gamma: png_fixed_point;
    let mut compose: c_int = 0;

    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    output_gamma = translate_gamma_flags(output_gamma, 1 /*screen*/);
    if unsupported_gamma(png_ptr, output_gamma, 0 /*error*/) != 0 {
        return;
    }

    /* The default file gamma is the inverse of the output gamma; the output
     * gamma may be changed below so get the file value first.  The default_gamma
     * is set here and from the simplified API (which uses a different algorithm)
     * so don't overwrite a set value:
     */
    file_gamma = (*png_ptr).default_gamma;
    if file_gamma == 0 {
        file_gamma = png_reciprocal(output_gamma);
        (*png_ptr).default_gamma = file_gamma;
    }

    /* There are really 8 possibilities here, composed of any combination
     * of:
     *
     *    premultiply the color channels
     *    do not encode non-opaque pixels
     *    encode the alpha as well as the color channels
     *
     * The differences disappear if the input/output ('screen') gamma is 1.0,
     * because then the encoding is a no-op and there is only the choice of
     * premultiplying the color channels or not.
     *
     * png_set_alpha_mode and png_set_background interact because both use
     * png_compose to do the work.  Calling both is only useful when
     * png_set_alpha_mode is used to set the default mode - PNG_ALPHA_PNG - along
     * with a default gamma value.  Otherwise PNG_COMPOSE must not be set.
     */
    match mode {
        PNG_ALPHA_PNG => {
            /* default: png standard */
            /* No compose, but it may be set by png_set_background! */
            (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
            (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
        }

        PNG_ALPHA_ASSOCIATED => {
            /* color channels premultiplied */
            compose = 1;
            (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
            (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
            /* The output is linear: */
            output_gamma = PNG_FP_1;
        }

        PNG_ALPHA_OPTIMIZED => {
            /* associated, non-opaque pixels linear */
            compose = 1;
            (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
            (*png_ptr).flags |= PNG_FLAG_OPTIMIZE_ALPHA;
            /* output_gamma records the encoding of opaque pixels! */
        }

        PNG_ALPHA_BROKEN => {
            /* associated, non-linear, alpha encoded */
            compose = 1;
            (*png_ptr).transformations |= PNG_ENCODE_ALPHA;
            (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
        }

        _ => {
            png_error(png_ptr, cstr!("invalid alpha mode"));
        }
    }

    /* Set the screen gamma values: */
    (*png_ptr).screen_gamma = output_gamma;

    /* Finally, if pre-multiplying, set the background fields to achieve the
     * desired result.
     */
    if compose != 0 {
        /* And obtain alpha pre-multiplication by composing on black: */
        memset(
            core::ptr::addr_of_mut!((*png_ptr).background) as *mut c_void,
            0,
            core::mem::size_of::<png_color_16>(),
        );
        (*png_ptr).background_gamma = file_gamma; /* just in case */
        (*png_ptr).background_gamma_type = PNG_BACKGROUND_GAMMA_FILE as png_byte;
        (*png_ptr).transformations &= !PNG_BACKGROUND_EXPAND;

        if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
            png_error(
                png_ptr,
                cstr!("conflicting calls to set alpha mode and background"),
            );
        }

        (*png_ptr).transformations |= PNG_COMPOSE;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_alpha_mode(png_ptr: png_structrp, mode: c_int, output_gamma: f64) {
    png_set_alpha_mode_fixed(png_ptr, mode, convert_gamma_value(png_ptr, output_gamma));
}

/* Dither file to 8-bit.  Supply a palette, the current number
 * of elements in the palette, the maximum number of elements
 * allowed, and a histogram if possible.  If the current number
 * of colors is greater than the maximum number, the palette will be
 * modified to fit in the maximum number.  "full_quantize" indicates
 * whether we need a quantizing cube set up for RGB images, or if we
 * simply are reducing the number of colors in a paletted image.
 */

#[repr(C)]
#[derive(Copy, Clone)]
struct png_dsort {
    next: *mut png_dsort,
    left: png_byte,
    right: png_byte,
}
type png_dsortp = *mut png_dsort;
type png_dsortpp = *mut *mut png_dsort;
