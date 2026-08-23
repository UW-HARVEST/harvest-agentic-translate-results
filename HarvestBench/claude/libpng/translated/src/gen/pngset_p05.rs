/* pngset.c lines 1557..2057 */

/* png_permit_mng_features */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_permit_mng_features(
    png_ptr: png_structrp,
    mng_features: png_uint_32,
) -> png_uint_32 {
    if png_ptr == core::ptr::null_mut() {
        return 0;
    }

    (*png_ptr).mng_features_permitted = mng_features & PNG_ALL_MNG_FEATURES;

    (*png_ptr).mng_features_permitted
}

/* add_one_chunk */
unsafe fn add_one_chunk(
    mut list: png_bytep,
    mut count: c_uint,
    add: png_const_bytep,
    keep: c_int,
) -> c_uint {
    let mut i: c_uint;

    /* Utility function: update the 'keep' state of a chunk if it is already in
     * the list, otherwise add it to the list.
     */
    i = 0;
    while i < count {
        if memcmp(list as *const c_void, add as *const c_void, 4) == 0 {
            *list.add(4) = keep as png_byte;

            return count;
        }

        i += 1;
        list = list.add(5);
    }

    if keep != PNG_HANDLE_CHUNK_AS_DEFAULT {
        count += 1;
        memcpy(list as *mut c_void, add as *const c_void, 4);
        *list.add(4) = keep as png_byte;
    }

    count
}

/* Ignore all unknown chunks and all chunks recognized by
 * libpng except for IHDR, PLTE, tRNS, IDAT, and IEND
 *
 * (hoisted out of png_set_keep_unknown_chunks)
 */
static chunks_to_ignore: [png_byte; 105] = [
    98, 75, 71, 68, b'\0', /* bKGD */
    99, 72, 82, 77, b'\0', /* cHRM */
    99, 73, 67, 80, b'\0', /* cICP */
    99, 76, 76, 73, b'\0', /* cLLI */
    101, 88, 73, 102, b'\0', /* eXIf */
    103, 65, 77, 65, b'\0', /* gAMA */
    104, 73, 83, 84, b'\0', /* hIST */
    105, 67, 67, 80, b'\0', /* iCCP */
    105, 84, 88, 116, b'\0', /* iTXt */
    109, 68, 67, 86, b'\0', /* mDCV */
    111, 70, 70, 115, b'\0', /* oFFs */
    112, 67, 65, 76, b'\0', /* pCAL */
    112, 72, 89, 115, b'\0', /* pHYs */
    115, 66, 73, 84, b'\0', /* sBIT */
    115, 67, 65, 76, b'\0', /* sCAL */
    115, 80, 76, 84, b'\0', /* sPLT */
    115, 84, 69, 82, b'\0', /* sTER */
    115, 82, 71, 66, b'\0', /* sRGB */
    116, 69, 88, 116, b'\0', /* tEXt */
    116, 73, 77, 69, b'\0', /* tIME */
    122, 84, 88, 116, b'\0', /* zTXt */
];

