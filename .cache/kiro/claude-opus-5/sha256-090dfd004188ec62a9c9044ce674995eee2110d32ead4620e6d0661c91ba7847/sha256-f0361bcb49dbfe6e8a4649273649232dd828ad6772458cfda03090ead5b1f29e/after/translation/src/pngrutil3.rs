//! Translation of c_src/src/pngrutil.c lines 2264..3710
use crate::prelude::*;

/// `PNG_FP_IS_POSITIVE(state)` (pngpriv.h)
#[inline]
fn PNG_FP_IS_POSITIVE(state: c_int) -> bool {
    (state & PNG_FP_NZ_MASK) == PNG_FP_Z_MASK
}

/* Read the sCAL chunk */
pub unsafe extern "C" fn png_handle_sCAL(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let buffer: png_bytep;
    let mut i: usize;
    let mut state: c_int;

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr(b"out of memory\0"));
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);
    *buffer.add(length as usize) = 0; /* Null terminate the last string */

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* Validate the unit. */
    if *buffer.add(0) != 1 && *buffer.add(0) != 2 {
        png_chunk_benign_error(png_ptr, cstr(b"invalid unit\0"));
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
        || {
            let old = i;
            i += 1;
            *buffer.add(old) != 0
        }
    {
        png_chunk_benign_error(png_ptr, cstr(b"bad width format\0"));
    } else if !PNG_FP_IS_POSITIVE(state) {
        png_chunk_benign_error(png_ptr, cstr(b"non-positive width\0"));
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
            png_chunk_benign_error(png_ptr, cstr(b"bad height format\0"));
        } else if !PNG_FP_IS_POSITIVE(state) {
            png_chunk_benign_error(png_ptr, cstr(b"non-positive height\0"));
        } else {
            /* This is the (only) success case. */
            png_set_sCAL_s(
                png_ptr,
                info_ptr,
                *buffer.add(0) as c_int,
                buffer.add(1) as png_charp,
                buffer.add(heighti) as png_charp,
            );
            return handled_ok;
        }
    }

    handled_error
}

pub unsafe extern "C" fn png_handle_tIME(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    _length: png_uint_32,
) -> png_handle_result_code {
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
    mod_time.year = png_get_uint_16(buf.as_ptr());

    png_set_tIME(png_ptr, info_ptr, &mod_time);
    handled_ok
}

/* Note: this does not properly handle chunks that are > 64K under DOS */
pub unsafe extern "C" fn png_handle_tEXt(
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

        (*png_ptr).user_chunk_cache_max -= 1;
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr(b"no space in chunk cache\0"));
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr(b"out of memory\0"));
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

    if png_set_text_2(png_ptr, info_ptr, &text_info, 1) == 0 {
        return handled_ok;
    }

    png_chunk_benign_error(png_ptr, cstr(b"out of memory\0"));
    handled_error
}

/* Note: this does not correctly handle chunks that are > 64K under DOS */
pub unsafe extern "C" fn png_handle_zTXt(
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

        (*png_ptr).user_chunk_cache_max -= 1;
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr(b"no space in chunk cache\0"));
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
        png_chunk_benign_error(png_ptr, cstr(b"out of memory\0"));
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* TODO: also check that the keyword contents match the spec! */
    keyword_length = 0;
    while keyword_length < length && *buffer.add(keyword_length as usize) != 0 {
        keyword_length += 1;
    }

    if keyword_length > 79 || keyword_length < 1 {
        errmsg = cstr(b"bad keyword\0");
    }
    /* zTXt must have some LZ data after the keyword, although it may expand to
     * zero bytes; we need a '\0' at the end of the keyword, the compression type
     * then the LZ data:
     */
    else if keyword_length + 3 > length {
        errmsg = cstr(b"truncated\0");
    } else if *buffer.add((keyword_length + 1) as usize) as c_int != PNG_COMPRESSION_TYPE_BASE {
        errmsg = cstr(b"unknown compression type\0");
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
            let mut text: png_text = core::mem::zeroed();

            if (*png_ptr).read_buffer.is_null() {
                errmsg = cstr(b"Read failure in png_handle_zTXt\0");
            } else {
                /* It worked; png_ptr->read_buffer now looks like a tEXt chunk
                 * except for the extra compression type byte and the fact that
                 * it isn't necessarily '\0' terminated.
                 */
                buffer = (*png_ptr).read_buffer;
                *buffer.add((uncompressed_length + (keyword_length as usize + 2)) as usize) = 0;

                text.compression = PNG_TEXT_COMPRESSION_zTXt;
                text.key = buffer as png_charp;
                text.text = buffer.add((keyword_length + 2) as usize) as png_charp;
                text.text_length = uncompressed_length;
                text.itxt_length = 0;
                text.lang = core::ptr::null_mut();
                text.lang_key = core::ptr::null_mut();

                if png_set_text_2(png_ptr, info_ptr, &text, 1) == 0 {
                    return handled_ok;
                }

                errmsg = cstr(b"out of memory\0");
            }
        } else {
            errmsg = (*png_ptr).zstream.msg;
        }
    }

    png_chunk_benign_error(png_ptr, errmsg);
    handled_error
}

