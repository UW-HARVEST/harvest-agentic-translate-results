// pngrutil.c - utilities to read a PNG file
//
// This file is part of the translation of libpng.  It contains the chunk
// readers png_handle_pCAL() .. png_handle_iTXt().
//
// All these functions are 'static' (PRIVATE) in C, but their addresses are
// stored in the chunk handler table, so they are declared 'extern "C"' here to
// get an ABI compatible function pointer.

use crate::*;

/* Read the pCAL chunk (described in the PNG Extensions document) */
unsafe extern "C" fn png_handle_pCAL(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buffer: png_bytep;
    let mut buf: png_bytep;
    let endptr: png_bytep;
    let X0: png_int_32;
    let X1: png_int_32;
    let type_: png_byte;
    let nparams: png_byte;
    let units: png_bytep;
    let params: png_charpp;
    let mut i: c_int;

    buffer = png_read_buffer(png_ptr, length.wrapping_add(1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr!("out of memory"));
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    *buffer.add(length as usize) = 0; /* Null terminate the last string */

    /* Finding end of pCAL purpose string */
    buf = buffer;
    while *buf != 0 {
        /* Empty loop */
        buf = buf.add(1);
    }

    endptr = buffer.add(length as usize);

    /* We need to have at least 12 bytes after the purpose string
     * in order to get the parameter information.
     */
    if (endptr as isize) - (buf as isize) <= 12 {
        png_chunk_benign_error(png_ptr, cstr!("invalid"));
        return handled_error;
    }

    /* Reading pCAL X0, X1, type, nparams, and units */
    X0 = png_get_int_32(buf.add(1) as png_const_bytep);
    X1 = png_get_int_32(buf.add(5) as png_const_bytep);
    type_ = *buf.add(9);
    nparams = *buf.add(10);
    units = buf.add(11);

    /* Checking pCAL equation type and number of parameters */
    /* Check that we have the right number of parameters for known
     * equation types.
     */
    if (type_ as c_int == PNG_EQUATION_LINEAR && nparams as c_int != 2)
        || (type_ as c_int == PNG_EQUATION_BASE_E && nparams as c_int != 3)
        || (type_ as c_int == PNG_EQUATION_ARBITRARY && nparams as c_int != 3)
        || (type_ as c_int == PNG_EQUATION_HYPERBOLIC && nparams as c_int != 4)
    {
        png_chunk_benign_error(png_ptr, cstr!("invalid parameter count"));
        return handled_error;
    } else if type_ as c_int >= PNG_EQUATION_LAST {
        png_chunk_benign_error(png_ptr, cstr!("unrecognized equation type"));
    }

    buf = units;
    while *buf != 0 {
        /* Empty loop to move past the units string. */
        buf = buf.add(1);
    }

    /* Allocating pCAL parameters array */
    params = png_malloc_warn(
        png_ptr,
        nparams as usize * core::mem::size_of::<png_charp>(),
    ) as png_charpp;

    if params.is_null() {
        png_chunk_benign_error(png_ptr, cstr!("out of memory"));
        return handled_error;
    }

    /* Get pointers to the start of each parameter string. */
    i = 0;
    while i < nparams as c_int {
        buf = buf.add(1); /* Skip the null string terminator from previous parameter. */

        *params.offset(i as isize) = buf as png_charp;
        while buf <= endptr && *buf != 0 {
            /* Empty loop to move past each parameter string */
            buf = buf.add(1);
        }

        /* Make sure we haven't run out of data yet */
        if buf > endptr {
            png_free(png_ptr, params as png_voidp);
            png_chunk_benign_error(png_ptr, cstr!("invalid data"));
            return handled_error;
        }

        i += 1;
    }

    png_set_pCAL(
        png_ptr,
        info_ptr,
        buffer as png_const_charp,
        X0,
        X1,
        type_ as c_int,
        nparams as c_int,
        units as png_const_charp,
        params,
    );

    /* TODO: BUG: png_set_pCAL calls png_chunk_report which, in this case, calls
     * png_benign_error and that can error out.
     *
     * png_read_buffer needs to be allocated with space for both nparams and the
     * parameter strings.  Not hard to do.
     */
    png_free(png_ptr, params as png_voidp);
    handled_ok
}

/* Read the sCAL chunk */
unsafe extern "C" fn png_handle_sCAL(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let buffer: png_bytep;
    let mut i: usize;
    let mut state: c_int;

    buffer = png_read_buffer(png_ptr, length.wrapping_add(1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr!("out of memory"));
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);
    *buffer.add(length as usize) = 0; /* Null terminate the last string */

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* Validate the unit. */
    if *buffer.add(0) != 1 && *buffer.add(0) != 2 {
        png_chunk_benign_error(png_ptr, cstr!("invalid unit"));
        return handled_error;
    }

    /* Validate the ASCII numbers, need two ASCII numbers separated by
     * a '\0' and they need to fit exactly in the chunk data.
     */
    i = 1;
    state = 0;

    if png_check_fp_number(
        buffer as png_const_charp,
        length as usize,
        &mut state,
        &mut i,
    ) == 0
        || i >= length as usize
        || ({
            let v = *buffer.add(i);
            i += 1;
            v
        }) != 0
    {
        png_chunk_benign_error(png_ptr, cstr!("bad width format"));
    } else if !PNG_FP_IS_POSITIVE(state) {
        png_chunk_benign_error(png_ptr, cstr!("non-positive width"));
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
            png_chunk_benign_error(png_ptr, cstr!("bad height format"));
        } else if !PNG_FP_IS_POSITIVE(state) {
            png_chunk_benign_error(png_ptr, cstr!("non-positive height"));
        } else {
            /* This is the (only) success case. */
            png_set_sCAL_s(
                png_ptr,
                info_ptr,
                *buffer.add(0) as c_int,
                buffer.add(1) as png_const_charp,
                buffer.add(heighti) as png_const_charp,
            );
            return handled_ok;
        }
    }

    handled_error
}

