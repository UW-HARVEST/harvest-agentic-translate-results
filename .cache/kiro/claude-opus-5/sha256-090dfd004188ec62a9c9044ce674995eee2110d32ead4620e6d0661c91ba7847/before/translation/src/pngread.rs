//! Translation of c_src/src/pngread.c lines 1..1127
//!
//! Contains: png_create_read_struct, png_create_read_struct_2, png_read_info,
//! png_read_update_info, png_start_read_image, png_do_read_intrapixel (static),
//! png_read_row, png_read_rows, png_read_image, png_read_end,
//! png_read_destroy (static), png_destroy_read_struct, png_set_read_status_fn,
//! png_read_png.
use crate::prelude::*;

/* Create a PNG structure for reading, and allocate any memory needed.
 * (PNG_USER_MEM_SUPPORTED is defined, so this delegates to _2.)
 */
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

    if png_ptr != core::ptr::null_mut() {
        (*png_ptr).mode = PNG_IS_READ_STRUCT;

        /* Added in libpng-1.6.0; this can be used to detect a read structure if
         * required (it will be zero in a write structure.)
         */
        (*png_ptr).IDAT_read_size = PNG_IDAT_READ_SIZE as uInt;

        (*png_ptr).flags |= PNG_FLAG_BENIGN_ERRORS_WARN;

        /* In stable builds only warn if an application error can be completely
         * handled.  PNG_RELEASE_BUILD == 0 for this build, so skipped.
         */
        if PNG_RELEASE_BUILD {
            (*png_ptr).flags |= PNG_FLAG_APP_WARNINGS_WARN;
        }

        png_set_read_fn(png_ptr, core::ptr::null_mut(), None);
    }

    png_ptr
}

/* Read the information before the actual image data. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    let mut keep: c_int;

    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
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
                png_chunk_error(png_ptr, cstr(b"Missing IHDR before IDAT\0"));
            } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
                && ((*png_ptr).mode & PNG_HAVE_PLTE) == 0
            {
                png_chunk_error(png_ptr, cstr(b"Missing PLTE before IDAT\0"));
            } else if ((*png_ptr).mode & PNG_AFTER_IDAT) != 0 {
                png_chunk_benign_error(png_ptr, cstr(b"Too many IDATs found\0"));
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
pub unsafe extern "C" fn png_read_update_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    if png_ptr != core::ptr::null_mut() {
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
            png_read_start_row(png_ptr);

            png_read_transform_info(png_ptr, info_ptr);
        }
        /* New in 1.6.0 this avoids the bug of doing the initializations twice */
        else {
            png_app_error(
                png_ptr,
                cstr(b"png_read_update_info/png_start_read_image: duplicate call\0"),
            );
        }
    }
}

/* Initialize palette, background, etc, after transformations
 * are set, but before any reading takes place.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_start_read_image(png_ptr: png_structrp) {
    if png_ptr != core::ptr::null_mut() {
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
            png_read_start_row(png_ptr);
        }
        /* New in 1.6.0 this avoids the bug of doing the initializations twice */
        else {
            png_app_error(
                png_ptr,
                cstr(b"png_start_read_image/png_read_update_info: duplicate call\0"),
            );
        }
    }
}

/* Undoes intrapixel differencing,
 * NOTE: this is apparently only supported in the 'sequential' reader.
 */
