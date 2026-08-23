use crate::*;

/* Performs intrapixel differencing  */
unsafe fn png_do_write_intrapixel(row_info: png_row_infop, row: png_bytep) {
    if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        let mut bytes_per_pixel: c_int;
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
                *rp = (*rp as c_int - *rp.offset(1) as c_int) as png_byte;
                *rp.offset(2) = (*rp.offset(2) as c_int - *rp.offset(1) as c_int) as png_byte;

                i += 1;
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
                    (((*rp as c_int) << 8) as png_uint_32) | *rp.offset(1) as png_uint_32;
                let s1: png_uint_32 =
                    (((*rp.offset(2) as c_int) << 8) as png_uint_32) | *rp.offset(3) as png_uint_32;
                let s2: png_uint_32 =
                    (((*rp.offset(4) as c_int) << 8) as png_uint_32) | *rp.offset(5) as png_uint_32;
                let red: png_uint_32 = s0.wrapping_sub(s1) & 0xffff;
                let blue: png_uint_32 = s2.wrapping_sub(s1) & 0xffff;
                *rp = (red >> 8) as png_byte;
                *rp.offset(1) = red as png_byte;
                *rp.offset(4) = (blue >> 8) as png_byte;
                *rp.offset(5) = blue as png_byte;

                i += 1;
                rp = rp.offset(bytes_per_pixel as isize);
            }
        }
    }
}

/* Called by user to write a row of image data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_row(png_ptr: png_structrp, row: png_const_bytep) {
    /* 1.5.6: moved from png_struct to be a local structure: */
    let mut row_info: png_row_info = core::mem::zeroed();

    if png_ptr.is_null() {
        return;
    }

    /* Initialize transformations and other stuff if first time */
    if (*png_ptr).row_number == 0 && (*png_ptr).pass == 0 {
        /* Make sure we wrote the header info */
        if ((*png_ptr).mode & PNG_WROTE_INFO_BEFORE_PLTE) == 0 {
            png_error(
                png_ptr as png_const_structrp,
                cstr!("png_write_info was never called before png_write_row"),
            );
        }

        /* Check for transforms that have been set but were defined out */

        png_write_start_row(png_ptr);
    }

    /* If interlaced and not interested in row, return */
    if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
        match (*png_ptr).pass {
            0 => {
                if ((*png_ptr).row_number & 0x07) != 0 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }

            1 => {
                if ((*png_ptr).row_number & 0x07) != 0 || (*png_ptr).width < 5 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }

            2 => {
                if ((*png_ptr).row_number & 0x07) != 4 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }

            3 => {
                if ((*png_ptr).row_number & 0x03) != 0 || (*png_ptr).width < 3 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }

            4 => {
                if ((*png_ptr).row_number & 0x03) != 2 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }

            5 => {
                if ((*png_ptr).row_number & 0x01) != 0 || (*png_ptr).width < 2 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }

            6 => {
                if ((*png_ptr).row_number & 0x01) == 0 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }

            _ => {
                /* error: ignore it */
            }
        }
    }

    /* Set up row info for transformations */
    row_info.color_type = (*png_ptr).color_type;
    row_info.width = (*png_ptr).usr_width;
    row_info.channels = (*png_ptr).usr_channels;
    row_info.bit_depth = (*png_ptr).usr_bit_depth;
    row_info.pixel_depth = (row_info.bit_depth as c_int * row_info.channels as c_int) as png_byte;
    row_info.rowbytes = PNG_ROWBYTES(row_info.pixel_depth as usize, row_info.width as usize);

    /* Copy user's row into buffer, leaving room for filter byte. */
    memcpy(
        (*png_ptr).row_buf.offset(1) as *mut c_void,
        row as *const c_void,
        row_info.rowbytes,
    );

    /* Handle interlacing */
    if (*png_ptr).interlaced != 0
        && (*png_ptr).pass < 6
        && ((*png_ptr).transformations & PNG_INTERLACE) != 0
    {
        png_do_write_interlace(
            &mut row_info,
            (*png_ptr).row_buf.offset(1),
            (*png_ptr).pass as c_int,
        );
        /* This should always get caught above, but still ... */
        if row_info.width == 0 {
            png_write_finish_row(png_ptr);
            return;
        }
    }

    /* Handle other transformations */
    if (*png_ptr).transformations != 0 {
        png_do_write_transformations(png_ptr, &mut row_info);
    }

    /* At this point the row_info pixel depth must match the 'transformed' depth,
     * which is also the output depth.
     */
    if row_info.pixel_depth != (*png_ptr).pixel_depth
        || row_info.pixel_depth != (*png_ptr).transformed_pixel_depth
    {
        png_error(
            png_ptr as png_const_structrp,
            cstr!("internal write transform logic error"),
        );
    }

    /* Write filter_method 64 (intrapixel differencing) only if
     * 1. Libpng was compiled with PNG_MNG_FEATURES_SUPPORTED and
     * 2. Libpng did not write a PNG signature (this filter_method is only
     *    used in PNG datastreams that are embedded in MNG datastreams) and
     * 3. The application called png_permit_mng_features with a mask that
     *    included PNG_FLAG_MNG_FILTER_64 and
     * 4. The filter_method is 64 and
     * 5. The color_type is RGB or RGBA
     */
    if ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
        && ((*png_ptr).filter_type as c_int == PNG_INTRAPIXEL_DIFFERENCING)
    {
        /* Intrapixel differencing */
        png_do_write_intrapixel(&mut row_info, (*png_ptr).row_buf.offset(1));
    }

    /* Added at libpng-1.5.10 */

    /* Check for out-of-range palette index */
    if row_info.color_type as c_int == PNG_COLOR_TYPE_PALETTE && (*png_ptr).num_palette_max >= 0 {
        png_do_check_palette_indexes(png_ptr, &mut row_info);
    }

    /* Find a filter if necessary, filter the row and write it out. */
    png_write_find_filter(png_ptr, &mut row_info);

    if (*png_ptr).write_row_fn.is_some() {
        ((*png_ptr).write_row_fn.unwrap())(
            png_ptr as png_structp,
            (*png_ptr).row_number,
            (*png_ptr).pass as c_int,
        );
    }
}

