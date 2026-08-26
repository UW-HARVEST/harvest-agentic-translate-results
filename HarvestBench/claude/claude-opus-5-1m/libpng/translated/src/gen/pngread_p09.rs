/* The guts of png_image_finish_read as a png_safe_execute callback. */
unsafe extern "C" fn png_image_read_direct(argument: png_voidp) -> c_int {
    let display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let image: png_imagep = (*display).image;
    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
    let info_ptr: png_inforp = (*(*image).opaque).info_ptr;

    let mut format: png_uint_32 = (*image).format;
    let linear: c_int = if (format & PNG_FORMAT_FLAG_LINEAR) != 0 {
        1
    } else {
        0
    };
    let mut do_local_compose: c_int = 0;
    let mut do_local_background: c_int = 0; /* to avoid double gamma correction bug */
    let mut do_local_scale: c_int = 0; /* for interlaced 16-to-8 bit conversion */
    let mut passes: c_int = 0;

    /* Add transforms to ensure the correct output format is produced then check
     * that the required implementation support is there.  Always expand; always
     * need 8 bits minimum, no palette and expanded tRNS.
     */
    png_set_expand(png_ptr);

    /* Now check the format to see if it was modified. */
    {
        let base_format: png_uint_32 = png_image_format(png_ptr)
            & !PNG_FORMAT_FLAG_COLORMAP /* removed by png_set_expand */;
        let mut change: png_uint_32 = format ^ base_format;
        let output_gamma: png_fixed_point;
        let mut mode: c_int; /* alpha mode */

        /* Do this first so that we have a record if rgb to gray is happening. */
        if (change & PNG_FORMAT_FLAG_COLOR) != 0 {
            /* gray<->color transformation required. */
            if (format & PNG_FORMAT_FLAG_COLOR) != 0 {
                png_set_gray_to_rgb(png_ptr);
            } else {
                /* libpng can't do both rgb to gray and
                 * background/pre-multiplication if there is also significant gamma
                 * correction, because both operations require linear colors and
                 * the code only supports one transform doing the gamma correction.
                 * Handle this by doing the pre-multiplication or background
                 * operation in this code, if necessary.
                 *
                 * TODO: fix this by rewriting pngrtran.c (!)
                 *
                 * For the moment (given that fixing this in pngrtran.c is an
                 * enormous change) 'do_local_background' is used to indicate that
                 * the problem exists.
                 */
                if (base_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                    do_local_background = 1; /*maybe*/
                }

                png_set_rgb_to_gray_fixed(
                    png_ptr,
                    PNG_ERROR_ACTION_NONE,
                    PNG_RGB_TO_GRAY_DEFAULT,
                    PNG_RGB_TO_GRAY_DEFAULT,
                );
            }

            change &= !PNG_FORMAT_FLAG_COLOR;
        }

        /* Set the gamma appropriately, linear for 16-bit input, sRGB otherwise.
         */
        {
            /* This is safe but should no longer be necessary as
             * png_ptr->default_gamma should have been set after the
             * info-before-IDAT was read in png_image_read_header.
             *
             * TODO: 1.8: remove this and see what happens.
             */
            let input_gamma_default: png_fixed_point;

            if (base_format & PNG_FORMAT_FLAG_LINEAR) != 0
                && ((*image).flags & PNG_IMAGE_FLAG_16BIT_sRGB) == 0
            {
                input_gamma_default = PNG_GAMMA_LINEAR;
            } else {
                input_gamma_default = PNG_DEFAULT_sRGB;
            }

            /* Call png_set_alpha_mode to set the default for the input gamma; the
             * output gamma is set by a second call below.
             */
            png_set_alpha_mode_fixed(png_ptr, PNG_ALPHA_PNG, input_gamma_default);
        }

        if linear != 0 {
            /* If there *is* an alpha channel in the input it must be multiplied
             * out; use PNG_ALPHA_STANDARD, otherwise just use PNG_ALPHA_PNG.
             */
            if (base_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                mode = PNG_ALPHA_STANDARD; /* associated alpha */
            } else {
                mode = PNG_ALPHA_PNG;
            }

            output_gamma = PNG_GAMMA_LINEAR;
        } else {
            mode = PNG_ALPHA_PNG;
            output_gamma = PNG_DEFAULT_sRGB;
        }

        if (change & PNG_FORMAT_FLAG_ASSOCIATED_ALPHA) != 0 {
            mode = PNG_ALPHA_OPTIMIZED;
            change &= !PNG_FORMAT_FLAG_ASSOCIATED_ALPHA;
        }

        /* If 'do_local_background' is set check for the presence of gamma
         * correction; this is part of the work-round for the libpng bug
         * described above.
         *
         * TODO: fix libpng and remove this.
         */
        if do_local_background != 0 {
            let mut gtest: png_fixed_point = 0;

            /* This is 'png_gamma_threshold' from pngrtran.c; the test used for
             * gamma correction, the screen gamma hasn't been set on png_struct
             * yet; it's set below.  png_struct::gamma, however, is set to the
             * final value.
             */
            if png_muldiv(
                &mut gtest,
                output_gamma,
                png_resolve_file_gamma(png_ptr),
                PNG_FP_1,
            ) != 0
                && png_gamma_significant(gtest) == 0
            {
                do_local_background = 0;
            } else if mode == PNG_ALPHA_STANDARD {
                do_local_background = 2; /*required*/
                mode = PNG_ALPHA_PNG; /* prevent libpng doing it */
            }

            /* else leave as 1 for the checks below */
        }

        /* If the bit-depth changes then handle that here. */
        if (change & PNG_FORMAT_FLAG_LINEAR) != 0 {
            if linear != 0
            /*16-bit output*/
            {
                png_set_expand_16(png_ptr);
            } else
            /* 8-bit output */
            {
                png_set_scale_16(png_ptr);

                /* For interlaced images, use local_row buffer to avoid overflow
                 * in png_combine_row() which writes using IHDR bit-depth.
                 */
                if (*png_ptr).interlaced != 0 {
                    do_local_scale = 1;
                }
            }

            change &= !PNG_FORMAT_FLAG_LINEAR;
        }

        /* Now the background/alpha channel changes. */
        if (change & PNG_FORMAT_FLAG_ALPHA) != 0 {
            /* Removing an alpha channel requires composition for the 8-bit
             * formats; for the 16-bit it is already done, above, by the
             * pre-multiplication and the channel just needs to be stripped.
             */
            if (base_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                /* If RGB->gray is happening the alpha channel must be left and the
                 * operation completed locally.
                 *
                 * TODO: fix libpng and remove this.
                 */
                if do_local_background != 0 {
                    do_local_background = 2; /*required*/
                }
                /* 16-bit output: just remove the channel */
                else if linear != 0
                /* compose on black (well, pre-multiply) */
                {
                    png_set_strip_alpha(png_ptr);
                }
                /* 8-bit output: do an appropriate compose */
                else if (*display).background != core::ptr::null() {
                    let mut c: png_color_16 = Default::default();

                    c.index = 0; /*unused*/
                    c.red = (*(*display).background).red as png_uint_16;
                    c.green = (*(*display).background).green as png_uint_16;
                    c.blue = (*(*display).background).blue as png_uint_16;
                    c.gray = (*(*display).background).green as png_uint_16;

                    /* This is always an 8-bit sRGB value, using the 'green' channel
                     * for gray is much better than calculating the luminance here;
                     * we can get off-by-one errors in that calculation relative to
                     * the app expectations and that will show up in transparent
                     * pixels.
                     */
                    png_set_background_fixed(
                        png_ptr,
                        &c,
                        PNG_BACKGROUND_GAMMA_SCREEN,
                        0, /*need_expand*/
                        0, /*gamma: not used*/
                    );
                } else
                /* compose on row: implemented below. */
                {
                    do_local_compose = 1;
                    /* This leaves the alpha channel in the output, so it has to be
                     * removed by the code below.  Set the encoding to the 'OPTIMIZE'
                     * one so the code only has to hack on the pixels that require
                     * composition.
                     */
                    mode = PNG_ALPHA_OPTIMIZED;
                }
            } else
            /* output needs an alpha channel */
            {
                /* This is tricky because it happens before the swap operation has
                 * been accomplished; however, the swap does *not* swap the added
                 * alpha channel (weird API), so it must be added in the correct
                 * place.
                 */
                let filler: png_uint_32; /* opaque filler */
                let where_: c_int;

                if linear != 0 {
                    filler = 65535;
                } else {
                    filler = 255;
                }

                if (format & PNG_FORMAT_FLAG_AFIRST) != 0 {
                    where_ = PNG_FILLER_BEFORE;
                    change &= !PNG_FORMAT_FLAG_AFIRST;
                } else {
                    where_ = PNG_FILLER_AFTER;
                }

                png_set_add_alpha(png_ptr, filler, where_);
            }

            /* This stops the (irrelevant) call to swap_alpha below. */
            change &= !PNG_FORMAT_FLAG_ALPHA;
        }

        /* Now set the alpha mode correctly; this is always done, even if there is
         * no alpha channel in either the input or the output because it correctly
         * sets the output gamma.
         */
        png_set_alpha_mode_fixed(png_ptr, mode, output_gamma);

        if (change & PNG_FORMAT_FLAG_BGR) != 0 {
            /* Check only the output format; PNG is never BGR; don't do this if
             * the output is gray, but fix up the 'format' value in that case.
             */
            if (format & PNG_FORMAT_FLAG_COLOR) != 0 {
                png_set_bgr(png_ptr);
            } else {
                format &= !PNG_FORMAT_FLAG_BGR;
            }

            change &= !PNG_FORMAT_FLAG_BGR;
        }

        if (change & PNG_FORMAT_FLAG_AFIRST) != 0 {
            /* Only relevant if there is an alpha channel - it's particularly
             * important to handle this correctly because do_local_compose may
             * be set above and then libpng will keep the alpha channel for this
             * code to remove.
             */
            if (format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                /* Disable this if doing a local background,
                 * TODO: remove this when local background is no longer required.
                 */
                if do_local_background != 2 {
                    png_set_swap_alpha(png_ptr);
                }
            } else {
                format &= !PNG_FORMAT_FLAG_AFIRST;
            }

            change &= !PNG_FORMAT_FLAG_AFIRST;
        }

        /* If the *output* is 16-bit then we need to check for a byte-swap on this
         * architecture.
         */
        if linear != 0 {
            let le: png_uint_16 = 0x0001;

            if *((&le) as *const png_uint_16 as png_const_bytep) != 0 {
                png_set_swap(png_ptr);
            }
        }

        /* If change is not now 0 some transformation is missing - error out. */
        if change != 0 {
            png_error(
                png_ptr,
                b"png_read_image: unsupported transformation\0".as_ptr() as png_const_charp,
            );
        }
    }

    png_image_skip_unused_chunks(png_ptr);

    /* Update the 'info' structure and make sure the result is as required; first
     * make sure to turn on the interlace handling if it will be required
     * (because it can't be turned on *after* the call to png_read_update_info!)
     *
     * TODO: remove the do_local_background fixup below.
     */
    if do_local_compose == 0 && do_local_background != 2 {
        passes = png_set_interlace_handling(png_ptr);
    }

    png_read_update_info(png_ptr, info_ptr);

    {
        let mut info_format: png_uint_32 = 0;

        if ((*info_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
            info_format |= PNG_FORMAT_FLAG_COLOR;
        }

        if ((*info_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0 {
            /* do_local_compose removes this channel below. */
            if do_local_compose == 0 {
                /* do_local_background does the same if required. */
                if do_local_background != 2 || (format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                    info_format |= PNG_FORMAT_FLAG_ALPHA;
                }
            }
        } else if do_local_compose != 0
        /* internal error */
        {
            png_error(
                png_ptr,
                b"png_image_read: alpha channel lost\0".as_ptr() as png_const_charp,
            );
        }

        if (format & PNG_FORMAT_FLAG_ASSOCIATED_ALPHA) != 0 {
            info_format |= PNG_FORMAT_FLAG_ASSOCIATED_ALPHA;
        }

        if (*info_ptr).bit_depth as c_int == 16 {
            info_format |= PNG_FORMAT_FLAG_LINEAR;
        }

        if ((*png_ptr).transformations & PNG_BGR) != 0 {
            info_format |= PNG_FORMAT_FLAG_BGR;
        }

        if do_local_background == 2 {
            if (format & PNG_FORMAT_FLAG_AFIRST) != 0 {
                info_format |= PNG_FORMAT_FLAG_AFIRST;
            }
        }

        if ((*png_ptr).transformations & PNG_SWAP_ALPHA) != 0
            || (((*png_ptr).transformations & PNG_ADD_ALPHA) != 0
                && ((*png_ptr).flags & PNG_FLAG_FILLER_AFTER) == 0)
        {
            if do_local_background == 2 {
                png_error(
                    png_ptr,
                    b"unexpected alpha swap transformation\0".as_ptr() as png_const_charp,
                );
            }

            info_format |= PNG_FORMAT_FLAG_AFIRST;
        }

        /* This is actually an internal error. */
        if info_format != format {
            png_error(
                png_ptr,
                b"png_read_image: invalid transformations\0".as_ptr() as png_const_charp,
            );
        }
    }

    /* Now read the rows.  If do_local_compose is set then it is necessary to use
     * a local row buffer.  The output will be GA, RGBA or BGRA and must be
     * converted to G, RGB or BGR as appropriate.  The 'local_row' member of the
     * display acts as a flag.
     */
    {
        let mut first_row: png_voidp = (*display).buffer;
        let mut row_step: isize = (*display).row_stride as isize;

        if linear != 0 {
            row_step *= 2;
        }

        /* The following adjustment is to ensure that calculations are correct,
         * regardless whether row_step is positive or negative.
         */
        if row_step < 0 {
            let mut ptr: *mut c_char = first_row as *mut c_char;
            ptr = ptr.offset(((*image).height.wrapping_sub(1) as isize) * (-row_step));
            first_row = ptr as png_voidp;
        }

        (*display).first_row = first_row;
        (*display).row_step = row_step;
    }

    if do_local_compose != 0 {
        let result: c_int;
        let row: png_voidp = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr));

        (*display).local_row = row;
        result = png_safe_execute(image, Some(png_image_read_composite), display as png_voidp);
        (*display).local_row = core::ptr::null_mut();
        png_free(png_ptr, row);

        return result;
    } else if do_local_background == 2 {
        let result: c_int;
        let row: png_voidp = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr));

        (*display).local_row = row;
        result = png_safe_execute(image, Some(png_image_read_background), display as png_voidp);
        (*display).local_row = core::ptr::null_mut();
        png_free(png_ptr, row);

        return result;
    } else if do_local_scale != 0 {
        /* For interlaced 16-to-8 conversion, use an intermediate row buffer
         * to avoid buffer overflows in png_combine_row. The local_row is sized
         * for the transformed (8-bit) output, preventing the overflow that would
         * occur if png_combine_row wrote 16-bit data directly to the user buffer.
         */
        let result: c_int;
        let row: png_voidp = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr));

        (*display).local_row = row;
        result = png_safe_execute(image, Some(png_image_read_direct_scaled), display as png_voidp);
        (*display).local_row = core::ptr::null_mut();
        png_free(png_ptr, row);

        return result;
    } else {
        let row_step: isize = (*display).row_step;

        loop {
            passes -= 1;
            if !(passes >= 0) {
                break;
            }

            let mut y: png_uint_32 = (*image).height;
            let mut row: png_bytep = (*display).first_row as png_bytep;

            while y > 0 {
                png_read_row(png_ptr, row, core::ptr::null_mut());
                row = row.offset(row_step);

                y -= 1;
            }
        }

        return 1;
    }
}

