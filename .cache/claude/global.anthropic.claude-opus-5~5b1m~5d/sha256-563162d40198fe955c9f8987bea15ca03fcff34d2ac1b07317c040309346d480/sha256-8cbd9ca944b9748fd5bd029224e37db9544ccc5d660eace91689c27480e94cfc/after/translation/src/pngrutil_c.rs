//! pngrutil.c lines 1999-3226: the remaining ancillary chunk handlers
//! (eXIf, hIST, pHYs, oFFs, pCAL, sCAL, tIME, tEXt, zTXt, iTXt), the unknown
//! chunk handling and the table driven chunk dispatcher.
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

pub unsafe fn png_handle_eXIf(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> c_int {
    let mut buffer: png_bytep = core::ptr::null_mut();

    buffer = png_read_buffer(png_ptr, length as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* PNGv3: the code used to check the byte order mark at the start for MM or
     * II, however PNGv3 states that the first 4 bytes should be checked.
     * The caller ensures that there are four bytes available.
     */
    {
        let header: png_uint_32 = png_get_uint_32(buffer as png_const_bytep);

        /* These numbers are copied from the PNGv3 spec: */
        if header != 0x49492A00 && header != 0x4D4D002A {
            png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
            return handled_error;
        }
    }

    png_set_eXIf_1(png_ptr, info_ptr, length, buffer);
    handled_ok
}

pub unsafe fn png_handle_hIST(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> c_int {
    let num: c_uint;
    let mut i: c_uint;
    let mut readbuf: [png_uint_16; PNG_MAX_PALETTE_LENGTH as usize] =
        [0; PNG_MAX_PALETTE_LENGTH as usize];

    /* This cast is safe because the chunk definition limits the length to a
     * maximum of 1024 bytes.
     *
     * TODO: maybe use png_uint_32 anyway, not unsigned int, to reduce the
     * casts.
     */
    num = (length as c_uint) / 2;

    if length != num.wrapping_mul(2)
        || num != (*png_ptr).num_palette as c_uint
        || num > PNG_MAX_PALETTE_LENGTH as c_uint
    {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
        return handled_error;
    }

    i = 0;
    while i < num {
        let mut buf: [png_byte; 2] = [0; 2];

        png_crc_read(png_ptr, buf.as_mut_ptr(), 2);
        readbuf[i as usize] = png_get_uint_16(buf.as_ptr() as png_const_bytep);
        i += 1;
    }

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    png_set_hIST(png_ptr, info_ptr, readbuf.as_ptr() as png_const_uint_16p);
    handled_ok
}

pub unsafe fn png_handle_pHYs(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> c_int {
    let mut buf: [png_byte; 9] = [0; 9];
    let res_x: png_uint_32;
    let res_y: png_uint_32;
    let unit_type: c_int;

    png_crc_read(png_ptr, buf.as_mut_ptr(), 9);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    res_x = png_get_uint_32(buf.as_ptr() as png_const_bytep);
    res_y = png_get_uint_32((buf.as_ptr() as png_const_bytep).add(4));
    unit_type = buf[8] as c_int;
    png_set_pHYs(png_ptr, info_ptr, res_x, res_y, unit_type);
    handled_ok
}

pub unsafe fn png_handle_oFFs(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> c_int {
    let mut buf: [png_byte; 9] = [0; 9];
    let offset_x: png_int_32;
    let offset_y: png_int_32;
    let unit_type: c_int;

    png_crc_read(png_ptr, buf.as_mut_ptr(), 9);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    offset_x = png_get_int_32(buf.as_ptr() as png_const_bytep);
    offset_y = png_get_int_32((buf.as_ptr() as png_const_bytep).add(4));
    unit_type = buf[8] as c_int;
    png_set_oFFs(png_ptr, info_ptr, offset_x, offset_y, unit_type);
    handled_ok
}

/* Read the pCAL chunk (described in the PNG Extensions document) */
pub unsafe fn png_handle_pCAL(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> c_int {
    let buffer: png_bytep;
    let mut buf: png_bytep;
    let endptr: png_bytep;
    let X0: png_int_32;
    let X1: png_int_32;
    let type_: png_byte;
    let nparams: png_byte;
    let units: *mut png_byte;
    let params: png_charpp;
    let mut i: c_int;

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
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
    if endptr.offset_from(buf) <= 12 {
        png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
        return handled_error;
    }

    /* Reading pCAL X0, X1, type, nparams, and units */
    X0 = png_get_int_32((buf as png_const_bytep).add(1));
    X1 = png_get_int_32((buf as png_const_bytep).add(5));
    type_ = *buf.add(9);
    nparams = *buf.add(10);
    units = buf.add(11);

    /* Checking pCAL equation type and number of parameters */
    /* Check that we have the right number of parameters for known
     * equation types.
     */
    if (type_ as c_int == PNG_EQUATION_LINEAR && nparams != 2)
        || (type_ as c_int == PNG_EQUATION_BASE_E && nparams != 3)
        || (type_ as c_int == PNG_EQUATION_ARBITRARY && nparams != 3)
        || (type_ as c_int == PNG_EQUATION_HYPERBOLIC && nparams != 4)
    {
        png_chunk_benign_error(png_ptr, c"invalid parameter count".as_ptr());
        return handled_error;
    } else if type_ as c_int >= PNG_EQUATION_LAST {
        png_chunk_benign_error(png_ptr, c"unrecognized equation type".as_ptr());
    }

    buf = units;
    while *buf != 0 {
        /* Empty loop to move past the units string. */
        buf = buf.add(1);
    }

    /* Allocating pCAL parameters array */

    params = png_malloc_warn(
        png_ptr,
        (nparams as png_alloc_size_t).wrapping_mul(core::mem::size_of::<png_charp>()),
    ) as png_charpp;

    if params.is_null() {
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
        return handled_error;
    }

    /* Get pointers to the start of each parameter string. */
    i = 0;
    while i < nparams as c_int {
        buf = buf.add(1); /* Skip the null string terminator from previous parameter. */

        *params.add(i as usize) = buf as png_charp;
        while buf <= endptr && *buf != 0 {
            /* Empty loop to move past each parameter string */
            buf = buf.add(1);
        }

        /* Make sure we haven't run out of data yet */
        if buf > endptr {
            png_free(png_ptr, params as png_voidp);
            png_chunk_benign_error(png_ptr, c"invalid data".as_ptr());
            return handled_error;
        }

        i += 1;
    }

    png_set_pCAL(
        png_ptr,
        info_ptr,
        buffer as png_charp,
        X0,
        X1,
        type_ as c_int,
        nparams as c_int,
        units as png_charp,
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
pub unsafe fn png_handle_sCAL(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> c_int {
    let buffer: png_bytep;
    let mut i: usize;
    let mut state: c_int;

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);
    *buffer.add(length as usize) = 0; /* Null terminate the last string */

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* Validate the unit. */
    if *buffer != 1 && *buffer != 2 {
        png_chunk_benign_error(png_ptr, c"invalid unit".as_ptr());
        return handled_error;
    }

    /* Validate the ASCII numbers, need two ASCII numbers separated by
     * a '\0' and they need to fit exactly in the chunk data.
     */
    i = 1;
    state = 0;

    /* NOTE: the ++ on 'i' below is only evaluated if the two preceding tests
     * both fail, exactly as in the C short-circuit expression.
     */
    let bad_width: bool = if png_check_fp_number(
        buffer as png_const_charp,
        length as usize,
        &mut state,
        &mut i,
    ) == 0
    {
        true
    } else if i >= length as usize {
        true
    } else {
        let v = *buffer.add(i);
        i = i.wrapping_add(1);
        v != 0
    };

    if bad_width {
        png_chunk_benign_error(png_ptr, c"bad width format".as_ptr());
    } else if !PNG_FP_IS_POSITIVE(state) {
        png_chunk_benign_error(png_ptr, c"non-positive width".as_ptr());
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
            png_chunk_benign_error(png_ptr, c"bad height format".as_ptr());
        } else if !PNG_FP_IS_POSITIVE(state) {
            png_chunk_benign_error(png_ptr, c"non-positive height".as_ptr());
        } else {
            /* This is the (only) success case. */
            png_set_sCAL_s(
                png_ptr,
                info_ptr,
                *buffer as c_int,
                (buffer as png_charp).add(1) as png_const_charp,
                (buffer as png_charp).add(heighti) as png_const_charp,
            );
            return handled_ok;
        }
    }

    handled_error
}

pub unsafe fn png_handle_tIME(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> c_int {
    let mut buf: [png_byte; 7] = [0; 7];
    let mut mod_time: png_time = png_time::default();

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
    mod_time.year = png_get_uint_16(buf.as_ptr() as png_const_bytep);

    png_set_tIME(png_ptr, info_ptr, &mod_time as png_const_timep);
    handled_ok
}

/* Note: this does not properly handle chunks that are > 64K under DOS */
pub unsafe fn png_handle_tEXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> c_int {
    let mut text_info: png_text = png_text::default();
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
            png_chunk_benign_error(png_ptr, c"no space in chunk cache".as_ptr());
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
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

    png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
    handled_error
}

/* Note: this does not correctly handle chunks that are > 64K under DOS */
pub unsafe fn png_handle_zTXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> c_int {
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
            png_chunk_benign_error(png_ptr, c"no space in chunk cache".as_ptr());
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
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
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
        errmsg = c"bad keyword".as_ptr();
    }
    /* zTXt must have some LZ data after the keyword, although it may expand to
     * zero bytes; we need a '\0' at the end of the keyword, the compression type
     * then the LZ data:
     */
    else if keyword_length.wrapping_add(3) > length {
        errmsg = c"truncated".as_ptr();
    } else if *buffer.add((keyword_length + 1) as usize) as c_int != PNG_COMPRESSION_TYPE_BASE {
        errmsg = c"unknown compression type".as_ptr();
    } else {
        let mut uncompressed_length: png_alloc_size_t = PNG_SIZE_MAX;

        /* TODO: at present png_decompress_chunk imposes a single application
         * level memory limit, this should be split to different values for iCCP
         * and text chunks.
         */
        if png_decompress_chunk(
            png_ptr,
            length,
            keyword_length + 2,
            &mut uncompressed_length,
            1, /*terminate*/
        ) == Z_STREAM_END
        {
            let mut text: png_text = png_text::default();

            if (*png_ptr).read_buffer.is_null() {
                errmsg = c"Read failure in png_handle_zTXt".as_ptr();
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

                if png_set_text_2(png_ptr, info_ptr, &text as png_const_textp, 1) == 0 {
                    return handled_ok;
                }

                errmsg = c"out of memory".as_ptr();
            }
        } else {
            errmsg = (*png_ptr).zstream.msg;
        }
    }

    png_chunk_benign_error(png_ptr, errmsg);
    handled_error
}

/* Note: this does not correctly handle chunks that are > 64K under DOS */
pub unsafe fn png_handle_iTXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> c_int {
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
            png_chunk_benign_error(png_ptr, c"no space in chunk cache".as_ptr());
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
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
        errmsg = c"bad keyword".as_ptr();
    }
    /* Expect keyword, compression flag, compression type, language, translated
     * keyword (both may be empty but are 0 terminated) then the text, which may
     * be empty.
     */
    else if prefix_length.wrapping_add(5) > length {
        errmsg = c"truncated".as_ptr();
    } else if *buffer.add((prefix_length + 1) as usize) == 0
        || (*buffer.add((prefix_length + 1) as usize) == 1
            && *buffer.add((prefix_length + 2) as usize) as c_int == PNG_COMPRESSION_TYPE_BASE)
    {
        let compressed: c_int = (*buffer.add((prefix_length + 1) as usize) != 0) as c_int;
        let language_offset: png_uint_32;
        let translated_keyword_offset: png_uint_32;
        let mut uncompressed_length: png_alloc_size_t = 0;

        /* Now the language tag */
        prefix_length = prefix_length.wrapping_add(3);
        language_offset = prefix_length;

        while prefix_length < length && *buffer.add(prefix_length as usize) != 0 {
            /* Empty loop */
            prefix_length += 1;
        }

        /* WARNING: the length may be invalid here, this is checked below. */
        prefix_length = prefix_length.wrapping_add(1);
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
            errmsg = c"truncated".as_ptr();
        }

        if errmsg.is_null() {
            let mut text: png_text = png_text::default();

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

            errmsg = c"out of memory".as_ptr();
        }
    } else {
        errmsg = c"bad compression info".as_ptr();
    }

    if !errmsg.is_null() {
        png_chunk_benign_error(png_ptr, errmsg);
    }
    handled_error
}

/* Utility function for png_handle_unknown; set up png_ptr::unknown_chunk */
pub unsafe fn png_cache_unknown_chunk(png_ptr: png_structrp, length: png_uint_32) -> c_int {
    let limit: png_alloc_size_t = (*png_ptr).user_chunk_malloc_max;

    if !(*png_ptr).unknown_chunk.data.is_null() {
        png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
        (*png_ptr).unknown_chunk.data = core::ptr::null_mut();
    }

    if length as png_alloc_size_t <= limit {
        PNG_CSTRING_FROM_CHUNK(
            (*png_ptr).unknown_chunk.name.as_mut_ptr(),
            (*png_ptr).chunk_name,
        );
        /* The following is safe because of the PNG_SIZE_MAX init above */
        (*png_ptr).unknown_chunk.size = length as usize /*SAFE*/;
        /* 'mode' is a flag array, only the bottom four bits matter here */
        (*png_ptr).unknown_chunk.location = (*png_ptr).mode as png_byte /*SAFE*/;

        if length == 0 {
            (*png_ptr).unknown_chunk.data = core::ptr::null_mut();
        } else {
            /* Do a 'warn' here - it is handled below. */
            (*png_ptr).unknown_chunk.data =
                png_malloc_warn(png_ptr, length as png_alloc_size_t) as png_bytep;
        }
    }

    if (*png_ptr).unknown_chunk.data.is_null() && length > 0 {
        /* This is benign because we clean up correctly */
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"unknown chunk exceeds memory limits".as_ptr());
        0
    } else {
        if length > 0 {
            png_crc_read(png_ptr, (*png_ptr).unknown_chunk.data, length);
        }
        png_crc_finish(png_ptr, 0);
        1
    }
}

/* Handle an unknown, or known but disabled, chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_handle_unknown(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
    keep: c_int,
) -> c_int {
    let mut keep = keep;
    let mut handled: c_int = handled_discarded; /* the default */

    /* NOTE: this code is based on the code in libpng-1.4.12 except for fixing
     * the bug which meant that setting a non-default behavior for a specific
     * chunk would be ignored (the default was always used unless a user
     * callback was installed).
     *
     * 'keep' is the value from the png_chunk_unknown_handling, the setting for
     * this specific chunk_name, if PNG_HANDLE_AS_UNKNOWN_SUPPORTED, if not it
     * will always be PNG_HANDLE_CHUNK_AS_DEFAULT and it needs to be set here.
     * This is just an optimization to avoid multiple calls to the lookup
     * function.
     */

    /* One of the following methods will read the chunk or skip it (at least one
     * of these is always defined because this is the only way to switch on
     * PNG_READ_UNKNOWN_CHUNKS_SUPPORTED)
     */
    /* The user callback takes precedence over the chunk keep value, but the
     * keep value is still required to validate a save of a critical chunk.
     */
    if (*png_ptr).read_user_chunk_fn.is_some() {
        if png_cache_unknown_chunk(png_ptr, length) != 0 {
            /* Callback to user unknown chunk handler */
            let ret: c_int = ((*png_ptr).read_user_chunk_fn.unwrap())(
                png_ptr,
                core::ptr::addr_of_mut!((*png_ptr).unknown_chunk),
            );

            /* ret is:
             * negative: An error occurred; png_chunk_error will be called.
             *     zero: The chunk was not handled, the chunk will be discarded
             *           unless png_set_keep_unknown_chunks has been used to set
             *           a 'keep' behavior for this particular chunk, in which
             *           case that will be used.  A critical chunk will cause an
             *           error at this point unless it is to be saved.
             * positive: The chunk was handled, libpng will ignore/discard it.
             */
            if ret < 0
            /* handled_error */
            {
                png_chunk_error(png_ptr, c"error in user chunk".as_ptr());
            } else if ret == 0 {
                /* If the keep value is 'default' or 'never' override it, but
                 * still error out on critical chunks unless the keep value is
                 * 'always'  While this is weird it is the behavior in 1.4.12.
                 * A possible improvement would be to obey the value set for the
                 * chunk, but this would be an API change that would probably
                 * damage some applications.
                 *
                 * The png_app_warning below catches the case that matters, where
                 * the application has not set specific save or ignore for this
                 * chunk or global save or ignore.
                 */
                if keep < PNG_HANDLE_CHUNK_IF_SAFE {
                    if (*png_ptr).unknown_default < PNG_HANDLE_CHUNK_IF_SAFE {
                        png_chunk_warning(png_ptr, c"Saving unknown chunk:".as_ptr());
                        png_app_warning(
                            png_ptr,
                            c"forcing save of an unhandled chunk; please call png_set_keep_unknown_chunks".as_ptr(),
                        );
                        /* with keep = PNG_HANDLE_CHUNK_IF_SAFE */
                    }
                    keep = PNG_HANDLE_CHUNK_IF_SAFE;
                }
            } else
            /* chunk was handled */
            {
                handled = handled_ok;
                /* Critical chunks can be safely discarded at this point. */
                keep = PNG_HANDLE_CHUNK_NEVER;
            }
        } else {
            keep = PNG_HANDLE_CHUNK_NEVER; /* insufficient memory */
        }
    } else
    /* Use the SAVE_UNKNOWN_CHUNKS code or skip the chunk */
    {
        /* keep is currently just the per-chunk setting, if there was no
         * setting change it to the global default now (not that this may
         * still be AS_DEFAULT) then obtain the cache of the chunk if required,
         * if not simply skip the chunk.
         */
        if keep == PNG_HANDLE_CHUNK_AS_DEFAULT {
            keep = (*png_ptr).unknown_default;
        }

        if keep == PNG_HANDLE_CHUNK_ALWAYS
            || (keep == PNG_HANDLE_CHUNK_IF_SAFE
                && PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name) != 0)
        {
            if png_cache_unknown_chunk(png_ptr, length) == 0 {
                keep = PNG_HANDLE_CHUNK_NEVER;
            }
        } else {
            png_crc_finish(png_ptr, length);
        }
    }

    /* Now store the chunk in the chunk list if appropriate, and if the limits
     * permit it.
     */
    if keep == PNG_HANDLE_CHUNK_ALWAYS
        || (keep == PNG_HANDLE_CHUNK_IF_SAFE && PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name) != 0)
    {
        match (*png_ptr).user_chunk_cache_max {
            2 => {
                (*png_ptr).user_chunk_cache_max = 1;
                png_chunk_benign_error(png_ptr, c"no space in chunk cache".as_ptr());
                /* FALLTHROUGH into case 1, which just breaks. */
            }
            1 => {
                /* NOTE: prior to 1.6.0 this case resulted in an unknown critical
                 * chunk being skipped, now there will be a hard error below.
                 */
            }
            0 => {
                /* no limit */
                /* Here when the limit isn't reached or when limits are compiled
                 * out; store the chunk.
                 */
                png_set_unknown_chunks(
                    png_ptr,
                    info_ptr,
                    core::ptr::addr_of!((*png_ptr).unknown_chunk),
                    1,
                );
                handled = handled_saved;
            }
            _ => {
                /* not at limit */
                (*png_ptr).user_chunk_cache_max =
                    (*png_ptr).user_chunk_cache_max.wrapping_sub(1);
                /* FALLTHROUGH into case 0 */
                png_set_unknown_chunks(
                    png_ptr,
                    info_ptr,
                    core::ptr::addr_of!((*png_ptr).unknown_chunk),
                    1,
                );
                handled = handled_saved;
            }
        }
    }

    /* Regardless of the error handling below the cached data (if any) can be
     * freed now.  Notice that the data is not freed if there is a png_error, but
     * it will be freed by destroy_read_struct.
     */
    if !(*png_ptr).unknown_chunk.data.is_null() {
        png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
    }
    (*png_ptr).unknown_chunk.data = core::ptr::null_mut();

    /* Check for unhandled critical chunks */
    if handled < handled_saved && PNG_CHUNK_CRITICAL((*png_ptr).chunk_name) {
        png_chunk_error(png_ptr, c"unhandled critical chunk".as_ptr());
    }

    handled
}

/* APNG handling: the minimal implementation of APNG handling in libpng 1.6
 * requires that those significant applications which already handle APNG not
 * get hosed.  To do this ensure the code here will have to ensure than APNG
 * data by default (at least in 1.6) gets stored in the unknown chunk list.
 * Maybe this can be relaxed in a few years but at present it's just the only
 * safe way.
 *
 * ATM just cause unknown handling for all three chunks:
 *
 *   #define png_handle_acTL NULL
 *   #define png_handle_fcTL NULL
 *   #define png_handle_fdAT NULL
 */

/*
 * 1.6.47: This is the new table driven interface to all the chunk handling.
 *
 * The table describes the PNG standard rules for **reading** known chunks -
 * every chunk which has an entry in PNG_KNOWN_CHUNKS.  The table contains an
 * entry for each PNG_INDEX_cHNK describing the rules.
 *
 * In this initial version the only information in the entry is the
 * png_handle_cHNK function for the chunk in question.  When chunk support is
 * compiled out the entry will be NULL.
 */
/// The C `png_handle_result_code (*handler)(png_structrp, png_inforp, png_uint_32)`
pub type png_handler = Option<unsafe fn(png_structrp, png_inforp, png_uint_32) -> c_int>;

#[derive(Copy, Clone)]
pub struct png_read_chunk_info {
    /* A chunk-specific 'handler', NULL if the chunk is not supported in this
     * build.
     */
    pub handler: png_handler,

    /* Crushing these values helps on modern 32-bit architectures because the
     * pointer and the following bit fields both end up requiring 32 bits.
     * Typically this will halve the table size.  On 64-bit architectures the
     * table entries will typically be 8 bytes.
     */
    pub max_length: png_uint_32, /* :12  Length min, max in bytes */
    pub min_length: png_uint_32, /* :8 */
    /* Length errors on critical chunks have special handling to preserve the
     * existing behaviour in libpng 1.6.  Ancillary chunks are checked below
     * and produce a 'benign' error.
     */
    pub pos_before: png_uint_32, /* :4  PNG_HAVE_ values chunk must precede */
    pub pos_after: png_uint_32,  /* :4  PNG_HAVE_ values chunk must follow */
    /* NOTE: PLTE, tRNS and bKGD require special handling which depends on
     * the colour type of the base image.
     */
    pub multiple: png_uint_32, /* :1  Multiple occurrences permitted */
                               /* This is enabled for PLTE because PLTE may, in practice, be optional */
}

pub const NoCheck: c_uint = 0x801; /* Do not check the maximum length */
pub const Limit: c_uint = 0x802; /* Limit to png_chunk_max bytes */
pub const LKMin: png_uint_32 = 3 + LZ77Min; /* Minimum length of keyword+LZ77 */

const hIHDR: png_uint_32 = PNG_HAVE_IHDR;
const hPLTE: png_uint_32 = PNG_HAVE_PLTE;
const hIDAT: png_uint_32 = PNG_HAVE_IDAT;
/* For the two chunks, tRNS and bKGD which can occur in PNGs without a PLTE
 * but must occur after the PLTE use this and put the check in the handler
 * routine for colour mapped images were PLTE is required.  Also put a check
 * in PLTE for other image types to drop the PLTE if tRNS or bKGD have been
 * seen.
 */
const hCOL: png_uint_32 = PNG_HAVE_PLTE | PNG_HAVE_IDAT;
/* Used for the decoding chunks which must be before PLTE. */
const aIDAT: png_uint_32 = PNG_AFTER_IDAT;

#[inline]
const fn CD(
    handler: png_handler,
    max_length: c_uint,
    min_length: png_uint_32,
    pos_before: png_uint_32,
    pos_after: png_uint_32,
    multiple: png_uint_32,
) -> png_read_chunk_info {
    png_read_chunk_info {
        handler,
        max_length: max_length as png_uint_32,
        min_length,
        pos_before,
        pos_after,
        multiple,
    }
}

/* Chunks from W3C PNG v3, in PNG_KNOWN_CHUNKS order:
 *          cHNK  max_len,   min, before, after, multiple
 */
pub static read_chunks: [png_read_chunk_info; PNG_INDEX_unknown as usize] = [
    /* 0 IHDR */ CD(Some(png_handle_IHDR), 13, 13, hIHDR, 0, 0),
    /* 1 PLTE: PLTE errors are only critical for colour-map images,
     * consequently the handler does all the checks. */
    CD(Some(png_handle_PLTE), NoCheck, 0, 0, hIHDR, 1),
    /* 2 IDAT */ CD(None, NoCheck, 0, aIDAT, hIHDR, 1),
    /* 3 IEND: historically data was allowed in IEND */
    CD(Some(png_handle_IEND), NoCheck, 0, 0, aIDAT, 0),
    /* 4 acTL */ CD(None, 8, 8, hIDAT, hIHDR, 0),
    /* 5 bKGD */ CD(Some(png_handle_bKGD), 6, 1, hIDAT, hIHDR, 0),
    /* 6 cHRM */ CD(Some(png_handle_cHRM), 32, 32, hCOL, hIHDR, 0),
    /* 7 cICP */ CD(Some(png_handle_cICP), 4, 4, hCOL, hIHDR, 0),
    /* 8 cLLI */ CD(Some(png_handle_cLLI), 8, 8, hCOL, hIHDR, 0),
    /* 9 eXIf */ CD(Some(png_handle_eXIf), Limit, 4, 0, hIHDR, 0),
    /* 10 fcTL */ CD(None, 25, 26, 0, hIHDR, 1),
    /* 11 fdAT */ CD(None, Limit, 4, hIDAT, hIHDR, 1),
    /* 12 gAMA */ CD(Some(png_handle_gAMA), 4, 4, hCOL, hIHDR, 0),
    /* 13 hIST */ CD(Some(png_handle_hIST), 1024, 0, hPLTE, hIHDR, 0),
    /* 14 iCCP */ CD(Some(png_handle_iCCP), NoCheck, LKMin, hCOL, hIHDR, 0),
    /* 15 iTXt: allocates 'length+1'; checked in the handler */
    CD(Some(png_handle_iTXt), NoCheck, 6, 0, hIHDR, 1),
    /* 16 mDCV */ CD(Some(png_handle_mDCV), 24, 24, hCOL, hIHDR, 0),
    /* 17 oFFs */ CD(Some(png_handle_oFFs), 9, 9, hIDAT, hIHDR, 0),
    /* 18 pCAL: allocates 'length+1'; checked in the handler */
    CD(Some(png_handle_pCAL), NoCheck, 14, hIDAT, hIHDR, 0),
    /* 19 pHYs */ CD(Some(png_handle_pHYs), 9, 9, hIDAT, hIHDR, 0),
    /* 20 sBIT */ CD(Some(png_handle_sBIT), 4, 1, hCOL, hIHDR, 0),
    /* 21 sCAL: allocates 'length+1'; checked in the handler */
    CD(Some(png_handle_sCAL), Limit, 4, hIDAT, hIHDR, 0),
    /* 22 sPLT: allocates 'length+1'; checked in the handler */
    CD(Some(png_handle_sPLT), NoCheck, 3, hIDAT, hIHDR, 1),
    /* 23 sRGB */ CD(Some(png_handle_sRGB), 1, 1, hCOL, hIHDR, 0),
    /* 24 tEXt: allocates 'length+1'; checked in the handler */
    CD(Some(png_handle_tEXt), NoCheck, 2, 0, hIHDR, 1),
    /* 25 tIME */ CD(Some(png_handle_tIME), 7, 7, 0, hIHDR, 0),
    /* 26 tRNS */ CD(Some(png_handle_tRNS), 256, 0, hIDAT, hIHDR, 0),
    /* 27 zTXt */ CD(Some(png_handle_zTXt), Limit, LKMin, 0, hIHDR, 1),
];

pub unsafe fn png_chunk_index_from_name(chunk_name: png_uint_32) -> png_uint_32 {
    /* For chunk png_cHNK return PNG_INDEX_cHNK.  Return PNG_INDEX_unknown if
     * chunk_name is not known.  Notice that in a particular build "known" does
     * not necessarily mean "supported", although the inverse applies.
     */
    match chunk_name {
        png_IHDR => PNG_INDEX_IHDR,
        png_PLTE => PNG_INDEX_PLTE,
        png_IDAT => PNG_INDEX_IDAT,
        png_IEND => PNG_INDEX_IEND,
        png_acTL => PNG_INDEX_acTL,
        png_bKGD => PNG_INDEX_bKGD,
        png_cHRM => PNG_INDEX_cHRM,
        png_cICP => PNG_INDEX_cICP,
        png_cLLI => PNG_INDEX_cLLI,
        png_eXIf => PNG_INDEX_eXIf,
        png_fcTL => PNG_INDEX_fcTL,
        png_fdAT => PNG_INDEX_fdAT,
        png_gAMA => PNG_INDEX_gAMA,
        png_hIST => PNG_INDEX_hIST,
        png_iCCP => PNG_INDEX_iCCP,
        png_iTXt => PNG_INDEX_iTXt,
        png_mDCV => PNG_INDEX_mDCV,
        png_oFFs => PNG_INDEX_oFFs,
        png_pCAL => PNG_INDEX_pCAL,
        png_pHYs => PNG_INDEX_pHYs,
        png_sBIT => PNG_INDEX_sBIT,
        png_sCAL => PNG_INDEX_sCAL,
        png_sPLT => PNG_INDEX_sPLT,
        png_sRGB => PNG_INDEX_sRGB,
        png_tEXt => PNG_INDEX_tEXt,
        png_tIME => PNG_INDEX_tIME,
        png_tRNS => PNG_INDEX_tRNS,
        png_zTXt => PNG_INDEX_zTXt,
        _ => PNG_INDEX_unknown,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_handle_chunk(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> c_int {
    /* CSE: these things don't change, these autos are just to save typing and
     * make the code more clear.
     */
    let chunk_name: png_uint_32 = (*png_ptr).chunk_name;
    let chunk_index: png_uint_32 = png_chunk_index_from_name(chunk_name);

    let mut handled: c_int = handled_error;
    let mut errmsg: png_const_charp = core::ptr::null();

    /* Is this a known chunk?  If not there are no checks performed here;
     * png_handle_unknown does the correct checks.  This means that the values
     * for known but unsupported chunks in the above table are not used here
     * however the chunks_seen fields in png_struct are still set.
     */
    if chunk_index == PNG_INDEX_unknown
        || read_chunks[chunk_index as usize].handler.is_none()
    {
        handled = png_handle_unknown(png_ptr, info_ptr, length, PNG_HANDLE_CHUNK_AS_DEFAULT);
    }
    /* First check the position.   The first check is historical; the stream must
     * start with IHDR and anything else causes libpng to give up immediately.
     */
    else if chunk_index != PNG_INDEX_IHDR && ((*png_ptr).mode & PNG_HAVE_IHDR) == 0 {
        png_chunk_error(png_ptr, c"missing IHDR".as_ptr()); /* NORETURN */
    }
    /* Before all the pos_before chunks, after all the pos_after chunks. */
    else if ((*png_ptr).mode & read_chunks[chunk_index as usize].pos_before) != 0
        || ((*png_ptr).mode & read_chunks[chunk_index as usize].pos_after)
            != read_chunks[chunk_index as usize].pos_after
    {
        errmsg = c"out of place".as_ptr();
    }
    /* Now check for duplicates: duplicated critical chunks also produce a
     * full error.
     */
    else if read_chunks[chunk_index as usize].multiple == 0
        && ((*png_ptr).chunks & png_chunk_flag_from_index(chunk_index)) != 0
    {
        errmsg = c"duplicate".as_ptr();
    } else if length < read_chunks[chunk_index as usize].min_length {
        errmsg = c"too short".as_ptr();
    } else {
        /* NOTE: apart from IHDR the critical chunks (PLTE, IDAT and IEND) are set
         * up above not to do any length checks.
         *
         * The png_chunk_max check ensures that the variable length chunks are
         * always checked at this point for being within the system allocation
         * limits.
         */
        let max_length: c_uint = read_chunks[chunk_index as usize].max_length as c_uint;

        let mut meets_limit: bool = false;

        if max_length == Limit {
            /* png_read_chunk_header has already png_error'ed chunks with a
             * length exceeding the 31-bit PNG limit, so just check the memory
             * limit:
             */
            if length as png_alloc_size_t <= (*png_ptr).user_chunk_malloc_max {
                meets_limit = true;
            } else {
                errmsg = c"length exceeds libpng limit".as_ptr();
            }
        } else if max_length == NoCheck {
            meets_limit = true;
        } else {
            if length <= max_length {
                meets_limit = true;
            } else {
                errmsg = c"too long".as_ptr();
            }
        }

        if meets_limit {
            handled = (read_chunks[chunk_index as usize].handler.unwrap())(
                png_ptr, info_ptr, length,
            );
        }
    }

    /* If there was an error or the chunk was simply skipped it is not counted as
     * 'seen'.
     */
    if !errmsg.is_null() {
        if PNG_CHUNK_CRITICAL(chunk_name) {
            /* stop immediately */
            png_chunk_error(png_ptr, errmsg);
        } else
        /* ancillary chunk */
        {
            /* The chunk data is skipped: */
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, errmsg);
        }
    } else if handled >= handled_saved {
        if chunk_index != PNG_INDEX_unknown {
            (*png_ptr).chunks |= png_chunk_flag_from_index(chunk_index);
        }
    }

    handled
}