/* Set the automatic flush interval or 0 to turn flushing off */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_flush(png_ptr: png_structrp, nrows: c_int) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).flush_dist = if nrows < 0 { 0 } else { nrows as png_uint_32 };
}

/* Flush the current output buffers now */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_flush(png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }

    /* We have already written out all of the data */
    if (*png_ptr).row_number >= (*png_ptr).num_rows {
        return;
    }

    png_compress_IDAT(png_ptr, core::ptr::null(), 0, Z_SYNC_FLUSH);
    (*png_ptr).flush_rows = 0;
    png_flush(png_ptr);
}

/* Free any memory used in png_ptr struct without freeing the struct itself. */
unsafe fn png_write_destroy(png_ptr: png_structrp) {
    /* Free any memory zlib uses */
    if ((*png_ptr).flags & PNG_FLAG_ZSTREAM_INITIALIZED) != 0 {
        deflateEnd(&mut (*png_ptr).zstream);
    }

    /* Free our memory.  png_free checks NULL for us. */
    png_free_buffer_list(png_ptr, &mut (*png_ptr).zbuffer_list);
    png_free(png_ptr as png_const_structrp, (*png_ptr).row_buf as png_voidp);
    (*png_ptr).row_buf = core::ptr::null_mut();

    png_free(
        png_ptr as png_const_structrp,
        (*png_ptr).prev_row as png_voidp,
    );
    png_free(png_ptr as png_const_structrp, (*png_ptr).try_row as png_voidp);
    png_free(png_ptr as png_const_structrp, (*png_ptr).tst_row as png_voidp);
    (*png_ptr).prev_row = core::ptr::null_mut();
    (*png_ptr).try_row = core::ptr::null_mut();
    (*png_ptr).tst_row = core::ptr::null_mut();

    png_free(
        png_ptr as png_const_structrp,
        (*png_ptr).chunk_list as png_voidp,
    );
    (*png_ptr).chunk_list = core::ptr::null_mut();

    /* Free the independent copy of trans_alpha owned by png_struct. */
    png_free(
        png_ptr as png_const_structrp,
        (*png_ptr).trans_alpha as png_voidp,
    );
    (*png_ptr).trans_alpha = core::ptr::null_mut();

    /* Free the independent copy of the palette owned by png_struct. */
    png_free(
        png_ptr as png_const_structrp,
        (*png_ptr).palette as png_voidp,
    );
    (*png_ptr).palette = core::ptr::null_mut();

    /* The error handling and memory handling information is left intact at this
     * point: the jmp_buf may still have to be freed.  See png_destroy_png_struct
     * for how this happens.
     */
}

