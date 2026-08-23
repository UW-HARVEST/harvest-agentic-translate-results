use crate::*;

/* The final part of the color-map read called from png_image_finish_read. */
unsafe extern "C" fn png_image_read_and_map(argument: png_voidp) -> c_int {
    let display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let image: png_imagep = (*display).image;
    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;

    /* Called when the libpng data must be transformed into the color-mapped
     * form.  There is a local row buffer in display->local and this routine must
     * do the interlace handling.
     */
    let passes: c_int = match (*png_ptr).interlaced as c_int {
        PNG_INTERLACE_NONE => 1,

        PNG_INTERLACE_ADAM7 => PNG_INTERLACE_ADAM7_PASSES,

        _ => png_error(
            png_ptr as png_const_structrp,
            cstr!("unknown interlace type"),
        ),
    };

    {
        let height: png_uint_32 = (*image).height;
        let width: png_uint_32 = (*image).width;
        let proc: c_int = (*display).colormap_processing;
        let first_row: png_bytep = (*display).first_row as png_bytep;
        let row_step: isize = (*display).row_step;

        for pass in 0..passes {
            let startx: c_uint;
            let stepx: c_uint;
            let stepy: c_uint;
            let mut y: png_uint_32;

            if (*png_ptr).interlaced as c_int == PNG_INTERLACE_ADAM7 {
                /* The row may be empty for a short image: */
                if PNG_PASS_COLS(width, pass) == 0 {
                    continue;
                }

                startx = PNG_PASS_START_COL(pass) as c_uint;
                stepx = PNG_PASS_COL_OFFSET(pass) as c_uint;
                y = PNG_PASS_START_ROW(pass) as png_uint_32;
                stepy = PNG_PASS_ROW_OFFSET(pass) as c_uint;
            } else {
                y = 0;
                startx = 0;
                stepx = 1;
                stepy = 1;
            }

            while y < height {
                let mut inrow: png_bytep = (*display).local_row as png_bytep;
                let mut outrow: png_bytep = first_row.offset((y as isize).wrapping_mul(row_step));
                let row_end: png_const_bytep = outrow.offset(width as isize) as png_const_bytep;

                /* Read the libpng data into the temporary buffer. */
                png_read_row(png_ptr, inrow, core::ptr::null_mut());

                /* Now process the row according to the processing option, note
                 * that the caller verifies that the format of the libpng output
                 * data is as required.
                 */
                outrow = outrow.offset(startx as isize);

                if proc == PNG_CMAP_GA as c_int {
                    while (outrow as png_const_bytep) < row_end {
                        /* The data is always in the PNG order */
                        let gray: c_uint = *inrow as c_uint;
                        inrow = inrow.offset(1);
                        let alpha: c_uint = *inrow as c_uint;
                        inrow = inrow.offset(1);
                        let entry: c_uint;

                        /* NOTE: this code is copied as a comment in
                         * make_ga_colormap above.  Please update the
                         * comment if you change this code!
                         */
                        if alpha > 229
                        /* opaque */
                        {
                            entry = (231 * gray + 128) >> 8;
                        } else if alpha < 26
                        /* transparent */
                        {
                            entry = 231;
                        } else
                        /* partially opaque */
                        {
                            /* entry = 226 + 6 * PNG_DIV51(alpha) + PNG_DIV51(gray) */
                            entry = 226 + 6 * ((alpha * 5 + 130) >> 8) + ((gray * 5 + 130) >> 8);
                        }

                        *outrow = entry as png_byte;

                        outrow = outrow.offset(stepx as isize);
                    }
                } else if proc == PNG_CMAP_TRANS as c_int {
                    while (outrow as png_const_bytep) < row_end {
                        let gray: png_byte = *inrow;
                        inrow = inrow.offset(1);
                        let alpha: png_byte = *inrow;
                        inrow = inrow.offset(1);

                        if alpha == 0 {
                            *outrow = PNG_CMAP_TRANS_BACKGROUND as png_byte;
                        } else if gray != PNG_CMAP_TRANS_BACKGROUND as png_byte {
                            *outrow = gray;
                        } else {
                            *outrow = (PNG_CMAP_TRANS_BACKGROUND + 1) as png_byte;
                        }

                        outrow = outrow.offset(stepx as isize);
                    }
                } else if proc == PNG_CMAP_RGB as c_int {
                    while (outrow as png_const_bytep) < row_end {
                        /* PNG_RGB_INDEX(inrow[0], inrow[1], inrow[2]), i.e.
                         * 6 * (6 * PNG_DIV51(r) + PNG_DIV51(g)) + PNG_DIV51(b)
                         */
                        let r: c_int = *inrow.offset(0) as c_int;
                        let g: c_int = *inrow.offset(1) as c_int;
                        let b: c_int = *inrow.offset(2) as c_int;

                        *outrow = (6 * (6 * ((r * 5 + 130) >> 8) + ((g * 5 + 130) >> 8))
                            + ((b * 5 + 130) >> 8)) as png_byte;
                        inrow = inrow.offset(3);

                        outrow = outrow.offset(stepx as isize);
                    }
                } else if proc == PNG_CMAP_RGB_ALPHA as c_int {
                    while (outrow as png_const_bytep) < row_end {
                        let alpha: c_uint = *inrow.offset(3) as c_uint;

                        /* Because the alpha entries only hold alpha==0.5 values
                         * split the processing at alpha==0.25 (64) and 0.75
                         * (196).
                         */

                        if alpha >= 196 {
                            /* PNG_RGB_INDEX(inrow[0], inrow[1], inrow[2]) */
                            let r: c_int = *inrow.offset(0) as c_int;
                            let g: c_int = *inrow.offset(1) as c_int;
                            let b: c_int = *inrow.offset(2) as c_int;

                            *outrow = (6 * (6 * ((r * 5 + 130) >> 8) + ((g * 5 + 130) >> 8))
                                + ((b * 5 + 130) >> 8)) as png_byte;
                        } else if alpha < 64 {
                            *outrow = PNG_CMAP_RGB_ALPHA_BACKGROUND as png_byte;
                        } else {
                            /* Likewise there are three entries for each of r, g
                             * and b.  We could select the entry by popcount on
                             * the top two bits on those architectures that
                             * support it, this is what the code below does,
                             * crudely.
                             */
                            let mut back_i: c_uint = PNG_CMAP_RGB_ALPHA_BACKGROUND as c_uint + 1;

                            /* Here are how the values map:
                             *
                             * 0x00 .. 0x3f -> 0
                             * 0x40 .. 0xbf -> 1
                             * 0xc0 .. 0xff -> 2
                             *
                             * So, as above with the explicit alpha checks, the
                             * breakpoints are at 64 and 196.
                             */
                            if (*inrow.offset(0) & 0x80) != 0 {
                                back_i += 9; /* red */
                            }
                            if (*inrow.offset(0) & 0x40) != 0 {
                                back_i += 9;
                            }
                            if (*inrow.offset(1) & 0x80) != 0 {
                                back_i += 3; /* green */
                            }
                            if (*inrow.offset(1) & 0x40) != 0 {
                                back_i += 3;
                            }
                            if (*inrow.offset(2) & 0x80) != 0 {
                                back_i += 1; /* blue */
                            }
                            if (*inrow.offset(2) & 0x40) != 0 {
                                back_i += 1;
                            }

                            *outrow = back_i as png_byte;
                        }

                        inrow = inrow.offset(4);

                        outrow = outrow.offset(stepx as isize);
                    }
                } else {
                    /* default: nothing to do */
                }

                y = y.wrapping_add(stepy);
            }
        }
    }

    1
}

