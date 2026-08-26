/* pngrutil.c lines 1327..1686 */

/* png_handle_iCCP */
/* Note: this does not properly handle profiles that are > 64K under DOS */
unsafe extern "C" fn png_handle_iCCP(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    mut length: png_uint_32,
) -> png_handle_result_code {
    let mut errmsg: png_const_charp = core::ptr::null(); /* error message output, or no error */
    let mut finished: c_int = 0; /* crc checked */

    /* PNGv3: allow PNG files with both sRGB and iCCP because the PNG spec only
     * ever said that there "should" be only one, not "shall" and the PNGv3
     * colour chunk precedence rules give a handling for this case anyway.
     */
    {
        let mut read_length: uInt;
        let mut keyword_length: uInt;
        let mut keyword: [c_char; 81] = [0; 81];

        /* Find the keyword; the keyword plus separator and compression method
         * bytes can be at most 81 characters long.
         */
        read_length = 81; /* maximum */
        if read_length as png_uint_32 > length {
            read_length = length as uInt /*SAFE*/;
        }

        png_crc_read(png_ptr, keyword.as_mut_ptr() as png_bytep, read_length);
        length = length.wrapping_sub(read_length as png_uint_32);

        if length < LZ77Min {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, b"too short\0".as_ptr() as png_const_charp);
            return handled_error;
        }

        keyword_length = 0;
        while keyword_length < 80
            && keyword_length < read_length
            && keyword[keyword_length as usize] != 0
        {
            keyword_length += 1;
        }

        /* TODO: make the keyword checking common */
        if keyword_length >= 1 && keyword_length <= 79 {
            /* We only understand '0' compression - deflate - so if we get a
             * different value we can't safely decode the chunk.
             */
            if keyword_length.wrapping_add(1) < read_length
                && keyword[keyword_length.wrapping_add(1) as usize] as c_int
                    == PNG_COMPRESSION_TYPE_BASE
            {
                read_length = read_length.wrapping_sub(keyword_length.wrapping_add(2));

                if png_inflate_claim(png_ptr, png_iCCP) == Z_OK {
                    let mut profile_header: [png_byte; 132] = [0; 132];
                    let mut local_buffer: [png_byte; PNG_INFLATE_BUF_SIZE] =
                        [0; PNG_INFLATE_BUF_SIZE];
                    let mut size: png_alloc_size_t = 132 /*sizeof profile_header*/;

                    (*png_ptr).zstream.next_in = (keyword.as_ptr() as *const Bytef)
                        .add(keyword_length.wrapping_add(2) as usize);
                    (*png_ptr).zstream.avail_in = read_length;
                    png_inflate_read(
                        png_ptr,
                        local_buffer.as_mut_ptr(),
                        PNG_INFLATE_BUF_SIZE as uInt,
                        &mut length,
                        profile_header.as_mut_ptr(),
                        &mut size,
                        0, /*finish: don't, because the output is too small*/
                    );

                    if size == 0 {
                        /* We have the ICC profile header; do the basic header checks.
                         */
                        let profile_length: png_uint_32 = PNG_get_uint_32(profile_header.as_ptr());

                        if png_icc_check_length(
                            png_ptr,
                            keyword.as_ptr() as png_const_charp,
                            profile_length,
                        ) != 0
                        {
                            /* The length is apparently ok, so we can check the 132
                             * byte header.
                             */
                            if png_icc_check_header(
                                png_ptr,
                                keyword.as_ptr() as png_const_charp,
                                profile_length,
                                profile_header.as_ptr(),
                                (*png_ptr).color_type as c_int,
                            ) != 0
                            {
                                /* Now read the tag table; a variable size buffer is
                                 * needed at this point, allocate one for the whole
                                 * profile.  The header check has already validated
                                 * that none of this stuff will overflow.
                                 */
                                let tag_count: png_uint_32 =
                                    PNG_get_uint_32(profile_header.as_ptr().add(128));
                                let profile: png_bytep =
                                    png_read_buffer(png_ptr, profile_length as png_alloc_size_t);

                                if profile != core::ptr::null_mut() {
                                    memcpy(
                                        profile as *mut c_void,
                                        profile_header.as_ptr() as *const c_void,
                                        132, /*sizeof profile_header*/
                                    );

                                    size = 12u32.wrapping_mul(tag_count) as png_alloc_size_t;

                                    png_inflate_read(
                                        png_ptr,
                                        local_buffer.as_mut_ptr(),
                                        PNG_INFLATE_BUF_SIZE as uInt,
                                        &mut length,
                                        profile.add(132 /*sizeof profile_header*/),
                                        &mut size,
                                        0,
                                    );

                                    /* Still expect a buffer error because we expect
                                     * there to be some tag data!
                                     */
                                    if size == 0 {
                                        if png_icc_check_tag_table(
                                            png_ptr,
                                            keyword.as_ptr() as png_const_charp,
                                            profile_length,
                                            profile,
                                        ) != 0
                                        {
                                            /* The profile has been validated for basic
                                             * security issues, so read the whole thing in.
                                             */
                                            size = (profile_length as png_alloc_size_t)
                                                .wrapping_sub(132 /*sizeof profile_header*/)
                                                .wrapping_sub(
                                                    12u32.wrapping_mul(tag_count)
                                                        as png_alloc_size_t,
                                                );

                                            png_inflate_read(
                                                png_ptr,
                                                local_buffer.as_mut_ptr(),
                                                PNG_INFLATE_BUF_SIZE as uInt,
                                                &mut length,
                                                profile
                                                    .add(132 /*sizeof profile_header*/)
                                                    .add(12u32.wrapping_mul(tag_count) as usize),
                                                &mut size,
                                                1, /*finish*/
                                            );

                                            if length > 0
                                                && ((*png_ptr).flags & PNG_FLAG_BENIGN_ERRORS_WARN)
                                                    == 0
                                            {
                                                errmsg = b"extra compressed data\0".as_ptr()
                                                    as png_const_charp;
                                            }
                                            /* But otherwise allow extra data: */
                                            else if size == 0 {
                                                if length > 0 {
                                                    /* This can be handled completely, so
                                                     * keep going.
                                                     */
                                                    png_chunk_warning(
                                                        png_ptr,
                                                        b"extra compressed data\0".as_ptr()
                                                            as png_const_charp,
                                                    );
                                                }

                                                png_crc_finish(png_ptr, length);
                                                finished = 1;

                                                /* Steal the profile for info_ptr. */
                                                if info_ptr != core::ptr::null_mut() {
                                                    png_free_data(
                                                        png_ptr,
                                                        info_ptr,
                                                        PNG_FREE_ICCP,
                                                        0,
                                                    );

                                                    (*info_ptr).iccp_name = png_malloc_base(
                                                        png_ptr,
                                                        keyword_length.wrapping_add(1)
                                                            as png_alloc_size_t,
                                                    ) as png_charp;
                                                    if (*info_ptr).iccp_name
                                                        != core::ptr::null_mut()
                                                    {
                                                        memcpy(
                                                            (*info_ptr).iccp_name as *mut c_void,
                                                            keyword.as_ptr() as *const c_void,
                                                            keyword_length.wrapping_add(1) as usize,
                                                        );
                                                        (*info_ptr).iccp_proflen = profile_length;
                                                        (*info_ptr).iccp_profile = profile;
                                                        (*png_ptr).read_buffer =
                                                            core::ptr::null_mut(); /*steal*/
                                                        (*info_ptr).free_me |= PNG_FREE_ICCP;
                                                        (*info_ptr).valid |= PNG_INFO_iCCP;
                                                    } else {
                                                        errmsg = b"out of memory\0".as_ptr()
                                                            as png_const_charp;
                                                    }
                                                }

                                                /* else the profile remains in the read
                                                 * buffer which gets reused for subsequent
                                                 * chunks.
                                                 */

                                                if errmsg == core::ptr::null() {
                                                    (*png_ptr).zowner = 0;
                                                    return handled_ok;
                                                }
                                            }
                                            if errmsg == core::ptr::null() {
                                                errmsg = (*png_ptr).zstream.msg;
                                            }
                                        }
                                        /* else png_icc_check_tag_table output an error */
                                    }
                                    /* else profile truncated */
                                    else {
                                        errmsg = (*png_ptr).zstream.msg;
                                    }
                                } else {
                                    errmsg = b"out of memory\0".as_ptr() as png_const_charp;
                                }
                            }

                            /* else png_icc_check_header output an error */
                        }

                        /* else png_icc_check_length output an error */
                    }
                    /* else profile truncated */
                    else {
                        errmsg = (*png_ptr).zstream.msg;
                    }

                    /* Release the stream */
                    (*png_ptr).zowner = 0;
                }
                /* png_inflate_claim failed */
                else {
                    errmsg = (*png_ptr).zstream.msg;
                }
            } else {
                errmsg = b"bad compression method\0".as_ptr() as png_const_charp;
                /* or missing */
            }
        } else {
            errmsg = b"bad keyword\0".as_ptr() as png_const_charp;
        }
    }

    /* Failure: the reason is in 'errmsg' */
    if finished == 0 {
        png_crc_finish(png_ptr, length);
    }

    if errmsg != core::ptr::null()
    /* else already output */
    {
        png_chunk_benign_error(png_ptr, errmsg);
    }

    handled_error
}