/* Free all memory used by the write.
 * In libpng 1.6.0 this API changed quietly to no longer accept a NULL value for
 * *png_ptr_ptr.  Prior to 1.6.0 it would accept such a value and it would free
 * the passed in info_structs but it would quietly fail to free any of the data
 * inside them.  In 1.6.0 it quietly does nothing (it has to be quiet because it
 * has no png_ptr.)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_write_struct(
    png_ptr_ptr: png_structpp,
    info_ptr_ptr: png_infopp,
) {
    if !png_ptr_ptr.is_null() {
        let png_ptr: png_structrp = *png_ptr_ptr;

        if !png_ptr.is_null()
        /* added in libpng 1.6.0 */
        {
            png_destroy_info_struct(png_ptr as png_const_structrp, info_ptr_ptr);

            *png_ptr_ptr = core::ptr::null_mut();
            png_write_destroy(png_ptr);
            png_destroy_png_struct(png_ptr);
        }
    }
}

/* Allow the application to select one or more row filters to use. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filter(png_ptr: png_structrp, method: c_int, filters: c_int) {
    let mut method = method;
    let mut filters = filters;

    if png_ptr.is_null() {
        return;
    }

    if ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
        && (method == PNG_INTRAPIXEL_DIFFERENCING)
    {
        method = PNG_FILTER_TYPE_BASE;
    }

    if method == PNG_FILTER_TYPE_BASE {
        match filters & (PNG_ALL_FILTERS | 0x07) {
            5 | 6 | 7 => {
                png_app_error(
                    png_ptr as png_const_structrp,
                    cstr!("Unknown row filter for method 0"),
                );

                /* FALLTHROUGH */
                (*png_ptr).do_filter = PNG_FILTER_NONE as png_byte;
            }

            PNG_FILTER_VALUE_NONE => {
                (*png_ptr).do_filter = PNG_FILTER_NONE as png_byte;
            }

            PNG_FILTER_VALUE_SUB => {
                (*png_ptr).do_filter = PNG_FILTER_SUB as png_byte;
            }

            PNG_FILTER_VALUE_UP => {
                (*png_ptr).do_filter = PNG_FILTER_UP as png_byte;
            }

            PNG_FILTER_VALUE_AVG => {
                (*png_ptr).do_filter = PNG_FILTER_AVG as png_byte;
            }

            PNG_FILTER_VALUE_PAETH => {
                (*png_ptr).do_filter = PNG_FILTER_PAETH as png_byte;
            }

            _ => {
                (*png_ptr).do_filter = filters as png_byte;
            }
        }

        /* If we have allocated the row_buf, this means we have already started
         * with the image and we should have allocated all of the filter buffers
         * that have been selected.  If prev_row isn't already allocated, then
         * it is too late to start using the filters that need it, since we
         * will be missing the data in the previous row.  If an application
         * wants to start and stop using particular filters during compression,
         * it should start out with all of the filters, and then remove them
         * or add them back after the start of compression.
         *
         * NOTE: this is a nasty constraint on the code, because it means that the
         * prev_row buffer must be maintained even if there are currently no
         * 'prev_row' requiring filters active.
         */
        if !(*png_ptr).row_buf.is_null() {
            let mut num_filters: c_int;
            let buf_size: png_alloc_size_t;

            /* Repeat the checks in png_write_start_row; 1 pixel high or wide
             * images cannot benefit from certain filters.  If this isn't done here
             * the check below will fire on 1 pixel high images.
             */
            if (*png_ptr).height == 1 {
                filters &= !(PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH);
            }

            if (*png_ptr).width == 1 {
                filters &= !(PNG_FILTER_SUB | PNG_FILTER_AVG | PNG_FILTER_PAETH);
            }

            if (filters & (PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH)) != 0
                && (*png_ptr).prev_row.is_null()
            {
                /* This is the error case, however it is benign - the previous row
                 * is not available so the filter can't be used.  Just warn here.
                 */
                png_app_warning(
                    png_ptr as png_const_structrp,
                    cstr!("png_set_filter: UP/AVG/PAETH cannot be added after start"),
                );
                filters &= !(PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH);
            }

            num_filters = 0;

            if (filters & PNG_FILTER_SUB) != 0 {
                num_filters += 1;
            }

            if (filters & PNG_FILTER_UP) != 0 {
                num_filters += 1;
            }

            if (filters & PNG_FILTER_AVG) != 0 {
                num_filters += 1;
            }

            if (filters & PNG_FILTER_PAETH) != 0 {
                num_filters += 1;
            }

            /* Allocate needed row buffers if they have not already been
             * allocated.
             */
            buf_size = PNG_ROWBYTES(
                ((*png_ptr).usr_channels as c_int * (*png_ptr).usr_bit_depth as c_int) as usize,
                (*png_ptr).width as usize,
            ) + 1;

            if (*png_ptr).try_row.is_null() {
                (*png_ptr).try_row =
                    png_malloc(png_ptr as png_const_structrp, buf_size) as png_bytep;
            }

            if num_filters > 1 {
                if (*png_ptr).tst_row.is_null() {
                    (*png_ptr).tst_row =
                        png_malloc(png_ptr as png_const_structrp, buf_size) as png_bytep;
                }
            }
        }
        (*png_ptr).do_filter = filters as png_byte;
    } else {
        png_error(
            png_ptr as png_const_structrp,
            cstr!("Unknown custom filter method"),
        );
    }
}