pub unsafe extern "C" fn png_do_read_intrapixel(row_info: png_row_infop, row: png_bytep) {
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
                *rp = ((256 + *rp as c_int + *rp.add(1) as c_int) & 0xff) as png_byte;
                *rp.add(2) = ((256 + *rp.add(2) as c_int + *rp.add(1) as c_int) & 0xff) as png_byte;

                i += 1;
                rp = rp.add(bytes_per_pixel as usize);
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
                    ((*rp as c_int) << 8) as png_uint_32 | *rp.add(1) as png_uint_32;
                let s1: png_uint_32 =
                    ((*rp.add(2) as c_int) << 8) as png_uint_32 | *rp.add(3) as png_uint_32;
                let s2: png_uint_32 =
                    ((*rp.add(4) as c_int) << 8) as png_uint_32 | *rp.add(5) as png_uint_32;
                let red: png_uint_32 = (s0.wrapping_add(s1).wrapping_add(65536)) & 0xffff;
                let blue: png_uint_32 = (s2.wrapping_add(s1).wrapping_add(65536)) & 0xffff;
                *rp = ((red >> 8) & 0xff) as png_byte;
                *rp.add(1) = (red & 0xff) as png_byte;
                *rp.add(4) = ((blue >> 8) & 0xff) as png_byte;
                *rp.add(5) = (blue & 0xff) as png_byte;

                i += 1;
                rp = rp.add(bytes_per_pixel as usize);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_row(png_ptr: png_structrp, row: png_bytep, dsp_row: png_bytep) {
    let mut row_info: png_row_info = png_row_info::default();

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

    if (*png_ptr).row_number == 0 && (*png_ptr).pass == 0 {
        /* Check for transforms that have been set but were defined out.
         * All of these READ transforms are supported in this build, so the
         * corresponding WRITE-only warning blocks are compiled out.
         */
    }

    /* If interlaced and we do not need a new row, combine row and return. */
    if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
        match (*png_ptr).pass {
            0 => {
                if (*png_ptr).row_number & 0x07 != 0 {
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

            _ => {
                /* default and case 6 */
                if ((*png_ptr).row_number & 1) == 0 {
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
        }
    }

    if ((*png_ptr).mode & PNG_HAVE_IDAT) == 0 {
        png_error(png_ptr, cstr(b"Invalid attempt to read row data\0"));
    }

    /* Fill the row with IDAT data: */
    *(*png_ptr).row_buf.add(0) = 255; /* to force error if no data was found */
    png_read_IDAT_data(png_ptr, (*png_ptr).row_buf, row_info.rowbytes + 1);

    if *(*png_ptr).row_buf.add(0) as c_int > PNG_FILTER_VALUE_NONE {
        if (*(*png_ptr).row_buf.add(0) as c_int) < PNG_FILTER_VALUE_LAST {
            png_read_filter_row(
                png_ptr,
                &mut row_info,
                (*png_ptr).row_buf.add(1),
                (*png_ptr).prev_row.add(1),
                *(*png_ptr).row_buf.add(0) as c_int,
            );
        } else {
            png_error(png_ptr, cstr(b"bad adaptive filter value\0"));
        }
    }

    /* libpng 1.5.6: copy the interlaced count. */
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
            png_error(png_ptr, cstr(b"sequential row overflow\0"));
        }
    } else if (*png_ptr).transformed_pixel_depth != row_info.pixel_depth {
        png_error(
            png_ptr,
            cstr(b"internal sequential row size calculation error\0"),
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

    if let Some(f) = (*png_ptr).read_row_fn {
        f(
            png_ptr as png_structp,
            (*png_ptr).row_number,
            (*png_ptr).pass as c_int,
        );
    }
}

/* Read one or more rows of image data. */
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
            i += 1;
        }
    } else if rp != core::ptr::null_mut() {
        i = 0;
        while i < num_rows {
            let rptr: png_bytep = *rp;
            png_read_row(png_ptr, rptr, core::ptr::null_mut());
            rp = rp.add(1);
            i += 1;
        }
    } else if dp != core::ptr::null_mut() {
        i = 0;
        while i < num_rows {
            let dptr: png_bytep = *dp;
            png_read_row(png_ptr, core::ptr::null_mut(), dptr);
            dp = dp.add(1);
            i += 1;
        }
    }
}

/* Read the entire image. */
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
        if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) == 0 {
            /* Caller called png_start_read_image or png_read_update_info without
             * first turning on the PNG_INTERLACE transform.
             */
            png_warning(
                png_ptr,
                cstr(b"Interlace handling should be turned on when using png_read_image\0"),
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
            i += 1;
        }
        j += 1;
    }
}

