/* pngread.c lines 2813..3188 */

/* The final part of the color-map read called from png_image_finish_read. */
unsafe extern "C" fn png_image_read_and_map(argument: png_voidp) -> c_int {
    let display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let image: png_imagep = (*display).image;
    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
    let passes: c_int;

    /* Called when the libpng data must be transformed into the color-mapped
     * form.  There is a local row buffer in display->local and this routine must
     * do the interlace handling.
     */
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
        let proc_: c_int = (*display).colormap_processing;
        let first_row: png_bytep = (*display).first_row as png_bytep;
        let row_step: isize = (*display).row_step;
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

                while y < height {
                    let mut inrow: png_bytep = (*display).local_row as png_bytep;
                    let mut outrow: png_bytep =
                        first_row.offset((y as isize).wrapping_mul(row_step));
                    let row_end: png_const_bytep = outrow.add(width as usize) as png_const_bytep;

                    /* Read the libpng data into the temporary buffer. */
                    png_read_row(png_ptr, inrow, core::ptr::null_mut());

                    /* Now process the row according to the processing option, note
                     * that the caller verifies that the format of the libpng output
                     * data is as required.
                     */
                    outrow = outrow.add(startx as usize);
                    match proc_ {
                        PNG_CMAP_GA => {
                            while (outrow as png_const_bytep) < row_end {
                                /* The data is always in the PNG order */
                                let gray: c_uint = *inrow as c_uint;
                                inrow = inrow.add(1);
                                let alpha: c_uint = *inrow as c_uint;
                                inrow = inrow.add(1);
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
                                    entry = 226 + 6 * PNG_DIV51(alpha) + PNG_DIV51(gray);
                                }

                                *outrow = entry as png_byte;

                                outrow = outrow.add(stepx as usize);
                            }
                        }

                        PNG_CMAP_TRANS => {
                            while (outrow as png_const_bytep) < row_end {
                                let gray: png_byte = *inrow;
                                inrow = inrow.add(1);
                                let alpha: png_byte = *inrow;
                                inrow = inrow.add(1);

                                if alpha == 0 {
                                    *outrow = PNG_CMAP_TRANS_BACKGROUND as png_byte;
                                } else if gray as c_uint != PNG_CMAP_TRANS_BACKGROUND {
                                    *outrow = gray;
                                } else {
                                    *outrow = (PNG_CMAP_TRANS_BACKGROUND + 1) as png_byte;
                                }

                                outrow = outrow.add(stepx as usize);
                            }
                        }

                        PNG_CMAP_RGB => {
                            while (outrow as png_const_bytep) < row_end {
                                *outrow = PNG_RGB_INDEX(
                                    *inrow.add(0) as png_uint_32,
                                    *inrow.add(1) as png_uint_32,
                                    *inrow.add(2) as png_uint_32,
                                );
                                inrow = inrow.add(3);

                                outrow = outrow.add(stepx as usize);
                            }
                        }

                        PNG_CMAP_RGB_ALPHA => {
                            while (outrow as png_const_bytep) < row_end {
                                let alpha: c_uint = *inrow.add(3) as c_uint;

                                /* Because the alpha entries only hold alpha==0.5 values
                                 * split the processing at alpha==0.25 (64) and 0.75
                                 * (196).
                                 */

                                if alpha >= 196 {
                                    *outrow = PNG_RGB_INDEX(
                                        *inrow.add(0) as png_uint_32,
                                        *inrow.add(1) as png_uint_32,
                                        *inrow.add(2) as png_uint_32,
                                    );
                                } else if alpha < 64 {
                                    *outrow = PNG_CMAP_RGB_ALPHA_BACKGROUND as png_byte;
                                } else {
                                    /* Likewise there are three entries for each of r, g
                                     * and b.  We could select the entry by popcount on
                                     * the top two bits on those architectures that
                                     * support it, this is what the code below does,
                                     * crudely.
                                     */
                                    let mut back_i: c_uint = PNG_CMAP_RGB_ALPHA_BACKGROUND + 1;

                                    /* Here are how the values map:
                                     *
                                     * 0x00 .. 0x3f -> 0
                                     * 0x40 .. 0xbf -> 1
                                     * 0xc0 .. 0xff -> 2
                                     *
                                     * So, as above with the explicit alpha checks, the
                                     * breakpoints are at 64 and 196.
                                     */
                                    if (*inrow.add(0) as c_int & 0x80) != 0 {
                                        back_i += 9;
                                    } /* red */
                                    if (*inrow.add(0) as c_int & 0x40) != 0 {
                                        back_i += 9;
                                    }
                                    if (*inrow.add(1) as c_int & 0x80) != 0 {
                                        back_i += 3;
                                    } /* green */
                                    if (*inrow.add(1) as c_int & 0x40) != 0 {
                                        back_i += 3;
                                    }
                                    if (*inrow.add(2) as c_int & 0x80) != 0 {
                                        back_i += 1;
                                    } /* blue */
                                    if (*inrow.add(2) as c_int & 0x40) != 0 {
                                        back_i += 1;
                                    }

                                    *outrow = back_i as png_byte;
                                }

                                inrow = inrow.add(4);

                                outrow = outrow.add(stepx as usize);
                            }
                        }

                        _ => {}
                    }

                    y = y.wrapping_add(stepy);
                }
            }

            pass += 1;
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
    if (*display).colormap_processing == PNG_CMAP_NONE {
        passes = png_set_interlace_handling(png_ptr);
    }

    png_read_update_info(png_ptr, info_ptr);

    /* The expected output can be deduced from the colormap_processing option. */
    'switch_end: {
        'bad_output: {
            match (*display).colormap_processing {
                PNG_CMAP_NONE => {
                    /* Output must be one channel and one byte per pixel, the output
                     * encoding can be anything.
                     */
                    if ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
                        || (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY)
                        && (*info_ptr).bit_depth as c_int == 8
                    {
                        break 'switch_end;
                    }

                    break 'bad_output;
                }

                PNG_CMAP_TRANS | PNG_CMAP_GA => {
                    /* Output must be two channels and the 'G' one must be sRGB, the latter
                     * can be checked with an exact number because it should have been set
                     * to this number above!
                     */
                    if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA
                        && (*info_ptr).bit_depth as c_int == 8
                        && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                        && (*image).colormap_entries == 256
                    {
                        break 'switch_end;
                    }

                    break 'bad_output;
                }

                PNG_CMAP_RGB => {
                    /* Output must be 8-bit sRGB encoded RGB */
                    if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB
                        && (*info_ptr).bit_depth as c_int == 8
                        && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                        && (*image).colormap_entries == 216
                    {
                        break 'switch_end;
                    }

                    break 'bad_output;
                }

                PNG_CMAP_RGB_ALPHA => {
                    /* Output must be 8-bit sRGB encoded RGBA */
                    if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
                        && (*info_ptr).bit_depth as c_int == 8
                        && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                        && (*image).colormap_entries == 244
                    /* 216 + 1 + 27 */
                    {
                        break 'switch_end;
                    }

                    break 'bad_output;
                }

                _ => {
                    break 'bad_output;
                }
            }
        }

        /* bad_output: */
        png_error(
            png_ptr,
            b"bad color-map processing (internal error)\0".as_ptr() as png_const_charp,
        );
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
            ptr = ptr.offset(
                ((*image).height.wrapping_sub(1) as isize).wrapping_mul(row_step.wrapping_neg()),
            );
            first_row = ptr as png_voidp;
        }

        (*display).first_row = first_row;
        (*display).row_step = row_step;
    }

    if passes == 0 {
        let result: c_int;
        let row: png_voidp = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr));

        (*display).local_row = row;
        result = png_safe_execute(image, Some(png_image_read_and_map), display as png_voidp);
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

/* Row reading for interlaced 16-to-8 bit depth conversion with local buffer. */
unsafe extern "C" fn png_image_read_direct_scaled(argument: png_voidp) -> c_int {
    let display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let image: png_imagep = (*display).image;
    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
    let info_ptr: png_inforp = (*(*image).opaque).info_ptr;
    let local_row: png_bytep = (*display).local_row as png_bytep;
    let first_row: png_bytep = (*display).first_row as png_bytep;
    let row_step: isize = (*display).row_step;
    let row_bytes: usize = png_get_rowbytes(png_ptr, info_ptr);
    let mut passes: c_int;

    /* Handle interlacing. */
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

    /* Read each pass using local_row as intermediate buffer. */
    loop {
        passes -= 1;
        if !(passes >= 0) {
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
