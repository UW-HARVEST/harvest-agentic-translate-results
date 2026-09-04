//! pngrtran.c lines 1-1145: read transformation setup APIs
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

/* Dither file to 8-bit.  Supply a palette, the current number
 * of elements in the palette, the maximum number of elements
 * allowed, and a histogram if possible.  If the current number
 * of colors is greater than the maximum number, the palette will be
 * modified to fit in the maximum number.  "full_quantize" indicates
 * whether we need a quantizing cube set up for RGB images, or if we
 * simply are reducing the number of colors in a paletted image.
 */

#[repr(C)]
pub struct png_dsort {
    pub next: *mut png_dsort,
    pub left: png_byte,
    pub right: png_byte,
}
pub type png_dsortp = *mut png_dsort;
pub type png_dsortpp = *mut png_dsortp;

/* Set the action on getting a CRC error for an ancillary or critical chunk. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_crc_action(
    png_ptr: png_structrp,
    crit_action: c_int,
    ancil_action: c_int,
) {
    if png_ptr.is_null() {
        return;
    }

    /* Tell libpng how we react to CRC errors in critical chunks */
    if crit_action == PNG_CRC_NO_CHANGE {
        /* Leave setting as is */
    } else if crit_action == PNG_CRC_WARN_USE {
        /* Warn/use data */
        (*png_ptr).flags &= !PNG_FLAG_CRC_CRITICAL_MASK;
        (*png_ptr).flags |= PNG_FLAG_CRC_CRITICAL_USE;
    } else if crit_action == PNG_CRC_QUIET_USE {
        /* Quiet/use data */
        (*png_ptr).flags &= !PNG_FLAG_CRC_CRITICAL_MASK;
        (*png_ptr).flags |= PNG_FLAG_CRC_CRITICAL_USE | PNG_FLAG_CRC_CRITICAL_IGNORE;
    } else {
        if crit_action == PNG_CRC_WARN_DISCARD {
            /* Not a valid action for critical data */
            png_warning(png_ptr, c"Can't discard critical data on CRC error".as_ptr());
            /* FALLTHROUGH */
        }
        /* PNG_CRC_ERROR_QUIT: Error/quit
         * PNG_CRC_DEFAULT and default
         */
        (*png_ptr).flags &= !PNG_FLAG_CRC_CRITICAL_MASK;
    }

    /* Tell libpng how we react to CRC errors in ancillary chunks */
    if ancil_action == PNG_CRC_NO_CHANGE {
        /* Leave setting as is */
    } else if ancil_action == PNG_CRC_WARN_USE {
        /* Warn/use data */
        (*png_ptr).flags &= !PNG_FLAG_CRC_ANCILLARY_MASK;
        (*png_ptr).flags |= PNG_FLAG_CRC_ANCILLARY_USE;
    } else if ancil_action == PNG_CRC_QUIET_USE {
        /* Quiet/use data */
        (*png_ptr).flags &= !PNG_FLAG_CRC_ANCILLARY_MASK;
        (*png_ptr).flags |= PNG_FLAG_CRC_ANCILLARY_USE | PNG_FLAG_CRC_ANCILLARY_NOWARN;
    } else if ancil_action == PNG_CRC_ERROR_QUIT {
        /* Error/quit */
        (*png_ptr).flags &= !PNG_FLAG_CRC_ANCILLARY_MASK;
        (*png_ptr).flags |= PNG_FLAG_CRC_ANCILLARY_NOWARN;
    } else {
        /* PNG_CRC_WARN_DISCARD: Warn/discard data
         * PNG_CRC_DEFAULT and default
         */
        (*png_ptr).flags &= !PNG_FLAG_CRC_ANCILLARY_MASK;
    }
}

/* Is it OK to set a transformation now?  Only if png_start_read_image or
 * png_read_update_info have not been called.  It is not necessary for the IHDR
 * to have been read in all cases; the need_IHDR parameter allows for this
 * check too.
 */