/* Read the end of the PNG file. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_end(png_ptr: png_structrp, info_ptr: png_inforp) {
    let mut keep: c_int;

    if png_ptr == core::ptr::null_mut() {
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
        png_benign_error(png_ptr, cstr(b"Read palette index exceeding num_palette\0"));
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
        } else if info_ptr == core::ptr::null_mut() {
            png_crc_finish(png_ptr, length);
        } else if {
            keep = png_chunk_unknown_handling(png_ptr, chunk_name);
            keep != 0
        } {
            if chunk_name == png_IDAT {
                if (length > 0 && ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0)
                    || ((*png_ptr).mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0
                {
                    png_benign_error(png_ptr, cstr(b".Too many IDATs found\0"));
                }
            }
            png_handle_unknown(png_ptr, info_ptr, length, keep);
            if chunk_name == png_PLTE {
                (*png_ptr).mode |= PNG_HAVE_PLTE;
            }
        } else if chunk_name == png_IDAT {
            /* Zero length IDATs are legal after the last IDAT has been read. */
            if (length > 0 && ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0)
                || ((*png_ptr).mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0
            {
                png_benign_error(png_ptr, cstr(b"..Too many IDATs found\0"));
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
pub unsafe extern "C" fn png_read_destroy(png_ptr: png_structrp) {
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

    /* png_ptr->palette is always independently allocated, so free it
     * unconditionally.
     */
    png_free(png_ptr, (*png_ptr).palette as png_voidp);
    (*png_ptr).palette = core::ptr::null_mut();

    /* png_ptr->trans_alpha is always independently allocated, so free it
     * unconditionally.
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
     * callbacks are still set at this point.
     */
}

/* Free all memory used by the read */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_read_struct(
    png_ptr_ptr: png_structpp,
    info_ptr_ptr: png_infopp,
    end_info_ptr_ptr: png_infopp,
) {
    let mut png_ptr: png_structrp = core::ptr::null_mut();

    if png_ptr_ptr != core::ptr::null_mut() {
        png_ptr = *png_ptr_ptr;
    }

    if png_ptr == core::ptr::null_mut() {
        return;
    }

    /* libpng 1.6.0: use the API to destroy info structs to ensure consistent
     * behavior.
     */
    png_destroy_info_struct(png_ptr, end_info_ptr_ptr);
    png_destroy_info_struct(png_ptr, info_ptr_ptr);

    *png_ptr_ptr = core::ptr::null_mut();
    png_read_destroy(png_ptr);
    png_destroy_png_struct(png_ptr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_read_status_fn(
    png_ptr: png_structrp,
    read_row_fn: png_read_status_ptr,
) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).read_row_fn = read_row_fn;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_png(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    transforms: c_int,
    params: png_voidp,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    /* png_read_info() gives us all of the information from the
     * PNG file before the first IDAT (image data chunk).
     */
    png_read_info(png_ptr, info_ptr);
    if (*info_ptr).height as png_uint_32
        > PNG_UINT_32_MAX / (core::mem::size_of::<png_bytep>() as png_uint_32)
    {
        png_error(
            png_ptr,
            cstr(b"Image is too high to process with png_read_png()\0"),
        );
    }

    /* -------------- image transformations start here ------------------- */

    /* Tell libpng to strip 16-bit/color files down to 8 bits per color. */
    if (transforms & PNG_TRANSFORM_SCALE_16) != 0 {
        png_set_scale_16(png_ptr);
    }

    /* If both SCALE and STRIP are required pngrtran will effectively cancel the
     * latter by doing SCALE first.
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
     * byte into separate bytes.
     */
    if (transforms & PNG_TRANSFORM_PACKING) != 0 {
        png_set_packing(png_ptr);
    }

    /* Change the order of packed pixels to least significant bit first. */
    if (transforms & PNG_TRANSFORM_PACKSWAP) != 0 {
        png_set_packswap(png_ptr);
    }

    /* Expand paletted colors into true RGB triplets, expand grayscale, etc. */
    if (transforms & PNG_TRANSFORM_EXPAND) != 0 {
        png_set_expand(png_ptr);
    }

    /* We don't handle background color or gamma transformation or quantizing. */

    /* Invert monochrome files to have 0 as white and 1 as black */
    if (transforms & PNG_TRANSFORM_INVERT_MONO) != 0 {
        png_set_invert_mono(png_ptr);
    }

    /* If you want to shift the pixel values from the range [0,255] or
     * [0,65535] to the original [0,7] or [0,31], etc.
     */
    if (transforms & PNG_TRANSFORM_SHIFT) != 0 {
        if ((*info_ptr).valid & PNG_INFO_sBIT) != 0 {
            png_set_shift(png_ptr, &mut (*info_ptr).sig_bit);
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
    let _ = png_set_interlace_handling(png_ptr);

    /* Optional call to gamma correct and add the background to the palette
     * and update info structure.
     */
    png_read_update_info(png_ptr, info_ptr);

    /* -------------- image transformations end here ------------------- */

    png_free_data(png_ptr, info_ptr, PNG_FREE_ROWS, 0);
    if (*info_ptr).row_pointers == core::ptr::null_mut() {
        let mut iptr: png_uint_32;

        (*info_ptr).row_pointers = png_malloc(
            png_ptr,
            (*info_ptr).height as png_alloc_size_t
                * (core::mem::size_of::<png_bytep>() as png_alloc_size_t),
        ) as png_bytepp;

        iptr = 0;
        while iptr < (*info_ptr).height {
            *(*info_ptr).row_pointers.add(iptr as usize) = core::ptr::null_mut();
            iptr += 1;
        }

        (*info_ptr).free_me |= PNG_FREE_ROWS;

        iptr = 0;
        while iptr < (*info_ptr).height {
            *(*info_ptr).row_pointers.add(iptr as usize) =
                png_malloc(png_ptr, (*info_ptr).rowbytes) as png_bytep;
            iptr += 1;
        }
    }

    png_read_image(png_ptr, (*info_ptr).row_pointers);
    (*info_ptr).valid |= PNG_INFO_IDAT;

    /* Read rest of file, and get additional chunks in info_ptr - REQUIRED */
    png_read_end(png_ptr, info_ptr);

    let _ = params;
}
