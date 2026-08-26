/* pngrutil.c lines 2263..2711 */

/* png_handle_sCAL */
unsafe extern "C" fn png_handle_sCAL(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let buffer: png_bytep;
    let mut i: usize;
    let mut state: c_int;

    buffer = png_read_buffer(png_ptr, length.wrapping_add(1) as png_alloc_size_t);

    if buffer == core::ptr::null_mut() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, b"out of memory\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);
    *buffer.add(length as usize) = 0; /* Null terminate the last string */

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* Validate the unit. */
    if *buffer.add(0) != 1 && *buffer.add(0) != 2 {
        png_chunk_benign_error(png_ptr, b"invalid unit\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    /* Validate the ASCII numbers, need two ASCII numbers separated by
     * a '\0' and they need to fit exactly in the chunk data.
     */
    i = 1;
    state = 0;

    /* if (png_check_fp_number(...) == 0 || i >= length || buffer[i++] != 0) */
    let bad_width: bool;
    if png_check_fp_number(
        buffer as png_const_charp,
        length as usize,
        &mut state,
        &mut i,
    ) == 0
    {
        bad_width = true;
    } else if i >= length as usize {
        bad_width = true;
    } else {
        let c: png_byte = *buffer.add(i);
        i += 1;
        bad_width = c != 0;
    }

    if bad_width {
        png_chunk_benign_error(png_ptr, b"bad width format\0".as_ptr() as png_const_charp);
    } else if !PNG_FP_IS_POSITIVE(state) {
        png_chunk_benign_error(png_ptr, b"non-positive width\0".as_ptr() as png_const_charp);
    } else {
        let heighti: usize = i;

        state = 0;
        if png_check_fp_number(
            buffer as png_const_charp,
            length as usize,
            &mut state,
            &mut i,
        ) == 0
            || i != length as usize
        {
            png_chunk_benign_error(png_ptr, b"bad height format\0".as_ptr() as png_const_charp);
        } else if !PNG_FP_IS_POSITIVE(state) {
            png_chunk_benign_error(png_ptr, b"non-positive height\0".as_ptr() as png_const_charp);
        } else {
            /* This is the (only) success case. */
            png_set_sCAL_s(
                png_ptr,
                info_ptr,
                *buffer.add(0) as c_int,
                (buffer as png_charp).add(1) as png_const_charp,
                (buffer as png_charp).add(heighti) as png_const_charp,
            );
            return handled_ok;
        }
    }

    handled_error
}

/* png_handle_tIME */
unsafe extern "C" fn png_handle_tIME(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buf: [png_byte; 7] = [0; 7];
    let mut mod_time: png_time = Default::default();

    /* TODO: what is this doing here?  It should be happened in pngread.c and
     * pngpread.c, although it could be moved to png_handle_chunk below and
     * thereby avoid some code duplication.
     */
    if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
        (*png_ptr).mode |= PNG_AFTER_IDAT;
    }

    png_crc_read(png_ptr, buf.as_mut_ptr(), 7);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    mod_time.second = buf[6];
    mod_time.minute = buf[5];
    mod_time.hour = buf[4];
    mod_time.day = buf[3];
    mod_time.month = buf[2];
    mod_time.year = PNG_get_uint_16(buf.as_ptr());

    png_set_tIME(png_ptr, info_ptr, &mod_time);
    handled_ok
}

/* Note: this does not properly handle chunks that are > 64K under DOS */
/* png_handle_tEXt */
unsafe extern "C" fn png_handle_tEXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut text_info: png_text = png_text {
        compression: 0,
        key: core::ptr::null_mut(),
        text: core::ptr::null_mut(),
        text_length: 0,
        itxt_length: 0,
        lang: core::ptr::null_mut(),
        lang_key: core::ptr::null_mut(),
    };
    let buffer: png_bytep;
    let key: png_charp;
    let mut text: png_charp;
    let skip: png_uint_32 = 0;

    /* PNG_USER_LIMITS_SUPPORTED */
    if (*png_ptr).user_chunk_cache_max != 0 {
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            return handled_error;
        }

        (*png_ptr).user_chunk_cache_max = (*png_ptr).user_chunk_cache_max.wrapping_sub(1);
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, b"no space in chunk cache\0".as_ptr() as png_const_charp);
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, length.wrapping_add(1) as png_alloc_size_t);

    if buffer == core::ptr::null_mut() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, b"out of memory\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, skip) != 0 {
        return handled_error;
    }

    key = buffer as png_charp;
    *key.add(length as usize) = 0;

    text = key;
    while *text != 0
    /* Empty loop to find end of key */
    {
        text = text.add(1);
    }

    if text != key.add(length as usize) {
        text = text.add(1);
    }

    text_info.compression = PNG_TEXT_COMPRESSION_NONE;
    text_info.key = key;
    text_info.lang = core::ptr::null_mut();
    text_info.lang_key = core::ptr::null_mut();
    text_info.itxt_length = 0;
    text_info.text = text;
    text_info.text_length = strlen(text as *const c_char);

    if png_set_text_2(png_ptr, info_ptr, &text_info, 1) == 0 {
        return handled_ok;
    }

    png_chunk_benign_error(png_ptr, b"out of memory\0".as_ptr() as png_const_charp);
    handled_error
}