/* png_image_finish_read */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_finish_read(
    image: png_imagep,
    background: png_const_colorp,
    buffer: png_voidp,
    mut row_stride: png_int_32,
    colormap: png_voidp,
) -> c_int {
    if image != core::ptr::null_mut() && (*image).version == PNG_IMAGE_VERSION {
        /* Check for row_stride overflow.  This check is not performed on the
         * original PNG format because it may not occur in the output PNG format
         * and libpng deals with the issues of reading the original.
         */
        let channels: c_uint = PNG_IMAGE_PIXEL_CHANNELS((*image).format);

        /* The following checks just the 'row_stride' calculation to ensure it
         * fits in a signed 32-bit value.  Because channels/components can be
         * either 1 or 2 bytes in size the length of a row can still overflow 32
         * bits; this is just to verify that the 'row_stride' argument can be
         * represented.
         */
        if (*image).width <= 0x7fffffffu32 / channels
        /* no overflow */
        {
            let check: png_uint_32;
            let png_row_stride: png_uint_32 = (*image).width.wrapping_mul(channels);

            if row_stride == 0 {
                row_stride = png_row_stride as png_int_32; /*SAFE*/
            }

            if row_stride < 0 {
                check = (row_stride as png_uint_32).wrapping_neg();
            } else {
                check = row_stride as png_uint_32;
            }

            /* This verifies 'check', the absolute value of the actual stride
             * passed in and detects overflow in the application calculation (i.e.
             * if the app did actually pass in a non-zero 'row_stride'.
             */
            if (*image).opaque != core::ptr::null_mut()
                && buffer != core::ptr::null_mut()
                && check >= png_row_stride
            {
                /* Now check for overflow of the image buffer calculation; this
                 * limits the whole image size to 32 bits for API compatibility with
                 * the current, 32-bit, PNG_IMAGE_BUFFER_SIZE macro.
                 *
                 * The PNG_IMAGE_BUFFER_SIZE macro is:
                 *
                 *    (PNG_IMAGE_PIXEL_COMPONENT_SIZE(fmt)*height*(row_stride))
                 *
                 * And the component size is always 1 or 2, so make sure that the
                 * number of *bytes* that the application is saying are available
                 * does actually fit into a 32-bit number.
                 *
                 * NOTE: this will be changed in 1.7 because PNG_IMAGE_BUFFER_SIZE
                 * will be changed to use png_alloc_size_t; bigger images can be
                 * accommodated on 64-bit systems.
                 */
                if (*image).height
                    <= 0xffffffffu32 / PNG_IMAGE_PIXEL_COMPONENT_SIZE((*image).format) / check
                {
                    if ((*image).format & PNG_FORMAT_FLAG_COLORMAP) == 0
                        || ((*image).colormap_entries > 0 && colormap != core::ptr::null_mut())
                    {
                        let result: c_int;
                        /* memset(&display, 0, (sizeof display)); */
                        let mut display: png_image_read_control = core::mem::zeroed();

                        display.image = image;
                        display.buffer = buffer;
                        display.row_stride = row_stride;
                        display.colormap = colormap;
                        display.background = background;
                        display.local_row = core::ptr::null_mut();

                        /* Choose the correct 'end' routine; for the color-map case
                         * all the setup has already been done.
                         */
                        if ((*image).format & PNG_FORMAT_FLAG_COLORMAP) != 0 {
                            result = if png_safe_execute(
                                image,
                                Some(png_image_read_colormap),
                                (&mut display) as *mut png_image_read_control as png_voidp,
                            ) != 0
                                && png_safe_execute(
                                    image,
                                    Some(png_image_read_colormapped),
                                    (&mut display) as *mut png_image_read_control as png_voidp,
                                ) != 0
                            {
                                1
                            } else {
                                0
                            };
                        } else {
                            result = png_safe_execute(
                                image,
                                Some(png_image_read_direct),
                                (&mut display) as *mut png_image_read_control as png_voidp,
                            );
                        }

                        png_image_free(image);
                        return result;
                    } else {
                        return png_image_error(
                            image,
                            b"png_image_finish_read[color-map]: no color-map\0".as_ptr()
                                as png_const_charp,
                        );
                    }
                } else {
                    return png_image_error(
                        image,
                        b"png_image_finish_read: image too large\0".as_ptr() as png_const_charp,
                    );
                }
            } else {
                return png_image_error(
                    image,
                    b"png_image_finish_read: invalid argument\0".as_ptr() as png_const_charp,
                );
            }
        } else {
            return png_image_error(
                image,
                b"png_image_finish_read: row_stride too large\0".as_ptr() as png_const_charp,
            );
        }
    } else if image != core::ptr::null_mut() {
        return png_image_error(
            image,
            b"png_image_finish_read: damaged PNG_IMAGE_VERSION\0".as_ptr() as png_const_charp,
        );
    }

    0
}
