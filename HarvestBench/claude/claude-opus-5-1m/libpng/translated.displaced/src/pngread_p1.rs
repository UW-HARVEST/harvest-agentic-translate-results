// pngread.c - read a PNG file
//
// This file contains routines that an application calls directly to
// read a PNG file or stream.
//
// Chunk 1: png_create_read_struct .. png_read_image

use crate::*;

/* Create a PNG structure for reading, and allocate any memory needed. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_read_struct(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
) -> png_structp {
    png_create_read_struct_2(
        user_png_ver,
        error_ptr,
        error_fn,
        warn_fn,
        core::ptr::null_mut(),
        None,
        None,
    )
}

/* Alternate create PNG structure for reading, and allocate any memory
 * needed.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_read_struct_2(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
    mem_ptr: png_voidp,
    malloc_fn: png_malloc_ptr,
    free_fn: png_free_ptr,
) -> png_structp {
    let png_ptr: png_structp = png_create_png_struct(
        user_png_ver,
        error_ptr,
        error_fn,
        warn_fn,
        mem_ptr,
        malloc_fn,
        free_fn,
    );

    if !png_ptr.is_null() {
        (*png_ptr).mode = PNG_IS_READ_STRUCT;

        /* Added in libpng-1.6.0; this can be used to detect a read structure if
         * required (it will be zero in a write structure.)
         */
        (*png_ptr).IDAT_read_size = PNG_IDAT_READ_SIZE as uInt;

        (*png_ptr).flags |= PNG_FLAG_BENIGN_ERRORS_WARN;

        /* In stable builds only warn if an application error can be completely
         * handled.
         */

        /* TODO: delay this, it can be done in png_init_io (if the app doesn't
         * do it itself) avoiding setting the default function if it is not
         * required.
         */
        png_set_read_fn(png_ptr, core::ptr::null_mut(), None);
    }

    png_ptr
}

/* Read the information before the actual image data.  This has been
 * changed in v0.90 to allow reading a file that already has the magic
 * bytes read from the stream.  You can tell libpng how many bytes have
 * been read from the beginning of the stream (up to the maximum of 8)
 * via png_set_sig_bytes(), and we will only check the remaining bytes
 * here.  The application can then have access to the signature bytes we
 * read if it is determined that this isn't a valid PNG file.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    /* Read and check the PNG file signature. */
    png_read_sig(png_ptr, info_ptr);

    loop {
        let length: png_uint_32 = png_read_chunk_header(png_ptr);
        let chunk_name: png_uint_32 = (*png_ptr).chunk_name;

        /* IDAT logic needs to happen here to simplify getting the two flags
         * right.
         */
        if chunk_name == png_IDAT {
            if ((*png_ptr).mode & PNG_HAVE_IHDR) == 0 {
                png_chunk_error(
                    png_ptr as png_const_structrp,
                    cstr!("Missing IHDR before IDAT"),
                );
            } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
                && ((*png_ptr).mode & PNG_HAVE_PLTE) == 0
            {
                png_chunk_error(
                    png_ptr as png_const_structrp,
                    cstr!("Missing PLTE before IDAT"),
                );
            } else if ((*png_ptr).mode & PNG_AFTER_IDAT) != 0 {
                png_chunk_benign_error(
                    png_ptr as png_const_structrp,
                    cstr!("Too many IDATs found"),
                );
            }

            (*png_ptr).mode |= PNG_HAVE_IDAT;
        } else if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
            (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT;
            (*png_ptr).mode |= PNG_AFTER_IDAT;
        }

        if chunk_name == png_IHDR {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else if chunk_name == png_IEND {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else {
            /* C: else if ((keep = png_chunk_unknown_handling(png_ptr,
             * chunk_name)) != 0) { ... } else if (...) ...
             */
            let keep: c_int = png_chunk_unknown_handling(png_ptr as png_const_structrp, chunk_name);

            if keep != 0 {
                png_handle_unknown(png_ptr, info_ptr, length, keep);

                if chunk_name == png_PLTE {
                    (*png_ptr).mode |= PNG_HAVE_PLTE;
                } else if chunk_name == png_IDAT {
                    (*png_ptr).idat_size = 0; /* It has been consumed */
                    break;
                }
            } else if chunk_name == png_IDAT {
                (*png_ptr).idat_size = length;
                break;
            } else {
                png_handle_chunk(png_ptr, info_ptr, length);
            }
        }
    }
}

/* Optional call to update the users info_ptr structure */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_update_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    if !png_ptr.is_null() {
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
            png_read_start_row(png_ptr);

            png_read_transform_info(png_ptr, info_ptr);
        }
        /* New in 1.6.0 this avoids the bug of doing the initializations twice */
        else {
            png_app_error(
                png_ptr as png_const_structrp,
                cstr!("png_read_update_info/png_start_read_image: duplicate call"),
            );
        }
    }
}