/* Provide floating and fixed point APIs */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filter_heuristics(
    png_ptr: png_structrp,
    heuristic_method: c_int,
    num_weights: c_int,
    filter_weights: png_const_doublep,
    filter_costs: png_const_doublep,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filter_heuristics_fixed(
    png_ptr: png_structrp,
    heuristic_method: c_int,
    num_weights: c_int,
    filter_weights: png_const_fixed_point_p,
    filter_costs: png_const_fixed_point_p,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_level(png_ptr: png_structrp, level: c_int) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).zlib_level = level;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_mem_level(png_ptr: png_structrp, mem_level: c_int) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).zlib_mem_level = mem_level;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_strategy(png_ptr: png_structrp, strategy: c_int) {
    if png_ptr.is_null() {
        return;
    }

    /* The flag setting here prevents the libpng dynamic selection of strategy.
     */
    (*png_ptr).flags |= PNG_FLAG_ZLIB_CUSTOM_STRATEGY;
    (*png_ptr).zlib_strategy = strategy;
}

/* If PNG_WRITE_OPTIMIZE_CMF_SUPPORTED is defined, libpng will use a
 * smaller value of window_bits if it can do so safely.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_window_bits(
    png_ptr: png_structrp,
    window_bits: c_int,
) {
    let mut window_bits = window_bits;

    if png_ptr.is_null() {
        return;
    }

    /* Prior to 1.6.0 this would warn but then set the window_bits value. This
     * meant that negative window bits values could be selected that would cause
     * libpng to write a non-standard PNG file with raw deflate or gzip
     * compressed IDAT or ancillary chunks.  Such files can be read and there is
     * no warning on read, so this seems like a very bad idea.
     */
    if window_bits > 15 {
        png_warning(
            png_ptr as png_const_structrp,
            cstr!("Only compression windows <= 32k supported by PNG"),
        );
        window_bits = 15;
    } else if window_bits < 8 {
        png_warning(
            png_ptr as png_const_structrp,
            cstr!("Only compression windows >= 256 supported by PNG"),
        );
        window_bits = 8;
    }

    (*png_ptr).zlib_window_bits = window_bits;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_method(png_ptr: png_structrp, method: c_int) {
    if png_ptr.is_null() {
        return;
    }

    /* This would produce an invalid PNG file if it worked, but it doesn't and
     * deflate will fault it, so it is harmless to just warn here.
     */
    if method != 8 {
        png_warning(
            png_ptr as png_const_structrp,
            cstr!("Only compression method 8 is supported by PNG"),
        );
    }

    (*png_ptr).zlib_method = method;
}