/* Note: this does not correctly handle chunks that are > 64K under DOS */
pub unsafe extern "C" fn png_handle_iTXt(
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

        (*png_ptr).user_chunk_cache_max -= 1;
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr(b"no space in chunk cache\0"));
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr(b"out of memory\0"));
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* First the keyword. */
    prefix_length = 0;
    while prefix_length < length && *buffer.add(prefix_length as usize) != 0 {
        prefix_length += 1;
    }

    /* Perform a basic check on the keyword length here. */
    if prefix_length > 79 || prefix_length < 1 {
        errmsg = cstr(b"bad keyword\0");
    }
    /* Expect keyword, compression flag, compression type, language, translated
     * keyword (both may be empty but are 0 terminated) then the text, which may
     * be empty.
     */
    else if prefix_length + 5 > length {
        errmsg = cstr(b"truncated\0");
    } else if *buffer.add((prefix_length + 1) as usize) == 0
        || (*buffer.add((prefix_length + 1) as usize) == 1
            && *buffer.add((prefix_length + 2) as usize) as c_int == PNG_COMPRESSION_TYPE_BASE)
    {
        let compressed: c_int = (*buffer.add((prefix_length + 1) as usize) != 0) as c_int;
        let language_offset: png_uint_32;
        let translated_keyword_offset: png_uint_32;
        let mut uncompressed_length: png_alloc_size_t = 0;

        /* Now the language tag */
        prefix_length += 3;
        language_offset = prefix_length;

        while prefix_length < length && *buffer.add(prefix_length as usize) != 0 {
            prefix_length += 1;
        }

        /* WARNING: the length may be invalid here, this is checked below. */
        prefix_length += 1;
        translated_keyword_offset = prefix_length;

        while prefix_length < length && *buffer.add(prefix_length as usize) != 0 {
            prefix_length += 1;
        }

        /* prefix_length should now be at the trailing '\0' of the translated
         * keyword, but it may already be over the end.  None of this arithmetic
         * can overflow because chunks are at most 2^31 bytes long, but on 16-bit
         * systems the available allocation may overflow.
         */
        prefix_length += 1;

        if compressed == 0 && prefix_length <= length {
            uncompressed_length = (length - prefix_length) as png_alloc_size_t;
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
            errmsg = cstr(b"truncated\0");
        }

        if errmsg.is_null() {
            let mut text: png_text = core::mem::zeroed();

            *buffer.add((uncompressed_length + prefix_length as usize) as usize) = 0;

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

            errmsg = cstr(b"out of memory\0");
        }
    } else {
        errmsg = cstr(b"bad compression info\0");
    }

    if !errmsg.is_null() {
        png_chunk_benign_error(png_ptr, errmsg);
    }
    handled_error
}