/* png_set_keep_unknown_chunks */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_keep_unknown_chunks(
    png_ptr: png_structrp,
    keep: c_int,
    mut chunk_list: png_const_bytep,
    num_chunks_in: c_int,
) {
    let mut new_list: png_bytep;
    let mut num_chunks: c_uint;
    let mut old_num_chunks: c_uint;

    if png_ptr == core::ptr::null_mut() {
        return;
    }

    if keep < 0 || keep >= PNG_HANDLE_CHUNK_LAST {
        png_app_error(
            png_ptr,
            b"png_set_keep_unknown_chunks: invalid keep\0".as_ptr() as png_const_charp,
        );

        return;
    }

    if num_chunks_in <= 0 {
        (*png_ptr).unknown_default = keep;

        /* '0' means just set the flags, so stop here */
        if num_chunks_in == 0 {
            return;
        }
    }

    if num_chunks_in < 0 {
        /* Ignore all unknown chunks and all chunks recognized by
         * libpng except for IHDR, PLTE, tRNS, IDAT, and IEND
         */
        chunk_list = chunks_to_ignore.as_ptr();
        num_chunks = (core::mem::size_of_val(&chunks_to_ignore) as c_uint) / 5u32;
    } else
    /* num_chunks_in > 0 */
    {
        if chunk_list == core::ptr::null() {
            /* Prior to 1.6.0 this was silently ignored, now it is an app_error
             * which can be switched off.
             */
            png_app_error(
                png_ptr,
                b"png_set_keep_unknown_chunks: no chunk list\0".as_ptr() as png_const_charp,
            );

            return;
        }

        num_chunks = num_chunks_in as c_uint;
    }

    old_num_chunks = (*png_ptr).num_chunk_list;
    if (*png_ptr).chunk_list == core::ptr::null_mut() {
        old_num_chunks = 0;
    }

    /* Since num_chunks is always restricted to UINT_MAX/5 this can't overflow.
     */
    if num_chunks.wrapping_add(old_num_chunks) > c_uint::MAX / 5 {
        png_app_error(
            png_ptr,
            b"png_set_keep_unknown_chunks: too many chunks\0".as_ptr() as png_const_charp,
        );

        return;
    }

    /* If these chunks are being reset to the default then no more memory is
     * required because add_one_chunk above doesn't extend the list if the 'keep'
     * parameter is the default.
     */
    if keep != 0 {
        new_list = png_malloc(
            png_ptr,
            (5u32.wrapping_mul(num_chunks.wrapping_add(old_num_chunks))) as png_alloc_size_t,
        ) as png_bytep;

        if old_num_chunks > 0 {
            memcpy(
                new_list as *mut c_void,
                (*png_ptr).chunk_list as *const c_void,
                (5u32.wrapping_mul(old_num_chunks)) as usize,
            );
        }
    } else if old_num_chunks > 0 {
        new_list = (*png_ptr).chunk_list;
    } else {
        new_list = core::ptr::null_mut();
    }

    /* Add the new chunks together with each one's handling code.  If the chunk
     * already exists the code is updated, otherwise the chunk is added to the
     * end.  (In libpng 1.6.0 order no longer matters because this code enforces
     * the earlier convention that the last setting is the one that is used.)
     */
    if new_list != core::ptr::null_mut() {
        let mut inlist: png_const_bytep;
        let mut outlist: png_bytep;
        let mut i: c_uint;

        i = 0;
        while i < num_chunks {
            old_num_chunks = add_one_chunk(
                new_list,
                old_num_chunks,
                chunk_list.add((5u32.wrapping_mul(i)) as usize),
                keep,
            );

            i += 1;
        }

        /* Now remove any spurious 'default' entries. */
        num_chunks = 0;
        i = 0;
        inlist = new_list as png_const_bytep;
        outlist = new_list;
        while i < old_num_chunks {
            if *inlist.add(4) != 0 {
                if outlist as png_const_bytep != inlist {
                    memcpy(outlist as *mut c_void, inlist as *const c_void, 5);
                }
                outlist = outlist.add(5);
                num_chunks += 1;
            }

            i += 1;
            inlist = inlist.add(5);
        }

        /* This means the application has removed all the specialized handling. */
        if num_chunks == 0 {
            if (*png_ptr).chunk_list != new_list {
                png_free(png_ptr, new_list as png_voidp);
            }

            new_list = core::ptr::null_mut();
        }
    } else {
        num_chunks = 0;
    }

    (*png_ptr).num_chunk_list = num_chunks;

    if (*png_ptr).chunk_list != new_list {
        if (*png_ptr).chunk_list != core::ptr::null_mut() {
            png_free(png_ptr, (*png_ptr).chunk_list as png_voidp);
        }

        (*png_ptr).chunk_list = new_list;
    }
}

/* png_set_read_user_chunk_fn */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_read_user_chunk_fn(
    png_ptr: png_structrp,
    user_chunk_ptr: png_voidp,
    read_user_chunk_fn: png_user_chunk_ptr,
) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).read_user_chunk_fn = read_user_chunk_fn;
    (*png_ptr).user_chunk_ptr = user_chunk_ptr;
}

