/* Just the row reading part of png_image_read. */
unsafe extern "C" fn png_image_read_composite(argument: png_voidp) -> c_int {
    let display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let image: png_imagep = (*display).image;
    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
    let passes: c_int;

    match (*png_ptr).interlaced as c_int {
        PNG_INTERLACE_NONE => {
            passes = 1;
        }

        PNG_INTERLACE_ADAM7 => {
            passes = PNG_INTERLACE_ADAM7_PASSES;
        }

        _ => {
            png_error(
                png_ptr,
                b"unknown interlace type\0".as_ptr() as png_const_charp,
            );
        }
    }

    {
        let height: png_uint_32 = (*image).height;
        let width: png_uint_32 = (*image).width;
        let row_step: isize = (*display).row_step;
        let channels: c_uint = if ((*image).format & PNG_FORMAT_FLAG_COLOR) != 0 {
            3
        } else {
            1
        };
        let optimize_alpha: c_int = if ((*png_ptr).flags & PNG_FLAG_OPTIMIZE_ALPHA) != 0 {
            1
        } else {
            0
        };
        let mut pass: c_int;

        pass = 0;
        while pass < passes {
            'cont: {
                let startx: c_uint;
                let stepx: c_uint;
                let stepy: c_uint;
                let mut y: png_uint_32;

                if (*png_ptr).interlaced as c_int == PNG_INTERLACE_ADAM7 {
                    /* The row may be empty for a short image: */
                    if PNG_PASS_COLS(width, pass) == 0 {
                        break 'cont;
                    }

                    startx = (PNG_PASS_START_COL(pass) as c_uint).wrapping_mul(channels);
                    stepx = (PNG_PASS_COL_OFFSET(pass) as c_uint).wrapping_mul(channels);
                    y = PNG_PASS_START_ROW(pass) as png_uint_32;
                    stepy = PNG_PASS_ROW_OFFSET(pass) as c_uint;
                } else {
                    y = 0;
                    startx = 0;
                    stepx = channels;
                    stepy = 1;
                }

                while y < height {
                    let mut inrow: png_bytep = (*display).local_row as png_bytep;
                    let mut outrow: png_bytep;
                    let row_end: png_const_bytep;

                    /* Read the row, which is packed: */
                    png_read_row(png_ptr, inrow, core::ptr::null_mut());

                    outrow = (*display).first_row as png_bytep;
                    outrow = outrow.offset((y as isize).wrapping_mul(row_step));
                    row_end = outrow.add(width.wrapping_mul(channels) as usize) as png_const_bytep;

                    /* Now do the composition on each pixel in this row. */
                    outrow = outrow.add(startx as usize);
                    while (outrow as png_const_bytep) < row_end {
                        let alpha: png_byte = *inrow.add(channels as usize);

                        if alpha > 0
                        /* else no change to the output */
                        {
                            let mut c: c_uint;

                            c = 0;
                            while c < channels {
                                let mut component: png_uint_32 =
                                    *inrow.add(c as usize) as png_uint_32;

                                if alpha < 255
                                /* else just use component */
                                {
                                    if optimize_alpha != 0 {
                                        /* This is PNG_OPTIMIZED_ALPHA, the component value
                                         * is a linear 8-bit value.  Combine this with the
                                         * current outrow[c] value which is sRGB encoded.
                                         * Arithmetic here is 16-bits to preserve the output
                                         * values correctly.
                                         */
                                        component =
                                            component.wrapping_mul(257 * 255); /* =65535 */
                                        component = component.wrapping_add(
                                            ((255 - alpha as c_int)
                                                * png_sRGB_table
                                                    [*outrow.add(c as usize) as usize]
                                                    as c_int)
                                                as png_uint_32,
                                        );

                                        /* Clamp to the valid range to defend against
                                         * unforeseen cases where the data might be sRGB
                                         * instead of linear premultiplied.
                                         * (Belt-and-suspenders for CVE-2025-66293.)
                                         */
                                        if component > 255 * 65535 {
                                            component = 255 * 65535;
                                        }

                                        /* So 'component' is scaled by 255*65535 and is
                                         * therefore appropriate for the sRGB-to-linear
                                         * conversion table.
                                         */
                                        component = PNG_sRGB_FROM_LINEAR(component) as png_uint_32;
                                    } else {
                                        /* Compositing was already done on the palette
                                         * entries.  The data is sRGB premultiplied on black.
                                         * Composite with the background in sRGB space.
                                         * This is not gamma-correct, but matches what was
                                         * done to the palette.
                                         */
                                        let background: png_uint_32 =
                                            *outrow.add(c as usize) as png_uint_32;
                                        component = component.wrapping_add(
                                            ((255 - alpha as c_int) as png_uint_32)
                                                .wrapping_mul(background)
                                                .wrapping_add(127)
                                                / 255,
                                        );
                                        if component > 255 {
                                            component = 255;
                                        }
                                    }
                                }

                                *outrow.add(c as usize) = component as png_byte;

                                c += 1;
                            }
                        }

                        inrow = inrow.add(channels.wrapping_add(1) as usize); /* components and alpha channel */

                        outrow = outrow.add(stepx as usize);
                    }

                    y = y.wrapping_add(stepy);
                }
            }

            pass += 1;
        }
    }

    1
}

