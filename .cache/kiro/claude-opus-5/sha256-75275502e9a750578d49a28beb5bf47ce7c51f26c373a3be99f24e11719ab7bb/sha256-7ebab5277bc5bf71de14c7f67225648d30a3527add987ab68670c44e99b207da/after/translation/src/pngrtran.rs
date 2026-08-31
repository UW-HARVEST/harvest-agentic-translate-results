//! Translation of pngrtran.c

use crate::*;

/* png_composite (NODIV variant, the configuration uses
 * PNG_READ_COMPOSITE_NODIV_SUPPORTED). */
#[inline]
unsafe fn png_composite(fg: png_uint_16, alpha: png_uint_16, bg: png_uint_16) -> png_byte {
    let temp: png_uint_16 = (fg)
        .wrapping_mul(alpha)
        .wrapping_add((bg).wrapping_mul(255u16.wrapping_sub(alpha)))
        .wrapping_add(128);
    (((temp.wrapping_add(temp >> 8)) >> 8) & 0xff) as png_byte
}

#[inline]
unsafe fn png_composite_16(fg: png_uint_32, alpha: png_uint_32, bg: png_uint_32) -> png_uint_16 {
    let temp: png_uint_32 = (fg)
        .wrapping_mul(alpha)
        .wrapping_add((bg).wrapping_mul(65535u32.wrapping_sub(alpha)))
        .wrapping_add(32768);
    (0xffff & ((temp.wrapping_add(temp >> 16)) >> 16)) as png_uint_16
}

/* Set the action on getting a CRC error for an ancillary or critical chunk. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_crc_action(
    png_ptr: png_structrp,
    crit_action: c_int,
    ancil_action: c_int,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        /* Tell libpng how we react to CRC errors in critical chunks */
        match crit_action {
            PNG_CRC_NO_CHANGE => {} /* Leave setting as is */

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
                png_warning(
                    png_ptr,
                    c"Can't discard critical data on CRC error".as_ptr(),
                );
                /* FALLTHROUGH */
                (*png_ptr).flags &= !PNG_FLAG_CRC_CRITICAL_MASK;
            }
            /* case PNG_CRC_ERROR_QUIT, PNG_CRC_DEFAULT, default */
            _ => {
                (*png_ptr).flags &= !PNG_FLAG_CRC_CRITICAL_MASK;
            }
        }

        /* Tell libpng how we react to CRC errors in ancillary chunks */
        match ancil_action {
            PNG_CRC_NO_CHANGE => {} /* Leave setting as is */

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

            /* case PNG_CRC_WARN_DISCARD, PNG_CRC_DEFAULT, default */
            _ => {
                (*png_ptr).flags &= !PNG_FLAG_CRC_ANCILLARY_MASK;
            }
        }
    }
}

/* Is it OK to set a transformation now?  Only if png_start_read_image or
 * png_read_update_info have not been called.  It is not necessary for the IHDR
 * to have been read in all cases; the need_IHDR parameter allows for this
 * check too.
 */