/* Initialize palette, background, etc, after transformations
 * are set, but before any reading takes place.  This allows
 * the user to obtain a gamma-corrected palette, for example.
 * If the user doesn't call this, we will do it ourselves.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_start_read_image(png_ptr: png_structrp) {
    if !png_ptr.is_null() {
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
            png_read_start_row(png_ptr);
        }
        /* New in 1.6.0 this avoids the bug of doing the initializations twice */
        else {
            png_app_error(
                png_ptr as png_const_structrp,
                cstr!("png_start_read_image/png_read_update_info: duplicate call"),
            );
        }
    }
}

/* Undoes intrapixel differencing,
 * NOTE: this is apparently only supported in the 'sequential' reader.
 */
unsafe fn png_do_read_intrapixel(row_info: png_row_infop, row: png_bytep) {
    if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        let row_width: png_uint_32 = (*row_info).width;

        if (*row_info).bit_depth == 8 {
            let bytes_per_pixel: c_int;
            let mut rp: png_bytep;
            let mut i: png_uint_32;

            if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB {
                bytes_per_pixel = 3;
            } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                bytes_per_pixel = 4;
            } else {
                return;
            }

            i = 0;
            rp = row;
            while i < row_width {
                *rp = ((256 + *rp as c_int + *rp.offset(1) as c_int) & 0xff) as png_byte;
                *rp.offset(2) =
                    ((256 + *rp.offset(2) as c_int + *rp.offset(1) as c_int) & 0xff) as png_byte;

                i += 1;
                rp = rp.offset(bytes_per_pixel as isize);
            }
        } else if (*row_info).bit_depth == 16 {
            let bytes_per_pixel: c_int;
            let mut rp: png_bytep;
            let mut i: png_uint_32;

            if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB {
                bytes_per_pixel = 6;
            } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                bytes_per_pixel = 8;
            } else {
                return;
            }

            i = 0;
            rp = row;
            while i < row_width {
                let s0: png_uint_32 =
                    (((*rp as c_int) << 8) as png_uint_32) | *rp.offset(1) as png_uint_32;
                let s1: png_uint_32 =
                    (((*rp.offset(2) as c_int) << 8) as png_uint_32) | *rp.offset(3) as png_uint_32;
                let s2: png_uint_32 =
                    (((*rp.offset(4) as c_int) << 8) as png_uint_32) | *rp.offset(5) as png_uint_32;
                let red: png_uint_32 = s0.wrapping_add(s1).wrapping_add(65536) & 0xffff;
                let blue: png_uint_32 = s2.wrapping_add(s1).wrapping_add(65536) & 0xffff;
                *rp = ((red >> 8) & 0xff) as png_byte;
                *rp.offset(1) = (red & 0xff) as png_byte;
                *rp.offset(4) = ((blue >> 8) & 0xff) as png_byte;
                *rp.offset(5) = (blue & 0xff) as png_byte;

                i += 1;
                rp = rp.offset(bytes_per_pixel as isize);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_row(
    png_ptr: png_structrp,
    row: png_bytep,
    display_row: png_bytep,
) {
    let mut row_info: png_row_info = core::mem::zeroed();

    if png_ptr.is_null() {
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

    if (*png_ptr).row_number == 0 && (*png_ptr).pass == 0 {
        /* Check for transforms that have been set but were defined out */
    }

    /* If interlaced and we do not need a new row, combine row and return.
     * Notice that the pixels we have from previous rows have been transformed
     * already; we can only combine like with like (transformed or
     * untransformed) and, because of the libpng API for interlaced images, this
     * means we must transform before de-interlacing.
     */
    if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
        match (*png_ptr).pass {
            0 => {
                if ((*png_ptr).row_number & 0x07) != 0 {
                    if !display_row.is_null() {
                        png_combine_row(png_ptr as png_const_structrp, display_row, 1 /*display*/);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            1 => {
                if ((*png_ptr).row_number & 0x07) != 0 || (*png_ptr).width < 5 {
                    if !display_row.is_null() {
                        png_combine_row(png_ptr as png_const_structrp, display_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            2 => {
                if ((*png_ptr).row_number & 0x07) != 4 {
                    if !display_row.is_null() && ((*png_ptr).row_number & 4) != 0 {
                        png_combine_row(png_ptr as png_const_structrp, display_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            3 => {
                if ((*png_ptr).row_number & 3) != 0 || (*png_ptr).width < 3 {
                    if !display_row.is_null() {
                        png_combine_row(png_ptr as png_const_structrp, display_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            4 => {
                if ((*png_ptr).row_number & 3) != 2 {
                    if !display_row.is_null() && ((*png_ptr).row_number & 2) != 0 {
                        png_combine_row(png_ptr as png_const_structrp, display_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            5 => {
                if ((*png_ptr).row_number & 1) != 0 || (*png_ptr).width < 2 {
                    if !display_row.is_null() {
                        png_combine_row(png_ptr as png_const_structrp, display_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            /* default and case 6: */
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
            png_ptr as png_const_structrp,
            cstr!("Invalid attempt to read row data"),
        );
    }

    /* Fill the row with IDAT data: */
    *(*png_ptr).row_buf = 255; /* to force error if no data was found */
    png_read_IDAT_data(png_ptr, (*png_ptr).row_buf, row_info.rowbytes + 1);

    if (*(*png_ptr).row_buf as c_int) > PNG_FILTER_VALUE_NONE {
        if (*(*png_ptr).row_buf as c_int) < PNG_FILTER_VALUE_LAST {
            png_read_filter_row(
                png_ptr,
                &mut row_info,
                (*png_ptr).row_buf.offset(1),
                (*png_ptr).prev_row.offset(1) as png_const_bytep,
                *(*png_ptr).row_buf as c_int,
            );
        } else {
            png_error(
                png_ptr as png_const_structrp,
                cstr!("bad adaptive filter value"),
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
        png_do_read_intrapixel(&mut row_info, (*png_ptr).row_buf.offset(1));
    }

    if (*png_ptr).transformations != 0 || (*png_ptr).num_palette_max >= 0 {
        png_do_read_transformations(png_ptr, &mut row_info);
    }

    /* The transformed pixel depth should match the depth now in row_info. */
    if (*png_ptr).transformed_pixel_depth == 0 {
        (*png_ptr).transformed_pixel_depth = row_info.pixel_depth;
        if row_info.pixel_depth > (*png_ptr).maximum_pixel_depth {
            png_error(
                png_ptr as png_const_structrp,
                cstr!("sequential row overflow"),
            );
        }
    } else if (*png_ptr).transformed_pixel_depth != row_info.pixel_depth {
        png_error(
            png_ptr as png_const_structrp,
            cstr!("internal sequential row size calculation error"),
        );
    }

    /* Expand interlaced rows to full size */
    if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
        if (*png_ptr).pass < 6 {
            png_do_read_interlace(
                &mut row_info,
                (*png_ptr).row_buf.offset(1),
                (*png_ptr).pass as c_int,
                (*png_ptr).transformations,
            );
        }

        if !display_row.is_null() {
            png_combine_row(png_ptr as png_const_structrp, display_row, 1 /*display*/);
        }

        if !row.is_null() {
            png_combine_row(png_ptr as png_const_structrp, row, 0 /*row*/);
        }
    } else {
        if !row.is_null() {
            png_combine_row(png_ptr as png_const_structrp, row, -1 /*ignored*/);
        }

        if !display_row.is_null() {
            png_combine_row(png_ptr as png_const_structrp, display_row, -1 /*ignored*/);
        }
    }
    png_read_finish_row(png_ptr);

    if (*png_ptr).read_row_fn.is_some() {
        ((*png_ptr).read_row_fn.unwrap())(
            png_ptr,
            (*png_ptr).row_number,
            (*png_ptr).pass as c_int,
        );
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

    if png_ptr.is_null() {
        return;
    }

    rp = row;
    dp = display_row;
    if !rp.is_null() && !dp.is_null() {
        i = 0;
        while i < num_rows {
            let rptr: png_bytep = *rp;
            rp = rp.offset(1);
            let dptr: png_bytep = *dp;
            dp = dp.offset(1);

            png_read_row(png_ptr, rptr, dptr);

            i += 1;
        }
    } else if !rp.is_null() {
        i = 0;
        while i < num_rows {
            let rptr: png_bytep = *rp;
            png_read_row(png_ptr, rptr, core::ptr::null_mut());
            rp = rp.offset(1);

            i += 1;
        }
    } else if !dp.is_null() {
        i = 0;
        while i < num_rows {
            let dptr: png_bytep = *dp;
            png_read_row(png_ptr, core::ptr::null_mut(), dptr);
            dp = dp.offset(1);

            i += 1;
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_image(png_ptr: png_structrp, image: png_bytepp) {
    let mut i: png_uint_32;
    let image_height: png_uint_32;
    let pass: c_int;
    let mut j: c_int;
    let mut rp: png_bytepp;

    if png_ptr.is_null() {
        return;
    }

    if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
        pass = png_set_interlace_handling(png_ptr);
        /* And make sure transforms are initialized. */
        png_start_read_image(png_ptr);
    } else {
        if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) == 0 {
            /* Caller called png_start_read_image or png_read_update_info without
             * first turning on the PNG_INTERLACE transform.  We can fix this here,
             * but the caller should do it!
             */
            png_warning(
                png_ptr as png_const_structrp,
                cstr!("Interlace handling should be turned on when using png_read_image"),
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
            rp = rp.offset(1);

            i += 1;
        }

        j += 1;
    }
}