unsafe extern "C" fn png_image_read_colormapped(argument: png_voidp) -> c_int {
    let display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let image: png_imagep = (*display).image;
    let control: png_controlp = (*image).opaque;
    let png_ptr: png_structrp = (*control).png_ptr;
    let info_ptr: png_inforp = (*control).info_ptr;

    let mut passes: c_int = 0; /* As a flag */

    png_image_skip_unused_chunks(png_ptr);

    /* Update the 'info' structure and make sure the result is as required; first
     * make sure to turn on the interlace handling if it will be required
     * (because it can't be turned on *after* the call to png_read_update_info!)
     */
    if (*display).colormap_processing == PNG_CMAP_NONE as c_int {
        passes = png_set_interlace_handling(png_ptr);
    }

    png_read_update_info(png_ptr, info_ptr);

    /* The expected output can be deduced from the colormap_processing option. */
    {
        let colormap_processing: c_int = (*display).colormap_processing;
        let ok: bool;

        if colormap_processing == PNG_CMAP_NONE as c_int {
            /* Output must be one channel and one byte per pixel, the output
             * encoding can be anything.
             */
            ok = ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
                || (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY)
                && (*info_ptr).bit_depth as c_int == 8;
        } else if colormap_processing == PNG_CMAP_TRANS as c_int
            || colormap_processing == PNG_CMAP_GA as c_int
        {
            /* Output must be two channels and the 'G' one must be sRGB, the latter
             * can be checked with an exact number because it should have been set
             * to this number above!
             */
            ok = (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA
                && (*info_ptr).bit_depth as c_int == 8
                && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                && (*image).colormap_entries == 256;
        } else if colormap_processing == PNG_CMAP_RGB as c_int {
            /* Output must be 8-bit sRGB encoded RGB */
            ok = (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB
                && (*info_ptr).bit_depth as c_int == 8
                && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                && (*image).colormap_entries == 216;
        } else if colormap_processing == PNG_CMAP_RGB_ALPHA as c_int {
            /* Output must be 8-bit sRGB encoded RGBA */
            ok = (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
                && (*info_ptr).bit_depth as c_int == 8
                && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                && (*image).colormap_entries == 244/* 216 + 1 + 27 */;
        } else {
            ok = false;
        }

        if !ok {
            /* bad_output: */
            png_error(
                png_ptr as png_const_structrp,
                cstr!("bad color-map processing (internal error)"),
            );
        }
    }

    /* Now read the rows.  Do this here if it is possible to read directly into
     * the output buffer, otherwise allocate a local row buffer of the maximum
     * size libpng requires and call the relevant processing routine safely.
     */
    {
        let mut first_row: png_voidp = (*display).buffer;
        let row_step: isize = (*display).row_stride as isize;

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

    if passes == 0 {
        let result: c_int;
        let row: png_voidp = png_malloc(
            png_ptr as png_const_structrp,
            png_get_rowbytes(png_ptr as png_const_structrp, info_ptr as png_const_inforp)
                as png_alloc_size_t,
        );

        (*display).local_row = row;
        result = png_safe_execute(image, Some(png_image_read_and_map), display as png_voidp);
        (*display).local_row = core::ptr::null_mut();
        png_free(png_ptr as png_const_structrp, row);

        result
    } else {
        let row_step: isize = (*display).row_step;

        loop {
            passes -= 1;
            if passes < 0 {
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

        1
    }
}

/* Row reading for interlaced 16-to-8 bit depth conversion with local buffer. */
unsafe extern "C" fn png_image_read_direct_scaled(argument: png_voidp) -> c_int {
    let display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let image: png_imagep = (*display).image;
    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
    let info_ptr: png_inforp = (*(*image).opaque).info_ptr;
    let local_row: png_bytep = (*display).local_row as png_bytep;
    let first_row: png_bytep = (*display).first_row as png_bytep;
    let row_step: isize = (*display).row_step;
    let row_bytes: usize =
        png_get_rowbytes(png_ptr as png_const_structrp, info_ptr as png_const_inforp);

    /* Handle interlacing. */
    let mut passes: c_int = match (*png_ptr).interlaced as c_int {
        PNG_INTERLACE_NONE => 1,

        PNG_INTERLACE_ADAM7 => PNG_INTERLACE_ADAM7_PASSES,

        _ => png_error(
            png_ptr as png_const_structrp,
            cstr!("unknown interlace type"),
        ),
    };

    /* Read each pass using local_row as intermediate buffer. */
    loop {
        passes -= 1;
        if passes < 0 {
            break;
        }

        let mut y: png_uint_32 = (*image).height;
        let mut output_row: png_bytep = first_row;

        while y > 0 {
            /* Read into local_row (gets transformed 8-bit data). */
            png_read_row(png_ptr, local_row, core::ptr::null_mut());

            /* Copy from local_row to user buffer.
             * Use row_bytes (i.e. the actual size in bytes of the row data) for
             * copying into output_row. Use row_step for advancing output_row,
             * to respect the caller's stride for padding or negative (bottom-up)
             * layouts.
             */
            memcpy(
                output_row as *mut c_void,
                local_row as *const c_void,
                row_bytes,
            );
            output_row = output_row.offset(row_step);

            y -= 1;
        }
    }

    1
}

/* Just the row reading part of png_image_read. */
unsafe extern "C" fn png_image_read_composite(argument: png_voidp) -> c_int {
    let display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let image: png_imagep = (*display).image;
    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;

    let passes: c_int = match (*png_ptr).interlaced as c_int {
        PNG_INTERLACE_NONE => 1,

        PNG_INTERLACE_ADAM7 => PNG_INTERLACE_ADAM7_PASSES,

        _ => png_error(
            png_ptr as png_const_structrp,
            cstr!("unknown interlace type"),
        ),
    };

    {
        let height: png_uint_32 = (*image).height;
        let width: png_uint_32 = (*image).width;
        let row_step: isize = (*display).row_step;
        let channels: c_uint = if ((*image).format & PNG_FORMAT_FLAG_COLOR) != 0 {
            3
        } else {
            1
        };
        let optimize_alpha: c_int = (((*png_ptr).flags & PNG_FLAG_OPTIMIZE_ALPHA) != 0) as c_int;

        for pass in 0..passes {
            let startx: c_uint;
            let stepx: c_uint;
            let stepy: c_uint;
            let mut y: png_uint_32;

            if (*png_ptr).interlaced as c_int == PNG_INTERLACE_ADAM7 {
                /* The row may be empty for a short image: */
                if PNG_PASS_COLS(width, pass) == 0 {
                    continue;
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
                row_end = outrow.offset(width.wrapping_mul(channels) as isize) as png_const_bytep;

                /* Now do the composition on each pixel in this row. */
                outrow = outrow.offset(startx as isize);
                while (outrow as png_const_bytep) < row_end {
                    let alpha: png_byte = *inrow.offset(channels as isize);

                    if alpha > 0
                    /* else no change to the output */
                    {
                        let mut c: c_uint = 0;

                        while c < channels {
                            let mut component: png_uint_32 = *inrow.offset(c as isize) as png_uint_32;

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
                                    component *= 257 * 255; /* =65535 */
                                    component += (255 - alpha as c_uint)
                                        * png_sRGB_table[*outrow.offset(c as isize) as usize]
                                            as c_uint;

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
                                        *outrow.offset(c as isize) as png_uint_32;
                                    component +=
                                        ((255 - alpha as png_uint_32) * background + 127) / 255;
                                    if component > 255 {
                                        component = 255;
                                    }
                                }
                            }

                            *outrow.offset(c as isize) = component as png_byte;

                            c += 1;
                        }
                    }

                    inrow = inrow.offset((channels + 1) as isize); /* components and alpha channel */

                    outrow = outrow.offset(stepx as isize);
                }

                y = y.wrapping_add(stepy);
            }
        }
    }

    1
}