/* png_set_rows */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_rows(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    row_pointers: png_bytepp,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    if (*info_ptr).row_pointers != core::ptr::null_mut()
        && (*info_ptr).row_pointers != row_pointers
    {
        png_free_data(png_ptr, info_ptr, PNG_FREE_ROWS, 0);
    }

    (*info_ptr).row_pointers = row_pointers;

    if row_pointers != core::ptr::null_mut() {
        (*info_ptr).valid |= PNG_INFO_IDAT;
    }
}

/* png_set_compression_buffer_size */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_buffer_size(png_ptr: png_structrp, mut size: usize) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    if size == 0 || size > PNG_UINT_31_MAX as usize {
        png_error(
            png_ptr,
            b"invalid compression buffer size\0".as_ptr() as png_const_charp,
        );
    }

    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 {
        (*png_ptr).IDAT_read_size = size as png_uint_32; /* checked above */
        return;
    }

    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) == 0 {
        if (*png_ptr).zowner != 0 {
            png_warning(
                png_ptr,
                b"Compression buffer size cannot be changed because it is in use\0".as_ptr()
                    as png_const_charp,
            );

            return;
        }

        /* Some compilers complain that this is always false.  However, it
         * can be true when integer overflow happens.
         */
        if size > ZLIB_IO_MAX as usize {
            png_warning(
                png_ptr,
                b"Compression buffer size limited to system maximum\0".as_ptr() as png_const_charp,
            );
            size = ZLIB_IO_MAX as usize; /* must fit */
        }

        if size < 6 {
            /* Deflate will potentially go into an infinite loop on a SYNC_FLUSH
             * if this is permitted.
             */
            png_warning(
                png_ptr,
                b"Compression buffer size cannot be reduced below 6\0".as_ptr() as png_const_charp,
            );

            return;
        }

        if (*png_ptr).zbuffer_size as usize != size {
            png_free_buffer_list(png_ptr, core::ptr::addr_of_mut!((*png_ptr).zbuffer_list));
            (*png_ptr).zbuffer_size = size as uInt;
        }
    }
}

/* png_set_invalid */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_invalid(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mask: c_int,
) {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        (*info_ptr).valid &= (!mask) as c_uint;
    }
}

/* This function was added to libpng 1.2.6 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_user_limits(
    png_ptr: png_structrp,
    user_width_max: png_uint_32,
    user_height_max: png_uint_32,
) {
    /* Images with dimensions larger than these limits will be
     * rejected by png_set_IHDR().  To accept any PNG datastream
     * regardless of dimensions, set both limits to 0x7fffffff.
     */
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).user_width_max = user_width_max;
    (*png_ptr).user_height_max = user_height_max;
}

/* This function was added to libpng 1.4.0 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_chunk_cache_max(
    png_ptr: png_structrp,
    user_chunk_cache_max: png_uint_32,
) {
    if png_ptr != core::ptr::null_mut() {
        (*png_ptr).user_chunk_cache_max = user_chunk_cache_max;
    }
}

/* This function was added to libpng 1.4.1 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_chunk_malloc_max(
    png_ptr: png_structrp,
    user_chunk_malloc_max: png_alloc_size_t,
) {
    /* pngstruct::user_chunk_malloc_max is initialized to a non-zero value in
     * png.c.  This API supports '0' for unlimited, make sure the correct
     * (unlimited) value is set here to avoid a need to check for 0 everywhere
     * the parameter is used.
     */
    if png_ptr != core::ptr::null_mut() {
        if user_chunk_malloc_max == 0 {
            /* unlimited */
            (*png_ptr).user_chunk_malloc_max = PNG_SIZE_MAX;
        } else {
            (*png_ptr).user_chunk_malloc_max = user_chunk_malloc_max;
        }
    }
}

