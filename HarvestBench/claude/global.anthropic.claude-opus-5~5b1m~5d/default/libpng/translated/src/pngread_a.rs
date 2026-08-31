//! pngread.c lines 1-1121: routines that an application calls directly to read
//! a PNG file or stream (creation, info, row/image reading, destruction).
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

/* Create a PNG structure for reading, and allocate any memory needed. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_create_read_struct(
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
pub unsafe extern "C-unwind" fn png_create_read_struct_2(
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
        /* PNG_RELEASE_BUILD is false in this build, so
         * PNG_FLAG_APP_WARNINGS_WARN is not set here.
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
pub unsafe extern "C-unwind" fn png_read_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    let mut keep: c_int;

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
                png_chunk_error(png_ptr, c"Missing IHDR before IDAT".as_ptr());
            } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
                && ((*png_ptr).mode & PNG_HAVE_PLTE) == 0
            {
                png_chunk_error(png_ptr, c"Missing PLTE before IDAT".as_ptr());
            } else if ((*png_ptr).mode & PNG_AFTER_IDAT) != 0 {
                png_chunk_benign_error(png_ptr, c"Too many IDATs found".as_ptr());
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
        } else if {
            keep = png_chunk_unknown_handling(png_ptr, chunk_name);
            keep != 0
        } {
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

/* Optional call to update the users info_ptr structure */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_update_info(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
) {
    if !png_ptr.is_null() {
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
            png_read_start_row(png_ptr);

            png_read_transform_info(png_ptr, info_ptr);
        }
        /* New in 1.6.0 this avoids the bug of doing the initializations twice */
        else {
            png_app_error(
                png_ptr,
                c"png_read_update_info/png_start_read_image: duplicate call".as_ptr(),
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
pub unsafe extern "C-unwind" fn png_start_read_image(png_ptr: png_structrp) {
    if !png_ptr.is_null() {
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
            png_read_start_row(png_ptr);
        }
        /* New in 1.6.0 this avoids the bug of doing the initializations twice */
        else {
            png_app_error(
                png_ptr,
                c"png_start_read_image/png_read_update_info: duplicate call".as_ptr(),
            );
        }
    }
}

/* Undoes intrapixel differencing,
 * NOTE: this is apparently only supported in the 'sequential' reader.
 */
pub unsafe fn png_do_read_intrapixel(row_info: png_row_infop, row: png_bytep) {
    if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        let bytes_per_pixel: c_int;
        let row_width: png_uint_32 = (*row_info).width;

        if (*row_info).bit_depth == 8 {
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
                *rp = ((256 + (*rp as c_int) + (*(rp.add(1)) as c_int)) & 0xff) as png_byte;
                *(rp.add(2)) =
                    ((256 + (*(rp.add(2)) as c_int) + (*(rp.add(1)) as c_int)) & 0xff) as png_byte;

                i = i.wrapping_add(1);
                rp = rp.offset(bytes_per_pixel as isize);
            }
        } else if (*row_info).bit_depth == 16 {
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
                    (((*(rp) as c_int) << 8) as png_uint_32) | (*(rp.add(1)) as png_uint_32);
                let s1: png_uint_32 =
                    (((*(rp.add(2)) as c_int) << 8) as png_uint_32) | (*(rp.add(3)) as png_uint_32);
                let s2: png_uint_32 =
                    (((*(rp.add(4)) as c_int) << 8) as png_uint_32) | (*(rp.add(5)) as png_uint_32);
                let red: png_uint_32 = (s0.wrapping_add(s1).wrapping_add(65536)) & 0xffff;
                let blue: png_uint_32 = (s2.wrapping_add(s1).wrapping_add(65536)) & 0xffff;
                *(rp) = ((red >> 8) & 0xff) as png_byte;
                *(rp.add(1)) = (red & 0xff) as png_byte;
                *(rp.add(4)) = ((blue >> 8) & 0xff) as png_byte;
                *(rp.add(5)) = (blue & 0xff) as png_byte;

                i = i.wrapping_add(1);
                rp = rp.offset(bytes_per_pixel as isize);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_row(
    png_ptr: png_structrp,
    row: png_bytep,
    dsp_row: png_bytep,
) {
    let mut row_info: png_row_info = png_row_info::default();

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
    row_info.rowbytes = PNG_ROWBYTES(row_info.pixel_depth as u32, row_info.width);

    if (*png_ptr).row_number == 0 && (*png_ptr).pass == 0 {
        /* Check for transforms that have been set but were defined out.
         *
         * All of PNG_READ_INVERT, PNG_READ_FILLER, PNG_READ_PACKSWAP,
         * PNG_READ_PACK, PNG_READ_SHIFT, PNG_READ_BGR and PNG_READ_SWAP are
         * supported in this build, so there is nothing to warn about here.
         */
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
                    if !dsp_row.is_null() {
                        png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            1 => {
                if ((*png_ptr).row_number & 0x07) != 0 || (*png_ptr).width < 5 {
                    if !dsp_row.is_null() {
                        png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            2 => {
                if ((*png_ptr).row_number & 0x07) != 4 {
                    if !dsp_row.is_null() && ((*png_ptr).row_number & 4) != 0 {
                        png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            3 => {
                if ((*png_ptr).row_number & 3) != 0 || (*png_ptr).width < 3 {
                    if !dsp_row.is_null() {
                        png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            4 => {
                if ((*png_ptr).row_number & 3) != 2 {
                    if !dsp_row.is_null() && ((*png_ptr).row_number & 2) != 0 {
                        png_combine_row(png_ptr, dsp_row, 1 /*display*/);
                    }

                    png_read_finish_row(png_ptr);
                    return;
                }
            }

            5 => {
                if ((*png_ptr).row_number & 1) != 0 || (*png_ptr).width < 2 {
                    if !dsp_row.is_null() {
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
        png_error(png_ptr, c"Invalid attempt to read row data".as_ptr());
    }

    /* Fill the row with IDAT data: */
    *(*png_ptr).row_buf.add(0) = 255; /* to force error if no data was found */
    png_read_IDAT_data(png_ptr, (*png_ptr).row_buf, row_info.rowbytes + 1);

    if (*(*png_ptr).row_buf.add(0) as c_int) > PNG_FILTER_VALUE_NONE {
        if (*(*png_ptr).row_buf.add(0) as c_int) < PNG_FILTER_VALUE_LAST {
            png_read_filter_row(
                png_ptr,
                &mut row_info,
                (*png_ptr).row_buf.add(1),
                (*png_ptr).prev_row.add(1),
                *(*png_ptr).row_buf.add(0) as c_int,
            );
        } else {
            png_error(png_ptr, c"bad adaptive filter value".as_ptr());
        }
    }

    /* libpng 1.5.6: the following line was copying png_ptr->rowbytes before
     * 1.5.6, while the buffer really is this big in current versions of libpng
     * it may not be in the future, so this was changed just to copy the
     * interlaced count:
     */
    memcpy(
        (*png_ptr).prev_row as *mut u8,
        (*png_ptr).row_buf as *const u8,
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
            png_error(png_ptr, c"sequential row overflow".as_ptr());
        }
    } else if (*png_ptr).transformed_pixel_depth != row_info.pixel_depth {
        png_error(
            png_ptr,
            c"internal sequential row size calculation error".as_ptr(),
        );
    }

    /* Expand interlaced rows to full size */
    if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
        if (*png_ptr).pass < 6 {
            png_do_read_interlace(
                &mut row_info,
                (*png_ptr).row_buf.add(1),
                (*png_ptr).pass as c_int,
                (*png_ptr).transformations,
            );
        }

        if !dsp_row.is_null() {
            png_combine_row(png_ptr, dsp_row, 1 /*display*/);
        }

        if !row.is_null() {
            png_combine_row(png_ptr, row, 0 /*row*/);
        }
    } else {
        if !row.is_null() {
            png_combine_row(png_ptr, row, -1 /*ignored*/);
        }

        if !dsp_row.is_null() {
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_rows(
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
            rp = rp.add(1);
            let dptr: png_bytep = *dp;
            dp = dp.add(1);

            png_read_row(png_ptr, rptr, dptr);

            i = i.wrapping_add(1);
        }
    } else if !rp.is_null() {
        i = 0;
        while i < num_rows {
            let rptr: png_bytep = *rp;
            png_read_row(png_ptr, rptr, core::ptr::null_mut());
            rp = rp.add(1);

            i = i.wrapping_add(1);
        }
    } else if !dp.is_null() {
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
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_image(png_ptr: png_structrp, image: png_bytepp) {
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
                png_ptr,
                c"Interlace handling should be turned on when using png_read_image".as_ptr(),
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

/* Read the end of the PNG file.  Will not read past the end of the
 * file, will verify the end is accurate, and will read any comments
 * or time information at the end of the file, if info is not NULL.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_end(png_ptr: png_structrp, info_ptr: png_inforp) {
    let mut keep: c_int;

    if png_ptr.is_null() {
        return;
    }

    /* If png_read_end is called in the middle of reading the rows there may
     * still be pending IDAT data and an owned zstream.  Deal with this here.
     */
    if png_chunk_unknown_handling(png_ptr, png_IDAT) == 0 {
        png_read_finish_IDAT(png_ptr);
    }

    /* Report invalid palette index; added at libpng-1.5.10 */
    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
        && (*png_ptr).num_palette_max >= (*png_ptr).num_palette as c_int
    {
        png_benign_error(
            png_ptr,
            c"Read palette index exceeding num_palette".as_ptr(),
        );
    }

    loop {
        let length: png_uint_32 = png_read_chunk_header(png_ptr);
        let chunk_name: png_uint_32 = (*png_ptr).chunk_name;

        if chunk_name != png_IDAT {
            /* These flags must be set consistently for all non-IDAT chunks,
             * including the unknown chunks.
             */
            (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT | PNG_AFTER_IDAT;
        }

        if chunk_name == png_IEND {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else if chunk_name == png_IHDR {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else if info_ptr.is_null() {
            png_crc_finish(png_ptr, length);
        } else if {
            keep = png_chunk_unknown_handling(png_ptr, chunk_name);
            keep != 0
        } {
            if chunk_name == png_IDAT {
                if (length > 0 && ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0)
                    || ((*png_ptr).mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0
                {
                    png_benign_error(png_ptr, c".Too many IDATs found".as_ptr());
                }
            }
            png_handle_unknown(png_ptr, info_ptr, length, keep);
            if chunk_name == png_PLTE {
                (*png_ptr).mode |= PNG_HAVE_PLTE;
            }
        } else if chunk_name == png_IDAT {
            /* Zero length IDATs are legal after the last IDAT has been
             * read, but not after other chunks have been read.  1.6 does not
             * always read all the deflate data; specifically it cannot be relied
             * upon to read the Adler32 at the end.  If it doesn't ignore IDAT
             * chunks which are longer than zero as well:
             */
            if (length > 0 && ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0)
                || ((*png_ptr).mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0
            {
                png_benign_error(png_ptr, c"..Too many IDATs found".as_ptr());
            }

            png_crc_finish(png_ptr, length);
        } else {
            png_handle_chunk(png_ptr, info_ptr, length);
        }

        if ((*png_ptr).mode & PNG_HAVE_IEND) != 0 {
            break;
        }
    }
}

/* Free all memory used in the read struct */
pub unsafe fn png_read_destroy(png_ptr: png_structrp) {
    png_destroy_gamma_table(png_ptr);

    png_free(png_ptr, (*png_ptr).big_row_buf as png_voidp);
    (*png_ptr).big_row_buf = core::ptr::null_mut();
    png_free(png_ptr, (*png_ptr).big_prev_row as png_voidp);
    (*png_ptr).big_prev_row = core::ptr::null_mut();
    png_free(png_ptr, (*png_ptr).read_buffer as png_voidp);
    (*png_ptr).read_buffer = core::ptr::null_mut();

    png_free(png_ptr, (*png_ptr).palette_lookup as png_voidp);
    (*png_ptr).palette_lookup = core::ptr::null_mut();
    png_free(png_ptr, (*png_ptr).quantize_index as png_voidp);
    (*png_ptr).quantize_index = core::ptr::null_mut();

    /* png_ptr->palette is always independently allocated (not aliased
     * with info_ptr->palette), so free it unconditionally.
     */
    png_free(png_ptr, (*png_ptr).palette as png_voidp);
    (*png_ptr).palette = core::ptr::null_mut();

    /* png_ptr->trans_alpha is always independently allocated (not aliased
     * with info_ptr->trans_alpha), so free it unconditionally.
     */
    png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
    (*png_ptr).trans_alpha = core::ptr::null_mut();

    inflateEnd(&mut (*png_ptr).zstream);

    png_free(png_ptr, (*png_ptr).save_buffer as png_voidp);
    (*png_ptr).save_buffer = core::ptr::null_mut();

    png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
    (*png_ptr).unknown_chunk.data = core::ptr::null_mut();

    png_free(png_ptr, (*png_ptr).chunk_list as png_voidp);
    (*png_ptr).chunk_list = core::ptr::null_mut();

    /* NOTE: the 'setjmp' buffer may still be allocated and the memory and error
     * callbacks are still set at this point.  They are required to complete the
     * destruction of the png_struct itself.
     */
}

/* Free all memory used by the read */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_destroy_read_struct(
    png_ptr_ptr: png_structpp,
    info_ptr_ptr: png_infopp,
    end_info_ptr_ptr: png_infopp,
) {
    let mut png_ptr: png_structrp = core::ptr::null_mut();

    if !png_ptr_ptr.is_null() {
        png_ptr = *png_ptr_ptr;
    }

    if png_ptr.is_null() {
        return;
    }

    /* libpng 1.6.0: use the API to destroy info structs to ensure consistent
     * behavior.  Prior to 1.6.0 libpng did extra 'info' destruction in this API.
     * The extra was, apparently, unnecessary yet this hides memory leak bugs.
     */
    png_destroy_info_struct(png_ptr, end_info_ptr_ptr);
    png_destroy_info_struct(png_ptr, info_ptr_ptr);

    *png_ptr_ptr = core::ptr::null_mut();
    png_read_destroy(png_ptr);
    png_destroy_png_struct(png_ptr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_read_status_fn(
    png_ptr: png_structrp,
    read_row_fn: png_read_status_ptr,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).read_row_fn = read_row_fn;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_png(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    transforms: c_int,
    params: png_voidp,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    /* png_read_info() gives us all of the information from the
     * PNG file before the first IDAT (image data chunk).
     */
    png_read_info(png_ptr, info_ptr);
    if (*info_ptr).height as usize
        > (PNG_UINT_32_MAX as usize) / core::mem::size_of::<png_bytep>()
    {
        png_error(
            png_ptr,
            c"Image is too high to process with png_read_png()".as_ptr(),
        );
    }

    /* -------------- image transformations start here ------------------- */
    /* libpng 1.6.10: add code to cause a png_app_error if a selected TRANSFORM
     * is not implemented.  This will only happen in de-configured (non-default)
     * libpng builds.  The results can be unexpected - png_read_png may return
     * short or mal-formed rows because the transform is skipped.
     */

    /* Tell libpng to strip 16-bit/color files down to 8 bits per color.
     */
    if (transforms & PNG_TRANSFORM_SCALE_16) != 0 {
        /* Added at libpng-1.5.4. "strip_16" produces the same result that it
         * did in earlier versions, while "scale_16" is now more accurate.
         */
        png_set_scale_16(png_ptr);
    }

    /* If both SCALE and STRIP are required pngrtran will effectively cancel the
     * latter by doing SCALE first.  This is ok and allows apps not to check for
     * which is supported to get the right answer.
     */
    if (transforms & PNG_TRANSFORM_STRIP_16) != 0 {
        png_set_strip_16(png_ptr);
    }

    /* Strip alpha bytes from the input data without combining with
     * the background (not recommended).
     */
    if (transforms & PNG_TRANSFORM_STRIP_ALPHA) != 0 {
        png_set_strip_alpha(png_ptr);
    }

    /* Extract multiple pixels with bit depths of 1, 2, or 4 from a single
     * byte into separate bytes (useful for paletted and grayscale images).
     */
    if (transforms & PNG_TRANSFORM_PACKING) != 0 {
        png_set_packing(png_ptr);
    }

    /* Change the order of packed pixels to least significant bit first
     * (not useful if you are using png_set_packing).
     */
    if (transforms & PNG_TRANSFORM_PACKSWAP) != 0 {
        png_set_packswap(png_ptr);
    }

    /* Expand paletted colors into true RGB triplets
     * Expand grayscale images to full 8 bits from 1, 2, or 4 bits/pixel
     * Expand paletted or RGB images with transparency to full alpha
     * channels so the data will be available as RGBA quartets.
     */
    if (transforms & PNG_TRANSFORM_EXPAND) != 0 {
        png_set_expand(png_ptr);
    }

    /* We don't handle background color or gamma transformation or quantizing.
     */

    /* Invert monochrome files to have 0 as white and 1 as black
     */
    if (transforms & PNG_TRANSFORM_INVERT_MONO) != 0 {
        png_set_invert_mono(png_ptr);
    }

    /* If you want to shift the pixel values from the range [0,255] or
     * [0,65535] to the original [0,7] or [0,31], or whatever range the
     * colors were originally in:
     */
    if (transforms & PNG_TRANSFORM_SHIFT) != 0 {
        if ((*info_ptr).valid & PNG_INFO_sBIT) != 0 {
            png_set_shift(png_ptr, core::ptr::addr_of!((*info_ptr).sig_bit));
        }
    }

    /* Flip the RGB pixels to BGR (or RGBA to BGRA) */
    if (transforms & PNG_TRANSFORM_BGR) != 0 {
        png_set_bgr(png_ptr);
    }

    /* Swap the RGBA or GA data to ARGB or AG (or BGRA to ABGR) */
    if (transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0 {
        png_set_swap_alpha(png_ptr);
    }

    /* Swap bytes of 16-bit files to least significant byte first */
    if (transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0 {
        png_set_swap(png_ptr);
    }

    /* Added at libpng-1.2.41 */
    /* Invert the alpha channel from opacity to transparency */
    if (transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0 {
        png_set_invert_alpha(png_ptr);
    }

    /* Added at libpng-1.2.41 */
    /* Expand grayscale image to RGB */
    if (transforms & PNG_TRANSFORM_GRAY_TO_RGB) != 0 {
        png_set_gray_to_rgb(png_ptr);
    }

    /* Added at libpng-1.5.4 */
    if (transforms & PNG_TRANSFORM_EXPAND_16) != 0 {
        png_set_expand_16(png_ptr);
    }

    /* We don't handle adding filler bytes */

    /* We use png_read_image and rely on that for interlace handling, but we also
     * call png_read_update_info therefore must turn on interlace handling now:
     */
    png_set_interlace_handling(png_ptr);

    /* Optional call to gamma correct and add the background to the palette
     * and update info structure.  REQUIRED if you are expecting libpng to
     * update the palette for you (i.e., you selected such a transform above).
     */
    png_read_update_info(png_ptr, info_ptr);

    /* -------------- image transformations end here ------------------- */

    png_free_data(png_ptr, info_ptr, PNG_FREE_ROWS, 0);
    if (*info_ptr).row_pointers.is_null() {
        let mut iptr: png_uint_32;

        (*info_ptr).row_pointers = png_malloc(
            png_ptr,
            ((*info_ptr).height as usize) * core::mem::size_of::<png_bytep>(),
        ) as png_bytepp;

        iptr = 0;
        while iptr < (*info_ptr).height {
            *(*info_ptr).row_pointers.add(iptr as usize) = core::ptr::null_mut();
            iptr = iptr.wrapping_add(1);
        }

        (*info_ptr).free_me |= PNG_FREE_ROWS;

        iptr = 0;
        while iptr < (*info_ptr).height {
            *(*info_ptr).row_pointers.add(iptr as usize) =
                png_malloc(png_ptr, (*info_ptr).rowbytes) as png_bytep;
            iptr = iptr.wrapping_add(1);
        }
    }

    png_read_image(png_ptr, (*info_ptr).row_pointers);
    (*info_ptr).valid |= PNG_INFO_IDAT;

    /* Read rest of file, and get additional chunks in info_ptr - REQUIRED */
    png_read_end(png_ptr, info_ptr);

    let _ = params;
}
