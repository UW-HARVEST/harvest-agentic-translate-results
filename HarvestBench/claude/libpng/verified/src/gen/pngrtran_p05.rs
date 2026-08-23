/* pngrtran.c lines 1425..2069 */

/* png_init_read_transformations */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_init_read_transformations(png_ptr: png_structrp) {
    /* This internal function is called from png_read_start_row in pngrutil.c
     * and it is called before the 'rowbytes' calculation is done, so the code
     * in here can change or update the transformations flags.
     *
     * First do updates that do not depend on the details of the PNG image data
     * being processed.
     */

    /* Prior to 1.5.4 these tests were performed from png_set_gamma, 1.5.4 adds
     * png_set_alpha_mode and this is another source for a default file gamma so
     * the test needs to be performed later - here.  In addition prior to 1.5.4
     * the tests were repeated for the PALETTE color type here - this is no
     * longer necessary (and doesn't seem to have been necessary before.)
     *
     * PNGv3: the new mandatory precedence/priority rules for colour space chunks
     * are handled here (by calling the above function).
     *
     * Turn the gamma transformation on or off as appropriate.  Notice that
     * PNG_GAMMA just refers to the file->screen correction.  Alpha composition
     * may independently cause gamma correction because it needs linear data
     * (e.g. if the file has a gAMA chunk but the screen gamma hasn't been
     * specified.)  In any case this flag may get turned off in the code
     * immediately below if the transform can be handled outside the row loop.
     */
    if png_init_gamma_values(png_ptr) != 0 {
        (*png_ptr).transformations |= PNG_GAMMA;
    } else {
        (*png_ptr).transformations &= !PNG_GAMMA;
    }

    /* Certain transformations have the effect of preventing other
     * transformations that happen afterward in png_do_read_transformations;
     * resolve the interdependencies here.  From the code of
     * png_do_read_transformations the order is:
     *
     *  1) PNG_EXPAND (including PNG_EXPAND_tRNS)
     *  2) PNG_STRIP_ALPHA (if no compose)
     *  3) PNG_RGB_TO_GRAY
     *  4) PNG_GRAY_TO_RGB iff !PNG_BACKGROUND_IS_GRAY
     *  5) PNG_COMPOSE
     *  6) PNG_GAMMA
     *  7) PNG_STRIP_ALPHA (if compose)
     *  8) PNG_ENCODE_ALPHA
     *  9) PNG_SCALE_16_TO_8
     * 10) PNG_16_TO_8
     * 11) PNG_QUANTIZE (converts to palette)
     * 12) PNG_EXPAND_16
     * 13) PNG_GRAY_TO_RGB iff PNG_BACKGROUND_IS_GRAY
     * 14) PNG_INVERT_MONO
     * 15) PNG_INVERT_ALPHA
     * 16) PNG_SHIFT
     * 17) PNG_PACK
     * 18) PNG_BGR
     * 19) PNG_PACKSWAP
     * 20) PNG_FILLER (includes PNG_ADD_ALPHA)
     * 21) PNG_SWAP_ALPHA
     * 22) PNG_SWAP_BYTES
     * 23) PNG_USER_TRANSFORM [must be last]
     */
    if ((*png_ptr).transformations & PNG_STRIP_ALPHA) != 0
        && ((*png_ptr).transformations & PNG_COMPOSE) == 0
    {
        /* Stripping the alpha channel happens immediately after the 'expand'
         * transformations, before all other transformation, so it cancels out
         * the alpha handling.  It has the side effect negating the effect of
         * PNG_EXPAND_tRNS too:
         */
        (*png_ptr).transformations &=
            !(PNG_BACKGROUND_EXPAND | PNG_ENCODE_ALPHA | PNG_EXPAND_tRNS);
        (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;

        /* Kill the tRNS chunk itself too.  Prior to 1.5.4 this did not happen
         * so transparency information would remain just so long as it wasn't
         * expanded.  This produces unexpected API changes if the set of things
         * that do PNG_EXPAND_tRNS changes (perfectly possible given the
         * documentation - which says ask for what you want, accept what you
         * get.)  This makes the behavior consistent from 1.5.4:
         */
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
     * gray --> RGB case.
     *
     * Note:  if PNG_BACKGROUND_EXPAND is set and color_type is either RGB or
     * RGB_ALPHA (in which case need_expand is superfluous anyway), the
     * background color might actually be gray yet not be flagged as such.
     * This is not a problem for the current code, which uses
     * PNG_BACKGROUND_IS_GRAY only to decide when to do the
     * png_do_gray_to_rgb() transformation.
     *
     * TODO: this code needs to be revised to avoid the complexity and
     * interdependencies.  The color type of the background should be recorded in
     * png_set_background, along with the bit depth, then the code has a record
     * of exactly what color space the background is currently in.
     */
    if ((*png_ptr).transformations & PNG_BACKGROUND_EXPAND) != 0 {
        /* PNG_BACKGROUND_EXPAND: the background is in the file color space, so if
         * the file was grayscale the background value is gray.
         */
        if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0 {
            (*png_ptr).mode |= PNG_BACKGROUND_IS_GRAY;
        }
    } else if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
        /* PNG_COMPOSE: png_set_background was called with need_expand false,
         * so the color is in the color space of the output or png_set_alpha_mode
         * was called and the color is black.  Ignore RGB_TO_GRAY because that
         * happens before GRAY_TO_RGB.
         */
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
     * can be performed directly on the palette, and some (such as rgb to gray)
     * can be optimized inside the palette.  This is particularly true of the
     * composite (background and alpha) stuff, which can be pretty much all done
     * in the palette even if the result is expanded to RGB or gray afterward.
     *
     * NOTE: this is Not Yet Implemented, the code behaves as in 1.5.1 and
     * earlier and the palette stuff is actually handled on the first row.  This
     * leads to the reported bug that the palette returned by png_get_PLTE is not
     * updated.
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
        /* TODO: fix this.  Because the expand_16 operation is after the compose
         * handling the background color must be 8, not 16, bits deep, but the
         * application will supply a 16-bit value so reduce it here.
         *
         * The PNG_BACKGROUND_EXPAND code above does not expand to 16 bits at
         * present, so that case is ok (until do_expand_16 is moved.)
         *
         * NOTE: this discards the low 16 bits of the user supplied background
         * color, but until expand_16 works properly there is no choice!
         */
        /* CHOP(x) (x)=((png_uint_16)PNG_DIV257(x)) */
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
        /* On the other hand, if a 16-bit file is to be reduced to 8-bits per
         * component this will also happen after PNG_COMPOSE and so the background
         * color must be pre-expanded here.
         *
         * TODO: fix this too.
         */
        (*png_ptr).background.red =
            ((*png_ptr).background.red as c_int).wrapping_mul(257) as png_uint_16;
        (*png_ptr).background.green =
            ((*png_ptr).background.green as c_int).wrapping_mul(257) as png_uint_16;
        (*png_ptr).background.blue =
            ((*png_ptr).background.blue as c_int).wrapping_mul(257) as png_uint_16;
        (*png_ptr).background.gray =
            ((*png_ptr).background.gray as c_int).wrapping_mul(257) as png_uint_16;
    }

    /* NOTE: below 'PNG_READ_ALPHA_MODE_SUPPORTED' is presumed to also enable the
     * background support (see the comments in scripts/pnglibconf.dfa), this
     * allows pre-multiplication of the alpha channel to be implemented as
     * compositing on black.  This is probably sub-optimal and has been done in
     * 1.5.4 betas simply to enable external critique and testing (i.e. to
     * implement the new API quickly, without lots of internal changes.)
     */

    /* Includes ALPHA_MODE */
    (*png_ptr).background_1 = (*png_ptr).background;

    /* This needs to change - in the palette image case a whole set of tables are
     * built when it would be quicker to just calculate the correct value for
     * each palette entry directly.  Also, the test is too tricky - why check
     * PNG_RGB_TO_GRAY if PNG_GAMMA is not set?  The answer seems to be that
     * PNG_GAMMA is cancelled even if the gamma is known?  The test excludes the
     * PNG_COMPOSE case, so apparently if there is no *overall* gamma correction
     * the gamma tables will not be built even if composition is required on a
     * gamma encoded value.
     *
     * In 1.5.4 this is addressed below by an additional check on the individual
     * file gamma - if it is not 1.0 both RGB_TO_GRAY and COMPOSE need the
     * tables.
     */
    if ((*png_ptr).transformations & PNG_GAMMA) != 0
        || (((*png_ptr).transformations & PNG_RGB_TO_GRAY) != 0
            && (png_gamma_significant((*png_ptr).file_gamma) != 0
                || png_gamma_significant((*png_ptr).screen_gamma) != 0))
        || (((*png_ptr).transformations & PNG_COMPOSE) != 0
            && (png_gamma_significant((*png_ptr).file_gamma) != 0
                || png_gamma_significant((*png_ptr).screen_gamma) != 0
                || ((*png_ptr).background_gamma_type as c_int
                    == PNG_BACKGROUND_GAMMA_UNIQUE
                    && png_gamma_significant((*png_ptr).background_gamma) != 0)))
        || (((*png_ptr).transformations & PNG_ENCODE_ALPHA) != 0
            && png_gamma_significant((*png_ptr).screen_gamma) != 0)
    {
        png_build_gamma_table(png_ptr, (*png_ptr).bit_depth as c_int);

        if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
            /* Issue a warning about this combination: because RGB_TO_GRAY is
             * optimized to do the gamma transform if present yet do_background has
             * to do the same thing if both options are set a
             * double-gamma-correction happens.  This is true in all versions of
             * libpng to date.
             */
            if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) != 0 {
                png_warning(
                    png_ptr,
                    b"libpng does not support gamma+background+rgb_to_gray\0".as_ptr()
                        as png_const_charp,
                );
            }

            if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
                /* We don't get to here unless there is a tRNS chunk with non-opaque
                 * entries - see the checking code at the start of this function.
                 */
                let mut back: png_color = Default::default();
                let mut back_1: png_color = Default::default();
                let palette: png_colorp = (*png_ptr).palette;
                let num_palette: c_int = (*png_ptr).num_palette as c_int;
                let mut i: c_int;
                if (*png_ptr).background_gamma_type as c_int == PNG_BACKGROUND_GAMMA_FILE {
                    back.red = *(*png_ptr)
                        .gamma_table
                        .add((*png_ptr).background.red as usize);
                    back.green = *(*png_ptr)
                        .gamma_table
                        .add((*png_ptr).background.green as usize);
                    back.blue = *(*png_ptr)
                        .gamma_table
                        .add((*png_ptr).background.blue as usize);

                    back_1.red = *(*png_ptr)
                        .gamma_to_1
                        .add((*png_ptr).background.red as usize);
                    back_1.green = *(*png_ptr)
                        .gamma_to_1
                        .add((*png_ptr).background.green as usize);
                    back_1.blue = *(*png_ptr)
                        .gamma_to_1
                        .add((*png_ptr).background.blue as usize);
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
                            g = PNG_FP_1; /* back_1 */
                            gs = PNG_FP_1; /* back */
                        }
                    }

                    if png_gamma_significant(gs) != 0 {
                        back.red = png_gamma_8bit_correct((*png_ptr).background.red as c_uint, gs);
                        back.green =
                            png_gamma_8bit_correct((*png_ptr).background.green as c_uint, gs);
                        back.blue = png_gamma_8bit_correct((*png_ptr).background.blue as c_uint, gs);
                    } else {
                        back.red = (*png_ptr).background.red as png_byte;
                        back.green = (*png_ptr).background.green as png_byte;
                        back.blue = (*png_ptr).background.blue as png_byte;
                    }

                    if png_gamma_significant(g) != 0 {
                        back_1.red = png_gamma_8bit_correct((*png_ptr).background.red as c_uint, g);
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
                        /* if (png_ptr->trans_alpha[i] != 0xff) */
                        {
                            if ((*png_ptr).flags & PNG_FLAG_OPTIMIZE_ALPHA) != 0 {
                                /* Premultiply only:
                                 * component = round((component * alpha) / 255)
                                 */
                                let mut component: png_uint_32;

                                component = *(*png_ptr)
                                    .gamma_to_1
                                    .add((*palette.offset(i as isize)).red as usize)
                                    as png_uint_32;
                                component = (component
                                    .wrapping_mul(
                                        *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_32
                                    )
                                    .wrapping_add(128))
                                    / 255;
                                (*palette.offset(i as isize)).red =
                                    *(*png_ptr).gamma_from_1.add(component as usize);

                                component = *(*png_ptr)
                                    .gamma_to_1
                                    .add((*palette.offset(i as isize)).green as usize)
                                    as png_uint_32;
                                component = (component
                                    .wrapping_mul(
                                        *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_32
                                    )
                                    .wrapping_add(128))
                                    / 255;
                                (*palette.offset(i as isize)).green =
                                    *(*png_ptr).gamma_from_1.add(component as usize);

                                component = *(*png_ptr)
                                    .gamma_to_1
                                    .add((*palette.offset(i as isize)).blue as usize)
                                    as png_uint_32;
                                component = (component
                                    .wrapping_mul(
                                        *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_32
                                    )
                                    .wrapping_add(128))
                                    / 255;
                                (*palette.offset(i as isize)).blue =
                                    *(*png_ptr).gamma_from_1.add(component as usize);
                            } else {
                                /* Composite with background color:
                                 * component =
                                 *    alpha * component + (1 - alpha) * background
                                 */
                                let mut v: png_byte;
                                let mut w: png_byte;

                                v = *(*png_ptr)
                                    .gamma_to_1
                                    .add((*palette.offset(i as isize)).red as usize);
                                w = png_composite(
                                    v as png_uint_16,
                                    *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16,
                                    back_1.red as png_uint_16,
                                );
                                (*palette.offset(i as isize)).red =
                                    *(*png_ptr).gamma_from_1.add(w as usize);

                                v = *(*png_ptr)
                                    .gamma_to_1
                                    .add((*palette.offset(i as isize)).green as usize);
                                w = png_composite(
                                    v as png_uint_16,
                                    *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16,
                                    back_1.green as png_uint_16,
                                );
                                (*palette.offset(i as isize)).green =
                                    *(*png_ptr).gamma_from_1.add(w as usize);

                                v = *(*png_ptr)
                                    .gamma_to_1
                                    .add((*palette.offset(i as isize)).blue as usize);
                                w = png_composite(
                                    v as png_uint_16,
                                    *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16,
                                    back_1.blue as png_uint_16,
                                );
                                (*palette.offset(i as isize)).blue =
                                    *(*png_ptr).gamma_from_1.add(w as usize);
                            }
                        }
                    } else {
                        (*palette.offset(i as isize)).red = *(*png_ptr)
                            .gamma_table
                            .add((*palette.offset(i as isize)).red as usize);
                        (*palette.offset(i as isize)).green = *(*png_ptr)
                            .gamma_table
                            .add((*palette.offset(i as isize)).green as usize);
                        (*palette.offset(i as isize)).blue = *(*png_ptr)
                            .gamma_table
                            .add((*palette.offset(i as isize)).blue as usize);
                    }
                    i += 1;
                }

                /* Prevent the transformations being done again.
                 *
                 * NOTE: this is highly dubious; it removes the transformations in
                 * place.  This seems inconsistent with the general treatment of the
                 * transformations elsewhere.
                 */
                (*png_ptr).transformations &= !(PNG_COMPOSE | PNG_GAMMA);
                (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
            }
            /* color_type == PNG_COLOR_TYPE_PALETTE */
            /* if (png_ptr->background_gamma_type!=PNG_BACKGROUND_GAMMA_UNKNOWN) */
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
                        gs = png_reciprocal2((*png_ptr).background_gamma, (*png_ptr).screen_gamma);
                    }

                    _ => {
                        png_error(
                            png_ptr,
                            b"invalid background gamma type\0".as_ptr() as png_const_charp,
                        );
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

                        (*png_ptr).background_1.green =
                            png_gamma_correct(png_ptr, (*png_ptr).background.green as c_uint, g);

                        (*png_ptr).background_1.blue =
                            png_gamma_correct(png_ptr, (*png_ptr).background.blue as c_uint, g);
                    }

                    if gs_sig != 0 {
                        (*png_ptr).background.red =
                            png_gamma_correct(png_ptr, (*png_ptr).background.red as c_uint, gs);

                        (*png_ptr).background.green =
                            png_gamma_correct(png_ptr, (*png_ptr).background.green as c_uint, gs);

                        (*png_ptr).background.blue =
                            png_gamma_correct(png_ptr, (*png_ptr).background.blue as c_uint, gs);
                    }
                } else {
                    /* GRAY, GRAY ALPHA, RGB, or RGBA with gray background */
                    (*png_ptr).background_1.blue = (*png_ptr).background_1.gray;
                    (*png_ptr).background_1.green = (*png_ptr).background_1.blue;
                    (*png_ptr).background_1.red = (*png_ptr).background_1.green;

                    (*png_ptr).background.blue = (*png_ptr).background.gray;
                    (*png_ptr).background.green = (*png_ptr).background.blue;
                    (*png_ptr).background.red = (*png_ptr).background.green;
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

            /* NOTE: there are other transformations that should probably be in
             * here too.
             */
            i = 0;
            while i < num_palette {
                (*palette.offset(i as isize)).red = *(*png_ptr)
                    .gamma_table
                    .add((*palette.offset(i as isize)).red as usize);
                (*palette.offset(i as isize)).green = *(*png_ptr)
                    .gamma_table
                    .add((*palette.offset(i as isize)).green as usize);
                (*palette.offset(i as isize)).blue = *(*png_ptr)
                    .gamma_table
                    .add((*palette.offset(i as isize)).blue as usize);
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
        let mut back: png_color = Default::default();
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

        /* significant bits can be in the range 1 to 7 for a meaningful result, if
         * the number of significant bits is 0 then no shift is done (this is an
         * error condition which is silently ignored.)
         */
        if shift > 0 && shift < 8 {
            i = 0;
            while i < istop {
                let mut component: c_int = (*(*png_ptr).palette.offset(i as isize)).red as c_int;

                component >>= shift;
                (*(*png_ptr).palette.offset(i as isize)).red = component as png_byte;
                i += 1;
            }
        }

        shift = 8 - (*png_ptr).sig_bit.green as c_int;
        if shift > 0 && shift < 8 {
            i = 0;
            while i < istop {
                let mut component: c_int = (*(*png_ptr).palette.offset(i as isize)).green as c_int;

                component >>= shift;
                (*(*png_ptr).palette.offset(i as isize)).green = component as png_byte;
                i += 1;
            }
        }

        shift = 8 - (*png_ptr).sig_bit.blue as c_int;
        if shift > 0 && shift < 8 {
            i = 0;
            while i < istop {
                let mut component: c_int = (*(*png_ptr).palette.offset(i as isize)).blue as c_int;

                component >>= shift;
                (*(*png_ptr).palette.offset(i as isize)).blue = component as png_byte;
                i += 1;
            }
        }
    }
}