/* Utility function for png_handle_unknown; set up png_ptr::unknown_chunk */
pub unsafe extern "C" fn png_cache_unknown_chunk(
    png_ptr: png_structrp,
    length: png_uint_32,
) -> c_int {
    let limit: png_alloc_size_t = png_chunk_max(png_ptr);

    if !(*png_ptr).unknown_chunk.data.is_null() {
        png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
        (*png_ptr).unknown_chunk.data = core::ptr::null_mut();
    }

    if length as png_alloc_size_t <= limit {
        PNG_CSTRING_FROM_CHUNK(
            (*png_ptr).unknown_chunk.name.as_mut_ptr() as *mut c_char,
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
        png_chunk_benign_error(png_ptr, cstr(b"unknown chunk exceeds memory limits\0"));
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
pub unsafe extern "C" fn png_handle_unknown(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
    mut keep: c_int,
) -> png_handle_result_code {
    let mut handled: png_handle_result_code = handled_discarded; /* the default */

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
            let f = (*png_ptr).read_user_chunk_fn.unwrap();
            let ret: c_int = f(png_ptr as png_structp, &mut (*png_ptr).unknown_chunk);

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
                png_chunk_error(png_ptr, cstr(b"error in user chunk\0"));
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
                        png_chunk_warning(png_ptr, cstr(b"Saving unknown chunk:\0"));
                        png_app_warning(
                            png_ptr,
                            cstr(
                                b"forcing save of an unhandled chunk; please call png_set_keep_unknown_chunks\0",
                            ),
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
    } else {
        /* Use the SAVE_UNKNOWN_CHUNKS code or skip the chunk */
        /* keep is currently just the per-chunk setting, if there was no
         * setting change it to the global default now (not that this may
         * still be AS_DEFAULT) then obtain the cache of the chunk if required,
         * if not simply skip the chunk.
         */
        if keep == PNG_HANDLE_CHUNK_AS_DEFAULT {
            keep = (*png_ptr).unknown_default;
        }

        if keep == PNG_HANDLE_CHUNK_ALWAYS
            || (keep == PNG_HANDLE_CHUNK_IF_SAFE && PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name) != 0)
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
        let mut store = true;
        match (*png_ptr).user_chunk_cache_max {
            2 => {
                (*png_ptr).user_chunk_cache_max = 1;
                png_chunk_benign_error(png_ptr, cstr(b"no space in chunk cache\0"));
                /* FALLTHROUGH */
                /* NOTE: prior to 1.6.0 this case resulted in an unknown critical
                 * chunk being skipped, now there will be a hard error below.
                 */
                store = false;
            }
            1 => {
                /* NOTE: prior to 1.6.0 this case resulted in an unknown critical
                 * chunk being skipped, now there will be a hard error below.
                 */
                store = false;
            }
            0 => { /* no limit: store the chunk (fall through to store) */ }
            _ => {
                /* not at limit */
                (*png_ptr).user_chunk_cache_max -= 1;
                /* FALLTHROUGH: store the chunk */
            }
        }

        if store {
            /* Here when the limit isn't reached or when limits are compiled
             * out; store the chunk.
             */
            png_set_unknown_chunks(png_ptr, info_ptr, &(*png_ptr).unknown_chunk, 1);
            handled = handled_saved;
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
    if handled < handled_saved && PNG_CHUNK_CRITICAL((*png_ptr).chunk_name) != 0 {
        png_chunk_error(png_ptr, cstr(b"unhandled critical chunk\0"));
    }

    handled
}

/* APNG handling: acTL, fcTL and fdAT are all handled as unknown chunks
 * (png_handle_acTL/fcTL/fdAT are NULL).
 */

/*
 * 1.6.47: This is the new table driven interface to all the chunk handling.
 *
 * The table describes the PNG standard rules for **reading** known chunks.
 */
struct ReadChunk {
    handler: Option<
        unsafe extern "C" fn(png_structrp, png_inforp, png_uint_32) -> png_handle_result_code,
    >,
    max_length: png_uint_32, /* :12 Length min, max in bytes */
    min_length: png_uint_32, /* :8 */
    pos_before: png_uint_32, /* :4 PNG_HAVE_ values chunk must precede */
    pos_after: png_uint_32,  /* :4 PNG_HAVE_ values chunk must follow */
    multiple: png_uint_32,   /* :1 Multiple occurrences permitted */
}

/* NoCheck / Limit sentinels for max_length. */
const NoCheck: png_uint_32 = 0x801;
const Limit: png_uint_32 = 0x802;
/* LZ77Min == (2U+5U+4U) == 11; LKMin == 3U+LZ77Min */
const LKMin: png_uint_32 = 3 + (2 + 5 + 4);

/* PNG_HAVE_ combinations used in the table (see pngpriv.h). */
const hIHDR: png_uint_32 = PNG_HAVE_IHDR;
const hPLTE: png_uint_32 = PNG_HAVE_PLTE;
const hIDAT: png_uint_32 = PNG_HAVE_IDAT;
const hCOL: png_uint_32 = PNG_HAVE_PLTE | PNG_HAVE_IDAT;
const aIDAT: png_uint_32 = PNG_AFTER_IDAT;

/* read_chunks[PNG_INDEX_unknown], ordered exactly as PNG_KNOWN_CHUNKS.
 *   { handler, max_length, min_length, pos_before, pos_after, multiple }
 */
static read_chunks: [ReadChunk; PNG_INDEX_unknown as usize] = [
    /* IHDR */
    ReadChunk {
        handler: Some(png_handle_IHDR),
        max_length: 13,
        min_length: 13,
        pos_before: hIHDR,
        pos_after: 0,
        multiple: 0,
    },
    /* PLTE */
    ReadChunk {
        handler: Some(png_handle_PLTE),
        max_length: NoCheck,
        min_length: 0,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* IDAT */
    ReadChunk {
        handler: None,
        max_length: NoCheck,
        min_length: 0,
        pos_before: aIDAT,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* IEND */
    ReadChunk {
        handler: Some(png_handle_IEND),
        max_length: NoCheck,
        min_length: 0,
        pos_before: 0,
        pos_after: aIDAT,
        multiple: 0,
    },
    /* acTL */
    ReadChunk {
        handler: None,
        max_length: 8,
        min_length: 8,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* bKGD */
    ReadChunk {
        handler: Some(png_handle_bKGD),
        max_length: 6,
        min_length: 1,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* cHRM */
    ReadChunk {
        handler: Some(png_handle_cHRM),
        max_length: 32,
        min_length: 32,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* cICP */
    ReadChunk {
        handler: Some(png_handle_cICP),
        max_length: 4,
        min_length: 4,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* cLLI */
    ReadChunk {
        handler: Some(png_handle_cLLI),
        max_length: 8,
        min_length: 8,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* eXIf */
    ReadChunk {
        handler: Some(png_handle_eXIf),
        max_length: Limit,
        min_length: 4,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* fcTL */
    ReadChunk {
        handler: None,
        max_length: 25,
        min_length: 26,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* fdAT */
    ReadChunk {
        handler: None,
        max_length: Limit,
        min_length: 4,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* gAMA */
    ReadChunk {
        handler: Some(png_handle_gAMA),
        max_length: 4,
        min_length: 4,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* hIST */
    ReadChunk {
        handler: Some(png_handle_hIST),
        max_length: 1024,
        min_length: 0,
        pos_before: hPLTE,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* iCCP */
    ReadChunk {
        handler: Some(png_handle_iCCP),
        max_length: NoCheck,
        min_length: LKMin,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* iTXt */
    ReadChunk {
        handler: Some(png_handle_iTXt),
        max_length: NoCheck,
        min_length: 6,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* mDCV */
    ReadChunk {
        handler: Some(png_handle_mDCV),
        max_length: 24,
        min_length: 24,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* oFFs */
    ReadChunk {
        handler: Some(png_handle_oFFs),
        max_length: 9,
        min_length: 9,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* pCAL */
    ReadChunk {
        handler: Some(png_handle_pCAL),
        max_length: NoCheck,
        min_length: 14,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* pHYs */
    ReadChunk {
        handler: Some(png_handle_pHYs),
        max_length: 9,
        min_length: 9,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* sBIT */
    ReadChunk {
        handler: Some(png_handle_sBIT),
        max_length: 4,
        min_length: 1,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* sCAL */
    ReadChunk {
        handler: Some(png_handle_sCAL),
        max_length: Limit,
        min_length: 4,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* sPLT */
    ReadChunk {
        handler: Some(png_handle_sPLT),
        max_length: NoCheck,
        min_length: 3,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* sRGB */
    ReadChunk {
        handler: Some(png_handle_sRGB),
        max_length: 1,
        min_length: 1,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* tEXt */
    ReadChunk {
        handler: Some(png_handle_tEXt),
        max_length: NoCheck,
        min_length: 2,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* tIME */
    ReadChunk {
        handler: Some(png_handle_tIME),
        max_length: 7,
        min_length: 7,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* tRNS */
    ReadChunk {
        handler: Some(png_handle_tRNS),
        max_length: 256,
        min_length: 0,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* zTXt */
    ReadChunk {
        handler: Some(png_handle_zTXt),
        max_length: Limit,
        min_length: LKMin,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 1,
    },
];

pub unsafe extern "C" fn png_chunk_index_from_name(chunk_name: png_uint_32) -> c_int {
    /* For chunk png_cHNK return PNG_INDEX_cHNK.  Return PNG_INDEX_unknown if
     * chunk_name is not known.  Notice that in a particular build "known" does
     * not necessarily mean "supported", although the inverse applies.
     */
    match chunk_name {
        x if x == png_IHDR => PNG_INDEX_IHDR,
        x if x == png_PLTE => PNG_INDEX_PLTE,
        x if x == png_IDAT => PNG_INDEX_IDAT,
        x if x == png_IEND => PNG_INDEX_IEND,
        x if x == png_acTL => PNG_INDEX_acTL,
        x if x == png_bKGD => PNG_INDEX_bKGD,
        x if x == png_cHRM => PNG_INDEX_cHRM,
        x if x == png_cICP => PNG_INDEX_cICP,
        x if x == png_cLLI => PNG_INDEX_cLLI,
        x if x == png_eXIf => PNG_INDEX_eXIf,
        x if x == png_fcTL => PNG_INDEX_fcTL,
        x if x == png_fdAT => PNG_INDEX_fdAT,
        x if x == png_gAMA => PNG_INDEX_gAMA,
        x if x == png_hIST => PNG_INDEX_hIST,
        x if x == png_iCCP => PNG_INDEX_iCCP,
        x if x == png_iTXt => PNG_INDEX_iTXt,
        x if x == png_mDCV => PNG_INDEX_mDCV,
        x if x == png_oFFs => PNG_INDEX_oFFs,
        x if x == png_pCAL => PNG_INDEX_pCAL,
        x if x == png_pHYs => PNG_INDEX_pHYs,
        x if x == png_sBIT => PNG_INDEX_sBIT,
        x if x == png_sCAL => PNG_INDEX_sCAL,
        x if x == png_sPLT => PNG_INDEX_sPLT,
        x if x == png_sRGB => PNG_INDEX_sRGB,
        x if x == png_tEXt => PNG_INDEX_tEXt,
        x if x == png_tIME => PNG_INDEX_tIME,
        x if x == png_tRNS => PNG_INDEX_tRNS,
        x if x == png_zTXt => PNG_INDEX_zTXt,
        _ => PNG_INDEX_unknown,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_handle_chunk(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    /* CSE: these things don't change, these autos are just to save typing and
     * make the code more clear.
     */
    let chunk_name: png_uint_32 = (*png_ptr).chunk_name;
    let chunk_index: c_int = png_chunk_index_from_name(chunk_name);

    let mut handled: png_handle_result_code = handled_error;
    let mut errmsg: png_const_charp = core::ptr::null();

    /* Is this a known chunk?  If not there are no checks performed here;
     * png_handle_unknown does the correct checks.  This means that the values
     * for known but unsupported chunks in the above table are not used here
     * however the chunks_seen fields in png_struct are still set.
     */
    if chunk_index == PNG_INDEX_unknown || read_chunks[chunk_index as usize].handler.is_none() {
        handled = png_handle_unknown(png_ptr, info_ptr, length, PNG_HANDLE_CHUNK_AS_DEFAULT);
    }
    /* First check the position.   The first check is historical; the stream must
     * start with IHDR and anything else causes libpng to give up immediately.
     */
    else if chunk_index != PNG_INDEX_IHDR && ((*png_ptr).mode & PNG_HAVE_IHDR) == 0 {
        png_chunk_error(png_ptr, cstr(b"missing IHDR\0")); /* NORETURN */
    }
    /* Before all the pos_before chunks, after all the pos_after chunks. */
    else if ((*png_ptr).mode & read_chunks[chunk_index as usize].pos_before) != 0
        || ((*png_ptr).mode & read_chunks[chunk_index as usize].pos_after)
            != read_chunks[chunk_index as usize].pos_after
    {
        errmsg = cstr(b"out of place\0");
    }
    /* Now check for duplicates: duplicated critical chunks also produce a
     * full error.
     */
    else if read_chunks[chunk_index as usize].multiple == 0
        && png_file_has_chunk(png_ptr, chunk_index)
    {
        errmsg = cstr(b"duplicate\0");
    } else if length < read_chunks[chunk_index as usize].min_length {
        errmsg = cstr(b"too short\0");
    } else {
        /* NOTE: apart from IHDR the critical chunks (PLTE, IDAT and IEND) are set
         * up above not to do any length checks.
         *
         * The png_chunk_max check ensures that the variable length chunks are
         * always checked at this point for being within the system allocation
         * limits.
         */
        let max_length: png_uint_32 = read_chunks[chunk_index as usize].max_length;

        /* Emulate the C switch with its 'goto MeetsLimit' target. */
        let mut meets_limit = false;

        if max_length == Limit {
            /* png_read_chunk_header has already png_error'ed chunks with a
             * length exceeding the 31-bit PNG limit, so just check the memory
             * limit:
             */
            if length as png_alloc_size_t <= png_chunk_max(png_ptr) {
                meets_limit = true;
            } else {
                errmsg = cstr(b"length exceeds libpng limit\0");
            }
        } else if max_length == NoCheck {
            meets_limit = true;
        } else {
            /* default */
            if length <= max_length {
                meets_limit = true;
            } else {
                errmsg = cstr(b"too long\0");
            }
        }

        if meets_limit {
            handled =
                (read_chunks[chunk_index as usize].handler.unwrap())(png_ptr, info_ptr, length);
        }
    }

    /* If there was an error or the chunk was simply skipped it is not counted as
     * 'seen'.
     */
    if !errmsg.is_null() {
        if PNG_CHUNK_CRITICAL(chunk_name) != 0
        /* stop immediately */
        {
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
            png_file_add_chunk(png_ptr, chunk_index);
        }
    }

    handled
}

/* Pre-computed masks (PNG_USE_COMPILE_TIME_MASKS == 1).
 * row_mask[2 (PACKSWAP)][3 (depth)][6 (pass)]
 */
static row_mask: [[[png_uint_32; 6]; 3]; 2] = [
    /* Little-endian byte masks for PACKSWAP */
    [
        [
            0x01010101, 0x10101010, 0x11111111, 0x44444444, 0x55555555, 0xaaaaaaaa,
        ],
        [
            0x00030003, 0x03000300, 0x03030303, 0x30303030, 0x33333333, 0xcccccccc,
        ],
        [
            0x0000000f, 0x000f0000, 0x000f000f, 0x0f000f00, 0x0f0f0f0f, 0xf0f0f0f0,
        ],
    ],
    /* Normal (big-endian byte) masks - PNG format */
    [
        [
            0x80808080, 0x08080808, 0x88888888, 0x22222222, 0xaaaaaaaa, 0x55555555,
        ],
        [
            0x00c000c0, 0xc000c000, 0xc0c0c0c0, 0x0c0c0c0c, 0xcccccccc, 0x33333333,
        ],
        [
            0x000000f0, 0x00f00000, 0x00f000f0, 0xf000f000, 0xf0f0f0f0, 0x0f0f0f0f,
        ],
    ],
];

/* display_mask[2 (PACKSWAP)][3 (depth)][3 (odd passes)] */
static display_mask: [[[png_uint_32; 3]; 3]; 2] = [
    /* Little-endian byte masks for PACKSWAP */
    [
        [0xf0f0f0f0, 0xcccccccc, 0xaaaaaaaa],
        [0xff00ff00, 0xf0f0f0f0, 0xcccccccc],
        [0xffff0000, 0xff00ff00, 0xf0f0f0f0],
    ],
    /* Normal (big-endian byte) masks - PNG format */
    [
        [0x0f0f0f0f, 0x33333333, 0x55555555],
        [0xff00ff00, 0x0f0f0f0f, 0x33333333],
        [0xffff0000, 0xff00ff00, 0x0f0f0f0f],
    ],
];

/// `DEPTH_INDEX(d)` == (d)==1?0:((d)==2?1:2)
#[inline]
fn DEPTH_INDEX(d: c_uint) -> usize {
    if d == 1 {
        0
    } else if d == 2 {
        1
    } else {
        2
    }
}

/// `MASK(pass,depth,display,png)`
#[inline]
fn MASK(pass: c_uint, depth: c_uint, display: c_int, png: usize) -> png_uint_32 {
    if display != 0 {
        display_mask[png][DEPTH_INDEX(depth)][(pass >> 1) as usize]
    } else {
        row_mask[png][DEPTH_INDEX(depth)][pass as usize]
    }
}

/* Combines the row recently read in with the existing pixels in the row. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_combine_row(
    png_ptr: png_const_structrp,
    mut dp: png_bytep,
    display: c_int,
) {
    let mut pixel_depth: c_uint = (*png_ptr).transformed_pixel_depth as c_uint;
    let mut sp: png_const_bytep = (*png_ptr).row_buf.add(1);
    let mut row_width: png_alloc_size_t = (*png_ptr).width as png_alloc_size_t;
    let pass: c_uint = (*png_ptr).pass as c_uint;
    let mut end_ptr: png_bytep = core::ptr::null_mut();
    let mut end_byte: png_byte = 0;
    let mut end_mask: c_uint;

    /* Added in 1.5.6: it should not be possible to enter this routine until at
     * least one row has been read from the PNG data and transformed.
     */
    if pixel_depth == 0 {
        png_error(png_ptr, cstr(b"internal row logic error\0"));
    }

    /* Added in 1.5.4: the pixel depth should match the information returned by
     * any call to png_read_update_info at this point.  Do not continue if we got
     * this wrong.
     */
    if (*png_ptr).info_rowbytes != 0
        && (*png_ptr).info_rowbytes != PNG_ROWBYTES(pixel_depth as usize, row_width)
    {
        png_error(png_ptr, cstr(b"internal row size calculation error\0"));
    }

    /* Don't expect this to ever happen: */
    if row_width == 0 {
        png_error(png_ptr, cstr(b"internal row width error\0"));
    }

    /* Preserve the last byte in cases where only part of it will be overwritten,
     * the multiply below may overflow, we don't care because ANSI-C guarantees
     * we get the low bits.
     */
    end_mask = (pixel_depth.wrapping_mul(row_width as c_uint)) & 7;
    if end_mask != 0 {
        /* end_ptr == NULL is a flag to say do nothing */
        end_ptr = dp.add(PNG_ROWBYTES(pixel_depth as usize, row_width)).sub(1);
        end_byte = *end_ptr;
        if ((*png_ptr).transformations & PNG_PACKSWAP) != 0 {
            /* little-endian byte */
            end_mask = (0xffi32 << end_mask) as c_uint;
        } else {
            /* big-endian byte */
            end_mask = 0xffu32 >> end_mask;
        }
        /* end_mask is now the bits to *keep* from the destination row */
    }

    /* For non-interlaced images this reduces to a memcpy(). */
    if (*png_ptr).interlaced != 0
        && ((*png_ptr).transformations & PNG_INTERLACE) != 0
        && pass < 6
        && (display == 0 ||
            /* The following copies everything for 'display' on passes 0, 2 and 4. */
            (display == 1 && (pass & 1) != 0))
    {
        /* Narrow images may have no bits in a pass; the caller should handle
         * this, but this test is cheap:
         */
        if row_width <= PNG_PASS_START_COL(pass as c_int) as png_alloc_size_t {
            return;
        }

        if pixel_depth < 8 {
            /* Use the appropriate mask to copy the required bits. */
            let pixels_per_byte: png_uint_32 = 8 / pixel_depth;
            let mut mask: png_uint_32;

            if ((*png_ptr).transformations & PNG_PACKSWAP) != 0 {
                mask = MASK(pass, pixel_depth, display, 0);
            } else {
                mask = MASK(pass, pixel_depth, display, 1);
            }

            loop {
                /* It doesn't matter in the following if png_uint_32 has more than
                 * 32 bits because the high bits always match those in m<<24; it is,
                 * however, essential to use OR here, not +, because of this.
                 */
                let mut m: png_uint_32 = mask;
                mask = (m >> 8) | (m << 24); /* rotate right to good compilers */
                m &= 0xff;

                if m != 0
                /* something to copy */
                {
                    if m != 0xff {
                        *dp = ((*dp as png_uint_32 & !m) | (*sp as png_uint_32 & m)) as png_byte;
                    } else {
                        *dp = *sp;
                    }
                }

                /* NOTE: this may overwrite the last byte with garbage if the image
                 * is not an exact number of bytes wide; libpng has always done
                 * this.
                 */
                if row_width <= pixels_per_byte as png_alloc_size_t {
                    break; /* May need to restore part of the last byte */
                }

                row_width -= pixels_per_byte as png_alloc_size_t;
                dp = dp.add(1);
                sp = sp.add(1);
            }
        } else
        /* pixel_depth >= 8 */
        {
            let mut bytes_to_copy: c_uint;
            let bytes_to_jump: c_uint;

            /* Validate the depth - it must be a multiple of 8 */
            if pixel_depth & 7 != 0 {
                png_error(png_ptr, cstr(b"invalid user transform pixel depth\0"));
            }

            pixel_depth >>= 3; /* now in bytes */
            row_width *= pixel_depth as png_alloc_size_t;

            /* Regardless of pass number the Adam 7 interlace always results in a
             * fixed number of pixels to copy then to skip.
             */
            {
                let offset: c_uint = PNG_PASS_START_COL(pass as c_int) as c_uint * pixel_depth;

                row_width -= offset as png_alloc_size_t;
                dp = dp.add(offset as usize);
                sp = sp.add(offset as usize);
            }

            /* Work out the bytes to copy. */
            if display != 0 {
                /* When doing the 'block' algorithm the pixel in the pass gets
                 * replicated to adjacent pixels.
                 */
                bytes_to_copy = (1u32 << ((6 - pass) >> 1)) * pixel_depth;

                /* But don't allow this number to exceed the actual row width. */
                if bytes_to_copy as png_alloc_size_t > row_width {
                    bytes_to_copy = row_width as c_uint /*SAFE*/;
                }
            } else
            /* normal row; Adam7 only ever gives us one pixel to copy. */
            {
                bytes_to_copy = pixel_depth;
            }

            /* In Adam7 there is a constant offset between where the pixels go. */
            bytes_to_jump = PNG_PASS_COL_OFFSET(pass as c_int) as c_uint * pixel_depth;

            /* And simply copy these bytes. */
            match bytes_to_copy {
                1 => loop {
                    *dp = *sp;

                    if row_width <= bytes_to_jump as png_alloc_size_t {
                        return;
                    }

                    dp = dp.add(bytes_to_jump as usize);
                    sp = sp.add(bytes_to_jump as usize);
                    row_width -= bytes_to_jump as png_alloc_size_t;
                },

                2 => {
                    /* There is a possibility of a partial copy at the end here. */
                    loop {
                        *dp.add(0) = *sp.add(0);
                        *dp.add(1) = *sp.add(1);

                        if row_width <= bytes_to_jump as png_alloc_size_t {
                            return;
                        }

                        sp = sp.add(bytes_to_jump as usize);
                        dp = dp.add(bytes_to_jump as usize);
                        row_width -= bytes_to_jump as png_alloc_size_t;

                        if row_width <= 1 {
                            break;
                        }
                    }

                    /* And there can only be one byte left at this point: */
                    *dp = *sp;
                    return;
                }

                3 => loop {
                    /* This can only be the RGB case, so each copy is exactly one
                     * pixel and it is not necessary to check for a partial copy.
                     */
                    *dp.add(0) = *sp.add(0);
                    *dp.add(1) = *sp.add(1);
                    *dp.add(2) = *sp.add(2);

                    if row_width <= bytes_to_jump as png_alloc_size_t {
                        return;
                    }

                    sp = sp.add(bytes_to_jump as usize);
                    dp = dp.add(bytes_to_jump as usize);
                    row_width -= bytes_to_jump as png_alloc_size_t;
                },

                _ => {
                    /* Check for double byte alignment and, if possible, use a
                     * 16-bit copy.
                     */
                    if bytes_to_copy < 16
                        && png_isaligned::<png_uint_16>(dp as *const u8)
                        && png_isaligned::<png_uint_16>(sp as *const u8)
                        && bytes_to_copy as usize % core::mem::size_of::<png_uint_16>() == 0
                        && bytes_to_jump as usize % core::mem::size_of::<png_uint_16>() == 0
                    {
                        /* Everything is aligned for png_uint_16 copies, but try for
                         * png_uint_32 first.
                         */
                        if png_isaligned::<png_uint_32>(dp as *const u8)
                            && png_isaligned::<png_uint_32>(sp as *const u8)
                            && bytes_to_copy as usize % core::mem::size_of::<png_uint_32>() == 0
                            && bytes_to_jump as usize % core::mem::size_of::<png_uint_32>() == 0
                        {
                            let mut dp32: png_uint_32p = dp as png_uint_32p;
                            let mut sp32: png_const_uint_32p = sp as png_const_uint_32p;
                            let skip: usize = (bytes_to_jump - bytes_to_copy) as usize
                                / core::mem::size_of::<png_uint_32>();

                            loop {
                                let mut c: usize = bytes_to_copy as usize;
                                loop {
                                    *dp32 = *sp32;
                                    dp32 = dp32.add(1);
                                    sp32 = sp32.add(1);
                                    c -= core::mem::size_of::<png_uint_32>();
                                    if c == 0 {
                                        break;
                                    }
                                }

                                if row_width <= bytes_to_jump as png_alloc_size_t {
                                    return;
                                }

                                dp32 = dp32.add(skip);
                                sp32 = sp32.add(skip);
                                row_width -= bytes_to_jump as png_alloc_size_t;

                                if !(bytes_to_copy as png_alloc_size_t <= row_width) {
                                    break;
                                }
                            }

                            /* Get to here when the row_width truncates the final copy.
                             * There will be 1-3 bytes left to copy.
                             */
                            dp = dp32 as png_bytep;
                            sp = sp32 as png_const_bytep;
                            loop {
                                *dp = *sp;
                                dp = dp.add(1);
                                sp = sp.add(1);
                                row_width -= 1;
                                if row_width == 0 {
                                    break;
                                }
                            }
                            return;
                        }
                        /* Else do it in 16-bit quantities, but only if the size is
                         * not too large.
                         */
                        else {
                            let mut dp16: png_uint_16p = dp as png_uint_16p;
                            let mut sp16: png_const_uint_16p = sp as png_const_uint_16p;
                            let skip: usize = (bytes_to_jump - bytes_to_copy) as usize
                                / core::mem::size_of::<png_uint_16>();

                            loop {
                                let mut c: usize = bytes_to_copy as usize;
                                loop {
                                    *dp16 = *sp16;
                                    dp16 = dp16.add(1);
                                    sp16 = sp16.add(1);
                                    c -= core::mem::size_of::<png_uint_16>();
                                    if c == 0 {
                                        break;
                                    }
                                }

                                if row_width <= bytes_to_jump as png_alloc_size_t {
                                    return;
                                }

                                dp16 = dp16.add(skip);
                                sp16 = sp16.add(skip);
                                row_width -= bytes_to_jump as png_alloc_size_t;

                                if !(bytes_to_copy as png_alloc_size_t <= row_width) {
                                    break;
                                }
                            }

                            /* End of row - 1 byte left, bytes_to_copy > row_width: */
                            dp = dp16 as png_bytep;
                            sp = sp16 as png_const_bytep;
                            loop {
                                *dp = *sp;
                                dp = dp.add(1);
                                sp = sp.add(1);
                                row_width -= 1;
                                if row_width == 0 {
                                    break;
                                }
                            }
                            return;
                        }
                    }

                    /* The true default - use a memcpy: */
                    loop {
                        memcpy(
                            dp as *mut c_void,
                            sp as *const c_void,
                            bytes_to_copy as usize,
                        );

                        if row_width <= bytes_to_jump as png_alloc_size_t {
                            return;
                        }

                        sp = sp.add(bytes_to_jump as usize);
                        dp = dp.add(bytes_to_jump as usize);
                        row_width -= bytes_to_jump as png_alloc_size_t;
                        if bytes_to_copy as png_alloc_size_t > row_width {
                            bytes_to_copy = row_width as c_uint /*SAFE*/;
                        }
                    }
                }
            }
            /* NOT REACHED */
        } /* pixel_depth >= 8 */

    /* Here if pixel_depth < 8 to check 'end_ptr' below. */
    } else {
        /* If here then the switch above wasn't used so just memcpy the whole row
         * from the temporary row buffer (notice that this overwrites the end of
         * the destination row if it is a partial byte.)
         */
        memcpy(
            dp as *mut c_void,
            sp as *const c_void,
            PNG_ROWBYTES(pixel_depth as usize, row_width),
        );
    }

    /* Restore the overwritten bits from the last byte if necessary. */
    if !end_ptr.is_null() {
        *end_ptr = ((end_byte as c_uint & end_mask) | (*end_ptr as c_uint & !end_mask)) as png_byte;
    }
}