/* Note: this does not correctly handle chunks that are > 64K under DOS */
/* png_handle_zTXt */
unsafe extern "C" fn png_handle_zTXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut errmsg: png_const_charp = core::ptr::null();
    let mut buffer: png_bytep;
    let mut keyword_length: png_uint_32;

    /* PNG_USER_LIMITS_SUPPORTED */
    if (*png_ptr).user_chunk_cache_max != 0 {
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            return handled_error;
        }

        (*png_ptr).user_chunk_cache_max = (*png_ptr).user_chunk_cache_max.wrapping_sub(1);
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, b"no space in chunk cache\0".as_ptr() as png_const_charp);
            return handled_error;
        }
    }

    /* Note, "length" is sufficient here; we won't be adding
     * a null terminator later.  The limit check in png_handle_chunk should be
     * sufficient.
     */
    buffer = png_read_buffer(png_ptr, length as png_alloc_size_t);

    if buffer == core::ptr::null_mut() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, b"out of memory\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* TODO: also check that the keyword contents match the spec! */
    keyword_length = 0;
    while keyword_length < length && *buffer.add(keyword_length as usize) != 0
    /* Empty loop to find end of name */
    {
        keyword_length = keyword_length.wrapping_add(1);
    }

    if keyword_length > 79 || keyword_length < 1 {
        errmsg = b"bad keyword\0".as_ptr() as png_const_charp;
    }
    /* zTXt must have some LZ data after the keyword, although it may expand to
     * zero bytes; we need a '\0' at the end of the keyword, the compression type
     * then the LZ data:
     */
    else if keyword_length.wrapping_add(3) > length {
        errmsg = b"truncated\0".as_ptr() as png_const_charp;
    } else if *buffer.add(keyword_length.wrapping_add(1) as usize) as c_int
        != PNG_COMPRESSION_TYPE_BASE
    {
        errmsg = b"unknown compression type\0".as_ptr() as png_const_charp;
    } else {
        let mut uncompressed_length: png_alloc_size_t = PNG_SIZE_MAX;

        /* TODO: at present png_decompress_chunk imposes a single application
         * level memory limit, this should be split to different values for iCCP
         * and text chunks.
         */
        if png_decompress_chunk(
            png_ptr,
            length,
            keyword_length.wrapping_add(2),
            &mut uncompressed_length,
            1, /*terminate*/
        ) == Z_STREAM_END
        {
            let mut text: png_text = png_text {
                compression: 0,
                key: core::ptr::null_mut(),
                text: core::ptr::null_mut(),
                text_length: 0,
                itxt_length: 0,
                lang: core::ptr::null_mut(),
                lang_key: core::ptr::null_mut(),
            };

            if (*png_ptr).read_buffer == core::ptr::null_mut() {
                errmsg = b"Read failure in png_handle_zTXt\0".as_ptr() as png_const_charp;
            } else {
                /* It worked; png_ptr->read_buffer now looks like a tEXt chunk
                 * except for the extra compression type byte and the fact that
                 * it isn't necessarily '\0' terminated.
                 */
                buffer = (*png_ptr).read_buffer;
                *buffer.add(
                    uncompressed_length.wrapping_add(keyword_length.wrapping_add(2) as usize),
                ) = 0;

                text.compression = PNG_TEXT_COMPRESSION_zTXt;
                text.key = buffer as png_charp;
                text.text = buffer.add(keyword_length.wrapping_add(2) as usize) as png_charp;
                text.text_length = uncompressed_length;
                text.itxt_length = 0;
                text.lang = core::ptr::null_mut();
                text.lang_key = core::ptr::null_mut();

                if png_set_text_2(png_ptr, info_ptr, &text, 1) == 0 {
                    return handled_ok;
                }

                errmsg = b"out of memory\0".as_ptr() as png_const_charp;
            }
        } else {
            errmsg = (*png_ptr).zstream.msg;
        }
    }

    png_chunk_benign_error(png_ptr, errmsg);
    handled_error
}

