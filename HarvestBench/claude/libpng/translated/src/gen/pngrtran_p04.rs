/* pngrtran.c lines 1177..1424 */

/* Initialize everything needed for the read.  This includes modifying
 * the palette.
 */

/* For the moment 'png_init_palette_transformations' and
 * 'png_init_rgb_transformations' only do some flag canceling optimizations.
 * The intent is that these two routines should have palette or rgb operations
 * extracted from 'png_init_read_transformations'.
 */
/* png_init_palette_transformations */
unsafe fn png_init_palette_transformations(png_ptr: png_structrp) {
    /* Called to handle the (input) palette case.  In png_do_read_transformations
     * the first step is to expand the palette if requested, so this code must
     * take care to only make changes that are invariant with respect to the
     * palette expansion, or only do them if there is no expansion.
     *
     * STRIP_ALPHA has already been handled in the caller (by setting num_trans
     * to 0.)
     */
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
        /* Any alpha means background and associative alpha processing is
         * required, however if the alpha is 0 or 1 throughout OPTIMIZE_ALPHA
         * and ENCODE_ALPHA are irrelevant.
         */
        (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
        (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;

        if input_has_transparency == 0 {
            (*png_ptr).transformations &= !(PNG_COMPOSE | PNG_BACKGROUND_EXPAND);
        }
    }

    /* png_set_background handling - deals with the complexity of whether the
     * background color is in the file format or the screen format in the case
     * where an 'expand' will happen.
     */

    /* The following code cannot be entered in the alpha pre-multiplication case
     * because PNG_BACKGROUND_EXPAND is cancelled below.
     */
    if ((*png_ptr).transformations & PNG_BACKGROUND_EXPAND) != 0
        && ((*png_ptr).transformations & PNG_EXPAND) != 0
    {
        {
            (*png_ptr).background.red = (*(*png_ptr)
                .palette
                .add((*png_ptr).background.index as usize))
            .red as png_uint_16;
            (*png_ptr).background.green = (*(*png_ptr)
                .palette
                .add((*png_ptr).background.index as usize))
            .green as png_uint_16;
            (*png_ptr).background.blue = (*(*png_ptr)
                .palette
                .add((*png_ptr).background.index as usize))
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
                        *(*png_ptr).trans_alpha.offset(i as isize) =
                            (255 - *(*png_ptr).trans_alpha.offset(i as isize) as c_int) as png_byte;
                        i += 1;
                    }
                }
            }
        }
    } /* background expand and (therefore) no alpha association. */
}

/* png_init_rgb_transformations */
unsafe fn png_init_rgb_transformations(png_ptr: png_structrp) {
    /* Added to libpng-1.5.4: check the color type to determine whether there
     * is any alpha or transparency in the image and simply cancel the
     * background and alpha mode stuff if there isn't.
     */
    let input_has_alpha: c_int = (((*png_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0) as c_int;
    let input_has_transparency: c_int = ((*png_ptr).num_trans > 0) as c_int;

    /* If no alpha we can optimize. */
    if input_has_alpha == 0 {
        /* Any alpha means background and associative alpha processing is
         * required, however if the alpha is 0 or 1 throughout OPTIMIZE_ALPHA
         * and ENCODE_ALPHA are irrelevant.
         */
        (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
        (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;

        if input_has_transparency == 0 {
            (*png_ptr).transformations &= !(PNG_COMPOSE | PNG_BACKGROUND_EXPAND);
        }
    }

    /* png_set_background handling - deals with the complexity of whether the
     * background color is in the file format or the screen format in the case
     * where an 'expand' will happen.
     */

    /* The following code cannot be entered in the alpha pre-multiplication case
     * because PNG_BACKGROUND_EXPAND is cancelled below.
     */
    if ((*png_ptr).transformations & PNG_BACKGROUND_EXPAND) != 0
        && ((*png_ptr).transformations & PNG_EXPAND) != 0
        && ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0
    /* i.e., GRAY or GRAY_ALPHA */
    {
        {
            /* Expand background and tRNS chunks */
            let mut gray: c_int = (*png_ptr).background.gray as c_int;
            let mut trans_gray: c_int = (*png_ptr).trans_color.gray as c_int;

            match (*png_ptr).bit_depth as c_int {
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

                /* default, 8 (already 8 bits) and 16 (already a full 16 bits) */
                _ => {}
            }

            (*png_ptr).background.blue = gray as png_uint_16;
            (*png_ptr).background.green = (*png_ptr).background.blue;
            (*png_ptr).background.red = (*png_ptr).background.green;

            if ((*png_ptr).transformations & PNG_EXPAND_tRNS) == 0 {
                (*png_ptr).trans_color.blue = trans_gray as png_uint_16;
                (*png_ptr).trans_color.green = (*png_ptr).trans_color.blue;
                (*png_ptr).trans_color.red = (*png_ptr).trans_color.green;
            }
        }
    } /* background expand and (therefore) no alpha association. */
}

/* png_resolve_file_gamma */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_resolve_file_gamma(png_ptr: png_const_structrp) -> png_fixed_point {
    let mut file_gamma: png_fixed_point;

    /* The file gamma is determined by these precedence rules, in this order
     * (i.e. use the first value found):
     *
     *    png_set_gamma; png_struct::file_gammma if not zero, then:
     *    png_struct::chunk_gamma if not 0 (determined the PNGv3 rules), then:
     *    png_set_gamma; 1/png_struct::screen_gamma if not zero
     *
     *    0 (i.e. do no gamma handling)
     */
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

    /* If png_reciprocal overflows, it returns 0, indicating to the caller that
     * there is no usable file gamma.  (The checks added to png_set_gamma and
     * png_set_alpha_mode should prevent a screen_gamma which would overflow.)
     */
    if (*png_ptr).screen_gamma != 0 {
        file_gamma = png_reciprocal((*png_ptr).screen_gamma);
    }

    file_gamma
}

/* png_init_gamma_values */
unsafe fn png_init_gamma_values(png_ptr: png_structrp) -> c_int {
    /* The following temporary indicates if overall gamma correction is
     * required.
     */
    let mut gamma_correction: c_int = 0;
    let mut file_gamma: png_fixed_point;
    let mut screen_gamma: png_fixed_point;

    /* Resolve the file_gamma.  See above: if png_ptr::screen_gamma is set
     * file_gamma will always be set here:
     */
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
            /* Assume the output matches the input; a long time default behavior
             * of libpng, although the standard has nothing to say about this.
             */
            screen_gamma = png_reciprocal(file_gamma);
        }
    } else {
        /* both unset, prevent corrections: */
        screen_gamma = PNG_FP_1;
        file_gamma = screen_gamma;
    }

    (*png_ptr).file_gamma = file_gamma;
    (*png_ptr).screen_gamma = screen_gamma;
    gamma_correction
}
