/* pngpread.c lines 1..496 */

/* png_process_data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_process_data(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    buffer: png_bytep,
    buffer_size: usize,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    png_push_restore_buffer(png_ptr, buffer, buffer_size);

    while (*png_ptr).buffer_size != 0 {
        png_process_some_data(png_ptr, info_ptr);
    }
}

/* png_process_data_pause */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_process_data_pause(png_ptr: png_structrp, save: c_int) -> usize {
    if png_ptr != core::ptr::null_mut() {
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

/* png_process_data_skip */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_process_data_skip(png_ptr: png_structrp) -> png_uint_32 {
    /* TODO: Deprecate and remove this API.
     * Somewhere the implementation of this seems to have been lost,
     * or abandoned.  It was only to support some internal back-door access
     * to png_struct) in libpng-1.4.x.
     */
    png_app_warning(
        png_ptr,
        b"png_process_data_skip is not implemented in any current version of libpng\0".as_ptr()
            as png_const_charp,
    );
    0
}

/* What we do with the incoming data depends on what we were previously
 * doing before we ran out of data...
 */
/* png_process_some_data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_process_some_data(png_ptr: png_structrp, info_ptr: png_inforp) {
    if png_ptr == core::ptr::null_mut() {
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
/* png_push_read_sig */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_read_sig(png_ptr: png_structrp, info_ptr: png_inforp) {
    let num_checked: usize = (*png_ptr).sig_bytes as usize; /* SAFE, does not exceed 8 */
    let mut num_to_check: usize = 8 - num_checked;

    if (*png_ptr).buffer_size < num_to_check {
        num_to_check = (*png_ptr).buffer_size;
    }

    png_push_fill_buffer(
        png_ptr,
        (*info_ptr).signature.as_mut_ptr().add(num_checked),
        num_to_check,
    );
    (*png_ptr).sig_bytes = ((*png_ptr).sig_bytes as usize).wrapping_add(num_to_check) as png_byte;

    if png_sig_cmp((*info_ptr).signature.as_ptr(), num_checked, num_to_check) != 0 {
        if num_checked < 4
            && png_sig_cmp(
                (*info_ptr).signature.as_ptr(),
                num_checked,
                num_to_check.wrapping_sub(4),
            ) != 0
        {
            png_error(png_ptr, b"Not a PNG file\0".as_ptr() as png_const_charp);
        } else {
            png_error(
                png_ptr,
                b"PNG file corrupted by ASCII conversion\0".as_ptr() as png_const_charp,
            );
        }
    } else {
        if (*png_ptr).sig_bytes >= 8 {
            (*png_ptr).process_mode = PNG_READ_CHUNK_MODE;
        }
    }
}

/* png_push_read_chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_read_chunk(png_ptr: png_structrp, info_ptr: png_inforp) {
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
            png_error(
                png_ptr,
                b"Missing IHDR before IDAT\0".as_ptr() as png_const_charp,
            );
        } else if (*png_ptr).color_type == PNG_COLOR_TYPE_PALETTE as png_byte
            && ((*png_ptr).mode & PNG_HAVE_PLTE) == 0
        {
            png_error(
                png_ptr,
                b"Missing PLTE before IDAT\0".as_ptr() as png_const_charp,
            );
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
            png_benign_error(
                png_ptr,
                b"Too many IDATs found\0".as_ptr() as png_const_charp,
            );
        }
    } else if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
        /* These flags must be set consistently for all non-IDAT chunks,
         * including the unknown chunks.
         */
        (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT | PNG_AFTER_IDAT;
    }

    if chunk_name == png_IHDR {
        if (*png_ptr).push_length != 13 {
            png_error(
                png_ptr,
                b"Invalid IHDR length\0".as_ptr() as png_const_charp,
            );
        }

        /* PNG_PUSH_SAVE_BUFFER_IF_FULL */
        if ((*png_ptr).push_length as usize + 4) > (*png_ptr).buffer_size {
            png_push_save_buffer(png_ptr);
            return;
        }
        png_handle_chunk(png_ptr, info_ptr, (*png_ptr).push_length);
    } else if chunk_name == png_IEND {
        /* PNG_PUSH_SAVE_BUFFER_IF_FULL */
        if ((*png_ptr).push_length as usize + 4) > (*png_ptr).buffer_size {
            png_push_save_buffer(png_ptr);
            return;
        }
        png_handle_chunk(png_ptr, info_ptr, (*png_ptr).push_length);

        (*png_ptr).process_mode = PNG_READ_DONE_MODE;
        png_push_have_end(png_ptr, info_ptr);
    } else if {
        keep = png_chunk_unknown_handling(png_ptr, chunk_name);
        keep != 0
    } {
        /* PNG_PUSH_SAVE_BUFFER_IF_FULL */
        if ((*png_ptr).push_length as usize + 4) > (*png_ptr).buffer_size {
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
        (*png_ptr).zstream.avail_out = (PNG_ROWBYTES(
            (*png_ptr).pixel_depth as usize,
            (*png_ptr).iwidth as usize,
        ) as uInt)
            .wrapping_add(1);
        (*png_ptr).zstream.next_out = (*png_ptr).row_buf;
        return;
    } else {
        /* PNG_PUSH_SAVE_BUFFER_IF_FULL */
        if ((*png_ptr).push_length as usize + 4) > (*png_ptr).buffer_size {
            png_push_save_buffer(png_ptr);
            return;
        }
        png_handle_chunk(png_ptr, info_ptr, (*png_ptr).push_length);
    }

    (*png_ptr).mode &= !PNG_HAVE_CHUNK_HEADER;
}

/* png_push_fill_buffer */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_fill_buffer(
    png_ptr: png_structp,
    buffer: png_bytep,
    mut length: usize,
) {
    let mut ptr: png_bytep;

    if png_ptr == core::ptr::null_mut() {
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

        memcpy(
            ptr as *mut c_void,
            (*png_ptr).save_buffer_ptr as *const c_void,
            save_size,
        );
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

        memcpy(
            ptr as *mut c_void,
            (*png_ptr).current_buffer_ptr as *const c_void,
            save_size,
        );
        (*png_ptr).buffer_size -= save_size;
        (*png_ptr).current_buffer_size -= save_size;
        (*png_ptr).current_buffer_ptr = (*png_ptr).current_buffer_ptr.add(save_size);
    }
}

/* png_push_save_buffer */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_save_buffer(png_ptr: png_structrp) {
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
            png_error(
                png_ptr,
                b"Potential overflow of save_buffer\0".as_ptr() as png_const_charp,
            );
        }

        new_max = (*png_ptr).save_buffer_size + (*png_ptr).current_buffer_size + 256;
        old_buffer = (*png_ptr).save_buffer;
        (*png_ptr).save_buffer =
            png_malloc_warn(png_ptr, new_max as png_alloc_size_t) as png_bytep;

        if (*png_ptr).save_buffer == core::ptr::null_mut() {
            png_free(png_ptr, old_buffer as png_voidp);
            png_error(
                png_ptr,
                b"Insufficient memory for save_buffer\0".as_ptr() as png_const_charp,
            );
        }

        if !old_buffer.is_null() {
            memcpy(
                (*png_ptr).save_buffer as *mut c_void,
                old_buffer as *const c_void,
                (*png_ptr).save_buffer_size,
            );
        } else if (*png_ptr).save_buffer_size != 0 {
            png_error(png_ptr, b"save_buffer error\0".as_ptr() as png_const_charp);
        }
        png_free(png_ptr, old_buffer as png_voidp);
        (*png_ptr).save_buffer_max = new_max;
    }
    if (*png_ptr).current_buffer_size != 0 {
        memcpy(
            (*png_ptr).save_buffer.add((*png_ptr).save_buffer_size) as *mut c_void,
            (*png_ptr).current_buffer_ptr as *const c_void,
            (*png_ptr).current_buffer_size,
        );
        (*png_ptr).save_buffer_size += (*png_ptr).current_buffer_size;
        (*png_ptr).current_buffer_size = 0;
    }
    (*png_ptr).save_buffer_ptr = (*png_ptr).save_buffer;
    (*png_ptr).buffer_size = 0;
}

/* png_push_restore_buffer */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_restore_buffer(
    png_ptr: png_structrp,
    buffer: png_bytep,
    buffer_length: usize,
) {
    (*png_ptr).current_buffer = buffer;
    (*png_ptr).current_buffer_size = buffer_length;
    (*png_ptr).buffer_size = buffer_length + (*png_ptr).save_buffer_size;
    (*png_ptr).current_buffer_ptr = (*png_ptr).current_buffer;
}

/* png_push_read_IDAT */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_read_IDAT(png_ptr: png_structrp) {
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
                png_error(
                    png_ptr,
                    b"Not enough compressed data\0".as_ptr() as png_const_charp,
                );
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

        (*png_ptr).idat_size = (*png_ptr).idat_size.wrapping_sub(idat_size);
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

        (*png_ptr).idat_size = (*png_ptr).idat_size.wrapping_sub(idat_size);
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