/* The following were added to libpng-1.5.4 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_level(png_ptr: png_structrp, level: c_int) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).zlib_text_level = level;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_mem_level(
    png_ptr: png_structrp,
    mem_level: c_int,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).zlib_text_mem_level = mem_level;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_strategy(png_ptr: png_structrp, strategy: c_int) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).zlib_text_strategy = strategy;
}

/* If PNG_WRITE_OPTIMIZE_CMF_SUPPORTED is defined, libpng will use a
 * smaller value of window_bits if it can do so safely.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_window_bits(
    png_ptr: png_structrp,
    window_bits: c_int,
) {
    let mut window_bits = window_bits;

    if png_ptr.is_null() {
        return;
    }

    if window_bits > 15 {
        png_warning(
            png_ptr as png_const_structrp,
            cstr!("Only compression windows <= 32k supported by PNG"),
        );
        window_bits = 15;
    } else if window_bits < 8 {
        png_warning(
            png_ptr as png_const_structrp,
            cstr!("Only compression windows >= 256 supported by PNG"),
        );
        window_bits = 8;
    }

    (*png_ptr).zlib_text_window_bits = window_bits;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_method(png_ptr: png_structrp, method: c_int) {
    if png_ptr.is_null() {
        return;
    }

    if method != 8 {
        png_warning(
            png_ptr as png_const_structrp,
            cstr!("Only compression method 8 is supported by PNG"),
        );
    }

    (*png_ptr).zlib_text_method = method;
}

/* end of API added to libpng-1.5.4 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_write_status_fn(
    png_ptr: png_structrp,
    write_row_fn: png_write_status_ptr,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).write_row_fn = write_row_fn;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_write_user_transform_fn(
    png_ptr: png_structrp,
    write_user_transform_fn: png_user_transform_ptr,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).transformations |= PNG_USER_TRANSFORM;
    (*png_ptr).write_user_transform_fn = write_user_transform_fn;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_png(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    transforms: c_int,
    params: png_voidp,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    if ((*info_ptr).valid & PNG_INFO_IDAT) == 0 {
        png_app_error(
            png_ptr as png_const_structrp,
            cstr!("no rows for png_write_image to write"),
        );
        return;
    }

    /* Write the file header information. */
    png_write_info(png_ptr, info_ptr as png_const_inforp);

    /* ------ these transformations don't touch the info structure ------- */

    /* Invert monochrome pixels */
    if (transforms & PNG_TRANSFORM_INVERT_MONO) != 0 {
        png_set_invert_mono(png_ptr);
    }

    /* Shift the pixels up to a legal bit depth and fill in
     * as appropriate to correctly scale the image.
     */
    if (transforms & PNG_TRANSFORM_SHIFT) != 0 {
        if ((*info_ptr).valid & PNG_INFO_sBIT) != 0 {
            png_set_shift(
                png_ptr,
                core::ptr::addr_of!((*info_ptr).sig_bit) as png_const_color_8p,
            );
        }
    }

    /* Pack pixels into bytes */
    if (transforms & PNG_TRANSFORM_PACKING) != 0 {
        png_set_packing(png_ptr);
    }

    /* Swap location of alpha bytes from ARGB to RGBA */
    if (transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0 {
        png_set_swap_alpha(png_ptr);
    }

    /* Remove a filler (X) from XRGB/RGBX/AG/GA into to convert it into
     * RGB, note that the code expects the input color type to be G or RGB; no
     * alpha channel.
     */
    if (transforms & (PNG_TRANSFORM_STRIP_FILLER_AFTER | PNG_TRANSFORM_STRIP_FILLER_BEFORE)) != 0 {
        if (transforms & PNG_TRANSFORM_STRIP_FILLER_AFTER) != 0 {
            if (transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE) != 0 {
                png_app_error(
                    png_ptr as png_const_structrp,
                    cstr!("PNG_TRANSFORM_STRIP_FILLER: BEFORE+AFTER not supported"),
                );
            }

            /* Continue if ignored - this is the pre-1.6.10 behavior */
            png_set_filler(png_ptr, 0, PNG_FILLER_AFTER);
        } else if (transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE) != 0 {
            png_set_filler(png_ptr, 0, PNG_FILLER_BEFORE);
        }
    }

    /* Flip BGR pixels to RGB */
    if (transforms & PNG_TRANSFORM_BGR) != 0 {
        png_set_bgr(png_ptr);
    }

    /* Swap bytes of 16-bit files to most significant byte first */
    if (transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0 {
        png_set_swap(png_ptr);
    }

    /* Swap bits of 1-bit, 2-bit, 4-bit packed pixel formats */
    if (transforms & PNG_TRANSFORM_PACKSWAP) != 0 {
        png_set_packswap(png_ptr);
    }

    /* Invert the alpha channel from opacity to transparency */
    if (transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0 {
        png_set_invert_alpha(png_ptr);
    }

    /* ----------------------- end of transformations ------------------- */

    /* Write the bits */
    png_write_image(png_ptr, (*info_ptr).row_pointers);

    /* It is REQUIRED to call this to finish writing the rest of the file */
    png_write_end(png_ptr, info_ptr);
}