unsafe extern "C" fn png_handle_tIME(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buf: [png_byte; 7] = [0; 7];
    let mut mod_time: png_time = core::mem::zeroed();

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
    mod_time.year = png_get_uint_16(buf.as_ptr());

    png_set_tIME(png_ptr, info_ptr, &mod_time as png_const_timep);
    handled_ok
}

/* Note: this does not properly handle chunks that are > 64K under DOS */
unsafe extern "C" fn png_handle_tEXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut text_info: png_text = core::mem::zeroed();
    let buffer: png_bytep;
    let key: png_charp;
    let mut text: png_charp;
    let skip: png_uint_32 = 0;

    if (*png_ptr).user_chunk_cache_max != 0 {
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            return handled_error;
        }

        (*png_ptr).user_chunk_cache_max = (*png_ptr).user_chunk_cache_max.wrapping_sub(1);
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr!("no space in chunk cache"));
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, length.wrapping_add(1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr!("out of memory"));
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, skip) != 0 {
        return handled_error;
    }

    key = buffer as png_charp;
    *key.add(length as usize) = 0;

    text = key;
    while *text != 0 {
        /* Empty loop to find end of key */
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
    text_info.text_length = strlen(text);

    if png_set_text_2(png_ptr, info_ptr, &text_info as png_const_textp, 1) == 0 {
        return handled_ok;
    }

    png_chunk_benign_error(png_ptr, cstr!("out of memory"));
    handled_error
}