/* Note: this does not correctly handle chunks that are > 64K under DOS */
/* png_handle_iTXt */
unsafe extern "C" fn png_handle_iTXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut errmsg: png_const_charp = core::ptr::null();
    let mut buffer: png_bytep;
    let mut prefix_length: png_uint_32;

    /* PNG_USER_LIMITS_SUPPORTED */
    if (*png_ptr).user_chunk_cache_max != 0 {
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            return handled_error;
        }

        (*png_ptr).user_chunk_cache_max = (*png_ptr).user_chunk_cache_max.wrapping_sub(1);
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, b"no space in chunk cache\0".as_ptr() as png_const_charp);
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, length.wrapping_add(1) as png_alloc_size_t);

    if buffer == core::ptr::null_mut() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, b"out of memory\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* First the keyword. */
    prefix_length = 0;
    while prefix_length < length && *buffer.add(prefix_length as usize) != 0
    /* Empty loop */
    {
        prefix_length = prefix_length.wrapping_add(1);
    }

    /* Perform a basic check on the keyword length here. */
    if prefix_length > 79 || prefix_length < 1 {
        errmsg = b"bad keyword\0".as_ptr() as png_const_charp;
    }
    /* Expect keyword, compression flag, compression type, language, translated
     * keyword (both may be empty but are 0 terminated) then the text, which may
     * be empty.
     */
    else if prefix_length.wrapping_add(5) > length {
        errmsg = b"truncated\0".as_ptr() as png_const_charp;
    } else if *buffer.add(prefix_length.wrapping_add(1) as usize) == 0
        || (*buffer.add(prefix_length.wrapping_add(1) as usize) == 1
            && *buffer.add(prefix_length.wrapping_add(2) as usize) as c_int
                == PNG_COMPRESSION_TYPE_BASE)
    {
        let compressed: c_int = (*buffer.add(prefix_length.wrapping_add(1) as usize) != 0) as c_int;
        let language_offset: png_uint_32;
        let translated_keyword_offset: png_uint_32;
        let mut uncompressed_length: png_alloc_size_t = 0;

        /* Now the language tag */
        prefix_length = prefix_length.wrapping_add(3);
        language_offset = prefix_length;

        while prefix_length < length && *buffer.add(prefix_length as usize) != 0
        /* Empty loop */
        {
            prefix_length = prefix_length.wrapping_add(1);
        }

        /* WARNING: the length may be invalid here, this is checked below. */
        prefix_length = prefix_length.wrapping_add(1);
        translated_keyword_offset = prefix_length;

        while prefix_length < length && *buffer.add(prefix_length as usize) != 0
        /* Empty loop */
        {
            prefix_length = prefix_length.wrapping_add(1);
        }

        /* prefix_length should now be at the trailing '\0' of the translated
         * keyword, but it may already be over the end.  None of this arithmetic
         * can overflow because chunks are at most 2^31 bytes long, but on 16-bit
         * systems the available allocation may overflow.
         */
        prefix_length = prefix_length.wrapping_add(1);

        if compressed == 0 && prefix_length <= length {
            uncompressed_length = length.wrapping_sub(prefix_length) as png_alloc_size_t;
        } else if compressed != 0 && prefix_length < length {
            uncompressed_length = PNG_SIZE_MAX;

            /* TODO: at present png_decompress_chunk imposes a single application
             * level memory limit, this should be split to different values for
             * iCCP and text chunks.
             */
            if png_decompress_chunk(
                png_ptr,
                length,
                prefix_length,
                &mut uncompressed_length,
                1, /*terminate*/
            ) == Z_STREAM_END
            {
                buffer = (*png_ptr).read_buffer;
            } else {
                errmsg = (*png_ptr).zstream.msg;
            }
        } else {
            errmsg = b"truncated\0".as_ptr() as png_const_charp;
        }

        if errmsg == core::ptr::null() {
            let mut text: png_text = png_text {
                compression: 0,
                key: core::ptr::null_mut(),
                text: core::ptr::null_mut(),
                text_length: 0,
                itxt_length: 0,
                lang: core::ptr::null_mut(),
                lang_key: core::ptr::null_mut(),
            };

            *buffer.add(uncompressed_length.wrapping_add(prefix_length as usize)) = 0;

            if compressed == 0 {
                text.compression = PNG_ITXT_COMPRESSION_NONE;
            } else {
                text.compression = PNG_ITXT_COMPRESSION_zTXt;
            }

            text.key = buffer as png_charp;
            text.lang = (buffer as png_charp).add(language_offset as usize);
            text.lang_key = (buffer as png_charp).add(translated_keyword_offset as usize);
            text.text = (buffer as png_charp).add(prefix_length as usize);
            text.text_length = 0;
            text.itxt_length = uncompressed_length;

            if png_set_text_2(png_ptr, info_ptr, &text, 1) == 0 {
                return handled_ok;
            }

            errmsg = b"out of memory\0".as_ptr() as png_const_charp;
        }
    } else {
        errmsg = b"bad compression info\0".as_ptr() as png_const_charp;
    }

    if errmsg != core::ptr::null() {
        png_chunk_benign_error(png_ptr, errmsg);
    }
    handled_error
}
