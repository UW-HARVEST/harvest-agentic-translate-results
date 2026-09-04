//! Translation of c_src/src/pngrtran.c lines 1..1176
use crate::prelude::*;

/* PNG_BACKGROUND_GAMMA_* (png.h) - not present in consts.rs, defined locally. */
const PNG_BACKGROUND_GAMMA_UNKNOWN: c_int = 0;
#[allow(dead_code)]
const PNG_BACKGROUND_GAMMA_SCREEN: c_int = 1;
const PNG_BACKGROUND_GAMMA_FILE: c_int = 2;
#[allow(dead_code)]
const PNG_BACKGROUND_GAMMA_UNIQUE: c_int = 3;

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
            png_warning(png_ptr, cstr(b"Can't discard critical data on CRC error\0"));
            /* FALLTHROUGH */
            (*png_ptr).flags &= !PNG_FLAG_CRC_CRITICAL_MASK;
        }

        /* PNG_CRC_ERROR_QUIT, PNG_CRC_DEFAULT, default */
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

        /* PNG_CRC_WARN_DISCARD, PNG_CRC_DEFAULT, default */
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
pub unsafe extern "C" fn png_rtran_ok(png_ptr: png_structrp, need_IHDR: c_int) -> c_int {
    if !png_ptr.is_null() {
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) != 0 {
            png_app_error(
                png_ptr,
                cstr(b"invalid after png_start_read_image or png_read_update_info\0"),
            );
        } else if need_IHDR != 0 && ((*png_ptr).mode & PNG_HAVE_IHDR) == 0 {
            png_app_error(
                png_ptr,
                cstr(b"invalid before the PNG header has been read\0"),
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
            cstr(b"Application must supply a known background gamma\0"),
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
        png_fixed(png_ptr, background_gamma, cstr(b"png_set_background\0")),
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

/* PNGv3 conformance gamma parameter translation (see C comment block).
 *    translate_gamma_flags(gamma, is_screen)
 */
pub unsafe extern "C" fn translate_gamma_flags(
    mut output_gamma: png_fixed_point,
    is_screen: c_int,
) -> png_fixed_point {
    /* Check for flag values. */
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

pub unsafe extern "C" fn convert_gamma_value(
    png_ptr: png_structrp,
    mut output_gamma: f64,
) -> png_fixed_point {
    /* The following silently ignores cases where fixed point (times 100,000)
     * gamma values are passed to the floating point API.
     */
    if output_gamma > 0.0 && output_gamma < 128.0 {
        output_gamma *= PNG_FP_1 as f64;
    }

    /* This preserves -1 and -2 exactly: */
    output_gamma = floor(output_gamma + 0.5);

    if output_gamma > PNG_FP_MAX as f64 || output_gamma < PNG_FP_MIN as f64 {
        png_fixed_error(png_ptr, cstr(b"gamma value\0"));
    }

    png_double_to_int32(output_gamma)
}

pub unsafe extern "C" fn unsupported_gamma(
    png_ptr: png_structrp,
    gamma: png_fixed_point,
    warn: c_int,
) -> c_int {
    /* Validate a gamma value to ensure it is in a reasonable range. */
    if gamma < PNG_LIB_GAMMA_MIN || gamma > PNG_LIB_GAMMA_MAX {
        let msg = cstr(b"gamma out of supported range\0");
        if warn != 0 {
            png_app_warning(png_ptr, msg);
        } else {
            png_app_error(png_ptr, msg);
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

    /* The default file gamma is the inverse of the output gamma; get the file
     * value first.
     */
    file_gamma = (*png_ptr).default_gamma;
    if file_gamma == 0 {
        file_gamma = png_reciprocal(output_gamma);
        (*png_ptr).default_gamma = file_gamma;
    }

    /* There are really 8 possibilities here (see C comment). */
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
            png_error(png_ptr, cstr(b"invalid alpha mode\0"));
        }
    }

    /* Set the screen gamma values: */
    (*png_ptr).screen_gamma = output_gamma;

    /* Finally, if pre-multiplying, set the background fields. */
    if compose != 0 {
        /* And obtain alpha pre-multiplication by composing on black: */
        memset(
            &mut (*png_ptr).background as *mut png_color_16 as *mut c_void,
            0,
            core::mem::size_of::<png_color_16>(),
        );
        (*png_ptr).background_gamma = file_gamma; /* just in case */
        (*png_ptr).background_gamma_type = PNG_BACKGROUND_GAMMA_FILE as png_byte;
        (*png_ptr).transformations &= !PNG_BACKGROUND_EXPAND;

        if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
            png_error(
                png_ptr,
                cstr(b"conflicting calls to set alpha mode and background\0"),
            );
        }

        (*png_ptr).transformations |= PNG_COMPOSE;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_alpha_mode(png_ptr: png_structrp, mode: c_int, output_gamma: f64) {
    png_set_alpha_mode_fixed(png_ptr, mode, convert_gamma_value(png_ptr, output_gamma));
}

/* Dither file to 8-bit.  Supply a palette, the current number of elements in
 * the palette, the maximum number of elements allowed, and a histogram if
 * possible.
 */
#[repr(C)]
#[derive(Copy, Clone)]
struct png_dsort {
    next: *mut png_dsort,
    left: png_byte,
    right: png_byte,
}
type png_dsortp = *mut png_dsort;
type png_dsortpp = *mut png_dsortp;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_quantize(
    png_ptr: png_structrp,
    palette: png_colorp,
    mut num_palette: c_int,
    maximum_colors: c_int,
    histogram: png_const_uint_16p,
    full_quantize: c_int,
) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    if palette.is_null() {
        return;
    }

    (*png_ptr).transformations |= PNG_QUANTIZE;

    if full_quantize == 0 {
        let mut i: c_int;

        /* Initialize the array to index colors. */
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

            /* Find the least used palette entries by starting a bubble sort. */
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

                /* Put all the useful colors within the max, but don't move the
                 * others.
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

                /* Move all the used colors inside the max limit, and develop a
                 * translation table.
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

                            d = PNG_COLOR_DIST(
                                *palette.add(d_index as usize),
                                *palette.add(k as usize),
                            );

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
            /* This is much harder to do simply (and quickly).  ... just find
             * the closest two colors, and throw out one of them.
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
                (769usize * core::mem::size_of::<png_dsortp>()) as png_alloc_size_t,
            ) as png_dsortpp;

            num_new_palette = num_palette;

            /* Initial wild guess at how far apart the farthest pixel pair we
             * will be eliminating will be.
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
                                if ((*(*png_ptr).index_to_palette.add((*p).left as usize)) as c_int)
                                    < num_new_palette
                                    && ((*(*png_ptr).index_to_palette.add((*p).right as usize))
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
                                    *palette.add(
                                        *(*png_ptr).index_to_palette.add(j as usize) as usize
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
        /* Allocate an owned copy rather than aliasing the caller's pointer. */
        (*png_ptr).palette = png_calloc(
            png_ptr,
            (PNG_MAX_PALETTE_LENGTH as usize * core::mem::size_of::<png_color>())
                as png_alloc_size_t,
        ) as png_colorp;
        memcpy(
            (*png_ptr).palette as *mut c_void,
            palette as *const c_void,
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

        memset(distance as *mut c_void, 0xff, num_entries);

        i = 0;
        while i < num_palette {
            let mut ir: c_int;
            let mut ig: c_int;
            let mut ib: c_int;
            let r: c_int = ((*palette.add(i as usize)).red >> (8 - PNG_QUANTIZE_RED_BITS)) as c_int;
            let g: c_int =
                ((*palette.add(i as usize)).green >> (8 - PNG_QUANTIZE_GREEN_BITS)) as c_int;
            let b: c_int =
                ((*palette.add(i as usize)).blue >> (8 - PNG_QUANTIZE_BLUE_BITS)) as c_int;

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

                        if d < *distance.add(d_index as usize) as c_int {
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
pub unsafe extern "C" fn png_set_gamma_fixed(
    png_ptr: png_structrp,
    mut scrn_gamma: png_fixed_point,
    mut file_gamma: png_fixed_point,
) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    /* New in libpng-1.5.4 - reserve particular negative values as flags. */
    scrn_gamma = translate_gamma_flags(scrn_gamma, 1 /*screen*/);
    file_gamma = translate_gamma_flags(file_gamma, 0 /*file*/);

    if file_gamma <= 0 {
        png_app_error(png_ptr, cstr(b"invalid file gamma in png_set_gamma\0"));
    }
    if scrn_gamma <= 0 {
        png_app_error(png_ptr, cstr(b"invalid screen gamma in png_set_gamma\0"));
    }

    if unsupported_gamma(png_ptr, file_gamma, 1 /*warn*/) != 0
        || unsupported_gamma(png_ptr, scrn_gamma, 1 /*warn*/) != 0
    {
        return;
    }

    /* 1.6.47: png_struct::file_gamma and png_struct::screen_gamma are now only
     * written by this API.
     */
    (*png_ptr).file_gamma = file_gamma;
    (*png_ptr).screen_gamma = scrn_gamma;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gamma(png_ptr: png_structrp, scrn_gamma: f64, file_gamma: f64) {
    /* NOTE: right-to-left argument evaluation in the reference C build. */
    let file_fx = convert_gamma_value(png_ptr, file_gamma);
    let scrn_fx = convert_gamma_value(png_ptr, scrn_gamma);
    png_set_gamma_fixed(png_ptr, scrn_fx, file_fx);
}

/* Expand paletted images to RGB, expand grayscale images of less than 8-bit
 * depth to 8-bit depth, and expand tRNS chunks to alpha channels.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_expand(png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0) == 0 {
        return;
    }

    (*png_ptr).transformations |= PNG_EXPAND | PNG_EXPAND_tRNS;
}

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

/* Expand to 16-bit channels, expand the tRNS chunk too. */
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
            png_error(png_ptr, cstr(b"invalid error action to rgb_to_gray\0"));
        }
    }

    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        (*png_ptr).transformations |= PNG_EXPAND;
    }
    {
        if red >= 0 && green >= 0 && red + green <= PNG_FP_1 {
            let red_int: png_uint_16;
            let green_int: png_uint_16;

            /* NOTE: this calculation does not round, but this behavior is
             * retained for consistency; the inaccuracy is very small.
             */
            red_int = ((red as png_uint_32).wrapping_mul(32768) / 100000) as png_uint_16;
            green_int = ((green as png_uint_32).wrapping_mul(32768) / 100000) as png_uint_16;

            (*png_ptr).rgb_to_gray_red_coeff = red_int;
            (*png_ptr).rgb_to_gray_green_coeff = green_int;
            (*png_ptr).rgb_to_gray_coefficients_set = 1;
        } else if red >= 0 && green >= 0 {
            png_app_warning(
                png_ptr,
                cstr(b"ignoring out of range rgb_to_gray coefficients\0"),
            );
        }
    }
}

/* Convert a RGB image to a grayscale of the same width. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_rgb_to_gray(
    png_ptr: png_structrp,
    error_action: c_int,
    red: f64,
    green: f64,
) {
    /* NOTE: the reference C build evaluates this argument list right to left,
     * so the green coefficient is converted (and may raise png_fixed_error)
     * before the red one. */
    let green_fx = png_fixed(png_ptr, green, cstr(b"rgb to gray green coefficient\0"));
    let red_fx = png_fixed(png_ptr, red, cstr(b"rgb to gray red coefficient\0"));
    png_set_rgb_to_gray_fixed(png_ptr, error_action, red_fx, green_fx);
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
 * the [file] gamma and screen_gamma are not close reciprocals.
 */
pub unsafe extern "C" fn png_gamma_threshold(
    screen_gamma: png_fixed_point,
    file_gamma: png_fixed_point,
) -> c_int {
    /* We want to compare the threshold with s*f - 1. */
    let mut gtest: png_fixed_point = 0;
    (if png_muldiv(
        &mut gtest as *mut png_fixed_point,
        screen_gamma,
        file_gamma,
        PNG_FP_1,
    ) == 0
        || png_gamma_significant(gtest) != 0
    {
        1
    } else {
        0
    }) as c_int
}