/* png_handle_sPLT */
/* Note: this does not properly handle chunks that are > 64K under DOS */
unsafe extern "C" fn png_handle_sPLT(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let buffer: png_bytep;
    let mut entry_start: png_bytep;
    let mut new_palette: png_sPLT_t = png_sPLT_t {
        name: core::ptr::null_mut(),
        depth: 0,
        entries: core::ptr::null_mut(),
        nentries: 0,
    };
    let mut pp: png_sPLT_entryp;
    let data_length: png_uint_32;
    let entry_size: c_int;
    let mut i: c_int;
    let skip: png_uint_32 = 0;
    let dl: png_uint_32;
    let max_dl: usize;

    /* PNG_USER_LIMITS_SUPPORTED */
    if (*png_ptr).user_chunk_cache_max != 0 {
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            return handled_error;
        }

        (*png_ptr).user_chunk_cache_max = (*png_ptr).user_chunk_cache_max.wrapping_sub(1);
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_warning(
                png_ptr,
                b"No space in chunk cache for sPLT\0".as_ptr() as png_const_charp,
            );
            png_crc_finish(png_ptr, length);
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, length.wrapping_add(1) as png_alloc_size_t);
    if buffer == core::ptr::null_mut() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, b"out of memory\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    /* WARNING: this may break if size_t is less than 32 bits; it is assumed
     * that the PNG_MAX_MALLOC_64K test is enabled in this case, but this is a
     * potential breakage point if the types in pngconf.h aren't exactly right.
     */
    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, skip) != 0 {
        return handled_error;
    }

    *buffer.add(length as usize) = 0;

    entry_start = buffer;
    while *entry_start != 0 {
        /* Empty loop to find end of name */
        entry_start = entry_start.add(1);
    }

    entry_start = entry_start.add(1);

    /* A sample depth should follow the separator, and we should be on it  */
    if length < 2u32 || entry_start > buffer.add(length.wrapping_sub(2u32) as usize) {
        png_warning(
            png_ptr,
            b"malformed sPLT chunk\0".as_ptr() as png_const_charp,
        );
        return handled_error;
    }

    new_palette.depth = *entry_start;
    entry_start = entry_start.add(1);
    entry_size = if new_palette.depth as c_int == 8 { 6 } else { 10 };
    /* This must fit in a png_uint_32 because it is derived from the original
     * chunk data length.
     */
    data_length = length.wrapping_sub(entry_start.offset_from(buffer) as png_uint_32);

    /* Integrity-check the data length */
    if (data_length % (entry_size as c_uint)) != 0 {
        png_warning(
            png_ptr,
            b"sPLT chunk has bad length\0".as_ptr() as png_const_charp,
        );
        return handled_error;
    }

    dl = data_length / (entry_size as c_uint);
    max_dl = PNG_SIZE_MAX / core::mem::size_of::<png_sPLT_entry>();

    if dl as usize > max_dl {
        png_warning(png_ptr, b"sPLT chunk too long\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    new_palette.nentries = (data_length / (entry_size as c_uint)) as png_int_32;

    new_palette.entries = png_malloc_warn(
        png_ptr,
        (new_palette.nentries as png_alloc_size_t)
            .wrapping_mul(core::mem::size_of::<png_sPLT_entry>()),
    ) as png_sPLT_entryp;

    if new_palette.entries == core::ptr::null_mut() {
        png_warning(
            png_ptr,
            b"sPLT chunk requires too much memory\0".as_ptr() as png_const_charp,
        );
        return handled_error;
    }

    i = 0;
    while i < new_palette.nentries {
        pp = new_palette.entries.offset(i as isize);

        if new_palette.depth as c_int == 8 {
            (*pp).red = *entry_start as png_uint_16;
            entry_start = entry_start.add(1);
            (*pp).green = *entry_start as png_uint_16;
            entry_start = entry_start.add(1);
            (*pp).blue = *entry_start as png_uint_16;
            entry_start = entry_start.add(1);
            (*pp).alpha = *entry_start as png_uint_16;
            entry_start = entry_start.add(1);
        } else {
            (*pp).red = PNG_get_uint_16(entry_start);
            entry_start = entry_start.add(2);
            (*pp).green = PNG_get_uint_16(entry_start);
            entry_start = entry_start.add(2);
            (*pp).blue = PNG_get_uint_16(entry_start);
            entry_start = entry_start.add(2);
            (*pp).alpha = PNG_get_uint_16(entry_start);
            entry_start = entry_start.add(2);
        }

        (*pp).frequency = PNG_get_uint_16(entry_start);
        entry_start = entry_start.add(2);

        i += 1;
    }

    /* Discard all chunk data except the name and stash that */
    new_palette.name = buffer as png_charp;

    png_set_sPLT(png_ptr, info_ptr, &new_palette, 1);

    png_free(png_ptr, new_palette.entries as png_voidp);
    handled_ok
}
