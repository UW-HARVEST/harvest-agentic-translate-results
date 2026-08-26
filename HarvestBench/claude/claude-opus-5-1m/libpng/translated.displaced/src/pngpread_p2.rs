// pngpread.c - read a png file in push mode
//
// Chunk 2: png_process_IDAT_data .. png_get_progressive_ptr

use crate::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_process_IDAT_data(
    png_ptr: png_structrp,
    buffer: png_bytep,
    buffer_length: usize,
) {
    /* The caller checks for a non-zero buffer length. */
    if !(buffer_length > 0) || buffer.is_null() {
        png_error(png_ptr, cstr!("No IDAT data (internal error)"));
    }

    /* This routine must process all the data it has been given
     * before returning, calling the row callback as required to
     * handle the uncompressed results.
     */
    (*png_ptr).zstream.next_in = buffer as *const Bytef;
    /* TODO: WARNING: TRUNCATION ERROR: DANGER WILL ROBINSON: */
    (*png_ptr).zstream.avail_in = buffer_length as uInt;

    /* Keep going until the decompressed data is all processed
     * or the stream marked as finished.
     */
    while (*png_ptr).zstream.avail_in > 0 && ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0 {
        let ret: c_int;

        /* We have data for zlib, but we must check that zlib
         * has someplace to put the results.  It doesn't matter
         * if we don't expect any results -- it may be the input
         * data is just the LZ end code.
         */
        if !((*png_ptr).zstream.avail_out > 0) {
            /* TODO: WARNING: TRUNCATION ERROR: DANGER WILL ROBINSON: */
            (*png_ptr).zstream.avail_out = (PNG_ROWBYTES(
                (*png_ptr).pixel_depth as usize,
                (*png_ptr).iwidth as usize,
            ) + 1) as uInt;

            (*png_ptr).zstream.next_out = (*png_ptr).row_buf;
        }

        /* Using Z_SYNC_FLUSH here means that an unterminated
         * LZ stream (a stream with a missing end code) can still
         * be handled, otherwise (Z_NO_FLUSH) a future zlib
         * implementation might defer output and therefore
         * change the current behavior (see comments in inflate.c
         * for why this doesn't happen at present with zlib 1.2.5).
         */
        ret = png_zlib_inflate(png_ptr, Z_SYNC_FLUSH);

        /* Check for any failure before proceeding. */
        if ret != Z_OK && ret != Z_STREAM_END {
            /* Terminate the decompression. */
            (*png_ptr).flags |= PNG_FLAG_ZSTREAM_ENDED;
            (*png_ptr).zowner = 0;

            /* This may be a truncated stream (missing or
             * damaged end code).  Treat that as a warning.
             */
            if (*png_ptr).row_number >= (*png_ptr).num_rows || (*png_ptr).pass as c_int > 6 {
                png_warning(png_ptr, cstr!("Truncated compressed data in IDAT"));
            } else {
                if ret == Z_DATA_ERROR {
                    png_benign_error(png_ptr, cstr!("IDAT: ADLER32 checksum mismatch"));
                } else {
                    png_error(png_ptr, cstr!("Decompression error in IDAT"));
                }
            }

            /* Skip the check on unprocessed input */
            return;
        }

        /* Did inflate output any data? */
        if (*png_ptr).zstream.next_out != (*png_ptr).row_buf {
            /* Is this unexpected data after the last row?
             * If it is, artificially terminate the LZ output
             * here.
             */
            if (*png_ptr).row_number >= (*png_ptr).num_rows || (*png_ptr).pass as c_int > 6 {
                /* Extra data. */
                png_warning(png_ptr, cstr!("Extra compressed data in IDAT"));
                (*png_ptr).flags |= PNG_FLAG_ZSTREAM_ENDED;
                (*png_ptr).zowner = 0;

                /* Do no more processing; skip the unprocessed
                 * input check below.
                 */
                return;
            }

            /* Do we have a complete row? */
            if (*png_ptr).zstream.avail_out == 0 {
                png_push_process_row(png_ptr);
            }
        }

        /* And check for the end of the stream. */
        if ret == Z_STREAM_END {
            (*png_ptr).flags |= PNG_FLAG_ZSTREAM_ENDED;
        }
    }

    /* All the data should have been processed, if anything
     * is left at this point we have bytes of IDAT data
     * after the zlib end code.
     */
    if (*png_ptr).zstream.avail_in > 0 {
        png_warning(png_ptr, cstr!("Extra compression data in IDAT"));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_process_row(png_ptr: png_structrp) {
    /* 1.5.6: row_info moved out of png_struct to a local here. */
    let mut row_info: png_row_info = core::mem::zeroed();

    row_info.width = (*png_ptr).iwidth; /* NOTE: width of current interlaced row */
    row_info.color_type = (*png_ptr).color_type;
    row_info.bit_depth = (*png_ptr).bit_depth;
    row_info.channels = (*png_ptr).channels;
    row_info.pixel_depth = (*png_ptr).pixel_depth;
    row_info.rowbytes = PNG_ROWBYTES(row_info.pixel_depth as usize, row_info.width as usize);

    if (*(*png_ptr).row_buf) as c_int > PNG_FILTER_VALUE_NONE {
        if ((*(*png_ptr).row_buf) as c_int) < PNG_FILTER_VALUE_LAST {
            png_read_filter_row(
                png_ptr,
                &mut row_info,
                (*png_ptr).row_buf.add(1),
                (*png_ptr).prev_row.add(1) as png_const_bytep,
                (*(*png_ptr).row_buf) as c_int,
            );
        } else {
            png_error(png_ptr, cstr!("bad adaptive filter value"));
        }
    }

    /* libpng 1.5.6: the following line was copying png_ptr->rowbytes before
     * 1.5.6, while the buffer really is this big in current versions of libpng
     * it may not be in the future, so this was changed just to copy the
     * interlaced row count:
     */
    memcpy(
        (*png_ptr).prev_row as *mut c_void,
        (*png_ptr).row_buf as *const c_void,
        row_info.rowbytes + 1,
    );

    if (*png_ptr).transformations != 0 {
        png_do_read_transformations(png_ptr, &mut row_info);
    }

    /* The transformed pixel depth should match the depth now in row_info. */
    if (*png_ptr).transformed_pixel_depth == 0 {
        (*png_ptr).transformed_pixel_depth = row_info.pixel_depth;
        if row_info.pixel_depth > (*png_ptr).maximum_pixel_depth {
            png_error(png_ptr, cstr!("progressive row overflow"));
        }
    } else if (*png_ptr).transformed_pixel_depth != row_info.pixel_depth {
        png_error(
            png_ptr,
            cstr!("internal progressive row size calculation error"),
        );
    }

    /* Expand interlaced rows to full size */
    if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
        if ((*png_ptr).pass as c_int) < 6 {
            png_do_read_interlace(
                &mut row_info,
                (*png_ptr).row_buf.add(1),
                (*png_ptr).pass as c_int,
                (*png_ptr).transformations,
            );
        }

        match (*png_ptr).pass as c_int {
            0 => {
                let mut i: c_int;

                i = 0;
                while i < 8 && (*png_ptr).pass as c_int == 0 {
                    png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                    png_read_push_finish_row(png_ptr); /* Updates png_ptr->pass */
                    i += 1;
                }

                if (*png_ptr).pass as c_int == 2 {
                    /* Pass 1 might be empty */
                    i = 0;
                    while i < 4 && (*png_ptr).pass as c_int == 2 {
                        png_push_have_row(png_ptr, core::ptr::null_mut());
                        png_read_push_finish_row(png_ptr);
                        i += 1;
                    }
                }

                if (*png_ptr).pass as c_int == 4 && (*png_ptr).height <= 4 {
                    i = 0;
                    while i < 2 && (*png_ptr).pass as c_int == 4 {
                        png_push_have_row(png_ptr, core::ptr::null_mut());
                        png_read_push_finish_row(png_ptr);
                        i += 1;
                    }
                }

                if (*png_ptr).pass as c_int == 6 && (*png_ptr).height <= 4 {
                    png_push_have_row(png_ptr, core::ptr::null_mut());
                    png_read_push_finish_row(png_ptr);
                }
            }

            1 => {
                let mut i: c_int;

                i = 0;
                while i < 8 && (*png_ptr).pass as c_int == 1 {
                    png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                if (*png_ptr).pass as c_int == 2 {
                    /* Skip top 4 generated rows */
                    i = 0;
                    while i < 4 && (*png_ptr).pass as c_int == 2 {
                        png_push_have_row(png_ptr, core::ptr::null_mut());
                        png_read_push_finish_row(png_ptr);
                        i += 1;
                    }
                }
            }

            2 => {
                let mut i: c_int;

                i = 0;
                while i < 4 && (*png_ptr).pass as c_int == 2 {
                    png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                i = 0;
                while i < 4 && (*png_ptr).pass as c_int == 2 {
                    png_push_have_row(png_ptr, core::ptr::null_mut());
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                if (*png_ptr).pass as c_int == 4 {
                    /* Pass 3 might be empty */
                    i = 0;
                    while i < 2 && (*png_ptr).pass as c_int == 4 {
                        png_push_have_row(png_ptr, core::ptr::null_mut());
                        png_read_push_finish_row(png_ptr);
                        i += 1;
                    }
                }
            }

            3 => {
                let mut i: c_int;

                i = 0;
                while i < 4 && (*png_ptr).pass as c_int == 3 {
                    png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                if (*png_ptr).pass as c_int == 4 {
                    /* Skip top two generated rows */
                    i = 0;
                    while i < 2 && (*png_ptr).pass as c_int == 4 {
                        png_push_have_row(png_ptr, core::ptr::null_mut());
                        png_read_push_finish_row(png_ptr);
                        i += 1;
                    }
                }
            }

            4 => {
                let mut i: c_int;

                i = 0;
                while i < 2 && (*png_ptr).pass as c_int == 4 {
                    png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                i = 0;
                while i < 2 && (*png_ptr).pass as c_int == 4 {
                    png_push_have_row(png_ptr, core::ptr::null_mut());
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                if (*png_ptr).pass as c_int == 6 {
                    /* Pass 5 might be empty */
                    png_push_have_row(png_ptr, core::ptr::null_mut());
                    png_read_push_finish_row(png_ptr);
                }
            }

            5 => {
                let mut i: c_int;

                i = 0;
                while i < 2 && (*png_ptr).pass as c_int == 5 {
                    png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                if (*png_ptr).pass as c_int == 6 {
                    /* Skip top generated row */
                    png_push_have_row(png_ptr, core::ptr::null_mut());
                    png_read_push_finish_row(png_ptr);
                }
            }

            /* default: case 6: */
            _ => {
                png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                png_read_push_finish_row(png_ptr);

                /* C: if (png_ptr->pass != 6) break; */
                if (*png_ptr).pass as c_int == 6 {
                    png_push_have_row(png_ptr, core::ptr::null_mut());
                    png_read_push_finish_row(png_ptr);
                }
            }
        }
    } else {
        png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
        png_read_push_finish_row(png_ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_push_finish_row(png_ptr: png_structrp) {
    (*png_ptr).row_number = (*png_ptr).row_number.wrapping_add(1);
    if (*png_ptr).row_number < (*png_ptr).num_rows {
        return;
    }

    if (*png_ptr).interlaced != 0 {
        (*png_ptr).row_number = 0;
        memset(
            (*png_ptr).prev_row as *mut c_void,
            0,
            (*png_ptr).rowbytes + 1,
        );

        loop {
            (*png_ptr).pass = (*png_ptr).pass.wrapping_add(1);
            if ((*png_ptr).pass as c_int == 1 && (*png_ptr).width < 5)
                || ((*png_ptr).pass as c_int == 3 && (*png_ptr).width < 3)
                || ((*png_ptr).pass as c_int == 5 && (*png_ptr).width < 2)
            {
                (*png_ptr).pass = (*png_ptr).pass.wrapping_add(1);
            }

            if (*png_ptr).pass as c_int > 7 {
                (*png_ptr).pass = (*png_ptr).pass.wrapping_sub(1);
            }

            if (*png_ptr).pass as c_int >= 7 {
                break;
            }

            (*png_ptr).iwidth = ((*png_ptr)
                .width
                .wrapping_add(png_pass_inc[(*png_ptr).pass as usize] as png_uint_32)
                .wrapping_sub(1)
                .wrapping_sub(png_pass_start[(*png_ptr).pass as usize] as png_uint_32))
                / (png_pass_inc[(*png_ptr).pass as usize] as png_uint_32);

            if ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
                break;
            }

            (*png_ptr).num_rows = ((*png_ptr)
                .height
                .wrapping_add(png_pass_yinc[(*png_ptr).pass as usize] as png_uint_32)
                .wrapping_sub(1)
                .wrapping_sub(png_pass_ystart[(*png_ptr).pass as usize] as png_uint_32))
                / (png_pass_yinc[(*png_ptr).pass as usize] as png_uint_32);

            if !((*png_ptr).iwidth == 0 || (*png_ptr).num_rows == 0) {
                break;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_have_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    if (*png_ptr).info_fn.is_some() {
        ((*png_ptr).info_fn.unwrap())(png_ptr, info_ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_have_end(png_ptr: png_structrp, info_ptr: png_inforp) {
    if (*png_ptr).end_fn.is_some() {
        ((*png_ptr).end_fn.unwrap())(png_ptr, info_ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_have_row(png_ptr: png_structrp, row: png_bytep) {
    if (*png_ptr).row_fn.is_some() {
        ((*png_ptr).row_fn.unwrap())(
            png_ptr,
            row,
            (*png_ptr).row_number,
            (*png_ptr).pass as c_int,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_progressive_combine_row(
    png_ptr: png_const_structrp,
    old_row: png_bytep,
    new_row: png_const_bytep,
) {
    if png_ptr.is_null() {
        return;
    }

    /* new_row is a flag here - if it is NULL then the app callback was called
     * from an empty row (see the calls to png_struct::row_fn below), otherwise
     * it must be png_ptr->row_buf+1
     */
    if !new_row.is_null() {
        png_combine_row(png_ptr, old_row, 1 /*blocky display*/);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_progressive_read_fn(
    png_ptr: png_structrp,
    progressive_ptr: png_voidp,
    info_fn: png_progressive_info_ptr,
    row_fn: png_progressive_row_ptr,
    end_fn: png_progressive_end_ptr,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).info_fn = info_fn;
    (*png_ptr).row_fn = row_fn;
    (*png_ptr).end_fn = end_fn;

    png_set_read_fn(png_ptr, progressive_ptr, Some(png_push_fill_buffer));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_progressive_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr.is_null() {
        return core::ptr::null_mut();
    }

    (*png_ptr).io_ptr
}