unsafe fn png_rtran_ok(png_ptr: png_structrp, need_IHDR: c_int) -> c_int {
    unsafe {
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
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_background(
    png_ptr: png_structrp,
    background_color: png_const_color_16p,
    background_gamma_code: c_int,
    need_expand: c_int,
    background_gamma: f64,
) {
    unsafe {
        png_set_background_fixed(
            png_ptr,
            background_color,
            background_gamma_code,
            need_expand,
            png_fixed(png_ptr, background_gamma, c"png_set_background".as_ptr()),
        );
    }
}

/* Scale 16-bit depth files to 8-bit depth.  If both of these are set then the
 * one that pngrtran does first (scale) happens.  This is necessary to allow the
 * TRANSFORM and API behavior to be somewhat consistent, and it's simpler.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_scale_16(png_ptr: png_structrp) {
    unsafe {
        if png_rtran_ok(png_ptr, 0) == 0 {
            return;
        }

        (*png_ptr).transformations |= PNG_SCALE_16_TO_8;
    }
}

/* Chop 16-bit depth files to 8-bit depth */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_strip_16(png_ptr: png_structrp) {
    unsafe {
        if png_rtran_ok(png_ptr, 0) == 0 {
            return;
        }

        (*png_ptr).transformations |= PNG_16_TO_8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_strip_alpha(png_ptr: png_structrp) {
    unsafe {
        if png_rtran_ok(png_ptr, 0) == 0 {
            return;
        }

        (*png_ptr).transformations |= PNG_STRIP_ALPHA;
    }
}

fn translate_gamma_flags(mut output_gamma: png_fixed_point, is_screen: c_int) -> png_fixed_point {
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
    unsafe {
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
            png_fixed_error(png_ptr, c"gamma value".as_ptr());
        }

        output_gamma as png_fixed_point
    }
}

unsafe fn unsupported_gamma(png_ptr: png_structrp, gamma: png_fixed_point, warn: c_int) -> c_int {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_alpha_mode_fixed(
    png_ptr: png_structrp,
    mode: c_int,
    mut output_gamma: png_fixed_point,
) {
    unsafe {
        let file_gamma: png_fixed_point;
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
        if (*png_ptr).default_gamma == 0 {
            file_gamma = png_reciprocal(output_gamma);
            (*png_ptr).default_gamma = file_gamma;
        } else {
            file_gamma = (*png_ptr).default_gamma;
        }

        /* There are really 8 possibilities here ... (see C comment) */
        match mode {
            PNG_ALPHA_PNG => {
                /* default: png standard */
                /* No compose, but it may be set by png_set_background! */
                (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
                (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
            }

            /* PNG_ALPHA_ASSOCIATED (== PNG_ALPHA_STANDARD == PNG_ALPHA_PREMULTIPLIED):
             * color channels premultiplied.
             */
            PNG_ALPHA_ASSOCIATED => {
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
                png_error(png_ptr, c"invalid alpha mode".as_ptr());
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
                &raw mut (*png_ptr).background as *mut c_void,
                0,
                core::mem::size_of_val(&(*png_ptr).background),
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_alpha_mode(
    png_ptr: png_structrp,
    mode: c_int,
    output_gamma: f64,
) {
    unsafe {
        png_set_alpha_mode_fixed(png_ptr, mode, convert_gamma_value(png_ptr, output_gamma));
    }
}

/* Dither file to 8-bit.  Supply a palette, the current number
 * of elements in the palette, the maximum number of elements
 * allowed, and a histogram if possible.  ...
 */
#[repr(C)]
#[derive(Clone, Copy)]
struct png_dsort {
    next: *mut png_dsort,
    left: png_byte,
    right: png_byte,
}
type png_dsortp = *mut png_dsort;
type png_dsortpp = *mut *mut png_dsort;

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_quantize(
    png_ptr: png_structrp,
    palette: png_colorp,
    mut num_palette: c_int,
    maximum_colors: c_int,
    histogram: png_const_uint_16p,
    full_quantize: c_int,
) {
    unsafe {
        if png_rtran_ok(png_ptr, 0) == 0 {
            return;
        }

        if palette.is_null() {
            return;
        }

        (*png_ptr).transformations |= PNG_QUANTIZE;

        if full_quantize == 0 {
            let mut i: c_int;

            /* Initialize the array to index colors. ... */
            png_free(png_ptr, (*png_ptr).quantize_index as png_voidp);
            (*png_ptr).quantize_index = core::ptr::null_mut();
            (*png_ptr).quantize_index =
                png_malloc(png_ptr, PNG_MAX_PALETTE_LENGTH as png_alloc_size_t) as png_bytep;
            i = 0;
            while i < PNG_MAX_PALETTE_LENGTH {
                *(*png_ptr).quantize_index.offset(i as isize) = i as png_byte;
                i += 1;
            }
        }

        if num_palette > maximum_colors {
            if !histogram.is_null() {
                /* This is easy enough, just throw out the least used colors. ... */

                let quantize_sort: png_bytep;
                let mut i: c_int;
                let mut j: c_int;

                /* Initialize the local array to sort colors. */
                quantize_sort = png_malloc(png_ptr, num_palette as png_alloc_size_t) as png_bytep;
                i = 0;
                while i < num_palette {
                    *quantize_sort.offset(i as isize) = i as png_byte;
                    i += 1;
                }

                /* Find the least used palette entries by starting a bubble sort ... */
                i = num_palette - 1;
                while i >= maximum_colors {
                    let mut done: c_int; /* To stop early if the list is pre-sorted */

                    done = 1;
                    j = 0;
                    while j < i {
                        if *histogram.offset(*quantize_sort.offset(j as isize) as isize)
                            < *histogram.offset(*quantize_sort.offset((j + 1) as isize) as isize)
                        {
                            let t: png_byte;

                            t = *quantize_sort.offset(j as isize);
                            *quantize_sort.offset(j as isize) =
                                *quantize_sort.offset((j + 1) as isize);
                            *quantize_sort.offset((j + 1) as isize) = t;
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

                    /* Put all the useful colors within the max, but don't move the others. */
                    i = 0;
                    while i < maximum_colors {
                        if (*quantize_sort.offset(i as isize) as c_int) >= maximum_colors {
                            loop {
                                j -= 1;
                                if !((*quantize_sort.offset(j as isize) as c_int) >= maximum_colors)
                                {
                                    break;
                                }
                            }

                            *palette.offset(i as isize) = *palette.offset(j as isize);
                        }
                        i += 1;
                    }
                } else {
                    j = num_palette;

                    /* Move all the used colors inside the max limit, and develop a
                     * translation table.
                     */
                    i = 0;
                    while i < maximum_colors {
                        /* Only move the colors we need to */
                        if (*quantize_sort.offset(i as isize) as c_int) >= maximum_colors {
                            let tmp_color: png_color;

                            loop {
                                j -= 1;
                                if !((*quantize_sort.offset(j as isize) as c_int) >= maximum_colors)
                                {
                                    break;
                                }
                            }

                            tmp_color = *palette.offset(j as isize);
                            *palette.offset(j as isize) = *palette.offset(i as isize);
                            *palette.offset(i as isize) = tmp_color;
                            /* Indicate where the color went */
                            *(*png_ptr).quantize_index.offset(j as isize) = i as png_byte;
                            *(*png_ptr).quantize_index.offset(i as isize) = j as png_byte;
                        }
                        i += 1;
                    }

                    /* Find closest color for those colors we are not using */
                    i = 0;
                    while i < num_palette {
                        if (*(*png_ptr).quantize_index.offset(i as isize) as c_int)
                            >= maximum_colors
                        {
                            let mut min_d: c_int;
                            let mut k: c_int;
                            let mut min_k: c_int;
                            let d_index: c_int;

                            /* Find the closest color to one we threw out */
                            d_index = *(*png_ptr).quantize_index.offset(i as isize) as c_int;
                            min_d = PNG_COLOR_DIST(
                                *palette.offset(d_index as isize),
                                *palette.offset(0),
                            );
                            k = 1;
                            min_k = 0;
                            while k < maximum_colors {
                                let d: c_int;

                                d = PNG_COLOR_DIST(
                                    *palette.offset(d_index as isize),
                                    *palette.offset(k as isize),
                                );

                                if d < min_d {
                                    min_d = d;
                                    min_k = k;
                                }
                                k += 1;
                            }
                            /* Point to closest color */
                            *(*png_ptr).quantize_index.offset(i as isize) = min_k as png_byte;
                        }
                        i += 1;
                    }
                }
                png_free(png_ptr, quantize_sort as png_voidp);
            } else {
                /* This is much harder to do simply (and quickly). ... */
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
                    *(*png_ptr).index_to_palette.offset(i as isize) = i as png_byte;
                    *(*png_ptr).palette_to_index.offset(i as isize) = i as png_byte;
                    i += 1;
                }

                hash = png_calloc(
                    png_ptr,
                    (769 * core::mem::size_of::<png_dsortp>()) as png_alloc_size_t,
                ) as png_dsortpp;

                num_new_palette = num_palette;

                /* Initial wild guess ... */
                max_d = 96;

                while num_new_palette > maximum_colors {
                    i = 0;
                    while i < num_new_palette - 1 {
                        let mut j: c_int;

                        j = i + 1;
                        while j < num_new_palette {
                            let d: c_int;

                            d = PNG_COLOR_DIST(
                                *palette.offset(i as isize),
                                *palette.offset(j as isize),
                            );

                            if d <= max_d {
                                t = png_malloc_warn(
                                    png_ptr,
                                    core::mem::size_of::<png_dsort>() as png_alloc_size_t,
                                ) as png_dsortp;

                                if t.is_null() {
                                    break;
                                }

                                (*t).next = *hash.offset(d as isize);
                                (*t).left = *(*png_ptr).palette_to_index.offset(i as isize);
                                (*t).right = *(*png_ptr).palette_to_index.offset(j as isize);
                                *hash.offset(d as isize) = t;
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
                            if !(*hash.offset(i as isize)).is_null() {
                                let mut p: png_dsortp;

                                p = *hash.offset(i as isize);
                                while !p.is_null() {
                                    if ((*(*png_ptr).index_to_palette.offset((*p).left as isize))
                                        as c_int)
                                        < num_new_palette
                                        && ((*(*png_ptr)
                                            .index_to_palette
                                            .offset((*p).right as isize))
                                            as c_int)
                                            < num_new_palette
                                    {
                                        let j: c_int;
                                        let next_j: c_int;

                                        if num_new_palette & 0x01 != 0 {
                                            j = (*p).left as c_int;
                                            next_j = (*p).right as c_int;
                                        } else {
                                            j = (*p).right as c_int;
                                            next_j = (*p).left as c_int;
                                        }

                                        num_new_palette -= 1;
                                        *palette.offset(
                                            *(*png_ptr).index_to_palette.offset(j as isize)
                                                as isize,
                                        ) = *palette.offset(num_new_palette as isize);
                                        if full_quantize == 0 {
                                            let mut k: c_int;

                                            k = 0;
                                            while k < num_palette {
                                                if *(*png_ptr).quantize_index.offset(k as isize)
                                                    == *(*png_ptr)
                                                        .index_to_palette
                                                        .offset(j as isize)
                                                {
                                                    *(*png_ptr).quantize_index.offset(k as isize) =
                                                        *(*png_ptr)
                                                            .index_to_palette
                                                            .offset(next_j as isize);
                                                }

                                                if (*(*png_ptr).quantize_index.offset(k as isize)
                                                    as c_int)
                                                    == num_new_palette
                                                {
                                                    *(*png_ptr).quantize_index.offset(k as isize) =
                                                        *(*png_ptr)
                                                            .index_to_palette
                                                            .offset(j as isize);
                                                }
                                                k += 1;
                                            }
                                        }

                                        *(*png_ptr).index_to_palette.offset(
                                            *(*png_ptr)
                                                .palette_to_index
                                                .offset(num_new_palette as isize)
                                                as isize,
                                        ) = *(*png_ptr).index_to_palette.offset(j as isize);

                                        *(*png_ptr).palette_to_index.offset(
                                            *(*png_ptr).index_to_palette.offset(j as isize)
                                                as isize,
                                        ) = *(*png_ptr)
                                            .palette_to_index
                                            .offset(num_new_palette as isize);

                                        *(*png_ptr).index_to_palette.offset(j as isize) =
                                            num_new_palette as png_byte;

                                        *(*png_ptr)
                                            .palette_to_index
                                            .offset(num_new_palette as isize) = j as png_byte;
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
                        if !(*hash.offset(i as isize)).is_null() {
                            let mut p: png_dsortp = *hash.offset(i as isize);
                            while !p.is_null() {
                                t = (*p).next;
                                png_free(png_ptr, p as png_voidp);
                                p = t;
                            }
                        }
                        *hash.offset(i as isize) = core::ptr::null_mut();
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
            /* Allocate an owned copy rather than aliasing the caller's pointer, ... */
            (*png_ptr).palette = png_calloc(
                png_ptr,
                (PNG_MAX_PALETTE_LENGTH as usize * core::mem::size_of::<png_color>())
                    as png_alloc_size_t,
            ) as png_colorp;
            memcpy(
                (*png_ptr).palette as *mut c_void,
                palette as *const c_void,
                (num_palette as c_uint) as usize * core::mem::size_of::<png_color>(),
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

            memset(distance as *mut c_void, 0xff, num_entries);

            i = 0;
            while i < num_palette {
                let mut ir: c_int;
                let mut ig: c_int;
                let mut ib: c_int;
                let r: c_int =
                    ((*palette.offset(i as isize)).red as c_int) >> (8 - PNG_QUANTIZE_RED_BITS);
                let g: c_int =
                    ((*palette.offset(i as isize)).green as c_int) >> (8 - PNG_QUANTIZE_GREEN_BITS);
                let b: c_int =
                    ((*palette.offset(i as isize)).blue as c_int) >> (8 - PNG_QUANTIZE_BLUE_BITS);

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

                            if d < *distance.offset(d_index as isize) as c_int {
                                *distance.offset(d_index as isize) = d as png_byte;
                                *(*png_ptr).palette_lookup.offset(d_index as isize) = i as png_byte;
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_gamma_fixed(
    png_ptr: png_structrp,
    mut scrn_gamma: png_fixed_point,
    mut file_gamma: png_fixed_point,
) {
    unsafe {
        if png_rtran_ok(png_ptr, 0) == 0 {
            return;
        }

        /* New in libpng-1.5.4 - reserve particular negative values as flags. */
        scrn_gamma = translate_gamma_flags(scrn_gamma, 1 /*screen*/);
        file_gamma = translate_gamma_flags(file_gamma, 0 /*file*/);

        /* Checking the gamma values for being >0 was added in 1.5.4 ... */
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
         * written by this API. ...
         */
        (*png_ptr).file_gamma = file_gamma;
        (*png_ptr).screen_gamma = scrn_gamma;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_gamma(
    png_ptr: png_structrp,
    scrn_gamma: f64,
    file_gamma: f64,
) {
    unsafe {
        png_set_gamma_fixed(
            png_ptr,
            convert_gamma_value(png_ptr, scrn_gamma),
            convert_gamma_value(png_ptr, file_gamma),
        );
    }
}

/* Expand paletted images to RGB, expand grayscale images of
 * less than 8-bit depth to 8-bit depth, and expand tRNS chunks
 * to alpha channels.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_expand(png_ptr: png_structrp) {
    unsafe {
        if png_rtran_ok(png_ptr, 0) == 0 {
            return;
        }

        (*png_ptr).transformations |= PNG_EXPAND | PNG_EXPAND_tRNS;
    }
}

/* Expand paletted images to RGB. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_palette_to_rgb(png_ptr: png_structrp) {
    unsafe {
        if png_rtran_ok(png_ptr, 0) == 0 {
            return;
        }

        (*png_ptr).transformations |= PNG_EXPAND | PNG_EXPAND_tRNS;
    }
}

/* Expand grayscale images of less than 8-bit depth to 8 bits. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_expand_gray_1_2_4_to_8(png_ptr: png_structrp) {
    unsafe {
        if png_rtran_ok(png_ptr, 0) == 0 {
            return;
        }

        (*png_ptr).transformations |= PNG_EXPAND;
    }
}

/* Expand tRNS chunks to alpha channels. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_tRNS_to_alpha(png_ptr: png_structrp) {
    unsafe {
        if png_rtran_ok(png_ptr, 0) == 0 {
            return;
        }

        (*png_ptr).transformations |= PNG_EXPAND | PNG_EXPAND_tRNS;
    }
}

/* Expand to 16-bit channels, expand the tRNS chunk too ... */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_expand_16(png_ptr: png_structrp) {
    unsafe {
        if png_rtran_ok(png_ptr, 0) == 0 {
            return;
        }

        (*png_ptr).transformations |= PNG_EXPAND_16 | PNG_EXPAND | PNG_EXPAND_tRNS;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_gray_to_rgb(png_ptr: png_structrp) {
    unsafe {
        if png_rtran_ok(png_ptr, 0) == 0 {
            return;
        }

        /* Because rgb must be 8 bits or more: */
        png_set_expand_gray_1_2_4_to_8(png_ptr);
        (*png_ptr).transformations |= PNG_GRAY_TO_RGB;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_rgb_to_gray_fixed(
    png_ptr: png_structrp,
    error_action: c_int,
    red: png_fixed_point,
    green: png_fixed_point,
) {
    unsafe {
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

            _ => {
                png_error(png_ptr, c"invalid error action to rgb_to_gray".as_ptr());
            }
        }

        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            (*png_ptr).transformations |= PNG_EXPAND;
        }
        {
            if red >= 0 && green >= 0 && red.wrapping_add(green) <= PNG_FP_1 {
                let red_int: png_uint_16;
                let green_int: png_uint_16;

                /* NOTE: this calculation does not round ... */
                red_int = (((red as png_uint_32) * 32768) / 100000) as png_uint_16;
                green_int = (((green as png_uint_32) * 32768) / 100000) as png_uint_16;

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
}

/* Convert a RGB image to a grayscale of the same width. ... */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_rgb_to_gray(
    png_ptr: png_structrp,
    error_action: c_int,
    red: f64,
    green: f64,
) {
    unsafe {
        png_set_rgb_to_gray_fixed(
            png_ptr,
            error_action,
            png_fixed(png_ptr, red, c"rgb to gray red coefficient".as_ptr()),
            png_fixed(png_ptr, green, c"rgb to gray green coefficient".as_ptr()),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_read_user_transform_fn(
    png_ptr: png_structrp,
    read_user_transform_fn: png_user_transform_ptr,
) {
    unsafe {
        (*png_ptr).transformations |= PNG_USER_TRANSFORM;
        (*png_ptr).read_user_transform_fn = read_user_transform_fn;
    }
}

/* In the case of gamma transformations only do transformations on images where
 * the [file] gamma and screen_gamma are not close reciprocals ...
 */
unsafe fn png_gamma_threshold(screen_gamma: png_fixed_point, file_gamma: png_fixed_point) -> c_int {
    unsafe {
        let mut gtest: png_fixed_point = 0;
        (png_muldiv(&mut gtest, screen_gamma, file_gamma, PNG_FP_1) == 0
            || png_gamma_significant(gtest) != 0) as c_int
    }
}

/* For the moment 'png_init_palette_transformations' and
 * 'png_init_rgb_transformations' only do some flag canceling optimizations.
 */
unsafe fn png_init_palette_transformations(png_ptr: png_structrp) {
    unsafe {
        let mut input_has_alpha: c_int = 0;
        let mut input_has_transparency: c_int = 0;

        if (*png_ptr).num_trans > 0 {
            let mut i: c_int;

            /* Ignore if all the entries are opaque (unlikely!) */
            i = 0;
            while i < (*png_ptr).num_trans as c_int {
                if *(*png_ptr).trans_alpha.offset(i as isize) == 255 {
                    i += 1;
                    continue;
                } else if *(*png_ptr).trans_alpha.offset(i as isize) == 0 {
                    input_has_transparency = 1;
                } else {
                    input_has_transparency = 1;
                    input_has_alpha = 1;
                    break;
                }
                i += 1;
            }
        }

        /* If no alpha we can optimize. */
        if input_has_alpha == 0 {
            (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
            (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;

            if input_has_transparency == 0 {
                (*png_ptr).transformations &= !(PNG_COMPOSE | PNG_BACKGROUND_EXPAND);
            }
        }

        /* png_set_background handling ... */
        if ((*png_ptr).transformations & PNG_BACKGROUND_EXPAND) != 0
            && ((*png_ptr).transformations & PNG_EXPAND) != 0
        {
            {
                (*png_ptr).background.red = (*(*png_ptr)
                    .palette
                    .offset((*png_ptr).background.index as isize))
                .red as png_uint_16;
                (*png_ptr).background.green = (*(*png_ptr)
                    .palette
                    .offset((*png_ptr).background.index as isize))
                .green as png_uint_16;
                (*png_ptr).background.blue = (*(*png_ptr)
                    .palette
                    .offset((*png_ptr).background.index as isize))
                .blue as png_uint_16;

                if ((*png_ptr).transformations & PNG_INVERT_ALPHA) != 0 {
                    if ((*png_ptr).transformations & PNG_EXPAND_tRNS) == 0 {
                        /* Invert the alpha channel (in tRNS) unless the pixels are
                         * going to be expanded, in which case leave it for later
                         */
                        let mut i: c_int;
                        let istop: c_int = (*png_ptr).num_trans as c_int;

                        i = 0;
                        while i < istop {
                            *(*png_ptr).trans_alpha.offset(i as isize) = (255
                                - *(*png_ptr).trans_alpha.offset(i as isize) as c_int)
                                as png_byte;
                            i += 1;
                        }
                    }
                }
            }
        } /* background expand and (therefore) no alpha association. */
    }
}

unsafe fn png_init_rgb_transformations(png_ptr: png_structrp) {
    unsafe {
        /* Added to libpng-1.5.4: check the color type ... */
        let input_has_alpha: c_int =
            (((*png_ptr).color_type as c_int) & PNG_COLOR_MASK_ALPHA != 0) as c_int;
        let input_has_transparency: c_int = ((*png_ptr).num_trans > 0) as c_int;

        /* If no alpha we can optimize. */
        if input_has_alpha == 0 {
            (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
            (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;

            if input_has_transparency == 0 {
                (*png_ptr).transformations &= !(PNG_COMPOSE | PNG_BACKGROUND_EXPAND);
            }
        }

        /* png_set_background handling ... */
        if ((*png_ptr).transformations & PNG_BACKGROUND_EXPAND) != 0
            && ((*png_ptr).transformations & PNG_EXPAND) != 0
            && (((*png_ptr).color_type as c_int) & PNG_COLOR_MASK_COLOR) == 0
        /* i.e., GRAY or GRAY_ALPHA */
        {
            {
                /* Expand background and tRNS chunks */
                let mut gray: c_int = (*png_ptr).background.gray as c_int;
                let mut trans_gray: c_int = (*png_ptr).trans_color.gray as c_int;

                match (*png_ptr).bit_depth {
                    1 => {
                        gray *= 0xff;
                        trans_gray *= 0xff;
                    }

                    2 => {
                        gray *= 0x55;
                        trans_gray *= 0x55;
                    }

                    4 => {
                        gray *= 0x11;
                        trans_gray *= 0x11;
                    }

                    /* default, case 8 (FALLTHROUGH), case 16: already full bits */
                    _ => {}
                }

                (*png_ptr).background.red = gray as png_uint_16;
                (*png_ptr).background.green = gray as png_uint_16;
                (*png_ptr).background.blue = gray as png_uint_16;

                if ((*png_ptr).transformations & PNG_EXPAND_tRNS) == 0 {
                    (*png_ptr).trans_color.red = trans_gray as png_uint_16;
                    (*png_ptr).trans_color.green = trans_gray as png_uint_16;
                    (*png_ptr).trans_color.blue = trans_gray as png_uint_16;
                }
            }
        } /* background expand and (therefore) no alpha association. */
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_resolve_file_gamma(
    png_ptr: png_const_structrp,
) -> png_fixed_point {
    unsafe {
        let mut file_gamma: png_fixed_point;

        /* The file gamma is determined by these precedence rules ... */
        file_gamma = (*png_ptr).file_gamma;
        if file_gamma != 0 {
            return file_gamma;
        }

        file_gamma = (*png_ptr).chunk_gamma;
        if file_gamma != 0 {
            return file_gamma;
        }

        file_gamma = (*png_ptr).default_gamma;
        if file_gamma != 0 {
            return file_gamma;
        }

        /* If png_reciprocal overflows, it returns 0 ... */
        if (*png_ptr).screen_gamma != 0 {
            file_gamma = png_reciprocal((*png_ptr).screen_gamma);
        }

        file_gamma
    }
}

unsafe fn png_init_gamma_values(png_ptr: png_structrp) -> c_int {
    unsafe {
        /* The following temporary indicates if overall gamma correction is
         * required.
         */
        let mut gamma_correction: c_int = 0;
        let mut file_gamma: png_fixed_point;
        let mut screen_gamma: png_fixed_point;

        /* Resolve the file_gamma. ... */
        file_gamma = png_resolve_file_gamma(png_ptr);
        screen_gamma = (*png_ptr).screen_gamma;

        if file_gamma > 0
        /* file has been set */
        {
            if screen_gamma > 0
            /* screen set too */
            {
                gamma_correction = png_gamma_threshold(file_gamma, screen_gamma);
            } else {
                /* Assume the output matches the input ... */
                screen_gamma = png_reciprocal(file_gamma);
            }
        } else
        /* both unset, prevent corrections: */
        {
            file_gamma = PNG_FP_1;
            screen_gamma = PNG_FP_1;
        }

        (*png_ptr).file_gamma = file_gamma;
        (*png_ptr).screen_gamma = screen_gamma;
        gamma_correction
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_init_read_transformations(png_ptr: png_structrp) {
    unsafe {
        /* This internal function is called from png_read_start_row ... */

        if png_init_gamma_values(png_ptr) != 0 {
            (*png_ptr).transformations |= PNG_GAMMA;
        } else {
            (*png_ptr).transformations &= !PNG_GAMMA;
        }

        /* Certain transformations have the effect of preventing other
         * transformations ... (order list in comment)
         */
        if ((*png_ptr).transformations & PNG_STRIP_ALPHA) != 0
            && ((*png_ptr).transformations & PNG_COMPOSE) == 0
        {
            /* Stripping the alpha channel happens immediately after the 'expand'
             * transformations ...
             */
            (*png_ptr).transformations &=
                !(PNG_BACKGROUND_EXPAND | PNG_ENCODE_ALPHA | PNG_EXPAND_tRNS);
            (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;

            /* Kill the tRNS chunk itself too. ... */
            (*png_ptr).num_trans = 0;
        }

        /* If the screen gamma is about 1.0 then the OPTIMIZE_ALPHA and ENCODE_ALPHA
         * settings will have no effect.
         */
        if png_gamma_significant((*png_ptr).screen_gamma) == 0 {
            (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
            (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
        }

        /* Make sure the coefficients for the rgb to gray conversion are set
         * appropriately.
         */
        if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) != 0 {
            png_set_rgb_coefficients(png_ptr);
        }

        /* Detect gray background and attempt to enable optimization for
         * gray --> RGB case. ...
         */
        if ((*png_ptr).transformations & PNG_BACKGROUND_EXPAND) != 0 {
            /* PNG_BACKGROUND_EXPAND: the background is in the file color space ... */
            if (((*png_ptr).color_type as c_int) & PNG_COLOR_MASK_COLOR) == 0 {
                (*png_ptr).mode |= PNG_BACKGROUND_IS_GRAY;
            }
        } else if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
            /* PNG_COMPOSE: png_set_background was called with need_expand false ... */
            if ((*png_ptr).transformations & PNG_GRAY_TO_RGB) != 0 {
                if (*png_ptr).background.red == (*png_ptr).background.green
                    && (*png_ptr).background.red == (*png_ptr).background.blue
                {
                    (*png_ptr).mode |= PNG_BACKGROUND_IS_GRAY;
                    (*png_ptr).background.gray = (*png_ptr).background.red;
                }
            }
        }

        /* For indexed PNG data (PNG_COLOR_TYPE_PALETTE) many of the transformations
         * can be performed directly on the palette ...
         */
        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            png_init_palette_transformations(png_ptr);
        } else {
            png_init_rgb_transformations(png_ptr);
        }

        if ((*png_ptr).transformations & PNG_EXPAND_16) != 0
            && ((*png_ptr).transformations & PNG_COMPOSE) != 0
            && ((*png_ptr).transformations & PNG_BACKGROUND_EXPAND) == 0
            && (*png_ptr).bit_depth != 16
        {
            /* TODO: fix this. ... CHOP(x) = (png_uint_16)PNG_DIV257(x) */
            (*png_ptr).background.red =
                PNG_DIV257((*png_ptr).background.red as png_uint_32) as png_uint_16;
            (*png_ptr).background.green =
                PNG_DIV257((*png_ptr).background.green as png_uint_32) as png_uint_16;
            (*png_ptr).background.blue =
                PNG_DIV257((*png_ptr).background.blue as png_uint_32) as png_uint_16;
            (*png_ptr).background.gray =
                PNG_DIV257((*png_ptr).background.gray as png_uint_32) as png_uint_16;
        }

        if ((*png_ptr).transformations & (PNG_16_TO_8 | PNG_SCALE_16_TO_8)) != 0
            && ((*png_ptr).transformations & PNG_COMPOSE) != 0
            && ((*png_ptr).transformations & PNG_BACKGROUND_EXPAND) == 0
            && (*png_ptr).bit_depth == 16
        {
            /* ... the background color must be pre-expanded here. */
            (*png_ptr).background.red = ((*png_ptr).background.red as c_int * 257) as png_uint_16;
            (*png_ptr).background.green =
                ((*png_ptr).background.green as c_int * 257) as png_uint_16;
            (*png_ptr).background.blue = ((*png_ptr).background.blue as c_int * 257) as png_uint_16;
            (*png_ptr).background.gray = ((*png_ptr).background.gray as c_int * 257) as png_uint_16;
        }

        /* Includes ALPHA_MODE */
        (*png_ptr).background_1 = (*png_ptr).background;

        /* This needs to change ... */
        if ((*png_ptr).transformations & PNG_GAMMA) != 0
            || (((*png_ptr).transformations & PNG_RGB_TO_GRAY) != 0
                && (png_gamma_significant((*png_ptr).file_gamma) != 0
                    || png_gamma_significant((*png_ptr).screen_gamma) != 0))
            || (((*png_ptr).transformations & PNG_COMPOSE) != 0
                && (png_gamma_significant((*png_ptr).file_gamma) != 0
                    || png_gamma_significant((*png_ptr).screen_gamma) != 0
                    || ((*png_ptr).background_gamma_type as c_int == PNG_BACKGROUND_GAMMA_UNIQUE
                        && png_gamma_significant((*png_ptr).background_gamma) != 0)))
            || (((*png_ptr).transformations & PNG_ENCODE_ALPHA) != 0
                && png_gamma_significant((*png_ptr).screen_gamma) != 0)
        {
            png_build_gamma_table(png_ptr, (*png_ptr).bit_depth as c_int);

            if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
                /* Issue a warning about this combination ... */
                if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) != 0 {
                    png_warning(
                        png_ptr,
                        c"libpng does not support gamma+background+rgb_to_gray".as_ptr(),
                    );
                }

                if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
                    /* We don't get to here unless there is a tRNS chunk ... */
                    let mut back: png_color = png_color::default();
                    let mut back_1: png_color = png_color::default();
                    let palette: png_colorp = (*png_ptr).palette;
                    let num_palette: c_int = (*png_ptr).num_palette as c_int;
                    let mut i: c_int;
                    if (*png_ptr).background_gamma_type as c_int == PNG_BACKGROUND_GAMMA_FILE {
                        back.red = *(*png_ptr)
                            .gamma_table
                            .offset((*png_ptr).background.red as isize);
                        back.green = *(*png_ptr)
                            .gamma_table
                            .offset((*png_ptr).background.green as isize);
                        back.blue = *(*png_ptr)
                            .gamma_table
                            .offset((*png_ptr).background.blue as isize);

                        back_1.red = *(*png_ptr)
                            .gamma_to_1
                            .offset((*png_ptr).background.red as isize);
                        back_1.green = *(*png_ptr)
                            .gamma_to_1
                            .offset((*png_ptr).background.green as isize);
                        back_1.blue = *(*png_ptr)
                            .gamma_to_1
                            .offset((*png_ptr).background.blue as isize);
                    } else {
                        let g: png_fixed_point;
                        let gs: png_fixed_point;

                        match (*png_ptr).background_gamma_type as c_int {
                            PNG_BACKGROUND_GAMMA_SCREEN => {
                                g = (*png_ptr).screen_gamma;
                                gs = PNG_FP_1;
                            }

                            PNG_BACKGROUND_GAMMA_FILE => {
                                g = png_reciprocal((*png_ptr).file_gamma);
                                gs =
                                    png_reciprocal2((*png_ptr).file_gamma, (*png_ptr).screen_gamma);
                            }

                            PNG_BACKGROUND_GAMMA_UNIQUE => {
                                g = png_reciprocal((*png_ptr).background_gamma);
                                gs = png_reciprocal2(
                                    (*png_ptr).background_gamma,
                                    (*png_ptr).screen_gamma,
                                );
                            }
                            _ => {
                                g = PNG_FP_1; /* back_1 */
                                gs = PNG_FP_1; /* back */
                            }
                        }

                        if png_gamma_significant(gs) != 0 {
                            back.red =
                                png_gamma_8bit_correct((*png_ptr).background.red as c_uint, gs);
                            back.green =
                                png_gamma_8bit_correct((*png_ptr).background.green as c_uint, gs);
                            back.blue =
                                png_gamma_8bit_correct((*png_ptr).background.blue as c_uint, gs);
                        } else {
                            back.red = (*png_ptr).background.red as png_byte;
                            back.green = (*png_ptr).background.green as png_byte;
                            back.blue = (*png_ptr).background.blue as png_byte;
                        }

                        if png_gamma_significant(g) != 0 {
                            back_1.red =
                                png_gamma_8bit_correct((*png_ptr).background.red as c_uint, g);
                            back_1.green =
                                png_gamma_8bit_correct((*png_ptr).background.green as c_uint, g);
                            back_1.blue =
                                png_gamma_8bit_correct((*png_ptr).background.blue as c_uint, g);
                        } else {
                            back_1.red = (*png_ptr).background.red as png_byte;
                            back_1.green = (*png_ptr).background.green as png_byte;
                            back_1.blue = (*png_ptr).background.blue as png_byte;
                        }
                    }

                    i = 0;
                    while i < num_palette {
                        if i < (*png_ptr).num_trans as c_int
                            && *(*png_ptr).trans_alpha.offset(i as isize) != 0xff
                        {
                            if *(*png_ptr).trans_alpha.offset(i as isize) == 0 {
                                *palette.offset(i as isize) = back;
                            } else
                            /* if (trans_alpha[i] != 0xff) */
                            {
                                if ((*png_ptr).flags & PNG_FLAG_OPTIMIZE_ALPHA) != 0 {
                                    /* Premultiply only ... */
                                    let mut component: png_uint_32;

                                    component = *(*png_ptr)
                                        .gamma_to_1
                                        .offset((*palette.offset(i as isize)).red as isize)
                                        as png_uint_32;
                                    component = (component
                                        * *(*png_ptr).trans_alpha.offset(i as isize)
                                            as png_uint_32
                                        + 128)
                                        / 255;
                                    (*palette.offset(i as isize)).red =
                                        *(*png_ptr).gamma_from_1.offset(component as isize);

                                    component = *(*png_ptr)
                                        .gamma_to_1
                                        .offset((*palette.offset(i as isize)).green as isize)
                                        as png_uint_32;
                                    component = (component
                                        * *(*png_ptr).trans_alpha.offset(i as isize)
                                            as png_uint_32
                                        + 128)
                                        / 255;
                                    (*palette.offset(i as isize)).green =
                                        *(*png_ptr).gamma_from_1.offset(component as isize);

                                    component = *(*png_ptr)
                                        .gamma_to_1
                                        .offset((*palette.offset(i as isize)).blue as isize)
                                        as png_uint_32;
                                    component = (component
                                        * *(*png_ptr).trans_alpha.offset(i as isize)
                                            as png_uint_32
                                        + 128)
                                        / 255;
                                    (*palette.offset(i as isize)).blue =
                                        *(*png_ptr).gamma_from_1.offset(component as isize);
                                } else {
                                    /* Composite with background color ... */
                                    let v: png_byte;
                                    let w: png_byte;

                                    v = *(*png_ptr)
                                        .gamma_to_1
                                        .offset((*palette.offset(i as isize)).red as isize);
                                    w = png_composite(
                                        v as png_uint_16,
                                        *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16,
                                        back_1.red as png_uint_16,
                                    );
                                    (*palette.offset(i as isize)).red =
                                        *(*png_ptr).gamma_from_1.offset(w as isize);

                                    let v: png_byte = *(*png_ptr)
                                        .gamma_to_1
                                        .offset((*palette.offset(i as isize)).green as isize);
                                    let w: png_byte = png_composite(
                                        v as png_uint_16,
                                        *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16,
                                        back_1.green as png_uint_16,
                                    );
                                    (*palette.offset(i as isize)).green =
                                        *(*png_ptr).gamma_from_1.offset(w as isize);

                                    let v: png_byte = *(*png_ptr)
                                        .gamma_to_1
                                        .offset((*palette.offset(i as isize)).blue as isize);
                                    let w: png_byte = png_composite(
                                        v as png_uint_16,
                                        *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16,
                                        back_1.blue as png_uint_16,
                                    );
                                    (*palette.offset(i as isize)).blue =
                                        *(*png_ptr).gamma_from_1.offset(w as isize);
                                }
                            }
                        } else {
                            (*palette.offset(i as isize)).red = *(*png_ptr)
                                .gamma_table
                                .offset((*palette.offset(i as isize)).red as isize);
                            (*palette.offset(i as isize)).green = *(*png_ptr)
                                .gamma_table
                                .offset((*palette.offset(i as isize)).green as isize);
                            (*palette.offset(i as isize)).blue = *(*png_ptr)
                                .gamma_table
                                .offset((*palette.offset(i as isize)).blue as isize);
                        }
                        i += 1;
                    }

                    /* Prevent the transformations being done again. ... */
                    (*png_ptr).transformations &= !(PNG_COMPOSE | PNG_GAMMA);
                    (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
                }
                /* if (background_gamma_type != PNG_BACKGROUND_GAMMA_UNKNOWN) */
                else
                /* color_type != PNG_COLOR_TYPE_PALETTE */
                {
                    let gs_sig: c_int;
                    let g_sig: c_int;
                    let mut g: png_fixed_point = PNG_FP_1; /* Correction to linear */
                    let mut gs: png_fixed_point = PNG_FP_1; /* Correction to screen */

                    match (*png_ptr).background_gamma_type as c_int {
                        PNG_BACKGROUND_GAMMA_SCREEN => {
                            g = (*png_ptr).screen_gamma;
                            /* gs = PNG_FP_1; */
                        }

                        PNG_BACKGROUND_GAMMA_FILE => {
                            g = png_reciprocal((*png_ptr).file_gamma);
                            gs = png_reciprocal2((*png_ptr).file_gamma, (*png_ptr).screen_gamma);
                        }

                        PNG_BACKGROUND_GAMMA_UNIQUE => {
                            g = png_reciprocal((*png_ptr).background_gamma);
                            gs = png_reciprocal2(
                                (*png_ptr).background_gamma,
                                (*png_ptr).screen_gamma,
                            );
                        }

                        _ => {
                            png_error(png_ptr, c"invalid background gamma type".as_ptr());
                        }
                    }

                    g_sig = png_gamma_significant(g);
                    gs_sig = png_gamma_significant(gs);

                    if g_sig != 0 {
                        (*png_ptr).background_1.gray =
                            png_gamma_correct(png_ptr, (*png_ptr).background.gray as c_uint, g);
                    }

                    if gs_sig != 0 {
                        (*png_ptr).background.gray =
                            png_gamma_correct(png_ptr, (*png_ptr).background.gray as c_uint, gs);
                    }

                    if ((*png_ptr).background.red != (*png_ptr).background.green)
                        || ((*png_ptr).background.red != (*png_ptr).background.blue)
                        || ((*png_ptr).background.red != (*png_ptr).background.gray)
                    {
                        /* RGB or RGBA with color background */
                        if g_sig != 0 {
                            (*png_ptr).background_1.red =
                                png_gamma_correct(png_ptr, (*png_ptr).background.red as c_uint, g);

                            (*png_ptr).background_1.green = png_gamma_correct(
                                png_ptr,
                                (*png_ptr).background.green as c_uint,
                                g,
                            );

                            (*png_ptr).background_1.blue =
                                png_gamma_correct(png_ptr, (*png_ptr).background.blue as c_uint, g);
                        }

                        if gs_sig != 0 {
                            (*png_ptr).background.red =
                                png_gamma_correct(png_ptr, (*png_ptr).background.red as c_uint, gs);

                            (*png_ptr).background.green = png_gamma_correct(
                                png_ptr,
                                (*png_ptr).background.green as c_uint,
                                gs,
                            );

                            (*png_ptr).background.blue = png_gamma_correct(
                                png_ptr,
                                (*png_ptr).background.blue as c_uint,
                                gs,
                            );
                        }
                    } else {
                        /* GRAY, GRAY ALPHA, RGB, or RGBA with gray background */
                        (*png_ptr).background_1.red = (*png_ptr).background_1.gray;
                        (*png_ptr).background_1.green = (*png_ptr).background_1.gray;
                        (*png_ptr).background_1.blue = (*png_ptr).background_1.gray;

                        (*png_ptr).background.red = (*png_ptr).background.gray;
                        (*png_ptr).background.green = (*png_ptr).background.gray;
                        (*png_ptr).background.blue = (*png_ptr).background.gray;
                    }

                    /* The background is now in screen gamma: */
                    (*png_ptr).background_gamma_type = PNG_BACKGROUND_GAMMA_SCREEN as png_byte;
                } /* color_type != PNG_COLOR_TYPE_PALETTE */
            }
            /* png_ptr->transformations & PNG_BACKGROUND */
            else
            /* Transformation does not include PNG_BACKGROUND */
            if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
                /* RGB_TO_GRAY needs to have non-gamma-corrected values! */
                && (((*png_ptr).transformations & PNG_EXPAND) == 0
                    || ((*png_ptr).transformations & PNG_RGB_TO_GRAY) == 0)
            {
                let palette: png_colorp = (*png_ptr).palette;
                let num_palette: c_int = (*png_ptr).num_palette as c_int;
                let mut i: c_int;

                /* NOTE: there are other transformations ... */
                i = 0;
                while i < num_palette {
                    (*palette.offset(i as isize)).red = *(*png_ptr)
                        .gamma_table
                        .offset((*palette.offset(i as isize)).red as isize);
                    (*palette.offset(i as isize)).green = *(*png_ptr)
                        .gamma_table
                        .offset((*palette.offset(i as isize)).green as isize);
                    (*palette.offset(i as isize)).blue = *(*png_ptr)
                        .gamma_table
                        .offset((*palette.offset(i as isize)).blue as isize);
                    i += 1;
                }

                /* Done the gamma correction. */
                (*png_ptr).transformations &= !PNG_GAMMA;
            } /* color_type == PALETTE && !PNG_BACKGROUND transformation */
        }
        /* No GAMMA transformation (see the hanging else 4 lines above) */
        else if ((*png_ptr).transformations & PNG_COMPOSE) != 0
            && ((*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE)
        {
            let mut i: c_int;
            let istop: c_int = (*png_ptr).num_trans as c_int;
            let mut back: png_color = png_color::default();
            let palette: png_colorp = (*png_ptr).palette;

            back.red = (*png_ptr).background.red as png_byte;
            back.green = (*png_ptr).background.green as png_byte;
            back.blue = (*png_ptr).background.blue as png_byte;

            i = 0;
            while i < istop {
                if *(*png_ptr).trans_alpha.offset(i as isize) == 0 {
                    *palette.offset(i as isize) = back;
                } else if *(*png_ptr).trans_alpha.offset(i as isize) != 0xff {
                    /* The png_composite() macro is defined in png.h */
                    (*palette.offset(i as isize)).red = png_composite(
                        (*palette.offset(i as isize)).red as png_uint_16,
                        *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16,
                        back.red as png_uint_16,
                    );

                    (*palette.offset(i as isize)).green = png_composite(
                        (*palette.offset(i as isize)).green as png_uint_16,
                        *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16,
                        back.green as png_uint_16,
                    );

                    (*palette.offset(i as isize)).blue = png_composite(
                        (*palette.offset(i as isize)).blue as png_uint_16,
                        *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16,
                        back.blue as png_uint_16,
                    );
                }
                i += 1;
            }

            (*png_ptr).transformations &= !PNG_COMPOSE;
        }

        if ((*png_ptr).transformations & PNG_SHIFT) != 0
            && ((*png_ptr).transformations & PNG_EXPAND) == 0
            && ((*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE)
        {
            let mut i: c_int;
            let istop: c_int = (*png_ptr).num_palette as c_int;
            let mut shift: c_int = 8 - (*png_ptr).sig_bit.red as c_int;

            (*png_ptr).transformations &= !PNG_SHIFT;

            /* significant bits can be in the range 1 to 7 ... */
            if shift > 0 && shift < 8 {
                i = 0;
                while i < istop {
                    let mut component: c_int =
                        (*(*png_ptr).palette.offset(i as isize)).red as c_int;

                    component >>= shift;
                    (*(*png_ptr).palette.offset(i as isize)).red = component as png_byte;
                    i += 1;
                }
            }

            shift = 8 - (*png_ptr).sig_bit.green as c_int;
            if shift > 0 && shift < 8 {
                i = 0;
                while i < istop {
                    let mut component: c_int =
                        (*(*png_ptr).palette.offset(i as isize)).green as c_int;

                    component >>= shift;
                    (*(*png_ptr).palette.offset(i as isize)).green = component as png_byte;
                    i += 1;
                }
            }

            shift = 8 - (*png_ptr).sig_bit.blue as c_int;
            if shift > 0 && shift < 8 {
                i = 0;
                while i < istop {
                    let mut component: c_int =
                        (*(*png_ptr).palette.offset(i as isize)).blue as c_int;

                    component >>= shift;
                    (*(*png_ptr).palette.offset(i as isize)).blue = component as png_byte;
                    i += 1;
                }
            }
        }
    }
}

/* Modify the info structure to reflect the transformations. ... */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_transform_info(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
) {
    unsafe {
        if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
            && !(*info_ptr).palette.is_null()
            && !(*png_ptr).palette.is_null()
        {
            /* Sync info_ptr->palette with png_ptr->palette ... */
            memcpy(
                (*info_ptr).palette as *mut c_void,
                (*png_ptr).palette as *const c_void,
                PNG_MAX_PALETTE_LENGTH as usize * core::mem::size_of::<png_color>(),
            );
        }

        if ((*png_ptr).transformations & PNG_EXPAND) != 0 {
            if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
                /* This check must match what actually happens in
                 * png_do_expand_palette ...
                 */
                if (*png_ptr).num_trans > 0 {
                    (*info_ptr).color_type = PNG_COLOR_TYPE_RGB_ALPHA as png_byte;
                } else {
                    (*info_ptr).color_type = PNG_COLOR_TYPE_RGB as png_byte;
                }

                (*info_ptr).bit_depth = 8;
                (*info_ptr).num_trans = 0;

                if (*png_ptr).palette.is_null() {
                    png_error(png_ptr, c"Palette is NULL in indexed image".as_ptr());
                }
            } else {
                if (*png_ptr).num_trans != 0 {
                    if ((*png_ptr).transformations & PNG_EXPAND_tRNS) != 0 {
                        (*info_ptr).color_type |= PNG_COLOR_MASK_ALPHA as png_byte;
                    }
                }
                if (*info_ptr).bit_depth < 8 {
                    (*info_ptr).bit_depth = 8;
                }

                (*info_ptr).num_trans = 0;
            }
        }

        /* The following is almost certainly wrong unless the background value is in
         * the screen space!
         */
        if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
            (*info_ptr).background = (*png_ptr).background;
        }

        /* The following used to be conditional on PNG_GAMMA ... */
        (*info_ptr).gamma = (*png_ptr).file_gamma;

        if (*info_ptr).bit_depth == 16 {
            if ((*png_ptr).transformations & PNG_SCALE_16_TO_8) != 0 {
                (*info_ptr).bit_depth = 8;
            }

            if ((*png_ptr).transformations & PNG_16_TO_8) != 0 {
                (*info_ptr).bit_depth = 8;
            }
        }

        if ((*png_ptr).transformations & PNG_GRAY_TO_RGB) != 0 {
            (*info_ptr).color_type =
                ((*info_ptr).color_type as c_int | PNG_COLOR_MASK_COLOR) as png_byte;
        }

        if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) != 0 {
            (*info_ptr).color_type =
                ((*info_ptr).color_type as c_int & !PNG_COLOR_MASK_COLOR) as png_byte;
        }

        if ((*png_ptr).transformations & PNG_QUANTIZE) != 0 {
            if ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB
                || (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA)
                && !(*png_ptr).palette_lookup.is_null()
                && (*info_ptr).bit_depth == 8
            {
                (*info_ptr).color_type = PNG_COLOR_TYPE_PALETTE as png_byte;
            }
        }

        if ((*png_ptr).transformations & PNG_EXPAND_16) != 0
            && (*info_ptr).bit_depth == 8
            && (*info_ptr).color_type as c_int != PNG_COLOR_TYPE_PALETTE
        {
            (*info_ptr).bit_depth = 16;
        }

        if ((*png_ptr).transformations & PNG_PACK) != 0 && (*info_ptr).bit_depth < 8 {
            (*info_ptr).bit_depth = 8;
        }

        if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            (*info_ptr).channels = 1;
        } else if ((*info_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
            (*info_ptr).channels = 3;
        } else {
            (*info_ptr).channels = 1;
        }

        if ((*png_ptr).transformations & PNG_STRIP_ALPHA) != 0 {
            (*info_ptr).color_type =
                ((*info_ptr).color_type as c_int & !PNG_COLOR_MASK_ALPHA) as png_byte;
            (*info_ptr).num_trans = 0;
        }

        if ((*info_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0 {
            (*info_ptr).channels += 1;
        }

        /* STRIP_ALPHA and FILLER allowed:  MASK_ALPHA bit stripped above */
        if ((*png_ptr).transformations & PNG_FILLER) != 0
            && ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB
                || (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY)
        {
            (*info_ptr).channels += 1;
            /* If adding a true alpha channel not just filler */
            if ((*png_ptr).transformations & PNG_ADD_ALPHA) != 0 {
                (*info_ptr).color_type |= PNG_COLOR_MASK_ALPHA as png_byte;
            }
        }

        if ((*png_ptr).transformations & PNG_USER_TRANSFORM) != 0 {
            if (*png_ptr).user_transform_depth != 0 {
                (*info_ptr).bit_depth = (*png_ptr).user_transform_depth;
            }

            if (*png_ptr).user_transform_channels != 0 {
                (*info_ptr).channels = (*png_ptr).user_transform_channels;
            }
        }

        (*info_ptr).pixel_depth =
            ((*info_ptr).channels as c_int * (*info_ptr).bit_depth as c_int) as png_byte;

        (*info_ptr).rowbytes =
            PNG_ROWBYTES((*info_ptr).pixel_depth as usize, (*info_ptr).width as usize);

        /* Adding in 1.5.4: cache the above value in png_struct ... */
        (*png_ptr).info_rowbytes = (*info_ptr).rowbytes;
    }
}

/* Unpack pixels of 1, 2, or 4 bits per pixel into 1 byte per pixel ... */
unsafe fn png_do_unpack(row_info: png_row_infop, row: png_bytep) {
    unsafe {
        if (*row_info).bit_depth < 8 {
            let mut i: png_uint_32;
            let row_width: png_uint_32 = (*row_info).width;

            match (*row_info).bit_depth {
                1 => {
                    let mut sp: png_bytep = row.add(((row_width - 1) >> 3) as usize);
                    let mut dp: png_bytep = row.add(row_width as usize - 1);
                    let mut shift: png_uint_32 = 7u32 - ((row_width + 7u32) & 0x07);
                    i = 0;
                    while i < row_width {
                        *dp = ((*sp >> shift) & 0x01) as png_byte;

                        if shift == 7 {
                            shift = 0;
                            sp = sp.sub(1);
                        } else {
                            shift += 1;
                        }

                        dp = dp.sub(1);
                        i += 1;
                    }
                }

                2 => {
                    let mut sp: png_bytep = row.add(((row_width - 1) >> 2) as usize);
                    let mut dp: png_bytep = row.add(row_width as usize - 1);
                    let mut shift: png_uint_32 = (3u32 - ((row_width + 3u32) & 0x03)) << 1;
                    i = 0;
                    while i < row_width {
                        *dp = ((*sp >> shift) & 0x03) as png_byte;

                        if shift == 6 {
                            shift = 0;
                            sp = sp.sub(1);
                        } else {
                            shift += 2;
                        }

                        dp = dp.sub(1);
                        i += 1;
                    }
                }

                4 => {
                    let mut sp: png_bytep = row.add(((row_width - 1) >> 1) as usize);
                    let mut dp: png_bytep = row.add(row_width as usize - 1);
                    let mut shift: png_uint_32 = (1u32 - ((row_width + 1u32) & 0x01)) << 2;
                    i = 0;
                    while i < row_width {
                        *dp = ((*sp >> shift) & 0x0f) as png_byte;

                        if shift == 4 {
                            shift = 0;
                            sp = sp.sub(1);
                        } else {
                            shift = 4;
                        }

                        dp = dp.sub(1);
                        i += 1;
                    }
                }

                _ => {}
            }
            (*row_info).bit_depth = 8;
            (*row_info).pixel_depth = (8 * (*row_info).channels as c_int) as png_byte;
            (*row_info).rowbytes = row_width as usize * (*row_info).channels as usize;
        }
    }
}

/* Reverse the effects of png_do_shift. ... */
unsafe fn png_do_unshift(row_info: png_row_infop, row: png_bytep, sig_bits: png_const_color_8p) {
    unsafe {
        let color_type: c_int;

        /* The palette case has already been handled in the _init routine. */
        color_type = (*row_info).color_type as c_int;

        if color_type != PNG_COLOR_TYPE_PALETTE {
            let mut shift: [c_int; 4] = [0; 4];
            let mut channels: c_int = 0;
            let bit_depth: c_int = (*row_info).bit_depth as c_int;

            if (color_type & PNG_COLOR_MASK_COLOR) != 0 {
                shift[channels as usize] = bit_depth - (*sig_bits).red as c_int;
                channels += 1;
                shift[channels as usize] = bit_depth - (*sig_bits).green as c_int;
                channels += 1;
                shift[channels as usize] = bit_depth - (*sig_bits).blue as c_int;
                channels += 1;
            } else {
                shift[channels as usize] = bit_depth - (*sig_bits).gray as c_int;
                channels += 1;
            }

            if (color_type & PNG_COLOR_MASK_ALPHA) != 0 {
                shift[channels as usize] = bit_depth - (*sig_bits).alpha as c_int;
                channels += 1;
            }

            {
                let mut c: c_int;
                let mut have_shift: c_int;

                c = 0;
                have_shift = 0;
                while c < channels {
                    /* A shift of more than the bit depth is an error condition ... */
                    if shift[c as usize] <= 0 || shift[c as usize] >= bit_depth {
                        shift[c as usize] = 0;
                    } else {
                        have_shift = 1;
                    }
                    c += 1;
                }

                if have_shift == 0 {
                    return;
                }
            }

            match bit_depth {
                2 => {
                    /* Must be 2bpp gray */
                    let mut bp: png_bytep = row;
                    let bp_end: png_bytep = bp.add((*row_info).rowbytes);

                    while (bp as usize) < (bp_end as usize) {
                        let b: c_int = ((*bp >> 1) & 0x55) as c_int;
                        *bp = b as png_byte;
                        bp = bp.add(1);
                    }
                }

                4 => {
                    /* Must be 4bpp gray */
                    let mut bp: png_bytep = row;
                    let bp_end: png_bytep = bp.add((*row_info).rowbytes);
                    let gray_shift: c_int = shift[0];
                    let mut mask: c_int = 0xf >> gray_shift;

                    mask |= mask << 4;

                    while (bp as usize) < (bp_end as usize) {
                        let b: c_int = ((*bp >> gray_shift) & mask as u8) as c_int;
                        *bp = b as png_byte;
                        bp = bp.add(1);
                    }
                }

                8 => {
                    /* Single byte components, G, GA, RGB, RGBA */
                    let mut bp: png_bytep = row;
                    let bp_end: png_bytep = bp.add((*row_info).rowbytes);
                    let mut channel: c_int = 0;

                    while (bp as usize) < (bp_end as usize) {
                        let b: c_int = (*bp >> shift[channel as usize]) as c_int;
                        channel += 1;
                        if channel >= channels {
                            channel = 0;
                        }
                        *bp = b as png_byte;
                        bp = bp.add(1);
                    }
                }

                16 => {
                    /* Double byte components, G, GA, RGB, RGBA */
                    let mut bp: png_bytep = row;
                    let bp_end: png_bytep = bp.add((*row_info).rowbytes);
                    let mut channel: c_int = 0;

                    while (bp as usize) < (bp_end as usize) {
                        let mut value: c_int = ((*bp.add(0) as c_int) << 8) + *bp.add(1) as c_int;

                        value >>= shift[channel as usize];
                        channel += 1;
                        if channel >= channels {
                            channel = 0;
                        }
                        *bp = (value >> 8) as png_byte;
                        bp = bp.add(1);
                        *bp = value as png_byte;
                        bp = bp.add(1);
                    }
                }

                /* default: Must be 1bpp gray: should not be here! NOTREACHED */
                _ => {}
            }
        }
    }
}

/* Scale rows of bit depth 16 down to 8 accurately */
unsafe fn png_do_scale_16_to_8(row_info: png_row_infop, row: png_bytep) {
    unsafe {
        if (*row_info).bit_depth == 16 {
            let mut sp: png_bytep = row; /* source */
            let mut dp: png_bytep = row; /* destination */
            let ep: png_bytep = sp.add((*row_info).rowbytes); /* end+1 */

            while (sp as usize) < (ep as usize) {
                /* ... (see C comment for the derivation) */
                let mut tmp: png_int_32 = *sp as png_int_32; /* must be signed! */
                sp = sp.add(1);
                tmp += (((*sp as c_int - tmp) + 128) * 65535) >> 24;
                sp = sp.add(1);
                *dp = tmp as png_byte;
                dp = dp.add(1);
            }

            (*row_info).bit_depth = 8;
            (*row_info).pixel_depth = (8 * (*row_info).channels as c_int) as png_byte;
            (*row_info).rowbytes = (*row_info).width as usize * (*row_info).channels as usize;
        }
    }
}

/* Simply discard the low byte. ... */
unsafe fn png_do_chop(row_info: png_row_infop, row: png_bytep) {
    unsafe {
        if (*row_info).bit_depth == 16 {
            let mut sp: png_bytep = row; /* source */
            let mut dp: png_bytep = row; /* destination */
            let ep: png_bytep = sp.add((*row_info).rowbytes); /* end+1 */

            while (sp as usize) < (ep as usize) {
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(2); /* skip low byte */
            }

            (*row_info).bit_depth = 8;
            (*row_info).pixel_depth = (8 * (*row_info).channels as c_int) as png_byte;
            (*row_info).rowbytes = (*row_info).width as usize * (*row_info).channels as usize;
        }
    }
}

unsafe fn png_do_read_swap_alpha(row_info: png_row_infop, row: png_bytep) {
    unsafe {
        let row_width: png_uint_32 = (*row_info).width;

        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
            /* This converts from RGBA to ARGB */
            if (*row_info).bit_depth == 8 {
                let mut sp: png_bytep = row.add((*row_info).rowbytes);
                let mut dp: png_bytep = sp;
                let mut save: png_byte;
                let mut i: png_uint_32;

                i = 0;
                while i < row_width {
                    sp = sp.sub(1);
                    save = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = save;
                    i += 1;
                }
            }
            /* This converts from RRGGBBAA to AARRGGBB */
            else {
                let mut sp: png_bytep = row.add((*row_info).rowbytes);
                let mut dp: png_bytep = sp;
                let mut save: [png_byte; 2] = [0; 2];
                let mut i: png_uint_32;

                i = 0;
                while i < row_width {
                    sp = sp.sub(1);
                    save[0] = *sp;
                    sp = sp.sub(1);
                    save[1] = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = save[0];
                    dp = dp.sub(1);
                    *dp = save[1];
                    i += 1;
                }
            }
        } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
            /* This converts from GA to AG */
            if (*row_info).bit_depth == 8 {
                let mut sp: png_bytep = row.add((*row_info).rowbytes);
                let mut dp: png_bytep = sp;
                let mut save: png_byte;
                let mut i: png_uint_32;

                i = 0;
                while i < row_width {
                    sp = sp.sub(1);
                    save = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = save;
                    i += 1;
                }
            }
            /* This converts from GGAA to AAGG */
            else {
                let mut sp: png_bytep = row.add((*row_info).rowbytes);
                let mut dp: png_bytep = sp;
                let mut save: [png_byte; 2] = [0; 2];
                let mut i: png_uint_32;

                i = 0;
                while i < row_width {
                    sp = sp.sub(1);
                    save[0] = *sp;
                    sp = sp.sub(1);
                    save[1] = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = save[0];
                    dp = dp.sub(1);
                    *dp = save[1];
                    i += 1;
                }
            }
        }
    }
}

unsafe fn png_do_read_invert_alpha(row_info: png_row_infop, row: png_bytep) {
    unsafe {
        let row_width: png_uint_32;

        row_width = (*row_info).width;
        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
            if (*row_info).bit_depth == 8 {
                /* This inverts the alpha channel in RGBA */
                let mut sp: png_bytep = row.add((*row_info).rowbytes);
                let mut dp: png_bytep = sp;
                let mut i: png_uint_32;

                i = 0;
                while i < row_width {
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = (255 - *sp as c_int) as png_byte;

                    /* This does nothing: ... We can replace it with: */
                    sp = sp.sub(3);
                    dp = sp;
                    i += 1;
                }
            }
            /* This inverts the alpha channel in RRGGBBAA */
            else {
                let mut sp: png_bytep = row.add((*row_info).rowbytes);
                let mut dp: png_bytep = sp;
                let mut i: png_uint_32;

                i = 0;
                while i < row_width {
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = (255 - *sp as c_int) as png_byte;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = (255 - *sp as c_int) as png_byte;

                    /* This does nothing: ... We can replace it with: */
                    sp = sp.sub(6);
                    dp = sp;
                    i += 1;
                }
            }
        } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
            if (*row_info).bit_depth == 8 {
                /* This inverts the alpha channel in GA */
                let mut sp: png_bytep = row.add((*row_info).rowbytes);
                let mut dp: png_bytep = sp;
                let mut i: png_uint_32;

                i = 0;
                while i < row_width {
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = (255 - *sp as c_int) as png_byte;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    i += 1;
                }
            } else {
                /* This inverts the alpha channel in GGAA */
                let mut sp: png_bytep = row.add((*row_info).rowbytes);
                let mut dp: png_bytep = sp;
                let mut i: png_uint_32;

                i = 0;
                while i < row_width {
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = (255 - *sp as c_int) as png_byte;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = (255 - *sp as c_int) as png_byte;
                    /* *(--dp) = *(--sp); *(--dp) = *(--sp); (commented out in C) */
                    sp = sp.sub(2);
                    dp = sp;
                    i += 1;
                }
            }
        }
    }
}

/* Add filler channel if we have RGB color */
unsafe fn png_do_read_filler(
    row_info: png_row_infop,
    row: png_bytep,
    filler: png_uint_32,
    flags: png_uint_32,
) {
    unsafe {
        let mut i: png_uint_32;
        let row_width: png_uint_32 = (*row_info).width;

        let hi_filler: png_byte = (filler >> 8) as png_byte;
        let lo_filler: png_byte = filler as png_byte;

        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY {
            if (*row_info).bit_depth == 8 {
                if (flags & PNG_FLAG_FILLER_AFTER) != 0 {
                    /* This changes the data from G to GX */
                    let mut sp: png_bytep = row.add(row_width as usize);
                    let mut dp: png_bytep = sp.add(row_width as usize);
                    i = 1;
                    while i < row_width {
                        dp = dp.sub(1);
                        *dp = lo_filler;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        i += 1;
                    }
                    dp = dp.sub(1);
                    *dp = lo_filler;
                    (*row_info).channels = 2;
                    (*row_info).pixel_depth = 16;
                    (*row_info).rowbytes = row_width as usize * 2;
                } else {
                    /* This changes the data from G to XG */
                    let mut sp: png_bytep = row.add(row_width as usize);
                    let mut dp: png_bytep = sp.add(row_width as usize);
                    i = 0;
                    while i < row_width {
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        *dp = lo_filler;
                        i += 1;
                    }
                    (*row_info).channels = 2;
                    (*row_info).pixel_depth = 16;
                    (*row_info).rowbytes = row_width as usize * 2;
                }
            } else if (*row_info).bit_depth == 16 {
                if (flags & PNG_FLAG_FILLER_AFTER) != 0 {
                    /* This changes the data from GG to GGXX */
                    let mut sp: png_bytep = row.add(row_width as usize * 2);
                    let mut dp: png_bytep = sp.add(row_width as usize * 2);
                    i = 1;
                    while i < row_width {
                        dp = dp.sub(1);
                        *dp = lo_filler;
                        dp = dp.sub(1);
                        *dp = hi_filler;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        i += 1;
                    }
                    dp = dp.sub(1);
                    *dp = lo_filler;
                    dp = dp.sub(1);
                    *dp = hi_filler;
                    (*row_info).channels = 2;
                    (*row_info).pixel_depth = 32;
                    (*row_info).rowbytes = row_width as usize * 4;
                } else {
                    /* This changes the data from GG to XXGG */
                    let mut sp: png_bytep = row.add(row_width as usize * 2);
                    let mut dp: png_bytep = sp.add(row_width as usize * 2);
                    i = 0;
                    while i < row_width {
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        *dp = lo_filler;
                        dp = dp.sub(1);
                        *dp = hi_filler;
                        i += 1;
                    }
                    (*row_info).channels = 2;
                    (*row_info).pixel_depth = 32;
                    (*row_info).rowbytes = row_width as usize * 4;
                }
            }
        }
        /* COLOR_TYPE == GRAY */
        else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB {
            if (*row_info).bit_depth == 8 {
                if (flags & PNG_FLAG_FILLER_AFTER) != 0 {
                    /* This changes the data from RGB to RGBX */
                    let mut sp: png_bytep = row.add(row_width as usize * 3);
                    let mut dp: png_bytep = sp.add(row_width as usize);
                    i = 1;
                    while i < row_width {
                        dp = dp.sub(1);
                        *dp = lo_filler;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        i += 1;
                    }
                    dp = dp.sub(1);
                    *dp = lo_filler;
                    (*row_info).channels = 4;
                    (*row_info).pixel_depth = 32;
                    (*row_info).rowbytes = row_width as usize * 4;
                } else {
                    /* This changes the data from RGB to XRGB */
                    let mut sp: png_bytep = row.add(row_width as usize * 3);
                    let mut dp: png_bytep = sp.add(row_width as usize);
                    i = 0;
                    while i < row_width {
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        *dp = lo_filler;
                        i += 1;
                    }
                    (*row_info).channels = 4;
                    (*row_info).pixel_depth = 32;
                    (*row_info).rowbytes = row_width as usize * 4;
                }
            } else if (*row_info).bit_depth == 16 {
                if (flags & PNG_FLAG_FILLER_AFTER) != 0 {
                    /* This changes the data from RRGGBB to RRGGBBXX */
                    let mut sp: png_bytep = row.add(row_width as usize * 6);
                    let mut dp: png_bytep = sp.add(row_width as usize * 2);
                    i = 1;
                    while i < row_width {
                        dp = dp.sub(1);
                        *dp = lo_filler;
                        dp = dp.sub(1);
                        *dp = hi_filler;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        i += 1;
                    }
                    dp = dp.sub(1);
                    *dp = lo_filler;
                    dp = dp.sub(1);
                    *dp = hi_filler;
                    (*row_info).channels = 4;
                    (*row_info).pixel_depth = 64;
                    (*row_info).rowbytes = row_width as usize * 8;
                } else {
                    /* This changes the data from RRGGBB to XXRRGGBB */
                    let mut sp: png_bytep = row.add(row_width as usize * 6);
                    let mut dp: png_bytep = sp.add(row_width as usize * 2);
                    i = 0;
                    while i < row_width {
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        *dp = lo_filler;
                        dp = dp.sub(1);
                        *dp = hi_filler;
                        i += 1;
                    }

                    (*row_info).channels = 4;
                    (*row_info).pixel_depth = 64;
                    (*row_info).rowbytes = row_width as usize * 8;
                }
            }
        } /* COLOR_TYPE == RGB */
    }
}

/* Expand grayscale files to RGB, with or without alpha */
unsafe fn png_do_gray_to_rgb(row_info: png_row_infop, row: png_bytep) {
    unsafe {
        let mut i: png_uint_32;
        let row_width: png_uint_32 = (*row_info).width;

        if (*row_info).bit_depth >= 8
            && ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0
        {
            if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY {
                if (*row_info).bit_depth == 8 {
                    /* This changes G to RGB */
                    let mut sp: png_bytep = row.add(row_width as usize - 1);
                    let mut dp: png_bytep = sp.add(row_width as usize * 2);
                    i = 0;
                    while i < row_width {
                        *dp = *sp;
                        dp = dp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        i += 1;
                    }
                } else {
                    /* This changes GG to RRGGBB */
                    let mut sp: png_bytep = row.add(row_width as usize * 2 - 1);
                    let mut dp: png_bytep = sp.add(row_width as usize * 4);
                    i = 0;
                    while i < row_width {
                        *dp = *sp;
                        dp = dp.sub(1);
                        *dp = *sp.sub(1);
                        dp = dp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        *dp = *sp.sub(1);
                        dp = dp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        i += 1;
                    }
                }
            } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
                if (*row_info).bit_depth == 8 {
                    /* This changes GA to RGBA */
                    let mut sp: png_bytep = row.add(row_width as usize * 2 - 1);
                    let mut dp: png_bytep = sp.add(row_width as usize * 2);
                    i = 0;
                    while i < row_width {
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        i += 1;
                    }
                } else {
                    /* This changes GGAA to RRGGBBAA */
                    let mut sp: png_bytep = row.add(row_width as usize * 4 - 1);
                    let mut dp: png_bytep = sp.add(row_width as usize * 4);
                    i = 0;
                    while i < row_width {
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        *dp = *sp.sub(1);
                        dp = dp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        *dp = *sp.sub(1);
                        dp = dp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        i += 1;
                    }
                }
            }
            (*row_info).channels = ((*row_info).channels as c_int + 2) as png_byte;
            (*row_info).color_type |= PNG_COLOR_MASK_COLOR as png_byte;
            (*row_info).pixel_depth =
                ((*row_info).channels as c_int * (*row_info).bit_depth as c_int) as png_byte;
            (*row_info).rowbytes =
                PNG_ROWBYTES((*row_info).pixel_depth as usize, row_width as usize);
        }
    }
}

/* Reduce RGB files to grayscale, with or without alpha ... */
unsafe fn png_do_rgb_to_gray(
    png_ptr: png_structrp,
    row_info: png_row_infop,
    row: png_bytep,
) -> c_int {
    unsafe {
        let mut rgb_error: c_int = 0;

        if ((*row_info).color_type as c_int & PNG_COLOR_MASK_PALETTE) == 0
            && ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0
        {
            let rc: png_uint_32 = (*png_ptr).rgb_to_gray_red_coeff as png_uint_32;
            let gc: png_uint_32 = (*png_ptr).rgb_to_gray_green_coeff as png_uint_32;
            let bc: png_uint_32 = 32768 - rc - gc;
            let row_width: png_uint_32 = (*row_info).width;
            let have_alpha: c_int =
                (((*row_info).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0) as c_int;

            if (*row_info).bit_depth == 8 {
                if !(*png_ptr).gamma_from_1.is_null() && !(*png_ptr).gamma_to_1.is_null() {
                    let mut sp: png_bytep = row;
                    let mut dp: png_bytep = row;
                    let mut i: png_uint_32;

                    i = 0;
                    while i < row_width {
                        let mut red: png_byte = *sp;
                        sp = sp.add(1);
                        let mut green: png_byte = *sp;
                        sp = sp.add(1);
                        let mut blue: png_byte = *sp;
                        sp = sp.add(1);

                        if red != green || red != blue {
                            red = *(*png_ptr).gamma_to_1.offset(red as isize);
                            green = *(*png_ptr).gamma_to_1.offset(green as isize);
                            blue = *(*png_ptr).gamma_to_1.offset(blue as isize);

                            rgb_error |= 1;
                            *dp = *(*png_ptr).gamma_from_1.offset(
                                ((rc * red as png_uint_32
                                    + gc * green as png_uint_32
                                    + bc * blue as png_uint_32
                                    + 16384)
                                    >> 15) as isize,
                            );
                            dp = dp.add(1);
                        } else {
                            /* If there is no overall correction the table will not be set. */
                            if !(*png_ptr).gamma_table.is_null() {
                                red = *(*png_ptr).gamma_table.offset(red as isize);
                            }

                            *dp = red;
                            dp = dp.add(1);
                        }

                        if have_alpha != 0 {
                            *dp = *sp;
                            dp = dp.add(1);
                            sp = sp.add(1);
                        }
                        i += 1;
                    }
                } else {
                    let mut sp: png_bytep = row;
                    let mut dp: png_bytep = row;
                    let mut i: png_uint_32;

                    i = 0;
                    while i < row_width {
                        let red: png_byte = *sp;
                        sp = sp.add(1);
                        let green: png_byte = *sp;
                        sp = sp.add(1);
                        let blue: png_byte = *sp;
                        sp = sp.add(1);

                        if red != green || red != blue {
                            rgb_error |= 1;
                            /* NOTE: this is the historical approach which simply
                             * truncates the results.
                             */
                            *dp = ((rc * red as png_uint_32
                                + gc * green as png_uint_32
                                + bc * blue as png_uint_32)
                                >> 15) as png_byte;
                            dp = dp.add(1);
                        } else {
                            *dp = red;
                            dp = dp.add(1);
                        }

                        if have_alpha != 0 {
                            *dp = *sp;
                            dp = dp.add(1);
                            sp = sp.add(1);
                        }
                        i += 1;
                    }
                }
            } else
            /* RGB bit_depth == 16 */
            {
                if !(*png_ptr).gamma_16_to_1.is_null() && !(*png_ptr).gamma_16_from_1.is_null() {
                    let mut sp: png_bytep = row;
                    let mut dp: png_bytep = row;
                    let mut i: png_uint_32;

                    i = 0;
                    while i < row_width {
                        let red: png_uint_16;
                        let green: png_uint_16;
                        let blue: png_uint_16;
                        let w: png_uint_16;
                        let mut hi: png_byte;
                        let mut lo: png_byte;

                        hi = *sp;
                        sp = sp.add(1);
                        lo = *sp;
                        sp = sp.add(1);
                        red = (((hi as png_uint_16) << 8) | (lo as png_uint_16)) as png_uint_16;
                        hi = *sp;
                        sp = sp.add(1);
                        lo = *sp;
                        sp = sp.add(1);
                        green = (((hi as png_uint_16) << 8) | (lo as png_uint_16)) as png_uint_16;
                        hi = *sp;
                        sp = sp.add(1);
                        lo = *sp;
                        sp = sp.add(1);
                        blue = (((hi as png_uint_16) << 8) | (lo as png_uint_16)) as png_uint_16;

                        if red == green && red == blue {
                            if !(*png_ptr).gamma_16_table.is_null() {
                                w = *(*(*png_ptr)
                                    .gamma_16_table
                                    .offset(((red & 0xff) >> (*png_ptr).gamma_shift) as isize))
                                .offset((red >> 8) as isize);
                            } else {
                                w = red;
                            }
                        } else {
                            let red_1: png_uint_16 = *(*(*png_ptr)
                                .gamma_16_to_1
                                .offset(((red & 0xff) >> (*png_ptr).gamma_shift) as isize))
                            .offset((red >> 8) as isize);
                            let green_1: png_uint_16 = *(*(*png_ptr)
                                .gamma_16_to_1
                                .offset(((green & 0xff) >> (*png_ptr).gamma_shift) as isize))
                            .offset((green >> 8) as isize);
                            let blue_1: png_uint_16 = *(*(*png_ptr)
                                .gamma_16_to_1
                                .offset(((blue & 0xff) >> (*png_ptr).gamma_shift) as isize))
                            .offset((blue >> 8) as isize);
                            let gray16: png_uint_16 = ((rc * red_1 as png_uint_32
                                + gc * green_1 as png_uint_32
                                + bc * blue_1 as png_uint_32
                                + 16384)
                                >> 15)
                                as png_uint_16;
                            w = *(*(*png_ptr)
                                .gamma_16_from_1
                                .offset(((gray16 & 0xff) >> (*png_ptr).gamma_shift) as isize))
                            .offset((gray16 >> 8) as isize);
                            rgb_error |= 1;
                        }

                        *dp = ((w >> 8) & 0xff) as png_byte;
                        dp = dp.add(1);
                        *dp = (w & 0xff) as png_byte;
                        dp = dp.add(1);

                        if have_alpha != 0 {
                            *dp = *sp;
                            dp = dp.add(1);
                            sp = sp.add(1);
                            *dp = *sp;
                            dp = dp.add(1);
                            sp = sp.add(1);
                        }
                        i += 1;
                    }
                } else {
                    let mut sp: png_bytep = row;
                    let mut dp: png_bytep = row;
                    let mut i: png_uint_32;

                    i = 0;
                    while i < row_width {
                        let red: png_uint_16;
                        let green: png_uint_16;
                        let blue: png_uint_16;
                        let gray16: png_uint_16;
                        let mut hi: png_byte;
                        let mut lo: png_byte;

                        hi = *sp;
                        sp = sp.add(1);
                        lo = *sp;
                        sp = sp.add(1);
                        red = (((hi as png_uint_16) << 8) | (lo as png_uint_16)) as png_uint_16;
                        hi = *sp;
                        sp = sp.add(1);
                        lo = *sp;
                        sp = sp.add(1);
                        green = (((hi as png_uint_16) << 8) | (lo as png_uint_16)) as png_uint_16;
                        hi = *sp;
                        sp = sp.add(1);
                        lo = *sp;
                        sp = sp.add(1);
                        blue = (((hi as png_uint_16) << 8) | (lo as png_uint_16)) as png_uint_16;

                        if red != green || red != blue {
                            rgb_error |= 1;
                        }

                        /* From 1.5.5 in the 16-bit case do the accurate conversion ... */
                        gray16 = ((rc * red as png_uint_32
                            + gc * green as png_uint_32
                            + bc * blue as png_uint_32
                            + 16384)
                            >> 15) as png_uint_16;
                        *dp = ((gray16 >> 8) & 0xff) as png_byte;
                        dp = dp.add(1);
                        *dp = (gray16 & 0xff) as png_byte;
                        dp = dp.add(1);

                        if have_alpha != 0 {
                            *dp = *sp;
                            dp = dp.add(1);
                            sp = sp.add(1);
                            *dp = *sp;
                            dp = dp.add(1);
                            sp = sp.add(1);
                        }
                        i += 1;
                    }
                }
            }

            (*row_info).channels = ((*row_info).channels as c_int - 2) as png_byte;
            (*row_info).color_type =
                ((*row_info).color_type as c_int & !PNG_COLOR_MASK_COLOR) as png_byte;
            (*row_info).pixel_depth =
                ((*row_info).channels as c_int * (*row_info).bit_depth as c_int) as png_byte;
            (*row_info).rowbytes =
                PNG_ROWBYTES((*row_info).pixel_depth as usize, row_width as usize);
        }
        rgb_error
    }
}

/* Replace any alpha or transparency with the supplied background color. ... */
unsafe fn png_do_compose(row_info: png_row_infop, row: png_bytep, png_ptr: png_structrp) {
    unsafe {
        let gamma_table: png_const_bytep = (*png_ptr).gamma_table;
        let gamma_from_1: png_const_bytep = (*png_ptr).gamma_from_1;
        let gamma_to_1: png_const_bytep = (*png_ptr).gamma_to_1;
        let gamma_16: png_uint_16pp = (*png_ptr).gamma_16_table;
        let gamma_16_from_1: png_uint_16pp = (*png_ptr).gamma_16_from_1;
        let gamma_16_to_1: png_uint_16pp = (*png_ptr).gamma_16_to_1;
        let gamma_shift: c_int = (*png_ptr).gamma_shift;
        let optimize: c_int = (((*png_ptr).flags & PNG_FLAG_OPTIMIZE_ALPHA) != 0) as c_int;

        let mut sp: png_bytep;
        let mut i: png_uint_32;
        let row_width: png_uint_32 = (*row_info).width;
        let mut shift: c_int;

        match (*row_info).color_type as c_int {
            PNG_COLOR_TYPE_GRAY => {
                match (*row_info).bit_depth {
                    1 => {
                        sp = row;
                        shift = 7;
                        i = 0;
                        while i < row_width {
                            if (((*sp >> shift) & 0x01) as png_uint_16)
                                == (*png_ptr).trans_color.gray
                            {
                                let mut tmp: c_uint =
                                    (*sp & (0x7f7f >> (7 - shift)) as u8) as c_uint;
                                tmp |= ((*png_ptr).background.gray as c_uint) << shift;
                                *sp = (tmp & 0xff) as png_byte;
                            }

                            if shift == 0 {
                                shift = 7;
                                sp = sp.add(1);
                            } else {
                                shift -= 1;
                            }
                            i += 1;
                        }
                    }

                    2 => {
                        if !gamma_table.is_null() {
                            sp = row;
                            shift = 6;
                            i = 0;
                            while i < row_width {
                                if (((*sp >> shift) & 0x03) as png_uint_16)
                                    == (*png_ptr).trans_color.gray
                                {
                                    let mut tmp: c_uint =
                                        (*sp & (0x3f3f >> (6 - shift)) as u8) as c_uint;
                                    tmp |= ((*png_ptr).background.gray as c_uint) << shift;
                                    *sp = (tmp & 0xff) as png_byte;
                                } else {
                                    let p: c_uint = ((*sp >> shift) & 0x03) as c_uint;
                                    let g: c_uint = ((*gamma_table
                                        .offset((p | (p << 2) | (p << 4) | (p << 6)) as isize)
                                        as c_uint)
                                        >> 6)
                                        & 0x03;
                                    let mut tmp: c_uint =
                                        (*sp & (0x3f3f >> (6 - shift)) as u8) as c_uint;
                                    tmp |= g << shift;
                                    *sp = (tmp & 0xff) as png_byte;
                                }

                                if shift == 0 {
                                    shift = 6;
                                    sp = sp.add(1);
                                } else {
                                    shift -= 2;
                                }
                                i += 1;
                            }
                        } else {
                            sp = row;
                            shift = 6;
                            i = 0;
                            while i < row_width {
                                if (((*sp >> shift) & 0x03) as png_uint_16)
                                    == (*png_ptr).trans_color.gray
                                {
                                    let mut tmp: c_uint =
                                        (*sp & (0x3f3f >> (6 - shift)) as u8) as c_uint;
                                    tmp |= ((*png_ptr).background.gray as c_uint) << shift;
                                    *sp = (tmp & 0xff) as png_byte;
                                }

                                if shift == 0 {
                                    shift = 6;
                                    sp = sp.add(1);
                                } else {
                                    shift -= 2;
                                }
                                i += 1;
                            }
                        }
                    }

                    4 => {
                        if !gamma_table.is_null() {
                            sp = row;
                            shift = 4;
                            i = 0;
                            while i < row_width {
                                if (((*sp >> shift) & 0x0f) as png_uint_16)
                                    == (*png_ptr).trans_color.gray
                                {
                                    let mut tmp: c_uint =
                                        (*sp & (0x0f0f >> (4 - shift)) as u8) as c_uint;
                                    tmp |= ((*png_ptr).background.gray as c_uint) << shift;
                                    *sp = (tmp & 0xff) as png_byte;
                                } else {
                                    let p: c_uint = ((*sp >> shift) & 0x0f) as c_uint;
                                    let g: c_uint = ((*gamma_table.offset((p | (p << 4)) as isize)
                                        as c_uint)
                                        >> 4)
                                        & 0x0f;
                                    let mut tmp: c_uint =
                                        (*sp & (0x0f0f >> (4 - shift)) as u8) as c_uint;
                                    tmp |= g << shift;
                                    *sp = (tmp & 0xff) as png_byte;
                                }

                                if shift == 0 {
                                    shift = 4;
                                    sp = sp.add(1);
                                } else {
                                    shift -= 4;
                                }
                                i += 1;
                            }
                        } else {
                            sp = row;
                            shift = 4;
                            i = 0;
                            while i < row_width {
                                if (((*sp >> shift) & 0x0f) as png_uint_16)
                                    == (*png_ptr).trans_color.gray
                                {
                                    let mut tmp: c_uint =
                                        (*sp & (0x0f0f >> (4 - shift)) as u8) as c_uint;
                                    tmp |= ((*png_ptr).background.gray as c_uint) << shift;
                                    *sp = (tmp & 0xff) as png_byte;
                                }

                                if shift == 0 {
                                    shift = 4;
                                    sp = sp.add(1);
                                } else {
                                    shift -= 4;
                                }
                                i += 1;
                            }
                        }
                    }

                    8 => {
                        if !gamma_table.is_null() {
                            sp = row;
                            i = 0;
                            while i < row_width {
                                if *sp as png_uint_16 == (*png_ptr).trans_color.gray {
                                    *sp = (*png_ptr).background.gray as png_byte;
                                } else {
                                    *sp = *gamma_table.offset(*sp as isize);
                                }
                                i += 1;
                                sp = sp.add(1);
                            }
                        } else {
                            sp = row;
                            i = 0;
                            while i < row_width {
                                if *sp as png_uint_16 == (*png_ptr).trans_color.gray {
                                    *sp = (*png_ptr).background.gray as png_byte;
                                }
                                i += 1;
                                sp = sp.add(1);
                            }
                        }
                    }

                    16 => {
                        if !gamma_16.is_null() {
                            sp = row;
                            i = 0;
                            while i < row_width {
                                let mut v: png_uint_16;

                                v = (((*sp as png_uint_16) << 8) + *sp.add(1) as png_uint_16)
                                    as png_uint_16;

                                if v == (*png_ptr).trans_color.gray {
                                    /* Background is already in screen gamma */
                                    *sp = (((*png_ptr).background.gray >> 8) & 0xff) as png_byte;
                                    *sp.add(1) = ((*png_ptr).background.gray & 0xff) as png_byte;
                                } else {
                                    v = *(*gamma_16.offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                                        .offset(*sp as isize);
                                    *sp = ((v >> 8) & 0xff) as png_byte;
                                    *sp.add(1) = (v & 0xff) as png_byte;
                                }
                                i += 1;
                                sp = sp.add(2);
                            }
                        } else {
                            sp = row;
                            i = 0;
                            while i < row_width {
                                let v: png_uint_16;

                                v = (((*sp as png_uint_16) << 8) + *sp.add(1) as png_uint_16)
                                    as png_uint_16;

                                if v == (*png_ptr).trans_color.gray {
                                    *sp = (((*png_ptr).background.gray >> 8) & 0xff) as png_byte;
                                    *sp.add(1) = ((*png_ptr).background.gray & 0xff) as png_byte;
                                }
                                i += 1;
                                sp = sp.add(2);
                            }
                        }
                    }

                    _ => {}
                }
            }

            PNG_COLOR_TYPE_RGB => {
                if (*row_info).bit_depth == 8 {
                    if !gamma_table.is_null() {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            if *sp as png_uint_16 == (*png_ptr).trans_color.red
                                && *sp.add(1) as png_uint_16 == (*png_ptr).trans_color.green
                                && *sp.add(2) as png_uint_16 == (*png_ptr).trans_color.blue
                            {
                                *sp = (*png_ptr).background.red as png_byte;
                                *sp.add(1) = (*png_ptr).background.green as png_byte;
                                *sp.add(2) = (*png_ptr).background.blue as png_byte;
                            } else {
                                *sp = *gamma_table.offset(*sp as isize);
                                *sp.add(1) = *gamma_table.offset(*sp.add(1) as isize);
                                *sp.add(2) = *gamma_table.offset(*sp.add(2) as isize);
                            }
                            i += 1;
                            sp = sp.add(3);
                        }
                    } else {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            if *sp as png_uint_16 == (*png_ptr).trans_color.red
                                && *sp.add(1) as png_uint_16 == (*png_ptr).trans_color.green
                                && *sp.add(2) as png_uint_16 == (*png_ptr).trans_color.blue
                            {
                                *sp = (*png_ptr).background.red as png_byte;
                                *sp.add(1) = (*png_ptr).background.green as png_byte;
                                *sp.add(2) = (*png_ptr).background.blue as png_byte;
                            }
                            i += 1;
                            sp = sp.add(3);
                        }
                    }
                } else
                /* if (row_info->bit_depth == 16) */
                {
                    if !gamma_16.is_null() {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let r: png_uint_16 = (((*sp as png_uint_16) << 8)
                                + *sp.add(1) as png_uint_16)
                                as png_uint_16;

                            let g: png_uint_16 = (((*sp.add(2) as png_uint_16) << 8)
                                + *sp.add(3) as png_uint_16)
                                as png_uint_16;

                            let b: png_uint_16 = (((*sp.add(4) as png_uint_16) << 8)
                                + *sp.add(5) as png_uint_16)
                                as png_uint_16;

                            if r == (*png_ptr).trans_color.red
                                && g == (*png_ptr).trans_color.green
                                && b == (*png_ptr).trans_color.blue
                            {
                                /* Background is already in screen gamma */
                                *sp = (((*png_ptr).background.red >> 8) & 0xff) as png_byte;
                                *sp.add(1) = ((*png_ptr).background.red & 0xff) as png_byte;
                                *sp.add(2) =
                                    (((*png_ptr).background.green >> 8) & 0xff) as png_byte;
                                *sp.add(3) = ((*png_ptr).background.green & 0xff) as png_byte;
                                *sp.add(4) = (((*png_ptr).background.blue >> 8) & 0xff) as png_byte;
                                *sp.add(5) = ((*png_ptr).background.blue & 0xff) as png_byte;
                            } else {
                                let mut v: png_uint_16 = *(*gamma_16
                                    .offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                                .offset(*sp as isize);
                                *sp = ((v >> 8) & 0xff) as png_byte;
                                *sp.add(1) = (v & 0xff) as png_byte;

                                v = *(*gamma_16.offset(((*sp.add(3) as c_int) >> gamma_shift) as isize))
                                    .offset(*sp.add(2) as isize);
                                *sp.add(2) = ((v >> 8) & 0xff) as png_byte;
                                *sp.add(3) = (v & 0xff) as png_byte;

                                v = *(*gamma_16.offset(((*sp.add(5) as c_int) >> gamma_shift) as isize))
                                    .offset(*sp.add(4) as isize);
                                *sp.add(4) = ((v >> 8) & 0xff) as png_byte;
                                *sp.add(5) = (v & 0xff) as png_byte;
                            }
                            i += 1;
                            sp = sp.add(6);
                        }
                    } else {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let r: png_uint_16 = (((*sp as png_uint_16) << 8)
                                + *sp.add(1) as png_uint_16)
                                as png_uint_16;

                            let g: png_uint_16 = (((*sp.add(2) as png_uint_16) << 8)
                                + *sp.add(3) as png_uint_16)
                                as png_uint_16;

                            let b: png_uint_16 = (((*sp.add(4) as png_uint_16) << 8)
                                + *sp.add(5) as png_uint_16)
                                as png_uint_16;

                            if r == (*png_ptr).trans_color.red
                                && g == (*png_ptr).trans_color.green
                                && b == (*png_ptr).trans_color.blue
                            {
                                *sp = (((*png_ptr).background.red >> 8) & 0xff) as png_byte;
                                *sp.add(1) = ((*png_ptr).background.red & 0xff) as png_byte;
                                *sp.add(2) =
                                    (((*png_ptr).background.green >> 8) & 0xff) as png_byte;
                                *sp.add(3) = ((*png_ptr).background.green & 0xff) as png_byte;
                                *sp.add(4) = (((*png_ptr).background.blue >> 8) & 0xff) as png_byte;
                                *sp.add(5) = ((*png_ptr).background.blue & 0xff) as png_byte;
                            }
                            i += 1;
                            sp = sp.add(6);
                        }
                    }
                }
            }

            PNG_COLOR_TYPE_GRAY_ALPHA => {
                if (*row_info).bit_depth == 8 {
                    if !gamma_to_1.is_null() && !gamma_from_1.is_null() && !gamma_table.is_null() {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let a: png_uint_16 = *sp.add(1) as png_uint_16;

                            if a == 0xff {
                                *sp = *gamma_table.offset(*sp as isize);
                            } else if a == 0 {
                                /* Background is already in screen gamma */
                                *sp = (*png_ptr).background.gray as png_byte;
                            } else {
                                let v: png_byte;
                                let mut w: png_byte;

                                v = *gamma_to_1.offset(*sp as isize);
                                w = png_composite(
                                    v as png_uint_16,
                                    a,
                                    (*png_ptr).background_1.gray,
                                );
                                if optimize == 0 {
                                    w = *gamma_from_1.offset(w as isize);
                                }
                                *sp = w;
                            }
                            i += 1;
                            sp = sp.add(2);
                        }
                    } else {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let a: png_byte = *sp.add(1);

                            if a == 0 {
                                *sp = (*png_ptr).background.gray as png_byte;
                            } else if a < 0xff {
                                *sp = png_composite(
                                    *sp as png_uint_16,
                                    a as png_uint_16,
                                    (*png_ptr).background.gray,
                                );
                            }
                            i += 1;
                            sp = sp.add(2);
                        }
                    }
                } else
                /* if (png_ptr->bit_depth == 16) */
                {
                    if !gamma_16.is_null() && !gamma_16_from_1.is_null() && !gamma_16_to_1.is_null()
                    {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let a: png_uint_16 = (((*sp.add(2) as png_uint_16) << 8)
                                + *sp.add(3) as png_uint_16)
                                as png_uint_16;

                            if a == 0xffff {
                                let v: png_uint_16;

                                v = *(*gamma_16.offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                                    .offset(*sp as isize);
                                *sp = ((v >> 8) & 0xff) as png_byte;
                                *sp.add(1) = (v & 0xff) as png_byte;
                            } else if a == 0 {
                                /* Background is already in screen gamma */
                                *sp = (((*png_ptr).background.gray >> 8) & 0xff) as png_byte;
                                *sp.add(1) = ((*png_ptr).background.gray & 0xff) as png_byte;
                            } else {
                                let g: png_uint_16;
                                let v: png_uint_16;
                                let w: png_uint_16;

                                g = *(*gamma_16_to_1.offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                                    .offset(*sp as isize);
                                v = png_composite_16(
                                    g as png_uint_32,
                                    a as png_uint_32,
                                    (*png_ptr).background_1.gray as png_uint_32,
                                );
                                if optimize != 0 {
                                    w = v;
                                } else {
                                    w = *(*gamma_16_from_1
                                        .offset((((v as c_int) & 0xff) >> gamma_shift) as isize))
                                    .offset((v >> 8) as isize);
                                }
                                *sp = ((w >> 8) & 0xff) as png_byte;
                                *sp.add(1) = (w & 0xff) as png_byte;
                            }
                            i += 1;
                            sp = sp.add(4);
                        }
                    } else {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let a: png_uint_16 = (((*sp.add(2) as png_uint_16) << 8)
                                + *sp.add(3) as png_uint_16)
                                as png_uint_16;

                            if a == 0 {
                                *sp = (((*png_ptr).background.gray >> 8) & 0xff) as png_byte;
                                *sp.add(1) = ((*png_ptr).background.gray & 0xff) as png_byte;
                            } else if a < 0xffff {
                                let g: png_uint_16;
                                let v: png_uint_16;

                                g = (((*sp as png_uint_16) << 8) + *sp.add(1) as png_uint_16)
                                    as png_uint_16;
                                v = png_composite_16(
                                    g as png_uint_32,
                                    a as png_uint_32,
                                    (*png_ptr).background.gray as png_uint_32,
                                );
                                *sp = ((v >> 8) & 0xff) as png_byte;
                                *sp.add(1) = (v & 0xff) as png_byte;
                            }
                            i += 1;
                            sp = sp.add(4);
                        }
                    }
                }
            }

            PNG_COLOR_TYPE_RGB_ALPHA => {
                if (*row_info).bit_depth == 8 {
                    if !gamma_to_1.is_null() && !gamma_from_1.is_null() && !gamma_table.is_null() {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let a: png_byte = *sp.add(3);

                            if a == 0xff {
                                *sp = *gamma_table.offset(*sp as isize);
                                *sp.add(1) = *gamma_table.offset(*sp.add(1) as isize);
                                *sp.add(2) = *gamma_table.offset(*sp.add(2) as isize);
                            } else if a == 0 {
                                /* Background is already in screen gamma */
                                *sp = (*png_ptr).background.red as png_byte;
                                *sp.add(1) = (*png_ptr).background.green as png_byte;
                                *sp.add(2) = (*png_ptr).background.blue as png_byte;
                            } else {
                                let mut v: png_byte;
                                let mut w: png_byte;

                                v = *gamma_to_1.offset(*sp as isize);
                                w = png_composite(
                                    v as png_uint_16,
                                    a as png_uint_16,
                                    (*png_ptr).background_1.red,
                                );
                                if optimize == 0 {
                                    w = *gamma_from_1.offset(w as isize);
                                }
                                *sp = w;

                                v = *gamma_to_1.offset(*sp.add(1) as isize);
                                w = png_composite(
                                    v as png_uint_16,
                                    a as png_uint_16,
                                    (*png_ptr).background_1.green,
                                );
                                if optimize == 0 {
                                    w = *gamma_from_1.offset(w as isize);
                                }
                                *sp.add(1) = w;

                                v = *gamma_to_1.offset(*sp.add(2) as isize);
                                w = png_composite(
                                    v as png_uint_16,
                                    a as png_uint_16,
                                    (*png_ptr).background_1.blue,
                                );
                                if optimize == 0 {
                                    w = *gamma_from_1.offset(w as isize);
                                }
                                *sp.add(2) = w;
                            }
                            i += 1;
                            sp = sp.add(4);
                        }
                    } else {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let a: png_byte = *sp.add(3);

                            if a == 0 {
                                *sp = (*png_ptr).background.red as png_byte;
                                *sp.add(1) = (*png_ptr).background.green as png_byte;
                                *sp.add(2) = (*png_ptr).background.blue as png_byte;
                            } else if a < 0xff {
                                *sp = png_composite(
                                    *sp as png_uint_16,
                                    a as png_uint_16,
                                    (*png_ptr).background.red,
                                );

                                *sp.add(1) = png_composite(
                                    *sp.add(1) as png_uint_16,
                                    a as png_uint_16,
                                    (*png_ptr).background.green,
                                );

                                *sp.add(2) = png_composite(
                                    *sp.add(2) as png_uint_16,
                                    a as png_uint_16,
                                    (*png_ptr).background.blue,
                                );
                            }
                            i += 1;
                            sp = sp.add(4);
                        }
                    }
                } else
                /* if (row_info->bit_depth == 16) */
                {
                    if !gamma_16.is_null() && !gamma_16_from_1.is_null() && !gamma_16_to_1.is_null()
                    {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let a: png_uint_16 = (((*sp.add(6) as png_uint_16) << 8)
                                + (*sp.add(7) as png_uint_16))
                                as png_uint_16;

                            if a == 0xffff {
                                let mut v: png_uint_16;

                                v = *(*gamma_16.offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                                    .offset(*sp as isize);
                                *sp = ((v >> 8) & 0xff) as png_byte;
                                *sp.add(1) = (v & 0xff) as png_byte;

                                v = *(*gamma_16.offset(((*sp.add(3) as c_int) >> gamma_shift) as isize))
                                    .offset(*sp.add(2) as isize);
                                *sp.add(2) = ((v >> 8) & 0xff) as png_byte;
                                *sp.add(3) = (v & 0xff) as png_byte;

                                v = *(*gamma_16.offset(((*sp.add(5) as c_int) >> gamma_shift) as isize))
                                    .offset(*sp.add(4) as isize);
                                *sp.add(4) = ((v >> 8) & 0xff) as png_byte;
                                *sp.add(5) = (v & 0xff) as png_byte;
                            } else if a == 0 {
                                /* Background is already in screen gamma */
                                *sp = (((*png_ptr).background.red >> 8) & 0xff) as png_byte;
                                *sp.add(1) = ((*png_ptr).background.red & 0xff) as png_byte;
                                *sp.add(2) =
                                    (((*png_ptr).background.green >> 8) & 0xff) as png_byte;
                                *sp.add(3) = ((*png_ptr).background.green & 0xff) as png_byte;
                                *sp.add(4) = (((*png_ptr).background.blue >> 8) & 0xff) as png_byte;
                                *sp.add(5) = ((*png_ptr).background.blue & 0xff) as png_byte;
                            } else {
                                let mut v: png_uint_16;
                                let mut w: png_uint_16;

                                v = *(*gamma_16_to_1.offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                                    .offset(*sp as isize);
                                w = png_composite_16(
                                    v as png_uint_32,
                                    a as png_uint_32,
                                    (*png_ptr).background_1.red as png_uint_32,
                                );
                                if optimize == 0 {
                                    w = *(*gamma_16_from_1
                                        .offset((((w as c_int) & 0xff) >> gamma_shift) as isize))
                                    .offset((w >> 8) as isize);
                                }
                                *sp = ((w >> 8) & 0xff) as png_byte;
                                *sp.add(1) = (w & 0xff) as png_byte;

                                v = *(*gamma_16_to_1.offset(((*sp.add(3) as c_int) >> gamma_shift) as isize))
                                    .offset(*sp.add(2) as isize);
                                w = png_composite_16(
                                    v as png_uint_32,
                                    a as png_uint_32,
                                    (*png_ptr).background_1.green as png_uint_32,
                                );
                                if optimize == 0 {
                                    w = *(*gamma_16_from_1
                                        .offset((((w as c_int) & 0xff) >> gamma_shift) as isize))
                                    .offset((w >> 8) as isize);
                                }

                                *sp.add(2) = ((w >> 8) & 0xff) as png_byte;
                                *sp.add(3) = (w & 0xff) as png_byte;

                                v = *(*gamma_16_to_1.offset(((*sp.add(5) as c_int) >> gamma_shift) as isize))
                                    .offset(*sp.add(4) as isize);
                                w = png_composite_16(
                                    v as png_uint_32,
                                    a as png_uint_32,
                                    (*png_ptr).background_1.blue as png_uint_32,
                                );
                                if optimize == 0 {
                                    w = *(*gamma_16_from_1
                                        .offset((((w as c_int) & 0xff) >> gamma_shift) as isize))
                                    .offset((w >> 8) as isize);
                                }

                                *sp.add(4) = ((w >> 8) & 0xff) as png_byte;
                                *sp.add(5) = (w & 0xff) as png_byte;
                            }
                            i += 1;
                            sp = sp.add(8);
                        }
                    } else {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let a: png_uint_16 = (((*sp.add(6) as png_uint_16) << 8)
                                + (*sp.add(7) as png_uint_16))
                                as png_uint_16;

                            if a == 0 {
                                *sp = (((*png_ptr).background.red >> 8) & 0xff) as png_byte;
                                *sp.add(1) = ((*png_ptr).background.red & 0xff) as png_byte;
                                *sp.add(2) =
                                    (((*png_ptr).background.green >> 8) & 0xff) as png_byte;
                                *sp.add(3) = ((*png_ptr).background.green & 0xff) as png_byte;
                                *sp.add(4) = (((*png_ptr).background.blue >> 8) & 0xff) as png_byte;
                                *sp.add(5) = ((*png_ptr).background.blue & 0xff) as png_byte;
                            } else if a < 0xffff {
                                let mut v: png_uint_16;

                                let r: png_uint_16 = (((*sp as png_uint_16) << 8)
                                    + *sp.add(1) as png_uint_16)
                                    as png_uint_16;
                                let g: png_uint_16 = (((*sp.add(2) as png_uint_16) << 8)
                                    + *sp.add(3) as png_uint_16)
                                    as png_uint_16;
                                let b: png_uint_16 = (((*sp.add(4) as png_uint_16) << 8)
                                    + *sp.add(5) as png_uint_16)
                                    as png_uint_16;

                                v = png_composite_16(
                                    r as png_uint_32,
                                    a as png_uint_32,
                                    (*png_ptr).background.red as png_uint_32,
                                );
                                *sp = ((v >> 8) & 0xff) as png_byte;
                                *sp.add(1) = (v & 0xff) as png_byte;

                                v = png_composite_16(
                                    g as png_uint_32,
                                    a as png_uint_32,
                                    (*png_ptr).background.green as png_uint_32,
                                );
                                *sp.add(2) = ((v >> 8) & 0xff) as png_byte;
                                *sp.add(3) = (v & 0xff) as png_byte;

                                v = png_composite_16(
                                    b as png_uint_32,
                                    a as png_uint_32,
                                    (*png_ptr).background.blue as png_uint_32,
                                );
                                *sp.add(4) = ((v >> 8) & 0xff) as png_byte;
                                *sp.add(5) = (v & 0xff) as png_byte;
                            }
                            i += 1;
                            sp = sp.add(8);
                        }
                    }
                }
            }

            _ => {}
        }
    }
}

/* Gamma correct the image, avoiding the alpha channel. ... */
unsafe fn png_do_gamma(row_info: png_row_infop, row: png_bytep, png_ptr: png_structrp) {
    unsafe {
        let gamma_table: png_const_bytep = (*png_ptr).gamma_table;
        let gamma_16_table: png_uint_16pp = (*png_ptr).gamma_16_table;
        let gamma_shift: c_int = (*png_ptr).gamma_shift;

        let mut sp: png_bytep;
        let mut i: png_uint_32;
        let row_width: png_uint_32 = (*row_info).width;

        if ((*row_info).bit_depth <= 8 && !gamma_table.is_null())
            || ((*row_info).bit_depth == 16 && !gamma_16_table.is_null())
        {
            match (*row_info).color_type as c_int {
                PNG_COLOR_TYPE_RGB => {
                    if (*row_info).bit_depth == 8 {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            *sp = *gamma_table.offset(*sp as isize);
                            sp = sp.add(1);
                            *sp = *gamma_table.offset(*sp as isize);
                            sp = sp.add(1);
                            *sp = *gamma_table.offset(*sp as isize);
                            sp = sp.add(1);
                            i += 1;
                        }
                    } else
                    /* if (row_info->bit_depth == 16) */
                    {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let mut v: png_uint_16;

                            v = *(*gamma_16_table.offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                                .offset(*sp as isize);
                            *sp = ((v >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (v & 0xff) as png_byte;
                            sp = sp.add(2);

                            v = *(*gamma_16_table.offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                                .offset(*sp as isize);
                            *sp = ((v >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (v & 0xff) as png_byte;
                            sp = sp.add(2);

                            v = *(*gamma_16_table.offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                                .offset(*sp as isize);
                            *sp = ((v >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (v & 0xff) as png_byte;
                            sp = sp.add(2);
                            i += 1;
                        }
                    }
                }

                PNG_COLOR_TYPE_RGB_ALPHA => {
                    if (*row_info).bit_depth == 8 {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            *sp = *gamma_table.offset(*sp as isize);
                            sp = sp.add(1);

                            *sp = *gamma_table.offset(*sp as isize);
                            sp = sp.add(1);

                            *sp = *gamma_table.offset(*sp as isize);
                            sp = sp.add(1);

                            sp = sp.add(1);
                            i += 1;
                        }
                    } else
                    /* if (row_info->bit_depth == 16) */
                    {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let mut v: png_uint_16 = *(*gamma_16_table
                                .offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                            .offset(*sp as isize);
                            *sp = ((v >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (v & 0xff) as png_byte;
                            sp = sp.add(2);

                            v = *(*gamma_16_table.offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                                .offset(*sp as isize);
                            *sp = ((v >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (v & 0xff) as png_byte;
                            sp = sp.add(2);

                            v = *(*gamma_16_table.offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                                .offset(*sp as isize);
                            *sp = ((v >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (v & 0xff) as png_byte;
                            sp = sp.add(4);
                            i += 1;
                        }
                    }
                }

                PNG_COLOR_TYPE_GRAY_ALPHA => {
                    if (*row_info).bit_depth == 8 {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            *sp = *gamma_table.offset(*sp as isize);
                            sp = sp.add(2);
                            i += 1;
                        }
                    } else
                    /* if (row_info->bit_depth == 16) */
                    {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let v: png_uint_16 = *(*gamma_16_table
                                .offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                            .offset(*sp as isize);
                            *sp = ((v >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (v & 0xff) as png_byte;
                            sp = sp.add(4);
                            i += 1;
                        }
                    }
                }

                PNG_COLOR_TYPE_GRAY => {
                    if (*row_info).bit_depth == 2 {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let a: c_int = (*sp & 0xc0) as c_int;
                            let b: c_int = (*sp & 0x30) as c_int;
                            let c: c_int = (*sp & 0x0c) as c_int;
                            let d: c_int = (*sp & 0x03) as c_int;

                            *sp = (((*gamma_table
                                .offset((a | (a >> 2) | (a >> 4) | (a >> 6)) as isize)
                                as c_int)
                                & 0xc0)
                                | (((*gamma_table
                                    .offset(((b << 2) | b | (b >> 2) | (b >> 4)) as isize)
                                    as c_int)
                                    >> 2)
                                    & 0x30)
                                | (((*gamma_table
                                    .offset(((c << 4) | (c << 2) | c | (c >> 2)) as isize)
                                    as c_int)
                                    >> 4)
                                    & 0x0c)
                                | ((*gamma_table
                                    .offset(((d << 6) | (d << 4) | (d << 2) | d) as isize)
                                    as c_int)
                                    >> 6)) as png_byte;
                            sp = sp.add(1);
                            i += 4;
                        }
                    }

                    if (*row_info).bit_depth == 4 {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let msb: c_int = (*sp & 0xf0) as c_int;
                            let lsb: c_int = (*sp & 0x0f) as c_int;

                            *sp = (((*gamma_table.offset((msb | (msb >> 4)) as isize) as c_int)
                                & 0xf0)
                                | ((*gamma_table.offset(((lsb << 4) | lsb) as isize) as c_int)
                                    >> 4)) as png_byte;
                            sp = sp.add(1);
                            i += 2;
                        }
                    } else if (*row_info).bit_depth == 8 {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            *sp = *gamma_table.offset(*sp as isize);
                            sp = sp.add(1);
                            i += 1;
                        }
                    } else if (*row_info).bit_depth == 16 {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let v: png_uint_16 = *(*gamma_16_table
                                .offset(((*sp.add(1) as c_int) >> gamma_shift) as isize))
                            .offset(*sp as isize);
                            *sp = ((v >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (v & 0xff) as png_byte;
                            sp = sp.add(2);
                            i += 1;
                        }
                    }
                }

                _ => {}
            }
        }
    }
}

/* Encode the alpha channel to the output gamma ... */
unsafe fn png_do_encode_alpha(row_info: png_row_infop, mut row: png_bytep, png_ptr: png_structrp) {
    unsafe {
        let mut row_width: png_uint_32 = (*row_info).width;

        if ((*row_info).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0 {
            if (*row_info).bit_depth == 8 {
                let table: png_bytep = (*png_ptr).gamma_from_1;

                if !table.is_null() {
                    let step: c_int =
                        if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
                            4
                        } else {
                            2
                        };

                    /* The alpha channel is the last component: */
                    row = row.add((step - 1) as usize);

                    while row_width > 0 {
                        *row = *table.offset(*row as isize);
                        row_width -= 1;
                        row = row.add(step as usize);
                    }

                    return;
                }
            } else if (*row_info).bit_depth == 16 {
                let table: png_uint_16pp = (*png_ptr).gamma_16_from_1;
                let gamma_shift: c_int = (*png_ptr).gamma_shift;

                if !table.is_null() {
                    let step: c_int =
                        if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
                            8
                        } else {
                            4
                        };

                    /* The alpha channel is the last component: */
                    row = row.add((step - 2) as usize);

                    while row_width > 0 {
                        let v: png_uint_16;

                        v = *(*table.offset(((*row.add(1) as c_int) >> gamma_shift) as isize))
                            .offset(*row as isize);
                        *row = ((v >> 8) & 0xff) as png_byte;
                        *row.add(1) = (v & 0xff) as png_byte;
                        row_width -= 1;
                        row = row.add(step as usize);
                    }

                    return;
                }
            }
        }

        /* Only get to here if called with a weird row_info ... */
        png_warning(png_ptr, c"png_do_encode_alpha: unexpected call".as_ptr());
    }
}

/* Expands a palette row to an RGB or RGBA row ... */
unsafe fn png_do_expand_palette(
    png_ptr: png_structrp,
    row_info: png_row_infop,
    row: png_bytep,
    palette: png_const_colorp,
    trans_alpha: png_const_bytep,
    num_trans: c_int,
) {
    unsafe {
        let mut shift: c_int;
        let mut value: c_int = 0;
        let mut sp: png_bytep;
        let mut dp: png_bytep;
        let mut i: png_uint_32;
        let row_width: png_uint_32 = (*row_info).width;

        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            if (*row_info).bit_depth < 8 {
                match (*row_info).bit_depth {
                    1 => {
                        sp = row.add(((row_width - 1) >> 3) as usize);
                        dp = row.add(row_width as usize - 1);
                        shift = 7 - ((row_width + 7) & 0x07) as c_int;
                        i = 0;
                        while i < row_width {
                            if (*sp >> shift) & 0x01 != 0 {
                                *dp = 1;
                            } else {
                                *dp = 0;
                            }

                            if shift == 7 {
                                shift = 0;
                                sp = sp.sub(1);
                            } else {
                                shift += 1;
                            }

                            dp = dp.sub(1);
                            i += 1;
                        }
                    }

                    2 => {
                        sp = row.add(((row_width - 1) >> 2) as usize);
                        dp = row.add(row_width as usize - 1);
                        shift = ((3 - ((row_width + 3) & 0x03)) << 1) as c_int;
                        i = 0;
                        while i < row_width {
                            value = ((*sp >> shift) & 0x03) as c_int;
                            *dp = value as png_byte;
                            if shift == 6 {
                                shift = 0;
                                sp = sp.sub(1);
                            } else {
                                shift += 2;
                            }

                            dp = dp.sub(1);
                            i += 1;
                        }
                    }

                    4 => {
                        sp = row.add(((row_width - 1) >> 1) as usize);
                        dp = row.add(row_width as usize - 1);
                        shift = ((row_width & 0x01) << 2) as c_int;
                        i = 0;
                        while i < row_width {
                            value = ((*sp >> shift) & 0x0f) as c_int;
                            *dp = value as png_byte;
                            if shift == 4 {
                                shift = 0;
                                sp = sp.sub(1);
                            } else {
                                shift += 4;
                            }

                            dp = dp.sub(1);
                            i += 1;
                        }
                    }

                    _ => {}
                }
                (*row_info).bit_depth = 8;
                (*row_info).pixel_depth = 8;
                (*row_info).rowbytes = row_width as usize;
            }

            if (*row_info).bit_depth == 8 {
                {
                    if num_trans > 0 {
                        sp = row.add(row_width as usize - 1);
                        dp = row.add(((row_width as usize) << 2) - 1);

                        i = 0;
                        PNG_UNUSED(png_ptr);

                        while i < row_width {
                            if (*sp as c_int) >= num_trans {
                                *dp = 0xff;
                                dp = dp.sub(1);
                            } else {
                                *dp = *trans_alpha.offset(*sp as isize);
                                dp = dp.sub(1);
                            }
                            *dp = (*palette.offset(*sp as isize)).blue;
                            dp = dp.sub(1);
                            *dp = (*palette.offset(*sp as isize)).green;
                            dp = dp.sub(1);
                            *dp = (*palette.offset(*sp as isize)).red;
                            dp = dp.sub(1);
                            sp = sp.sub(1);
                            i += 1;
                        }
                        (*row_info).bit_depth = 8;
                        (*row_info).pixel_depth = 32;
                        (*row_info).rowbytes = row_width as usize * 4;
                        (*row_info).color_type = 6;
                        (*row_info).channels = 4;
                    } else {
                        sp = row.add(row_width as usize - 1);
                        dp = row.add(row_width as usize * 3 - 1);
                        i = 0;
                        PNG_UNUSED(png_ptr);

                        while i < row_width {
                            *dp = (*palette.offset(*sp as isize)).blue;
                            dp = dp.sub(1);
                            *dp = (*palette.offset(*sp as isize)).green;
                            dp = dp.sub(1);
                            *dp = (*palette.offset(*sp as isize)).red;
                            dp = dp.sub(1);
                            sp = sp.sub(1);
                            i += 1;
                        }

                        (*row_info).bit_depth = 8;
                        (*row_info).pixel_depth = 24;
                        (*row_info).rowbytes = row_width as usize * 3;
                        (*row_info).color_type = 2;
                        (*row_info).channels = 3;
                    }
                }
            }
        }
    }
}

/* If the bit depth < 8, it is expanded to 8. ... */
unsafe fn png_do_expand(row_info: png_row_infop, row: png_bytep, trans_color: png_const_color_16p) {
    unsafe {
        let mut shift: c_int;
        let mut value: c_int = 0;
        let mut sp: png_bytep;
        let mut dp: png_bytep;
        let mut i: png_uint_32;
        let row_width: png_uint_32 = (*row_info).width;

        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY {
            let mut gray: c_uint = if !trans_color.is_null() {
                (*trans_color).gray as c_uint
            } else {
                0
            };

            if (*row_info).bit_depth < 8 {
                match (*row_info).bit_depth {
                    1 => {
                        gray = (gray & 0x01) * 0xff;
                        sp = row.add(((row_width - 1) >> 3) as usize);
                        dp = row.add(row_width as usize - 1);
                        shift = 7 - ((row_width + 7) & 0x07) as c_int;
                        i = 0;
                        while i < row_width {
                            if (*sp >> shift) & 0x01 != 0 {
                                *dp = 0xff;
                            } else {
                                *dp = 0;
                            }

                            if shift == 7 {
                                shift = 0;
                                sp = sp.sub(1);
                            } else {
                                shift += 1;
                            }

                            dp = dp.sub(1);
                            i += 1;
                        }
                    }

                    2 => {
                        gray = (gray & 0x03) * 0x55;
                        sp = row.add(((row_width - 1) >> 2) as usize);
                        dp = row.add(row_width as usize - 1);
                        shift = ((3 - ((row_width + 3) & 0x03)) << 1) as c_int;
                        i = 0;
                        while i < row_width {
                            value = ((*sp >> shift) & 0x03) as c_int;
                            *dp = (value | (value << 2) | (value << 4) | (value << 6)) as png_byte;
                            if shift == 6 {
                                shift = 0;
                                sp = sp.sub(1);
                            } else {
                                shift += 2;
                            }

                            dp = dp.sub(1);
                            i += 1;
                        }
                    }

                    4 => {
                        gray = (gray & 0x0f) * 0x11;
                        sp = row.add(((row_width - 1) >> 1) as usize);
                        dp = row.add(row_width as usize - 1);
                        shift = ((1 - ((row_width + 1) & 0x01)) << 2) as c_int;
                        i = 0;
                        while i < row_width {
                            value = ((*sp >> shift) & 0x0f) as c_int;
                            *dp = (value | (value << 4)) as png_byte;
                            if shift == 4 {
                                shift = 0;
                                sp = sp.sub(1);
                            } else {
                                shift = 4;
                            }

                            dp = dp.sub(1);
                            i += 1;
                        }
                    }

                    _ => {}
                }

                (*row_info).bit_depth = 8;
                (*row_info).pixel_depth = 8;
                (*row_info).rowbytes = row_width as usize;
            }

            if !trans_color.is_null() {
                if (*row_info).bit_depth == 8 {
                    gray &= 0xff;
                    sp = row.add(row_width as usize - 1);
                    dp = row.add(((row_width as usize) << 1) - 1);

                    i = 0;
                    while i < row_width {
                        if (*sp as c_uint & 0xffu32) == gray {
                            *dp = 0;
                            dp = dp.sub(1);
                        } else {
                            *dp = 0xff;
                            dp = dp.sub(1);
                        }

                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        i += 1;
                    }
                } else if (*row_info).bit_depth == 16 {
                    let gray_high: c_uint = (gray >> 8) & 0xff;
                    let gray_low: c_uint = gray & 0xff;
                    sp = row.add((*row_info).rowbytes - 1);
                    dp = row.add(((*row_info).rowbytes << 1) - 1);
                    i = 0;
                    while i < row_width {
                        if (*sp.sub(1) as c_uint & 0xffu32) == gray_high
                            && (*sp as c_uint & 0xffu32) == gray_low
                        {
                            *dp = 0;
                            dp = dp.sub(1);
                            *dp = 0;
                            dp = dp.sub(1);
                        } else {
                            *dp = 0xff;
                            dp = dp.sub(1);
                            *dp = 0xff;
                            dp = dp.sub(1);
                        }

                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        *dp = *sp;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        i += 1;
                    }
                }

                (*row_info).color_type = PNG_COLOR_TYPE_GRAY_ALPHA as png_byte;
                (*row_info).channels = 2;
                (*row_info).pixel_depth = (((*row_info).bit_depth as c_int) << 1) as png_byte;
                (*row_info).rowbytes =
                    PNG_ROWBYTES((*row_info).pixel_depth as usize, row_width as usize);
            }
        } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB && !trans_color.is_null() {
            if (*row_info).bit_depth == 8 {
                let red: png_byte = ((*trans_color).red & 0xff) as png_byte;
                let green: png_byte = ((*trans_color).green & 0xff) as png_byte;
                let blue: png_byte = ((*trans_color).blue & 0xff) as png_byte;
                sp = row.add((*row_info).rowbytes - 1);
                dp = row.add(((row_width as usize) << 2) - 1);
                i = 0;
                while i < row_width {
                    if *sp.sub(2) == red && *sp.sub(1) == green && *sp == blue {
                        *dp = 0;
                        dp = dp.sub(1);
                    } else {
                        *dp = 0xff;
                        dp = dp.sub(1);
                    }

                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    i += 1;
                }
            } else if (*row_info).bit_depth == 16 {
                let red_high: png_byte = (((*trans_color).red >> 8) & 0xff) as png_byte;
                let green_high: png_byte = (((*trans_color).green >> 8) & 0xff) as png_byte;
                let blue_high: png_byte = (((*trans_color).blue >> 8) & 0xff) as png_byte;
                let red_low: png_byte = ((*trans_color).red & 0xff) as png_byte;
                let green_low: png_byte = ((*trans_color).green & 0xff) as png_byte;
                let blue_low: png_byte = ((*trans_color).blue & 0xff) as png_byte;
                sp = row.add((*row_info).rowbytes - 1);
                dp = row.add(((row_width as usize) << 3) - 1);
                i = 0;
                while i < row_width {
                    if *sp.sub(5) == red_high
                        && *sp.sub(4) == red_low
                        && *sp.sub(3) == green_high
                        && *sp.sub(2) == green_low
                        && *sp.sub(1) == blue_high
                        && *sp == blue_low
                    {
                        *dp = 0;
                        dp = dp.sub(1);
                        *dp = 0;
                        dp = dp.sub(1);
                    } else {
                        *dp = 0xff;
                        dp = dp.sub(1);
                        *dp = 0xff;
                        dp = dp.sub(1);
                    }

                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    i += 1;
                }
            }
            (*row_info).color_type = PNG_COLOR_TYPE_RGB_ALPHA as png_byte;
            (*row_info).channels = 4;
            (*row_info).pixel_depth = (((*row_info).bit_depth as c_int) << 2) as png_byte;
            (*row_info).rowbytes =
                PNG_ROWBYTES((*row_info).pixel_depth as usize, row_width as usize);
        }
    }
}

/* If the bit depth is 8 and the color type is not a palette type expand the
 * whole row to 16 bits.  Has no effect otherwise.
 */
unsafe fn png_do_expand_16(row_info: png_row_infop, row: png_bytep) {
    unsafe {
        if (*row_info).bit_depth == 8 && (*row_info).color_type as c_int != PNG_COLOR_TYPE_PALETTE {
            /* ... byte replication in place (copying backwards). */
            let mut sp: png_bytep = row.add((*row_info).rowbytes); /* source, last byte + 1 */
            let mut dp: png_bytep = sp.add((*row_info).rowbytes); /* destination, end + 1 */
            while (dp as usize) > (sp as usize) {
                sp = sp.sub(1);
                *dp.sub(2) = *sp;
                *dp.sub(1) = *sp;
                dp = dp.sub(2);
            }

            (*row_info).rowbytes *= 2;
            (*row_info).bit_depth = 16;
            (*row_info).pixel_depth = ((*row_info).channels as c_int * 16) as png_byte;
        }
    }
}

unsafe fn png_do_quantize(
    row_info: png_row_infop,
    row: png_bytep,
    palette_lookup: png_const_bytep,
    quantize_lookup: png_const_bytep,
) {
    unsafe {
        let mut sp: png_bytep;
        let mut dp: png_bytep;
        let mut i: png_uint_32;
        let row_width: png_uint_32 = (*row_info).width;

        if (*row_info).bit_depth == 8 {
            if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB && !palette_lookup.is_null() {
                let mut r: c_int;
                let mut g: c_int;
                let mut b: c_int;
                let mut p: c_int;
                sp = row;
                dp = row;
                i = 0;
                while i < row_width {
                    r = *sp as c_int;
                    sp = sp.add(1);
                    g = *sp as c_int;
                    sp = sp.add(1);
                    b = *sp as c_int;
                    sp = sp.add(1);

                    /* This looks real messy ... */
                    p = (((r >> (8 - PNG_QUANTIZE_RED_BITS)) & ((1 << PNG_QUANTIZE_RED_BITS) - 1))
                        << (PNG_QUANTIZE_GREEN_BITS + PNG_QUANTIZE_BLUE_BITS))
                        | (((g >> (8 - PNG_QUANTIZE_GREEN_BITS))
                            & ((1 << PNG_QUANTIZE_GREEN_BITS) - 1))
                            << PNG_QUANTIZE_BLUE_BITS)
                        | ((b >> (8 - PNG_QUANTIZE_BLUE_BITS))
                            & ((1 << PNG_QUANTIZE_BLUE_BITS) - 1));

                    *dp = *palette_lookup.offset(p as isize);
                    dp = dp.add(1);
                    i += 1;
                }

                (*row_info).color_type = PNG_COLOR_TYPE_PALETTE as png_byte;
                (*row_info).channels = 1;
                (*row_info).pixel_depth = (*row_info).bit_depth;
                (*row_info).rowbytes =
                    PNG_ROWBYTES((*row_info).pixel_depth as usize, row_width as usize);
            } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
                && !palette_lookup.is_null()
            {
                let mut r: c_int;
                let mut g: c_int;
                let mut b: c_int;
                let mut p: c_int;
                sp = row;
                dp = row;
                i = 0;
                while i < row_width {
                    r = *sp as c_int;
                    sp = sp.add(1);
                    g = *sp as c_int;
                    sp = sp.add(1);
                    b = *sp as c_int;
                    sp = sp.add(1);
                    sp = sp.add(1);

                    p = (((r >> (8 - PNG_QUANTIZE_RED_BITS)) & ((1 << PNG_QUANTIZE_RED_BITS) - 1))
                        << (PNG_QUANTIZE_GREEN_BITS + PNG_QUANTIZE_BLUE_BITS))
                        | (((g >> (8 - PNG_QUANTIZE_GREEN_BITS))
                            & ((1 << PNG_QUANTIZE_GREEN_BITS) - 1))
                            << PNG_QUANTIZE_BLUE_BITS)
                        | ((b >> (8 - PNG_QUANTIZE_BLUE_BITS))
                            & ((1 << PNG_QUANTIZE_BLUE_BITS) - 1));

                    *dp = *palette_lookup.offset(p as isize);
                    dp = dp.add(1);
                    i += 1;
                }

                (*row_info).color_type = PNG_COLOR_TYPE_PALETTE as png_byte;
                (*row_info).channels = 1;
                (*row_info).pixel_depth = (*row_info).bit_depth;
                (*row_info).rowbytes =
                    PNG_ROWBYTES((*row_info).pixel_depth as usize, row_width as usize);
            } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_PALETTE
                && !quantize_lookup.is_null()
            {
                sp = row;

                i = 0;
                while i < row_width {
                    *sp = *quantize_lookup.offset(*sp as isize);
                    i += 1;
                    sp = sp.add(1);
                }
            }
        }
    }
}

/* Transform the row.  The order of transformations is significant ... */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_do_read_transformations(
    png_ptr: png_structrp,
    row_info: png_row_infop,
) {
    unsafe {
        if (*png_ptr).row_buf.is_null() {
            /* Prior to 1.5.4 this output row/pass where the NULL pointer is ... */
            png_error(png_ptr, c"NULL row buffer".as_ptr());
        }

        /* The following is debugging ... */
        if ((*png_ptr).flags & PNG_FLAG_DETECT_UNINITIALIZED) != 0
            && ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0
        {
            /* Application has failed to call either png_read_start_image() or
             * png_read_update_info() ...
             */
            png_error(png_ptr, c"Uninitialized row".as_ptr());
        }

        if ((*png_ptr).transformations & PNG_EXPAND) != 0 {
            if (*row_info).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
                png_do_expand_palette(
                    png_ptr,
                    row_info,
                    (*png_ptr).row_buf.add(1),
                    (*png_ptr).palette,
                    (*png_ptr).trans_alpha,
                    (*png_ptr).num_trans as c_int,
                );
            } else {
                if (*png_ptr).num_trans != 0 && ((*png_ptr).transformations & PNG_EXPAND_tRNS) != 0
                {
                    png_do_expand(
                        row_info,
                        (*png_ptr).row_buf.add(1),
                        &raw const (*png_ptr).trans_color,
                    );
                } else {
                    png_do_expand(row_info, (*png_ptr).row_buf.add(1), core::ptr::null());
                }
            }
        }

        if ((*png_ptr).transformations & PNG_STRIP_ALPHA) != 0
            && ((*png_ptr).transformations & PNG_COMPOSE) == 0
            && ((*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
                || (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA)
        {
            png_do_strip_channel(
                row_info,
                (*png_ptr).row_buf.add(1),
                0, /* at_start == false, because SWAP_ALPHA happens later */
            );
        }

        if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) != 0 {
            let rgb_error: c_int = png_do_rgb_to_gray(png_ptr, row_info, (*png_ptr).row_buf.add(1));

            if rgb_error != 0 {
                (*png_ptr).rgb_to_gray_status = 1;
                if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) == PNG_RGB_TO_GRAY_WARN {
                    png_warning(png_ptr, c"png_do_rgb_to_gray found nongray pixel".as_ptr());
                }

                if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) == PNG_RGB_TO_GRAY_ERR {
                    png_error(png_ptr, c"png_do_rgb_to_gray found nongray pixel".as_ptr());
                }
            }
        }

        /* If gray -> RGB, do so now only if background is non-gray ... */
        if ((*png_ptr).transformations & PNG_GRAY_TO_RGB) != 0
            && ((*png_ptr).mode & PNG_BACKGROUND_IS_GRAY) == 0
        {
            png_do_gray_to_rgb(row_info, (*png_ptr).row_buf.add(1));
        }

        if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
            png_do_compose(row_info, (*png_ptr).row_buf.add(1), png_ptr);
        }

        if ((*png_ptr).transformations & PNG_GAMMA) != 0
            /* Because RGB_TO_GRAY does the gamma transform. */
            && ((*png_ptr).transformations & PNG_RGB_TO_GRAY) == 0
            /* Because PNG_COMPOSE does the gamma transform if there is something to
             * do (if there is an alpha channel or transparency.)
             */
            && !(((*png_ptr).transformations & PNG_COMPOSE) != 0
                && ((*png_ptr).num_trans != 0
                    || ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0))
            /* Because png_init_read_transformations transforms the palette ... */
            && ((*png_ptr).color_type as c_int != PNG_COLOR_TYPE_PALETTE)
        {
            png_do_gamma(row_info, (*png_ptr).row_buf.add(1), png_ptr);
        }

        if ((*png_ptr).transformations & PNG_STRIP_ALPHA) != 0
            && ((*png_ptr).transformations & PNG_COMPOSE) != 0
            && ((*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
                || (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA)
        {
            png_do_strip_channel(
                row_info,
                (*png_ptr).row_buf.add(1),
                0, /* at_start == false, because SWAP_ALPHA happens later */
            );
        }

        if ((*png_ptr).transformations & PNG_ENCODE_ALPHA) != 0
            && ((*row_info).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0
        {
            png_do_encode_alpha(row_info, (*png_ptr).row_buf.add(1), png_ptr);
        }

        if ((*png_ptr).transformations & PNG_SCALE_16_TO_8) != 0 {
            png_do_scale_16_to_8(row_info, (*png_ptr).row_buf.add(1));
        }

        /* There is no harm in doing both of these ... */
        if ((*png_ptr).transformations & PNG_16_TO_8) != 0 {
            png_do_chop(row_info, (*png_ptr).row_buf.add(1));
        }

        if ((*png_ptr).transformations & PNG_QUANTIZE) != 0 {
            png_do_quantize(
                row_info,
                (*png_ptr).row_buf.add(1),
                (*png_ptr).palette_lookup,
                (*png_ptr).quantize_index,
            );
        }

        /* Do the expansion now, after all the arithmetic has been done. ... */
        if ((*png_ptr).transformations & PNG_EXPAND_16) != 0 {
            png_do_expand_16(row_info, (*png_ptr).row_buf.add(1));
        }

        /* NOTE: moved here in 1.5.4 (from much later in this list.) */
        if ((*png_ptr).transformations & PNG_GRAY_TO_RGB) != 0
            && ((*png_ptr).mode & PNG_BACKGROUND_IS_GRAY) != 0
        {
            png_do_gray_to_rgb(row_info, (*png_ptr).row_buf.add(1));
        }

        if ((*png_ptr).transformations & PNG_INVERT_MONO) != 0 {
            png_do_invert(row_info, (*png_ptr).row_buf.add(1));
        }

        if ((*png_ptr).transformations & PNG_INVERT_ALPHA) != 0 {
            png_do_read_invert_alpha(row_info, (*png_ptr).row_buf.add(1));
        }

        if ((*png_ptr).transformations & PNG_SHIFT) != 0 {
            png_do_unshift(
                row_info,
                (*png_ptr).row_buf.add(1),
                &raw const (*png_ptr).shift,
            );
        }

        if ((*png_ptr).transformations & PNG_PACK) != 0 {
            png_do_unpack(row_info, (*png_ptr).row_buf.add(1));
        }

        /* Added at libpng-1.5.10 */
        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_PALETTE
            && (*png_ptr).num_palette_max >= 0
        {
            png_do_check_palette_indexes(png_ptr, row_info);
        }

        if ((*png_ptr).transformations & PNG_BGR) != 0 {
            png_do_bgr(row_info, (*png_ptr).row_buf.add(1));
        }

        if ((*png_ptr).transformations & PNG_PACKSWAP) != 0 {
            png_do_packswap(row_info, (*png_ptr).row_buf.add(1));
        }

        if ((*png_ptr).transformations & PNG_FILLER) != 0 {
            png_do_read_filler(
                row_info,
                (*png_ptr).row_buf.add(1),
                (*png_ptr).filler as png_uint_32,
                (*png_ptr).flags,
            );
        }

        if ((*png_ptr).transformations & PNG_SWAP_ALPHA) != 0 {
            png_do_read_swap_alpha(row_info, (*png_ptr).row_buf.add(1));
        }

        if ((*png_ptr).transformations & PNG_SWAP_BYTES) != 0 {
            png_do_swap(row_info, (*png_ptr).row_buf.add(1));
        }

        if ((*png_ptr).transformations & PNG_USER_TRANSFORM) != 0 {
            if (*png_ptr).read_user_transform_fn.is_some() {
                ((*png_ptr).read_user_transform_fn.unwrap())(
                    png_ptr,
                    row_info,
                    (*png_ptr).row_buf.add(1),
                );
            }
            if (*png_ptr).user_transform_depth != 0 {
                (*row_info).bit_depth = (*png_ptr).user_transform_depth;
            }

            if (*png_ptr).user_transform_channels != 0 {
                (*row_info).channels = (*png_ptr).user_transform_channels;
            }
            (*row_info).pixel_depth =
                ((*row_info).bit_depth as c_int * (*row_info).channels as c_int) as png_byte;

            (*row_info).rowbytes =
                PNG_ROWBYTES((*row_info).pixel_depth as usize, (*row_info).width as usize);
        }
    }
}
