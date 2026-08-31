//! pngpread.c lines 1-945: read a png file in push (progressive) mode.
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

/* Push model modes */
pub const PNG_READ_SIG_MODE: c_int = 0;
pub const PNG_READ_CHUNK_MODE: c_int = 1;
pub const PNG_READ_IDAT_MODE: c_int = 2;
pub const PNG_READ_tEXt_MODE: c_int = 4;
pub const PNG_READ_zTXt_MODE: c_int = 5;
pub const PNG_READ_DONE_MODE: c_int = 6;
pub const PNG_READ_iTXt_MODE: c_int = 7;
pub const PNG_ERROR_MODE: c_int = 8;

/* Arrays to facilitate interlacing - use pass (0 - 6) as index.
 *
 * png_pass_start / png_pass_inc / png_pass_ystart / png_pass_yinc are the
 * same arrays that pngrutil.c declares; they live in `crate::shared` so that
 * the duplicate C file-scope statics do not have to be repeated here.
 *
 * TODO: Move these arrays to a common utility module to avoid duplication.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_process_data(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    buffer: png_bytep,
    buffer_size: usize,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    png_push_restore_buffer(png_ptr, buffer, buffer_size);

    while (*png_ptr).buffer_size != 0 {
        png_process_some_data(png_ptr, info_ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_process_data_pause(
    png_ptr: png_structrp,
    save: c_int,
) -> usize {
    if !png_ptr.is_null() {
        /* It's easiest for the caller if we do the save; then the caller doesn't
         * have to supply the same data again:
         */
        if save != 0 {
            png_push_save_buffer(png_ptr);
        } else {
            /* This includes any pending saved bytes: */
            let remaining: usize = (*png_ptr).buffer_size;
            (*png_ptr).buffer_size = 0;

            /* So subtract the saved buffer size, unless all the data
             * is actually 'saved', in which case we just return 0
             */
            if (*png_ptr).save_buffer_size < remaining {
                return remaining - (*png_ptr).save_buffer_size;
            }
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_process_data_skip(png_ptr: png_structrp) -> png_uint_32 {
    /* TODO: Deprecate and remove this API.
     * Somewhere the implementation of this seems to have been lost,
     * or abandoned.  It was only to support some internal back-door access
     * to png_struct) in libpng-1.4.x.
     */
    png_app_warning(
        png_ptr,
        c"png_process_data_skip is not implemented in any current version of libpng".as_ptr(),
    );
    0
}

/* What we do with the incoming data depends on what we were previously
 * doing before we ran out of data...
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_process_some_data(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
) {
    if png_ptr.is_null() {
        return;
    }

    match (*png_ptr).process_mode {
        PNG_READ_SIG_MODE => {
            png_push_read_sig(png_ptr, info_ptr);
        }

        PNG_READ_CHUNK_MODE => {
            png_push_read_chunk(png_ptr, info_ptr);
        }

        PNG_READ_IDAT_MODE => {
            png_push_read_IDAT(png_ptr);
        }

        _ => {
            (*png_ptr).buffer_size = 0;
        }
    }
}

/* Read any remaining signature bytes from the stream and compare them with
 * the correct PNG signature.  It is possible that this routine is called
 * with bytes already read from the signature, either because they have been
 * checked by the calling application, or because of multiple calls to this
 * routine.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_push_read_sig(png_ptr: png_structrp, info_ptr: png_inforp) {
    let num_checked: usize = (*png_ptr).sig_bytes as usize; /* SAFE, does not exceed 8 */
    let mut num_to_check: usize = 8 - num_checked;

    if (*png_ptr).buffer_size < num_to_check {
        num_to_check = (*png_ptr).buffer_size;
    }

    let signature: png_bytep = core::ptr::addr_of_mut!((*info_ptr).signature) as png_bytep;

    png_push_fill_buffer(png_ptr, signature.add(num_checked), num_to_check);
    (*png_ptr).sig_bytes = ((*png_ptr).sig_bytes as usize).wrapping_add(num_to_check) as png_byte;

    if png_sig_cmp(signature, num_checked, num_to_check) != 0 {
        if num_checked < 4
            && png_sig_cmp(signature, num_checked, num_to_check.wrapping_sub(4)) != 0
        {
            png_error(png_ptr, c"Not a PNG file".as_ptr());
        } else {
            png_error(png_ptr, c"PNG file corrupted by ASCII conversion".as_ptr());
        }
    } else {
        if (*png_ptr).sig_bytes >= 8 {
            (*png_ptr).process_mode = PNG_READ_CHUNK_MODE;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_push_read_chunk(png_ptr: png_structrp, info_ptr: png_inforp) {
    let chunk_name: png_uint_32;
    let keep: c_int; /* unknown handling method */

    /* First we make sure we have enough data for the 4-byte chunk name
     * and the 4-byte chunk length before proceeding with decoding the
     * chunk data.  To fully decode each of these chunks, we also make
     * sure we have enough data in the buffer for the 4-byte CRC at the
     * end of every chunk (except IDAT, which is handled separately).
     */
    if ((*png_ptr).mode & PNG_HAVE_CHUNK_HEADER) == 0 {
        /* PNG_PUSH_SAVE_BUFFER_IF_LT(8) */
        if (*png_ptr).buffer_size < 8 {
            png_push_save_buffer(png_ptr);
            return;
        }
        (*png_ptr).push_length = png_read_chunk_header(png_ptr);
        (*png_ptr).mode |= PNG_HAVE_CHUNK_HEADER;
    }

    chunk_name = (*png_ptr).chunk_name;

    if chunk_name == png_IDAT {
        if ((*png_ptr).mode & PNG_AFTER_IDAT) != 0 {
            (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT;
        }

        /* If we reach an IDAT chunk, this means we have read all of the
         * header chunks, and we can start reading the image (or if this
         * is called after the image has been read - we have an error).
         */
        if ((*png_ptr).mode & PNG_HAVE_IHDR) == 0 {
            png_error(png_ptr, c"Missing IHDR before IDAT".as_ptr());
        } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
            && ((*png_ptr).mode & PNG_HAVE_PLTE) == 0
        {
            png_error(png_ptr, c"Missing PLTE before IDAT".as_ptr());
        }

        (*png_ptr).process_mode = PNG_READ_IDAT_MODE;

        if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
            if ((*png_ptr).mode & PNG_HAVE_CHUNK_AFTER_IDAT) == 0 {
                if (*png_ptr).push_length == 0 {
                    return;
                }
            }
        }

        (*png_ptr).mode |= PNG_HAVE_IDAT;

        if ((*png_ptr).mode & PNG_AFTER_IDAT) != 0 {
            png_benign_error(png_ptr, c"Too many IDATs found".as_ptr());
        }
    } else if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
        /* These flags must be set consistently for all non-IDAT chunks,
         * including the unknown chunks.
         */
        (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT | PNG_AFTER_IDAT;
    }

    if chunk_name == png_IHDR {
        if (*png_ptr).push_length != 13 {
            png_error(png_ptr, c"Invalid IHDR length".as_ptr());
        }

        /* PNG_PUSH_SAVE_BUFFER_IF_FULL */
        if (*png_ptr).push_length.wrapping_add(4) as usize > (*png_ptr).buffer_size {
            png_push_save_buffer(png_ptr);
            return;
        }
        png_handle_chunk(png_ptr, info_ptr, (*png_ptr).push_length);
    } else if chunk_name == png_IEND {
        /* PNG_PUSH_SAVE_BUFFER_IF_FULL */
        if (*png_ptr).push_length.wrapping_add(4) as usize > (*png_ptr).buffer_size {
            png_push_save_buffer(png_ptr);
            return;
        }
        png_handle_chunk(png_ptr, info_ptr, (*png_ptr).push_length);

        (*png_ptr).process_mode = PNG_READ_DONE_MODE;
        png_push_have_end(png_ptr, info_ptr);
    } else {
        /* else if ((keep = png_chunk_unknown_handling(png_ptr, chunk_name)) != 0) */
        keep = png_chunk_unknown_handling(png_ptr, chunk_name);

        if keep != 0 {
            /* PNG_PUSH_SAVE_BUFFER_IF_FULL */
            if (*png_ptr).push_length.wrapping_add(4) as usize > (*png_ptr).buffer_size {
                png_push_save_buffer(png_ptr);
                return;
            }
            png_handle_unknown(png_ptr, info_ptr, (*png_ptr).push_length, keep);

            if chunk_name == png_PLTE {
                (*png_ptr).mode |= PNG_HAVE_PLTE;
            }
        } else if chunk_name == png_IDAT {
            (*png_ptr).idat_size = (*png_ptr).push_length;
            (*png_ptr).process_mode = PNG_READ_IDAT_MODE;
            png_push_have_info(png_ptr, info_ptr);
            (*png_ptr).zstream.avail_out =
                (PNG_ROWBYTES((*png_ptr).pixel_depth as u32, (*png_ptr).iwidth) + 1) as uInt;
            (*png_ptr).zstream.next_out = (*png_ptr).row_buf;
            return;
        } else {
            /* PNG_PUSH_SAVE_BUFFER_IF_FULL */
            if (*png_ptr).push_length.wrapping_add(4) as usize > (*png_ptr).buffer_size {
                png_push_save_buffer(png_ptr);
                return;
            }
            png_handle_chunk(png_ptr, info_ptr, (*png_ptr).push_length);
        }
    }

    (*png_ptr).mode &= !PNG_HAVE_CHUNK_HEADER;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_push_fill_buffer(
    png_ptr: png_structp,
    buffer: png_bytep,
    mut length: usize,
) {
    let mut ptr: png_bytep;

    if png_ptr.is_null() {
        return;
    }

    ptr = buffer;
    if (*png_ptr).save_buffer_size != 0 {
        let save_size: usize;

        if length < (*png_ptr).save_buffer_size {
            save_size = length;
        } else {
            save_size = (*png_ptr).save_buffer_size;
        }

        memcpy(ptr, (*png_ptr).save_buffer_ptr, save_size);
        length -= save_size;
        ptr = ptr.add(save_size);
        (*png_ptr).buffer_size -= save_size;
        (*png_ptr).save_buffer_size -= save_size;
        (*png_ptr).save_buffer_ptr = (*png_ptr).save_buffer_ptr.add(save_size);
    }
    if length != 0 && (*png_ptr).current_buffer_size != 0 {
        let save_size: usize;

        if length < (*png_ptr).current_buffer_size {
            save_size = length;
        } else {
            save_size = (*png_ptr).current_buffer_size;
        }

        memcpy(ptr, (*png_ptr).current_buffer_ptr, save_size);
        (*png_ptr).buffer_size -= save_size;
        (*png_ptr).current_buffer_size -= save_size;
        (*png_ptr).current_buffer_ptr = (*png_ptr).current_buffer_ptr.add(save_size);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_push_save_buffer(png_ptr: png_structrp) {
    if (*png_ptr).save_buffer_size != 0 {
        if (*png_ptr).save_buffer_ptr != (*png_ptr).save_buffer {
            let mut i: usize;
            let istop: usize;
            let mut sp: png_bytep;
            let mut dp: png_bytep;

            istop = (*png_ptr).save_buffer_size;
            i = 0;
            sp = (*png_ptr).save_buffer_ptr;
            dp = (*png_ptr).save_buffer;
            while i < istop {
                *dp = *sp;

                i += 1;
                sp = sp.add(1);
                dp = dp.add(1);
            }
        }
    }
    if (*png_ptr).save_buffer_size + (*png_ptr).current_buffer_size > (*png_ptr).save_buffer_max {
        let new_max: usize;
        let old_buffer: png_bytep;

        if (*png_ptr).save_buffer_size
            > PNG_SIZE_MAX.wrapping_sub((*png_ptr).current_buffer_size.wrapping_add(256))
        {
            png_error(png_ptr, c"Potential overflow of save_buffer".as_ptr());
        }

        new_max = (*png_ptr).save_buffer_size + (*png_ptr).current_buffer_size + 256;
        old_buffer = (*png_ptr).save_buffer;
        (*png_ptr).save_buffer = png_malloc_warn(png_ptr, new_max as usize) as png_bytep;

        if (*png_ptr).save_buffer.is_null() {
            png_free(png_ptr, old_buffer as png_voidp);
            png_error(png_ptr, c"Insufficient memory for save_buffer".as_ptr());
        }

        if !old_buffer.is_null() {
            memcpy(
                (*png_ptr).save_buffer,
                old_buffer,
                (*png_ptr).save_buffer_size,
            );
        } else if (*png_ptr).save_buffer_size != 0 {
            png_error(png_ptr, c"save_buffer error".as_ptr());
        }
        png_free(png_ptr, old_buffer as png_voidp);
        (*png_ptr).save_buffer_max = new_max;
    }
    if (*png_ptr).current_buffer_size != 0 {
        memcpy(
            (*png_ptr).save_buffer.add((*png_ptr).save_buffer_size),
            (*png_ptr).current_buffer_ptr,
            (*png_ptr).current_buffer_size,
        );
        (*png_ptr).save_buffer_size += (*png_ptr).current_buffer_size;
        (*png_ptr).current_buffer_size = 0;
    }
    (*png_ptr).save_buffer_ptr = (*png_ptr).save_buffer;
    (*png_ptr).buffer_size = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_push_restore_buffer(
    png_ptr: png_structrp,
    buffer: png_bytep,
    buffer_length: usize,
) {
    (*png_ptr).current_buffer = buffer;
    (*png_ptr).current_buffer_size = buffer_length;
    (*png_ptr).buffer_size = buffer_length + (*png_ptr).save_buffer_size;
    (*png_ptr).current_buffer_ptr = (*png_ptr).current_buffer;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_push_read_IDAT(png_ptr: png_structrp) {
    if ((*png_ptr).mode & PNG_HAVE_CHUNK_HEADER) == 0 {
        let mut chunk_length: [png_byte; 4] = [0; 4];
        let mut chunk_tag: [png_byte; 4] = [0; 4];

        /* TODO: this code can be commoned up with the same code in push_read */
        /* PNG_PUSH_SAVE_BUFFER_IF_LT(8) */
        if (*png_ptr).buffer_size < 8 {
            png_push_save_buffer(png_ptr);
            return;
        }
        png_push_fill_buffer(png_ptr, chunk_length.as_mut_ptr(), 4);
        (*png_ptr).push_length = png_get_uint_31(png_ptr, chunk_length.as_ptr());
        png_reset_crc(png_ptr);
        png_crc_read(png_ptr, chunk_tag.as_mut_ptr(), 4);
        (*png_ptr).chunk_name = PNG_CHUNK_FROM_STRING(chunk_tag.as_ptr());
        (*png_ptr).mode |= PNG_HAVE_CHUNK_HEADER;

        if (*png_ptr).chunk_name != png_IDAT {
            (*png_ptr).process_mode = PNG_READ_CHUNK_MODE;

            if ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0 {
                png_error(png_ptr, c"Not enough compressed data".as_ptr());
            }

            return;
        }

        (*png_ptr).idat_size = (*png_ptr).push_length;
    }

    if (*png_ptr).idat_size != 0 && (*png_ptr).save_buffer_size != 0 {
        let mut save_size: usize = (*png_ptr).save_buffer_size;
        let mut idat_size: png_uint_32 = (*png_ptr).idat_size;

        /* We want the smaller of 'idat_size' and 'current_buffer_size', but they
         * are of different types and we don't know which variable has the fewest
         * bits.  Carefully select the smaller and cast it to the type of the
         * larger - this cannot overflow.  Do not cast in the following test - it
         * will break on either 16-bit or 64-bit platforms.
         */
        if (idat_size as usize) < save_size {
            save_size = idat_size as usize;
        } else {
            idat_size = save_size as png_uint_32;
        }

        png_calculate_crc(png_ptr, (*png_ptr).save_buffer_ptr, save_size);

        png_process_IDAT_data(png_ptr, (*png_ptr).save_buffer_ptr, save_size);

        (*png_ptr).idat_size -= idat_size;
        (*png_ptr).buffer_size -= save_size;
        (*png_ptr).save_buffer_size -= save_size;
        (*png_ptr).save_buffer_ptr = (*png_ptr).save_buffer_ptr.add(save_size);
    }

    if (*png_ptr).idat_size != 0 && (*png_ptr).current_buffer_size != 0 {
        let mut save_size: usize = (*png_ptr).current_buffer_size;
        let mut idat_size: png_uint_32 = (*png_ptr).idat_size;

        /* We want the smaller of 'idat_size' and 'current_buffer_size', but they
         * are of different types and we don't know which variable has the fewest
         * bits.  Carefully select the smaller and cast it to the type of the
         * larger - this cannot overflow.
         */
        if (idat_size as usize) < save_size {
            save_size = idat_size as usize;
        } else {
            idat_size = save_size as png_uint_32;
        }

        png_calculate_crc(png_ptr, (*png_ptr).current_buffer_ptr, save_size);

        png_process_IDAT_data(png_ptr, (*png_ptr).current_buffer_ptr, save_size);

        (*png_ptr).idat_size -= idat_size;
        (*png_ptr).buffer_size -= save_size;
        (*png_ptr).current_buffer_size -= save_size;
        (*png_ptr).current_buffer_ptr = (*png_ptr).current_buffer_ptr.add(save_size);
    }

    if (*png_ptr).idat_size == 0 {
        /* PNG_PUSH_SAVE_BUFFER_IF_LT(4) */
        if (*png_ptr).buffer_size < 4 {
            png_push_save_buffer(png_ptr);
            return;
        }
        png_crc_finish(png_ptr, 0);
        (*png_ptr).mode &= !PNG_HAVE_CHUNK_HEADER;
        (*png_ptr).mode |= PNG_AFTER_IDAT;
        (*png_ptr).zowner = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_process_IDAT_data(
    png_ptr: png_structrp,
    buffer: png_bytep,
    buffer_length: usize,
) {
    /* The caller checks for a non-zero buffer length. */
    if !(buffer_length > 0) || buffer.is_null() {
        png_error(png_ptr, c"No IDAT data (internal error)".as_ptr());
    }

    /* This routine must process all the data it has been given
     * before returning, calling the row callback as required to
     * handle the uncompressed results.
     */
    (*png_ptr).zstream.next_in = buffer as *const u8;
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
            (*png_ptr).zstream.avail_out =
                (PNG_ROWBYTES((*png_ptr).pixel_depth as u32, (*png_ptr).iwidth) + 1) as uInt;

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
            if (*png_ptr).row_number >= (*png_ptr).num_rows || (*png_ptr).pass > 6 {
                png_warning(png_ptr, c"Truncated compressed data in IDAT".as_ptr());
            } else {
                if ret == Z_DATA_ERROR {
                    png_benign_error(png_ptr, c"IDAT: ADLER32 checksum mismatch".as_ptr());
                } else {
                    png_error(png_ptr, c"Decompression error in IDAT".as_ptr());
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
            if (*png_ptr).row_number >= (*png_ptr).num_rows || (*png_ptr).pass > 6 {
                /* Extra data. */
                png_warning(png_ptr, c"Extra compressed data in IDAT".as_ptr());
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
        png_warning(png_ptr, c"Extra compression data in IDAT".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_push_process_row(png_ptr: png_structrp) {
    /* 1.5.6: row_info moved out of png_struct to a local here. */
    let mut row_info: png_row_info = png_row_info::default();

    row_info.width = (*png_ptr).iwidth; /* NOTE: width of current interlaced row */
    row_info.color_type = (*png_ptr).color_type;
    row_info.bit_depth = (*png_ptr).bit_depth;
    row_info.channels = (*png_ptr).channels;
    row_info.pixel_depth = (*png_ptr).pixel_depth;
    row_info.rowbytes = PNG_ROWBYTES(row_info.pixel_depth as u32, row_info.width);

    if (*(*png_ptr).row_buf) as c_int > PNG_FILTER_VALUE_NONE {
        if ((*(*png_ptr).row_buf) as c_int) < PNG_FILTER_VALUE_LAST {
            png_read_filter_row(
                png_ptr,
                &mut row_info,
                (*png_ptr).row_buf.add(1),
                (*png_ptr).prev_row.add(1),
                (*(*png_ptr).row_buf) as c_int,
            );
        } else {
            png_error(png_ptr, c"bad adaptive filter value".as_ptr());
        }
    }

    /* libpng 1.5.6: the following line was copying png_ptr->rowbytes before
     * 1.5.6, while the buffer really is this big in current versions of libpng
     * it may not be in the future, so this was changed just to copy the
     * interlaced row count:
     */
    memcpy(
        (*png_ptr).prev_row,
        (*png_ptr).row_buf,
        row_info.rowbytes + 1,
    );

    if (*png_ptr).transformations != 0 {
        png_do_read_transformations(png_ptr, &mut row_info);
    }

    /* The transformed pixel depth should match the depth now in row_info. */
    if (*png_ptr).transformed_pixel_depth == 0 {
        (*png_ptr).transformed_pixel_depth = row_info.pixel_depth;
        if row_info.pixel_depth > (*png_ptr).maximum_pixel_depth {
            png_error(png_ptr, c"progressive row overflow".as_ptr());
        }
    } else if (*png_ptr).transformed_pixel_depth != row_info.pixel_depth {
        png_error(
            png_ptr,
            c"internal progressive row size calculation error".as_ptr(),
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

        match (*png_ptr).pass {
            0 => {
                let mut i: c_int;
                i = 0;
                while i < 8 && (*png_ptr).pass == 0 {
                    png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                    png_read_push_finish_row(png_ptr); /* Updates png_ptr->pass */
                    i += 1;
                }

                if (*png_ptr).pass == 2 {
                    /* Pass 1 might be empty */
                    i = 0;
                    while i < 4 && (*png_ptr).pass == 2 {
                        png_push_have_row(png_ptr, core::ptr::null_mut());
                        png_read_push_finish_row(png_ptr);
                        i += 1;
                    }
                }

                if (*png_ptr).pass == 4 && (*png_ptr).height <= 4 {
                    i = 0;
                    while i < 2 && (*png_ptr).pass == 4 {
                        png_push_have_row(png_ptr, core::ptr::null_mut());
                        png_read_push_finish_row(png_ptr);
                        i += 1;
                    }
                }

                if (*png_ptr).pass == 6 && (*png_ptr).height <= 4 {
                    png_push_have_row(png_ptr, core::ptr::null_mut());
                    png_read_push_finish_row(png_ptr);
                }
            }

            1 => {
                let mut i: c_int;
                i = 0;
                while i < 8 && (*png_ptr).pass == 1 {
                    png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                if (*png_ptr).pass == 2 {
                    /* Skip top 4 generated rows */
                    i = 0;
                    while i < 4 && (*png_ptr).pass == 2 {
                        png_push_have_row(png_ptr, core::ptr::null_mut());
                        png_read_push_finish_row(png_ptr);
                        i += 1;
                    }
                }
            }

            2 => {
                let mut i: c_int;

                i = 0;
                while i < 4 && (*png_ptr).pass == 2 {
                    png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                i = 0;
                while i < 4 && (*png_ptr).pass == 2 {
                    png_push_have_row(png_ptr, core::ptr::null_mut());
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                if (*png_ptr).pass == 4 {
                    /* Pass 3 might be empty */
                    i = 0;
                    while i < 2 && (*png_ptr).pass == 4 {
                        png_push_have_row(png_ptr, core::ptr::null_mut());
                        png_read_push_finish_row(png_ptr);
                        i += 1;
                    }
                }
            }

            3 => {
                let mut i: c_int;

                i = 0;
                while i < 4 && (*png_ptr).pass == 3 {
                    png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                if (*png_ptr).pass == 4 {
                    /* Skip top two generated rows */
                    i = 0;
                    while i < 2 && (*png_ptr).pass == 4 {
                        png_push_have_row(png_ptr, core::ptr::null_mut());
                        png_read_push_finish_row(png_ptr);
                        i += 1;
                    }
                }
            }

            4 => {
                let mut i: c_int;

                i = 0;
                while i < 2 && (*png_ptr).pass == 4 {
                    png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                i = 0;
                while i < 2 && (*png_ptr).pass == 4 {
                    png_push_have_row(png_ptr, core::ptr::null_mut());
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                if (*png_ptr).pass == 6 {
                    /* Pass 5 might be empty */
                    png_push_have_row(png_ptr, core::ptr::null_mut());
                    png_read_push_finish_row(png_ptr);
                }
            }

            5 => {
                let mut i: c_int;

                i = 0;
                while i < 2 && (*png_ptr).pass == 5 {
                    png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }

                if (*png_ptr).pass == 6 {
                    /* Skip top generated row */
                    png_push_have_row(png_ptr, core::ptr::null_mut());
                    png_read_push_finish_row(png_ptr);
                }
            }

            /* default: case 6: */
            _ => 'case6: {
                png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
                png_read_push_finish_row(png_ptr);

                if (*png_ptr).pass != 6 {
                    break 'case6;
                }

                png_push_have_row(png_ptr, core::ptr::null_mut());
                png_read_push_finish_row(png_ptr);
            }
        }
    } else {
        png_push_have_row(png_ptr, (*png_ptr).row_buf.add(1));
        png_read_push_finish_row(png_ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_push_finish_row(png_ptr: png_structrp) {
    (*png_ptr).row_number = (*png_ptr).row_number.wrapping_add(1);
    if (*png_ptr).row_number < (*png_ptr).num_rows {
        return;
    }

    if (*png_ptr).interlaced != 0 {
        (*png_ptr).row_number = 0;
        memset((*png_ptr).prev_row, 0, (*png_ptr).rowbytes + 1);

        loop {
            (*png_ptr).pass = (*png_ptr).pass.wrapping_add(1);
            if ((*png_ptr).pass == 1 && (*png_ptr).width < 5)
                || ((*png_ptr).pass == 3 && (*png_ptr).width < 3)
                || ((*png_ptr).pass == 5 && (*png_ptr).width < 2)
            {
                (*png_ptr).pass = (*png_ptr).pass.wrapping_add(1);
            }

            if (*png_ptr).pass > 7 {
                (*png_ptr).pass = (*png_ptr).pass.wrapping_sub(1);
            }

            if (*png_ptr).pass >= 7 {
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
pub unsafe extern "C-unwind" fn png_push_have_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    if (*png_ptr).info_fn.is_some() {
        ((*png_ptr).info_fn.unwrap())(png_ptr, info_ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_push_have_end(png_ptr: png_structrp, info_ptr: png_inforp) {
    if (*png_ptr).end_fn.is_some() {
        ((*png_ptr).end_fn.unwrap())(png_ptr, info_ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_push_have_row(png_ptr: png_structrp, row: png_bytep) {
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
pub unsafe extern "C-unwind" fn png_progressive_combine_row(
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
pub unsafe extern "C-unwind" fn png_set_progressive_read_fn(
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
pub unsafe extern "C-unwind" fn png_get_progressive_ptr(
    png_ptr: png_const_structrp,
) -> png_voidp {
    if png_ptr.is_null() {
        return core::ptr::null_mut();
    }

    (*png_ptr).io_ptr
}