/* The do_local_background case; called when all the following transforms are to
 * be done:
 *
 * PNG_RGB_TO_GRAY
 * PNG_COMPOSITE
 * PNG_GAMMA
 *
 * This is a work-around for the fact that both the PNG_RGB_TO_GRAY and
 * PNG_COMPOSITE code performs gamma correction, so we get double gamma
 * correction.  The fix-up is to prevent the PNG_COMPOSITE operation from
 * happening inside libpng, so this routine sees an 8 or 16-bit gray+alpha
 * row and handles the removal or pre-multiplication of the alpha channel.
 */
unsafe extern "C" fn png_image_read_background(argument: png_voidp) -> c_int {
    let display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let image: png_imagep = (*display).image;
    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
    let info_ptr: png_inforp = (*(*image).opaque).info_ptr;
    let height: png_uint_32 = (*image).height;
    let width: png_uint_32 = (*image).width;
    let mut pass: c_int;
    let passes: c_int;

    /* Double check the convoluted logic below.  We expect to get here with
     * libpng doing rgb to gray and gamma correction but background processing
     * left to the png_image_read_background function.  The rows libpng produce
     * might be 8 or 16-bit but should always have two channels; gray plus alpha.
     */
    if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) == 0 {
        png_error(png_ptr, b"lost rgb to gray\0".as_ptr() as png_const_charp);
    }

    if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
        png_error(png_ptr, b"unexpected compose\0".as_ptr() as png_const_charp);
    }

    if png_get_channels(png_ptr, info_ptr) as c_int != 2 {
        png_error(
            png_ptr,
            b"lost/gained channels\0".as_ptr() as png_const_charp,
        );
    }

    /* Expect the 8-bit case to always remove the alpha channel */
    if ((*image).format & PNG_FORMAT_FLAG_LINEAR) == 0
        && ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0
    {
        png_error(
            png_ptr,
            b"unexpected 8-bit transformation\0".as_ptr() as png_const_charp,
        );
    }

    match (*png_ptr).interlaced as c_int {
        PNG_INTERLACE_NONE => {
            passes = 1;
        }

        PNG_INTERLACE_ADAM7 => {
            passes = PNG_INTERLACE_ADAM7_PASSES;
        }

        _ => {
            png_error(
                png_ptr,
                b"unknown interlace type\0".as_ptr() as png_const_charp,
            );
        }
    }

    /* Use direct access to info_ptr here because otherwise the simplified API
     * would require PNG_EASY_ACCESS_SUPPORTED (just for this.)  Note this is
     * checking the value after libpng expansions, not the original value in the
     * PNG.
     */
    match (*info_ptr).bit_depth as c_int {
        8 => {
            /* 8-bit sRGB gray values with an alpha channel; the alpha channel is
             * to be removed by composing on a background: either the row if
             * display->background is NULL or display->background->green if not.
             * Unlike the code above ALPHA_OPTIMIZED has *not* been done.
             */
            {
                let first_row: png_bytep = (*display).first_row as png_bytep;
                let row_step: isize = (*display).row_step;

                pass = 0;
                while pass < passes {
                    'cont: {
                        let startx: c_uint;
                        let stepx: c_uint;
                        let stepy: c_uint;
                        let mut y: png_uint_32;

                        if (*png_ptr).interlaced as c_int == PNG_INTERLACE_ADAM7 {
                            /* The row may be empty for a short image: */
                            if PNG_PASS_COLS(width, pass) == 0 {
                                break 'cont;
                            }

                            startx = PNG_PASS_START_COL(pass) as c_uint;
                            stepx = PNG_PASS_COL_OFFSET(pass) as c_uint;
                            y = PNG_PASS_START_ROW(pass) as png_uint_32;
                            stepy = PNG_PASS_ROW_OFFSET(pass) as c_uint;
                        } else {
                            y = 0;
                            startx = 0;
                            stepy = 1;
                            stepx = stepy;
                        }

                        if (*display).background == core::ptr::null() {
                            while y < height {
                                let mut inrow: png_bytep = (*display).local_row as png_bytep;
                                let mut outrow: png_bytep =
                                    first_row.offset((y as isize).wrapping_mul(row_step));
                                let row_end: png_const_bytep =
                                    outrow.add(width as usize) as png_const_bytep;

                                /* Read the row, which is packed: */
                                png_read_row(png_ptr, inrow, core::ptr::null_mut());

                                /* Now do the composition on each pixel in this row. */
                                outrow = outrow.add(startx as usize);
                                while (outrow as png_const_bytep) < row_end {
                                    let alpha: png_byte = *inrow.add(1);

                                    if alpha > 0
                                    /* else no change to the output */
                                    {
                                        let mut component: png_uint_32 =
                                            *inrow.add(0) as png_uint_32;

                                        if alpha < 255
                                        /* else just use component */
                                        {
                                            /* Since PNG_OPTIMIZED_ALPHA was not set it is
                                             * necessary to invert the sRGB transfer
                                             * function and multiply the alpha out.
                                             */
                                            component = (png_sRGB_table[component as usize]
                                                as c_int
                                                * alpha as c_int)
                                                as png_uint_32;
                                            component = component.wrapping_add(
                                                (png_sRGB_table[*outrow.add(0) as usize] as c_int
                                                    * (255 - alpha as c_int))
                                                    as png_uint_32,
                                            );
                                            component =
                                                PNG_sRGB_FROM_LINEAR(component) as png_uint_32;
                                        }

                                        *outrow.add(0) = component as png_byte;
                                    }

                                    inrow = inrow.add(2); /* gray and alpha channel */

                                    outrow = outrow.add(stepx as usize);
                                }

                                y = y.wrapping_add(stepy);
                            }
                        } else
                        /* constant background value */
                        {
                            let background8: png_byte = (*(*display).background).green;
                            let background: png_uint_16 = png_sRGB_table[background8 as usize];

                            while y < height {
                                let mut inrow: png_bytep = (*display).local_row as png_bytep;
                                let mut outrow: png_bytep =
                                    first_row.offset((y as isize).wrapping_mul(row_step));
                                let row_end: png_const_bytep =
                                    outrow.add(width as usize) as png_const_bytep;

                                /* Read the row, which is packed: */
                                png_read_row(png_ptr, inrow, core::ptr::null_mut());

                                /* Now do the composition on each pixel in this row. */
                                outrow = outrow.add(startx as usize);
                                while (outrow as png_const_bytep) < row_end {
                                    let alpha: png_byte = *inrow.add(1);

                                    if alpha > 0
                                    /* else use background */
                                    {
                                        let mut component: png_uint_32 =
                                            *inrow.add(0) as png_uint_32;

                                        if alpha < 255
                                        /* else just use component */
                                        {
                                            component = (png_sRGB_table[component as usize]
                                                as c_int
                                                * alpha as c_int)
                                                as png_uint_32;
                                            component = component.wrapping_add(
                                                (background as c_int * (255 - alpha as c_int))
                                                    as png_uint_32,
                                            );
                                            component =
                                                PNG_sRGB_FROM_LINEAR(component) as png_uint_32;
                                        }

                                        *outrow.add(0) = component as png_byte;
                                    } else {
                                        *outrow.add(0) = background8;
                                    }

                                    inrow = inrow.add(2); /* gray and alpha channel */

                                    outrow = outrow.add(stepx as usize);
                                }

                                y = y.wrapping_add(stepy);
                            }
                        }
                    }

                    pass += 1;
                }
            }
        }

        16 => {
            /* 16-bit linear with pre-multiplied alpha; the pre-multiplication must
             * still be done and, maybe, the alpha channel removed.  This code also
             * handles the alpha-first option.
             */
            {
                let first_row: png_uint_16p = (*display).first_row as png_uint_16p;
                /* The division by two is safe because the caller passed in a
                 * stride which was multiplied by 2 (below) to get row_step.
                 */
                let row_step: isize = (*display).row_step / 2;
                let preserve_alpha: c_uint = if ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                    1
                } else {
                    0
                };
                let outchannels: c_uint = 1u32.wrapping_add(preserve_alpha);
                let mut swap_alpha: c_int = 0;

                if preserve_alpha != 0 && ((*image).format & PNG_FORMAT_FLAG_AFIRST) != 0 {
                    swap_alpha = 1;
                }

                pass = 0;
                while pass < passes {
                    'cont: {
                        let startx: c_uint;
                        let stepx: c_uint;
                        let stepy: c_uint;
                        let mut y: png_uint_32;

                        /* The 'x' start and step are adjusted to output components here.
                         */
                        if (*png_ptr).interlaced as c_int == PNG_INTERLACE_ADAM7 {
                            /* The row may be empty for a short image: */
                            if PNG_PASS_COLS(width, pass) == 0 {
                                break 'cont;
                            }

                            startx = (PNG_PASS_START_COL(pass) as c_uint).wrapping_mul(outchannels);
                            stepx = (PNG_PASS_COL_OFFSET(pass) as c_uint).wrapping_mul(outchannels);
                            y = PNG_PASS_START_ROW(pass) as png_uint_32;
                            stepy = PNG_PASS_ROW_OFFSET(pass) as c_uint;
                        } else {
                            y = 0;
                            startx = 0;
                            stepx = outchannels;
                            stepy = 1;
                        }

                        while y < height {
                            let mut inrow: png_const_uint_16p;
                            let mut outrow: png_uint_16p =
                                first_row.offset((y as isize).wrapping_mul(row_step));
                            let row_end: png_uint_16p =
                                outrow.add(width.wrapping_mul(outchannels) as usize);

                            /* Read the row, which is packed: */
                            png_read_row(
                                png_ptr,
                                (*display).local_row as png_bytep,
                                core::ptr::null_mut(),
                            );
                            inrow = (*display).local_row as png_const_uint_16p;

                            /* Now do the pre-multiplication on each pixel in this row.
                             */
                            outrow = outrow.add(startx as usize);
                            while outrow < row_end {
                                let mut component: png_uint_32 = *inrow.add(0) as png_uint_32;
                                let alpha: png_uint_16 = *inrow.add(1);

                                if alpha > 0
                                /* else 0 */
                                {
                                    if alpha < 65535
                                    /* else just use component */
                                    {
                                        component = component.wrapping_mul(alpha as png_uint_32);
                                        component = component.wrapping_add(32767);
                                        component /= 65535;
                                    }
                                } else {
                                    component = 0;
                                }

                                *outrow.offset(swap_alpha as isize) = component as png_uint_16;
                                if preserve_alpha != 0 {
                                    *outrow.offset((1 ^ swap_alpha) as isize) = alpha;
                                }

                                inrow = inrow.add(2); /* components and alpha channel */

                                outrow = outrow.add(stepx as usize);
                            }

                            y = y.wrapping_add(stepy);
                        }
                    }

                    pass += 1;
                }
            }
        }

        _ => {
            png_error(
                png_ptr,
                b"unexpected bit depth\0".as_ptr() as png_const_charp,
            );
        }
    }

    1
}