pub unsafe fn png_rtran_ok(png_ptr: png_structrp, need_IHDR: c_int) -> c_int {
    if !png_ptr.is_null() {
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) != 0 {
            png_app_error(
                png_ptr,
                c"invalid after png_start_read_image or png_read_update_info".as_ptr(),
            );
        } else if need_IHDR != 0 && ((*png_ptr).mode & PNG_HAVE_IHDR) == 0 {
            png_app_error(
                png_ptr,
                c"invalid before the PNG header has been read".as_ptr(),
            );
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
pub unsafe extern "C-unwind" fn png_set_background_fixed(
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
            c"Application must supply a known background gamma".as_ptr(),
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
pub unsafe extern "C-unwind" fn png_set_background(
    png_ptr: png_structrp,
    background_color: png_const_color_16p,
    background_gamma_code: c_int,
    need_expand: c_int,
    background_gamma: c_double,
) {
    png_set_background_fixed(
        png_ptr,
        background_color,
        background_gamma_code,
        need_expand,
        png_fixed(png_ptr, background_gamma, c"png_set_background".as_ptr()),
    );
}

/* Scale 16-bit depth files to 8-bit depth.  If both of these are set then the
 * one that pngrtran does first (scale) happens.  This is necessary to allow the
 * TRANSFORM and API behavior to be somewhat consistent, and it's simpler.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_scale_16(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_SCALE_16_TO_8;
}

/* Chop 16-bit depth files to 8-bit depth */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_strip_16(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_16_TO_8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_strip_alpha(png_ptr: png_structrp) {
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
pub unsafe fn translate_gamma_flags(
    output_gamma_in: png_fixed_point,
    is_screen: c_int,
) -> png_fixed_point {
    let mut output_gamma = output_gamma_in;

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

pub unsafe fn convert_gamma_value(
    png_ptr: png_structrp,
    output_gamma_in: c_double,
) -> png_fixed_point {
    let mut output_gamma = output_gamma_in;

    /* The following silently ignores cases where fixed point (times 100,000)
     * gamma values are passed to the floating point API.  This is safe and it
     * means the fixed point constants work just fine with the floating point
     * API.  The alternative would just lead to undetected errors and spurious
     * bug reports.  Negative values fail inside the _fixed API unless they
     * correspond to the flag values.
     */
    if output_gamma > 0. && output_gamma < 128. {
        output_gamma *= PNG_FP_1 as c_double;
    }

    /* This preserves -1 and -2 exactly: */
    output_gamma = (output_gamma + 0.5).floor();

    if output_gamma > PNG_FP_MAX as c_double || output_gamma < PNG_FP_MIN as c_double {
        png_fixed_error(png_ptr, c"gamma value".as_ptr());
    }

    output_gamma as png_fixed_point
}

pub unsafe fn unsupported_gamma(
    png_ptr: png_structrp,
    gamma: png_fixed_point,
    warn: c_int,
) -> c_int {
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
        if warn != 0 {
            png_app_warning(png_ptr, c"gamma out of supported range".as_ptr());
        } else {
            png_app_error(png_ptr, c"gamma out of supported range".as_ptr());
        }
        return 1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_alpha_mode_fixed(
    png_ptr: png_structrp,
    mode: c_int,
    output_gamma_in: png_fixed_point,
) {
    let mut output_gamma = output_gamma_in;
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
    if mode == PNG_ALPHA_PNG {
        /* default: png standard */
        /* No compose, but it may be set by png_set_background! */
        (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
        (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
    } else if mode == PNG_ALPHA_ASSOCIATED {
        /* color channels premultiplied */
        compose = 1;
        (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
        (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
        /* The output is linear: */
        output_gamma = PNG_FP_1;
    } else if mode == PNG_ALPHA_OPTIMIZED {
        /* associated, non-opaque pixels linear */
        compose = 1;
        (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
        (*png_ptr).flags |= PNG_FLAG_OPTIMIZE_ALPHA;
        /* output_gamma records the encoding of opaque pixels! */
    } else if mode == PNG_ALPHA_BROKEN {
        /* associated, non-linear, alpha encoded */
        compose = 1;
        (*png_ptr).transformations |= PNG_ENCODE_ALPHA;
        (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
    } else {
        png_error(png_ptr, c"invalid alpha mode".as_ptr());
    }

    /* Set the screen gamma values: */
    (*png_ptr).screen_gamma = output_gamma;

    /* Finally, if pre-multiplying, set the background fields to achieve the
     * desired result.
     */
    if compose != 0 {
        /* And obtain alpha pre-multiplication by composing on black: */
        memset(
            core::ptr::addr_of_mut!((*png_ptr).background) as *mut u8,
            0,
            core::mem::size_of::<png_color_16>(),
        );
        (*png_ptr).background_gamma = file_gamma; /* just in case */
        (*png_ptr).background_gamma_type = PNG_BACKGROUND_GAMMA_FILE as png_byte;
        (*png_ptr).transformations &= !PNG_BACKGROUND_EXPAND;

        if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
            png_error(
                png_ptr,
                c"conflicting calls to set alpha mode and background".as_ptr(),
            );
        }

        (*png_ptr).transformations |= PNG_COMPOSE;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_alpha_mode(
    png_ptr: png_structrp,
    mode: c_int,
    output_gamma: c_double,
) {
    png_set_alpha_mode_fixed(png_ptr, mode, convert_gamma_value(png_ptr, output_gamma));
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_quantize(
    png_ptr: png_structrp,
    palette: png_colorp,
    num_palette_in: c_int,
    maximum_colors: c_int,
    histogram: png_const_uint_16p,
    full_quantize: c_int,
) {
    let mut num_palette = num_palette_in;

    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    if palette.is_null() {
        return;
    }

    (*png_ptr).transformations |= PNG_QUANTIZE;

    if full_quantize == 0 {
        let mut i: c_int;

        /* Initialize the array to index colors.
         *
         * Ensure quantize_index can fit 256 elements (PNG_MAX_PALETTE_LENGTH)
         * rather than num_palette elements. This is to prevent buffer overflows
         * caused by malformed PNG files with out-of-range palette indices.
         *
         * Be careful to avoid leaking memory. Applications are allowed to call
         * this function more than once per png_struct.
         */
        png_free(png_ptr, (*png_ptr).quantize_index as png_voidp);
        (*png_ptr).quantize_index = core::ptr::null_mut();
        (*png_ptr).quantize_index =
            png_malloc(png_ptr, PNG_MAX_PALETTE_LENGTH as png_alloc_size_t) as png_bytep;
        i = 0;
        while i < PNG_MAX_PALETTE_LENGTH {
            *(*png_ptr).quantize_index.add(i as usize) = i as png_byte;
            i += 1;
        }
    }

    if num_palette > maximum_colors {
        if !histogram.is_null() {
            /* This is easy enough, just throw out the least used colors.
             * Perhaps not the best solution, but good enough.
             */

            let quantize_sort: png_bytep;
            let mut i: c_int;
            let mut j: c_int;

            /* Initialize the local array to sort colors. */
            quantize_sort = png_malloc(png_ptr, num_palette as png_alloc_size_t) as png_bytep;
            i = 0;
            while i < num_palette {
                *quantize_sort.add(i as usize) = i as png_byte;
                i += 1;
            }

            /* Find the least used palette entries by starting a
             * bubble sort, and running it until we have sorted
             * out enough colors.  Note that we don't care about
             * sorting all the colors, just finding which are
             * least used.
             */

            i = num_palette - 1;
            while i >= maximum_colors {
                let mut done: c_int; /* To stop early if the list is pre-sorted */

                done = 1;
                j = 0;
                while j < i {
                    if *histogram.add(*quantize_sort.add(j as usize) as usize)
                        < *histogram.add(*quantize_sort.add((j + 1) as usize) as usize)
                    {
                        let t: png_byte;

                        t = *quantize_sort.add(j as usize);
                        *quantize_sort.add(j as usize) = *quantize_sort.add((j + 1) as usize);
                        *quantize_sort.add((j + 1) as usize) = t;
                        done = 0;
                    }
                    j += 1;
                }

                if done != 0 {
                    break;
                }

                i -= 1;
            }

            /* Swap the palette around, and set up a table, if necessary */
            if full_quantize != 0 {
                j = num_palette;

                /* Put all the useful colors within the max, but don't
                 * move the others.
                 */
                i = 0;
                while i < maximum_colors {
                    if (*quantize_sort.add(i as usize) as c_int) >= maximum_colors {
                        loop {
                            j -= 1;
                            if !((*quantize_sort.add(j as usize) as c_int) >= maximum_colors) {
                                break;
                            }
                        }

                        *palette.add(i as usize) = *palette.add(j as usize);
                    }
                    i += 1;
                }
            } else {
                j = num_palette;

                /* Move all the used colors inside the max limit, and
                 * develop a translation table.
                 */
                i = 0;
                while i < maximum_colors {
                    /* Only move the colors we need to */
                    if (*quantize_sort.add(i as usize) as c_int) >= maximum_colors {
                        let tmp_color: png_color;

                        loop {
                            j -= 1;
                            if !((*quantize_sort.add(j as usize) as c_int) >= maximum_colors) {
                                break;
                            }
                        }

                        tmp_color = *palette.add(j as usize);
                        *palette.add(j as usize) = *palette.add(i as usize);
                        *palette.add(i as usize) = tmp_color;
                        /* Indicate where the color went */
                        *(*png_ptr).quantize_index.add(j as usize) = i as png_byte;
                        *(*png_ptr).quantize_index.add(i as usize) = j as png_byte;
                    }
                    i += 1;
                }

                /* Find closest color for those colors we are not using */
                i = 0;
                while i < num_palette {
                    if (*(*png_ptr).quantize_index.add(i as usize) as c_int) >= maximum_colors {
                        let mut min_d: c_int;
                        let mut k: c_int;
                        let mut min_k: c_int;
                        let d_index: c_int;

                        /* Find the closest color to one we threw out */
                        d_index = *(*png_ptr).quantize_index.add(i as usize) as c_int;
                        min_d = PNG_COLOR_DIST(*palette.add(d_index as usize), *palette.add(0));
                        k = 1;
                        min_k = 0;
                        while k < maximum_colors {
                            let d: c_int;

                            d = PNG_COLOR_DIST(*palette.add(d_index as usize), *palette.add(k as usize));

                            if d < min_d {
                                min_d = d;
                                min_k = k;
                            }

                            k += 1;
                        }
                        /* Point to closest color */
                        *(*png_ptr).quantize_index.add(i as usize) = min_k as png_byte;
                    }
                    i += 1;
                }
            }
            png_free(png_ptr, quantize_sort as png_voidp);
        } else {
            /* This is much harder to do simply (and quickly).  Perhaps
             * we need to go through a median cut routine, but those
             * don't always behave themselves with only a few colors
             * as input.  So we will just find the closest two colors,
             * and throw out one of them (chosen somewhat randomly).
             * [We don't understand this at all, so if someone wants to
             *  work on improving it, be our guest - AED, GRP]
             */
            let mut i: c_int;
            let mut max_d: c_int;
            let mut num_new_palette: c_int;
            let mut t: png_dsortp;
            let hash: png_dsortpp;

            t = core::ptr::null_mut();

            /* Initialize palette index arrays */
            (*png_ptr).index_to_palette =
                png_malloc(png_ptr, num_palette as png_alloc_size_t) as png_bytep;
            (*png_ptr).palette_to_index =
                png_malloc(png_ptr, num_palette as png_alloc_size_t) as png_bytep;

            /* Initialize the sort array */
            i = 0;
            while i < num_palette {
                *(*png_ptr).index_to_palette.add(i as usize) = i as png_byte;
                *(*png_ptr).palette_to_index.add(i as usize) = i as png_byte;
                i += 1;
            }

            hash = png_calloc(
                png_ptr,
                (769 * core::mem::size_of::<png_dsortp>()) as png_alloc_size_t,
            ) as png_dsortpp;

            num_new_palette = num_palette;

            /* Initial wild guess at how far apart the farthest pixel
             * pair we will be eliminating will be.  Larger
             * numbers mean more areas will be allocated, Smaller
             * numbers run the risk of not saving enough data, and
             * having to do this all over again.
             *
             * I have not done extensive checking on this number.
             */
            max_d = 96;

            while num_new_palette > maximum_colors {
                i = 0;
                while i < num_new_palette - 1 {
                    let mut j: c_int;

                    j = i + 1;
                    while j < num_new_palette {
                        let d: c_int;

                        d = PNG_COLOR_DIST(*palette.add(i as usize), *palette.add(j as usize));

                        if d <= max_d {
                            t = png_malloc_warn(
                                png_ptr,
                                core::mem::size_of::<png_dsort>() as png_alloc_size_t,
                            ) as png_dsortp;

                            if t.is_null() {
                                break;
                            }

                            (*t).next = *hash.add(d as usize);
                            (*t).left = *(*png_ptr).palette_to_index.add(i as usize);
                            (*t).right = *(*png_ptr).palette_to_index.add(j as usize);
                            *hash.add(d as usize) = t;
                        }

                        j += 1;
                    }
                    if t.is_null() {
                        break;
                    }
                    i += 1;
                }

                if !t.is_null() {
                    i = 0;
                    while i <= max_d {
                        if !(*hash.add(i as usize)).is_null() {
                            let mut p: png_dsortp;

                            p = *hash.add(i as usize);
                            while !p.is_null() {
                                if (*(*png_ptr).index_to_palette.add((*p).left as usize) as c_int)
                                    < num_new_palette
                                    && (*(*png_ptr).index_to_palette.add((*p).right as usize)
                                        as c_int)
                                        < num_new_palette
                                {
                                    let j: c_int;
                                    let next_j: c_int;

                                    if (num_new_palette & 0x01) != 0 {
                                        j = (*p).left as c_int;
                                        next_j = (*p).right as c_int;
                                    } else {
                                        j = (*p).right as c_int;
                                        next_j = (*p).left as c_int;
                                    }

                                    num_new_palette -= 1;
                                    *palette.add(
                                        *(*png_ptr).index_to_palette.add(j as usize) as usize,
                                    ) = *palette.add(num_new_palette as usize);
                                    if full_quantize == 0 {
                                        let mut k: c_int;

                                        k = 0;
                                        while k < num_palette {
                                            if *(*png_ptr).quantize_index.add(k as usize)
                                                == *(*png_ptr).index_to_palette.add(j as usize)
                                            {
                                                *(*png_ptr).quantize_index.add(k as usize) =
                                                    *(*png_ptr)
                                                        .index_to_palette
                                                        .add(next_j as usize);
                                            }

                                            if (*(*png_ptr).quantize_index.add(k as usize) as c_int)
                                                == num_new_palette
                                            {
                                                *(*png_ptr).quantize_index.add(k as usize) =
                                                    *(*png_ptr).index_to_palette.add(j as usize);
                                            }

                                            k += 1;
                                        }
                                    }

                                    *(*png_ptr).index_to_palette.add(
                                        *(*png_ptr).palette_to_index.add(num_new_palette as usize)
                                            as usize,
                                    ) = *(*png_ptr).index_to_palette.add(j as usize);

                                    *(*png_ptr).palette_to_index.add(
                                        *(*png_ptr).index_to_palette.add(j as usize) as usize,
                                    ) = *(*png_ptr).palette_to_index.add(num_new_palette as usize);

                                    *(*png_ptr).index_to_palette.add(j as usize) =
                                        num_new_palette as png_byte;

                                    *(*png_ptr).palette_to_index.add(num_new_palette as usize) =
                                        j as png_byte;
                                }
                                if num_new_palette <= maximum_colors {
                                    break;
                                }

                                p = (*p).next;
                            }
                            if num_new_palette <= maximum_colors {
                                break;
                            }
                        }

                        i += 1;
                    }
                }

                i = 0;
                while i < 769 {
                    if !(*hash.add(i as usize)).is_null() {
                        let mut p: png_dsortp = *hash.add(i as usize);
                        while !p.is_null() {
                            t = (*p).next;
                            png_free(png_ptr, p as png_voidp);
                            p = t;
                        }
                    }
                    *hash.add(i as usize) = core::ptr::null_mut();
                    i += 1;
                }
                max_d += 96;
            }
            png_free(png_ptr, hash as png_voidp);
            png_free(png_ptr, (*png_ptr).palette_to_index as png_voidp);
            png_free(png_ptr, (*png_ptr).index_to_palette as png_voidp);
            (*png_ptr).palette_to_index = core::ptr::null_mut();
            (*png_ptr).index_to_palette = core::ptr::null_mut();
        }
        num_palette = maximum_colors;
    }
    if (*png_ptr).palette.is_null() {
        /* Allocate an owned copy rather than aliasing the caller's pointer,
         * so that png_read_destroy can free png_ptr->palette unconditionally.
         */
        (*png_ptr).palette = png_calloc(
            png_ptr,
            (PNG_MAX_PALETTE_LENGTH as usize) * core::mem::size_of::<png_color>(),
        ) as png_colorp;
        memcpy(
            (*png_ptr).palette as *mut u8,
            palette as *const u8,
            (num_palette as c_uint as usize) * core::mem::size_of::<png_color>(),
        );
    }
    (*png_ptr).num_palette = num_palette as png_uint_16;

    if full_quantize != 0 {
        let mut i: c_int;
        let distance: png_bytep;
        let total_bits: c_int =
            PNG_QUANTIZE_RED_BITS + PNG_QUANTIZE_GREEN_BITS + PNG_QUANTIZE_BLUE_BITS;
        let num_red: c_int = 1 << PNG_QUANTIZE_RED_BITS;
        let num_green: c_int = 1 << PNG_QUANTIZE_GREEN_BITS;
        let num_blue: c_int = 1 << PNG_QUANTIZE_BLUE_BITS;
        let num_entries: usize = 1usize << total_bits;

        (*png_ptr).palette_lookup =
            png_calloc(png_ptr, num_entries as png_alloc_size_t) as png_bytep;

        distance = png_malloc(png_ptr, num_entries as png_alloc_size_t) as png_bytep;

        memset(distance as *mut u8, 0xff, num_entries);

        i = 0;
        while i < num_palette {
            let mut ir: c_int;
            let mut ig: c_int;
            let mut ib: c_int;
            let r: c_int = ((*palette.add(i as usize)).red as c_int) >> (8 - PNG_QUANTIZE_RED_BITS);
            let g: c_int =
                ((*palette.add(i as usize)).green as c_int) >> (8 - PNG_QUANTIZE_GREEN_BITS);
            let b: c_int =
                ((*palette.add(i as usize)).blue as c_int) >> (8 - PNG_QUANTIZE_BLUE_BITS);

            ir = 0;
            while ir < num_red {
                /* int dr = abs(ir - r); */
                let dr: c_int = if ir > r { ir - r } else { r - ir };
                let index_r: c_int = ir << (PNG_QUANTIZE_BLUE_BITS + PNG_QUANTIZE_GREEN_BITS);

                ig = 0;
                while ig < num_green {
                    /* int dg = abs(ig - g); */
                    let dg: c_int = if ig > g { ig - g } else { g - ig };
                    let dt: c_int = dr + dg;
                    let dm: c_int = if dr > dg { dr } else { dg };
                    let index_g: c_int = index_r | (ig << PNG_QUANTIZE_BLUE_BITS);

                    ib = 0;
                    while ib < num_blue {
                        let d_index: c_int = index_g | ib;
                        /* int db = abs(ib - b); */
                        let db: c_int = if ib > b { ib - b } else { b - ib };
                        let dmax: c_int = if dm > db { dm } else { db };
                        let d: c_int = dmax + dt + db;

                        if d < (*distance.add(d_index as usize) as c_int) {
                            *distance.add(d_index as usize) = d as png_byte;
                            *(*png_ptr).palette_lookup.add(d_index as usize) = i as png_byte;
                        }

                        ib += 1;
                    }

                    ig += 1;
                }

                ir += 1;
            }

            i += 1;
        }

        png_free(png_ptr, distance as png_voidp);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_gamma_fixed(
    png_ptr: png_structrp,
    scrn_gamma_in: png_fixed_point,
    file_gamma_in: png_fixed_point,
) {
    let mut scrn_gamma = scrn_gamma_in;
    let mut file_gamma = file_gamma_in;

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
        png_app_error(png_ptr, c"invalid file gamma in png_set_gamma".as_ptr());
    }
    if scrn_gamma <= 0 {
        png_app_error(png_ptr, c"invalid screen gamma in png_set_gamma".as_ptr());
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
pub unsafe extern "C-unwind" fn png_set_gamma(
    png_ptr: png_structrp,
    scrn_gamma: c_double,
    file_gamma: c_double,
) {
    png_set_gamma_fixed(
        png_ptr,
        convert_gamma_value(png_ptr, scrn_gamma),
        convert_gamma_value(png_ptr, file_gamma),
    );
}

/* Expand paletted images to RGB, expand grayscale images of
 * less than 8-bit depth to 8-bit depth, and expand tRNS chunks
 * to alpha channels.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_expand(png_ptr: png_structrp) {
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
pub unsafe extern "C-unwind" fn png_set_palette_to_rgb(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_EXPAND | PNG_EXPAND_tRNS;
}

/* Expand grayscale images of less than 8-bit depth to 8 bits. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_expand_gray_1_2_4_to_8(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_EXPAND;
}

/* Expand tRNS chunks to alpha channels. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_tRNS_to_alpha(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_EXPAND | PNG_EXPAND_tRNS;
}

/* Expand to 16-bit channels, expand the tRNS chunk too (because otherwise
 * it may not work correctly.)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_expand_16(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_EXPAND_16 | PNG_EXPAND | PNG_EXPAND_tRNS;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_gray_to_rgb(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    /* Because rgb must be 8 bits or more: */
    png_set_expand_gray_1_2_4_to_8(png_ptr);
    (*png_ptr).transformations |= PNG_GRAY_TO_RGB;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_rgb_to_gray_fixed(
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

    if error_action == PNG_ERROR_ACTION_NONE {
        (*png_ptr).transformations |= PNG_RGB_TO_GRAY;
    } else if error_action == PNG_ERROR_ACTION_WARN {
        (*png_ptr).transformations |= PNG_RGB_TO_GRAY_WARN;
    } else if error_action == PNG_ERROR_ACTION_ERROR {
        (*png_ptr).transformations |= PNG_RGB_TO_GRAY_ERR;
    } else {
        png_error(png_ptr, c"invalid error action to rgb_to_gray".as_ptr());
    }

    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        (*png_ptr).transformations |= PNG_EXPAND;
    }
    {
        if red >= 0 && green >= 0 && red + green <= PNG_FP_1 {
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
                c"ignoring out of range rgb_to_gray coefficients".as_ptr(),
            );
        }
    }
}

/* Convert a RGB image to a grayscale of the same width.  This allows us,
 * for example, to convert a 24 bpp RGB image into an 8 bpp grayscale image.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_rgb_to_gray(
    png_ptr: png_structrp,
    error_action: c_int,
    red: c_double,
    green: c_double,
) {
    png_set_rgb_to_gray_fixed(
        png_ptr,
        error_action,
        png_fixed(png_ptr, red, c"rgb to gray red coefficient".as_ptr()),
        png_fixed(png_ptr, green, c"rgb to gray green coefficient".as_ptr()),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_read_user_transform_fn(
    png_ptr: png_structrp,
    read_user_transform_fn: png_user_transform_ptr,
) {
    (*png_ptr).transformations |= PNG_USER_TRANSFORM;
    (*png_ptr).read_user_transform_fn = read_user_transform_fn;
}