/* png_set_benign_errors */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_benign_errors(png_ptr: png_structrp, allowed: c_int) {
    /* If allowed is 1, png_benign_error() is treated as a warning.
     *
     * If allowed is 0, png_benign_error() is treated as an error (which
     * is the default behavior if png_set_benign_errors() is not called).
     */

    if allowed != 0 {
        (*png_ptr).flags |=
            PNG_FLAG_BENIGN_ERRORS_WARN | PNG_FLAG_APP_WARNINGS_WARN | PNG_FLAG_APP_ERRORS_WARN;
    } else {
        (*png_ptr).flags &=
            !(PNG_FLAG_BENIGN_ERRORS_WARN | PNG_FLAG_APP_WARNINGS_WARN | PNG_FLAG_APP_ERRORS_WARN);
    }
}

/* Whether to report invalid palette index; added at libpng-1.5.10.
 * It is possible for an indexed (color-type==3) PNG file to contain
 * pixels with invalid (out-of-range) indexes if the PLTE chunk has
 * fewer entries than the image's bit-depth would allow. We recover
 * from this gracefully by filling any incomplete palette with zeros
 * (opaque black).  By default, when this occurs libpng will issue
 * a benign error.  This API can be used to override that behavior.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_check_for_invalid_index(png_ptr: png_structrp, allowed: c_int) {
    if allowed > 0 {
        (*png_ptr).num_palette_max = 0;
    } else {
        (*png_ptr).num_palette_max = -1;
    }
}

/* Check that the tEXt or zTXt keyword is valid per PNG 1.0 specification,
 * and if invalid, correct the keyword rather than discarding the entire
 * chunk.  The PNG 1.0 specification requires keywords 1-79 characters in
 * length, forbids leading or trailing whitespace, multiple internal spaces,
 * and the non-break space (0x80) from ISO 8859-1.  Returns keyword length.
 *
 * The 'new_key' buffer must be 80 characters in size (for the keyword plus a
 * trailing '\0').  If this routine returns 0 then there was no keyword, or a
 * valid one could not be generated, and the caller must png_error.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_check_keyword(
    png_ptr: png_structrp,
    mut key: png_const_charp,
    mut new_key: png_bytep,
) -> png_uint_32 {
    let orig_key: png_const_charp = key;
    let mut key_len: png_uint_32 = 0;
    let mut bad_character: c_int = 0;
    let mut space: c_int = 1;

    if key == core::ptr::null() {
        *new_key = 0;
        return 0;
    }

    while *key != 0 && key_len < 79 {
        let ch: png_byte = *key as png_byte;
        key = key.add(1);

        if ((ch as c_int) > 32 && (ch as c_int) <= 126) || (ch as c_int) >= 161
        /*&& ch <= 255*/
        {
            *new_key = ch;
            new_key = new_key.add(1);
            key_len += 1;
            space = 0;
        } else if space == 0 {
            /* A space or an invalid character when one wasn't seen immediately
             * before; output just a space.
             */
            *new_key = 32;
            new_key = new_key.add(1);
            key_len += 1;
            space = 1;

            /* If the character was not a space then it is invalid. */
            if ch as c_int != 32 {
                bad_character = ch as c_int;
            }
        } else if bad_character == 0 {
            bad_character = ch as c_int; /* just skip it, record the first error */
        }
    }

    if key_len > 0 && space != 0
    /* trailing space */
    {
        key_len -= 1;
        new_key = new_key.sub(1);
        if bad_character == 0 {
            bad_character = 32;
        }
    }

    /* Terminate the keyword */
    *new_key = 0;

    if key_len == 0 {
        return 0;
    }

    /* Try to only output one warning per keyword: */
    if *key != 0
    /* keyword too long */
    {
        png_warning(png_ptr, b"keyword truncated\0".as_ptr() as png_const_charp);
    } else if bad_character != 0 {
        let mut p: png_warning_parameters =
            [[0; PNG_WARNING_PARAMETER_SIZE]; PNG_WARNING_PARAMETER_COUNT];

        png_warning_parameter(p.as_mut_ptr(), 1, orig_key);
        png_warning_parameter_signed(p.as_mut_ptr(), 2, PNG_NUMBER_FORMAT_02x, bad_character);

        png_formatted_warning(
            png_ptr,
            p.as_mut_ptr(),
            b"keyword \"@1\": bad character '0x@2'\0".as_ptr() as png_const_charp,
        );
    }

    key_len
}