/* Note: this does not correctly handle chunks that are > 64K under DOS */
unsafe extern "C" fn png_handle_zTXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut errmsg: png_const_charp = core::ptr::null();
    let mut buffer: png_bytep;
    let mut keyword_length: png_uint_32;

    if (*png_ptr).user_chunk_cache_max != 0 {
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            return handled_error;
        }

        (*png_ptr).user_chunk_cache_max = (*png_ptr).user_chunk_cache_max.wrapping_sub(1);
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr!("no space in chunk cache"));
            return handled_error;
        }
    }

    /* Note, "length" is sufficient here; we won't be adding
     * a null terminator later.  The limit check in png_handle_chunk should be
     * sufficient.
     */
    buffer = png_read_buffer(png_ptr, length as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr!("out of memory"));
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* TODO: also check that the keyword contents match the spec! */
    keyword_length = 0;
    while keyword_length < length && *buffer.add(keyword_length as usize) != 0 {
        /* Empty loop to find end of name */
        keyword_length += 1;
    }

    if keyword_length > 79 || keyword_length < 1 {
        errmsg = cstr!("bad keyword");
    }
    /* zTXt must have some LZ data after the keyword, although it may expand to
     * zero bytes; we need a '\0' at the end of the keyword, the compression type
     * then the LZ data:
     */
    else if keyword_length.wrapping_add(3) > length {
        errmsg = cstr!("truncated");
    } else if *buffer.add(keyword_length as usize + 1) as c_int != PNG_COMPRESSION_TYPE_BASE {
        errmsg = cstr!("unknown compression type");
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
            let mut text: png_text = core::mem::zeroed();

            if (*png_ptr).read_buffer.is_null() {
                errmsg = cstr!("Read failure in png_handle_zTXt");
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
                text.text = buffer.add(keyword_length as usize + 2) as png_charp;
                text.text_length = uncompressed_length;
                text.itxt_length = 0;
                text.lang = core::ptr::null_mut();
                text.lang_key = core::ptr::null_mut();

                if png_set_text_2(png_ptr, info_ptr, &text as png_const_textp, 1) == 0 {
                    return handled_ok;
                }

                errmsg = cstr!("out of memory");
            }
        } else {
            errmsg = (*png_ptr).zstream.msg;
        }
    }

    png_chunk_benign_error(png_ptr, errmsg);
    handled_error
}

/* Note: this does not correctly handle chunks that are > 64K under DOS */
unsafe extern "C" fn png_handle_iTXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut errmsg: png_const_charp = core::ptr::null();
    let mut buffer: png_bytep;
    let mut prefix_length: png_uint_32;

    if (*png_ptr).user_chunk_cache_max != 0 {
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            return handled_error;
        }

        (*png_ptr).user_chunk_cache_max = (*png_ptr).user_chunk_cache_max.wrapping_sub(1);
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr!("no space in chunk cache"));
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, length.wrapping_add(1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr!("out of memory"));
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* First the keyword. */
    prefix_length = 0;
    while prefix_length < length && *buffer.add(prefix_length as usize) != 0 {
        /* Empty loop */
        prefix_length += 1;
    }

    /* Perform a basic check on the keyword length here. */
    if prefix_length > 79 || prefix_length < 1 {
        errmsg = cstr!("bad keyword");
    }
    /* Expect keyword, compression flag, compression type, language, translated
     * keyword (both may be empty but are 0 terminated) then the text, which may
     * be empty.
     */
    else if prefix_length.wrapping_add(5) > length {
        errmsg = cstr!("truncated");
    } else if *buffer.add(prefix_length as usize + 1) == 0
        || (*buffer.add(prefix_length as usize + 1) == 1
            && *buffer.add(prefix_length as usize + 2) as c_int == PNG_COMPRESSION_TYPE_BASE)
    {
        let compressed: c_int = (*buffer.add(prefix_length as usize + 1) != 0) as c_int;
        let language_offset: png_uint_32;
        let translated_keyword_offset: png_uint_32;
        let mut uncompressed_length: png_alloc_size_t = 0;

        /* Now the language tag */
        prefix_length += 3;
        language_offset = prefix_length;

        while prefix_length < length && *buffer.add(prefix_length as usize) != 0 {
            /* Empty loop */
            prefix_length += 1;
        }

        /* WARNING: the length may be invalid here, this is checked below. */
        prefix_length += 1;
        translated_keyword_offset = prefix_length;

        while prefix_length < length && *buffer.add(prefix_length as usize) != 0 {
            /* Empty loop */
            prefix_length += 1;
        }

        /* prefix_length should now be at the trailing '\0' of the translated
         * keyword, but it may already be over the end.  None of this arithmetic
         * can overflow because chunks are at most 2^31 bytes long, but on 16-bit
         * systems the available allocation may overflow.
         */
        prefix_length += 1;

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
            errmsg = cstr!("truncated");
        }

        if errmsg.is_null() {
            let mut text: png_text = core::mem::zeroed();

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

            if png_set_text_2(png_ptr, info_ptr, &text as png_const_textp, 1) == 0 {
                return handled_ok;
            }

            errmsg = cstr!("out of memory");
        }
    } else {
        errmsg = cstr!("bad compression info");
    }

    if !errmsg.is_null() {
        png_chunk_benign_error(png_ptr, errmsg);
    }
    handled_error
}
