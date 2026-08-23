/* pngread.c lines 288..673 */

/* png_read_row */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_row(png_ptr: png_structrp, row: png_bytep, dsp_row: png_bytep) {
    let mut row_info: png_row_info = Default::default();

    if png_ptr == core::ptr::null_mut() {
        return;
    }

    /* png_read_start_row sets the information (in particular iwidth) for this
     * interlace pass.
     */
    if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
        png_read_start_row(png_ptr);
    }

    /* 1.5.6: row_info moved out of png_struct to a local here. */
    row_info.width = (*png_ptr).iwidth; /* NOTE: width of current interlaced row */
    row_info.color_type = (*png_ptr).color_type;
    row_info.bit_depth = (*png_ptr).bit_depth;
    row_info.channels = (*png_ptr).channels;
    row_info.pixel_depth = (*png_ptr).pixel_depth;
    row_info.rowbytes = PNG_ROWBYTES(row_info.pixel_depth as usize, row_info.width as usize);

    /* PNG_WARNINGS_SUPPORTED: the checks for transforms that have been set but
     * were defined out are all conditional on a PNG_READ_*_SUPPORTED macro
     * being undefined; all of them are defined here, so nothing remains.
     */

    /* If interlaced and we do not need a new row, combine row and return.
     * Notice that the pixels we have from previous rows have been transformed
     * already; we can only combine like with like (transformed or
     * untransformed) and, because of the libpng API for interlaced images, this
     * means we must transform before de-interlacing.
     */
    if (*png_ptr).interlaced as c_int != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
        match (*png_ptr).pass as c_int {
            0 => {
                if ((*png_ptr).row_number & 0x07) != 0 {
                    if dsp_row != core::ptr::null_mut() {
                        png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            1 => {
                if ((*png_ptr).row_number & 0x07) != 0 || (*png_ptr).width < 5 {
                    if dsp_row != core::ptr::null_mut() {
                        png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            2 => {
                if ((*png_ptr).row_number & 0x07) != 4 {
                    if dsp_row != core::ptr::null_mut() && ((*png_ptr).row_number & 4) != 0 {
                        png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            3 => {
                if ((*png_ptr).row_number & 3) != 0 || (*png_ptr).width < 3 {
                    if dsp_row != core::ptr::null_mut() {
                        png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            4 => {
                if ((*png_ptr).row_number & 3) != 2 {
                    if dsp_row != core::ptr::null_mut() && ((*png_ptr).row_number & 2) != 0 {
                        png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            5 => {
                if ((*png_ptr).row_number & 1) != 0 || (*png_ptr).width < 2 {
                    if dsp_row != core::ptr::null_mut() {
                        png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            /* default: case 6: */
            _ => {
                if ((*png_ptr).row_number & 1) == 0 {
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
        }
    }

    if ((*png_ptr).mode & PNG_HAVE_IDAT) == 0 {
        png_error(
            png_ptr,
            b"Invalid attempt to read row data\0".as_ptr() as png_const_charp,
        );
    }

    /* Fill the row with IDAT data: */
    *(*png_ptr).row_buf = 255; /* to force error if no data was found */
    png_read_IDAT_data(
        png_ptr,
        (*png_ptr).row_buf,
        (row_info.rowbytes + 1) as png_alloc_size_t,
    );

    if (*(*png_ptr).row_buf) as c_int > PNG_FILTER_VALUE_NONE {
        if ((*(*png_ptr).row_buf) as c_int) < PNG_FILTER_VALUE_LAST {
            png_read_filter_row(
                png_ptr,
                &mut row_info,
                (*png_ptr).row_buf.add(1),
                (*png_ptr).prev_row.add(1) as png_const_bytep,
                *(*png_ptr).row_buf as c_int,
            );
        } else {
            png_error(
                png_ptr,
                b"bad adaptive filter value\0".as_ptr() as png_const_charp,
            );
        }
    }

    /* libpng 1.5.6: the following line was copying png_ptr->rowbytes before
     * 1.5.6, while the buffer really is this big in current versions of libpng
     * it may not be in the future, so this was changed just to copy the
     * interlaced count:
     */
    memcpy(
        (*png_ptr).prev_row as *mut c_void,
        (*png_ptr).row_buf as *const c_void,
        row_info.rowbytes + 1,
    );

    if ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
        && ((*png_ptr).filter_type as c_int == PNG_INTRAPIXEL_DIFFERENCING)
    {
        /* Intrapixel differencing */
        png_do_read_intrapixel(&mut row_info, (*png_ptr).row_buf.add(1));
    }

    if (*png_ptr).transformations != 0 || (*png_ptr).num_palette_max >= 0 {
        png_do_read_transformations(png_ptr, &mut row_info);
    }

    /* The transformed pixel depth should match the depth now in row_info. */
    if (*png_ptr).transformed_pixel_depth == 0 {
        (*png_ptr).transformed_pixel_depth = row_info.pixel_depth;
        if row_info.pixel_depth > (*png_ptr).maximum_pixel_depth {
            png_error(
                png_ptr,
                b"sequential row overflow\0".as_ptr() as png_const_charp,
            );
        }
    } else if (*png_ptr).transformed_pixel_depth != row_info.pixel_depth {
        png_error(
            png_ptr,
            b"internal sequential row size calculation error\0".as_ptr() as png_const_charp,
        );
    }

    /* Expand interlaced rows to full size */
    if (*png_ptr).interlaced as c_int != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
        if ((*png_ptr).pass as c_int) < 6 {
            png_do_read_interlace(
                &mut row_info,
                (*png_ptr).row_buf.add(1),
                (*png_ptr).pass as c_int,
                (*png_ptr).transformations,
            );
        }

        if dsp_row != core::ptr::null_mut() {
            png_combine_row(png_ptr, dsp_row, 1 /*display*/);
        }

        if row != core::ptr::null_mut() {
            png_combine_row(png_ptr, row, 0 /*row*/);
        }
    } else {
        if row != core::ptr::null_mut() {
            png_combine_row(png_ptr, row, -1 /*ignored*/);
        }

        if dsp_row != core::ptr::null_mut() {
            png_combine_row(png_ptr, dsp_row, -1 /*ignored*/);
        }
    }
    png_read_finish_row(png_ptr);

    if (*png_ptr).read_row_fn.is_some() {
        ((*png_ptr).read_row_fn.unwrap())(png_ptr, (*png_ptr).row_number, (*png_ptr).pass as c_int);
    }
}

/* Read one or more rows of image data.  If the image is interlaced,
 * and png_set_interlace_handling() has been called, the rows need to
 * contain the contents of the rows from the previous pass.  If the
 * image has alpha or transparency, and png_handle_alpha()[*] has been
 * called, the rows contents must be initialized to the contents of the
 * screen.
 *
 * "row" holds the actual image, and pixels are placed in it
 * as they arrive.  If the image is displayed after each pass, it will
 * appear to "sparkle" in.  "display_row" can be used to display a
 * "chunky" progressive image, with finer detail added as it becomes
 * available.  If you do not want this "chunky" display, you may pass
 * NULL for display_row.  If you do not want the sparkle display, and
 * you have not called png_handle_alpha(), you may pass NULL for rows.
 * If you have called png_handle_alpha(), and the image has either an
 * alpha channel or a transparency chunk, you must provide a buffer for
 * rows.  In this case, you do not have to provide a display_row buffer
 * also, but you may.  If the image is not interlaced, or if you have
 * not called png_set_interlace_handling(), the display_row buffer will
 * be ignored, so pass NULL to it.
 *
 * [*] png_handle_alpha() does not exist yet, as of this version of libpng
 */

/* png_read_rows */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_rows(
    png_ptr: png_structrp,
    row: png_bytepp,
    display_row: png_bytepp,
    num_rows: png_uint_32,
) {
    let mut i: png_uint_32;
    let mut rp: png_bytepp;
    let mut dp: png_bytepp;

    if png_ptr == core::ptr::null_mut() {
        return;
    }

    rp = row;
    dp = display_row;
    if rp != core::ptr::null_mut() && dp != core::ptr::null_mut() {
        i = 0;
        while i < num_rows {
            let rptr: png_bytep = *rp;
            rp = rp.add(1);
            let dptr: png_bytep = *dp;
            dp = dp.add(1);

            png_read_row(png_ptr, rptr, dptr);

            i = i.wrapping_add(1);
        }
    } else if rp != core::ptr::null_mut() {
        i = 0;
        while i < num_rows {
            let rptr: png_bytep = *rp;
            png_read_row(png_ptr, rptr, core::ptr::null_mut());
            rp = rp.add(1);

            i = i.wrapping_add(1);
        }
    } else if dp != core::ptr::null_mut() {
        i = 0;
        while i < num_rows {
            let dptr: png_bytep = *dp;
            png_read_row(png_ptr, core::ptr::null_mut(), dptr);
            dp = dp.add(1);

            i = i.wrapping_add(1);
        }
    }
}

/* Read the entire image.  If the image has an alpha channel or a tRNS
 * chunk, and you have called png_handle_alpha()[*], you will need to
 * initialize the image to the current image that PNG will be overlaying.
 * We set the num_rows again here, in case it was incorrectly set in
 * png_read_start_row() by a call to png_read_update_info() or
 * png_start_read_image() if png_set_interlace_handling() wasn't called
 * prior to either of these functions like it should have been.  You can
 * only call this function once.  If you desire to have an image for
 * each pass of a interlaced image, use png_read_rows() instead.
 *
 * [*] png_handle_alpha() does not exist yet, as of this version of libpng
 */
/* png_read_image */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_image(png_ptr: png_structrp, image: png_bytepp) {
    let mut i: png_uint_32;
    let image_height: png_uint_32;
    let pass: c_int;
    let mut j: c_int;
    let mut rp: png_bytepp;

    if png_ptr == core::ptr::null_mut() {
        return;
    }

    if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
        pass = png_set_interlace_handling(png_ptr);
        /* And make sure transforms are initialized. */
        png_start_read_image(png_ptr);
    } else {
        if (*png_ptr).interlaced as c_int != 0
            && ((*png_ptr).transformations & PNG_INTERLACE) == 0
        {
            /* Caller called png_start_read_image or png_read_update_info without
             * first turning on the PNG_INTERLACE transform.  We can fix this here,
             * but the caller should do it!
             */
            png_warning(
                png_ptr,
                b"Interlace handling should be turned on when using png_read_image\0".as_ptr()
                    as png_const_charp,
            );
            /* Make sure this is set correctly */
            (*png_ptr).num_rows = (*png_ptr).height;
        }

        /* Obtain the pass number, which also turns on the PNG_INTERLACE flag in
         * the above error case.
         */
        pass = png_set_interlace_handling(png_ptr);
    }

    image_height = (*png_ptr).height;

    j = 0;
    while j < pass {
        rp = image;
        i = 0;
        while i < image_height {
            png_read_row(png_ptr, *rp, core::ptr::null_mut());
            rp = rp.add(1);

            i = i.wrapping_add(1);
        }

        j += 1;
    }
}
