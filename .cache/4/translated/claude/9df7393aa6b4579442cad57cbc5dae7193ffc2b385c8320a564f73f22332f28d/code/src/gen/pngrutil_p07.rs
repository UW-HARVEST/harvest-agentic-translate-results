/* Utility function for png_handle_unknown; set up png_ptr::unknown_chunk */
unsafe fn png_cache_unknown_chunk(png_ptr: png_structrp, length: png_uint_32) -> c_int {
    let limit: png_alloc_size_t = png_chunk_max(png_ptr);

    if (*png_ptr).unknown_chunk.data != core::ptr::null_mut() {
        png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
        (*png_ptr).unknown_chunk.data = core::ptr::null_mut();
    }

    if (length as png_alloc_size_t) <= limit {
        PNG_CSTRING_FROM_CHUNK(
            core::ptr::addr_of_mut!((*png_ptr).unknown_chunk.name) as png_bytep,
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

    if (*png_ptr).unknown_chunk.data == core::ptr::null_mut() && length > 0 {
        /* This is benign because we clean up correctly */
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(
            png_ptr,
            b"unknown chunk exceeds memory limits\0".as_ptr() as png_const_charp,
        );
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
                png_chunk_error(png_ptr, b"error in user chunk\0".as_ptr() as png_const_charp);
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
                        png_chunk_warning(
                            png_ptr,
                            b"Saving unknown chunk:\0".as_ptr() as png_const_charp,
                        );
                        png_app_warning(
                            png_ptr,
                            b"forcing save of an unhandled chunk; please call png_set_keep_unknown_chunks\0"
                                .as_ptr() as png_const_charp,
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
    }
    /* Use the SAVE_UNKNOWN_CHUNKS code or skip the chunk */
    else {
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
                png_chunk_benign_error(
                    png_ptr,
                    b"no space in chunk cache\0".as_ptr() as png_const_charp,
                );
                /* FALLTHROUGH */
                /* case 1:
                 * NOTE: prior to 1.6.0 this case resulted in an unknown critical
                 * chunk being skipped, now there will be a hard error below.
                 */
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
                    core::ptr::addr_of_mut!((*png_ptr).unknown_chunk),
                    1,
                );
                handled = handled_saved;
            }

            _ => {
                /* not at limit */
                (*png_ptr).user_chunk_cache_max = (*png_ptr).user_chunk_cache_max.wrapping_sub(1);
                /* FALLTHROUGH */
                /* Here when the limit isn't reached or when limits are compiled
                 * out; store the chunk.
                 */
                png_set_unknown_chunks(
                    png_ptr,
                    info_ptr,
                    core::ptr::addr_of_mut!((*png_ptr).unknown_chunk),
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
    if (*png_ptr).unknown_chunk.data != core::ptr::null_mut() {
        png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
    }
    (*png_ptr).unknown_chunk.data = core::ptr::null_mut();

    /* Check for unhandled critical chunks */
    if handled < handled_saved && PNG_CHUNK_CRITICAL((*png_ptr).chunk_name) {
        png_chunk_error(
            png_ptr,
            b"unhandled critical chunk\0".as_ptr() as png_const_charp,
        );
    }

    handled
}

unsafe fn png_chunk_index_from_name(chunk_name: png_uint_32) -> png_index {
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
pub unsafe extern "C" fn png_handle_chunk(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    /* CSE: these things don't change, these autos are just to save typing and
     * make the code more clear.
     */
    let chunk_name: png_uint_32 = (*png_ptr).chunk_name;
    let chunk_index: png_index = png_chunk_index_from_name(chunk_name);

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
        png_chunk_error(png_ptr, b"missing IHDR\0".as_ptr() as png_const_charp);
        /* NORETURN */
    }
    /* Before all the pos_before chunks, after all the pos_after chunks. */
    else if ((*png_ptr).mode & read_chunks[chunk_index as usize].pos_before) != 0
        || ((*png_ptr).mode & read_chunks[chunk_index as usize].pos_after)
            != read_chunks[chunk_index as usize].pos_after
    {
        errmsg = b"out of place\0".as_ptr() as png_const_charp;
    }
    /* Now check for duplicates: duplicated critical chunks also produce a
     * full error.
     */
    else if read_chunks[chunk_index as usize].multiple == 0
        && png_file_has_chunk(png_ptr, chunk_index)
    {
        errmsg = b"duplicate\0".as_ptr() as png_const_charp;
    } else if length < read_chunks[chunk_index as usize].min_length {
        errmsg = b"too short\0".as_ptr() as png_const_charp;
    } else {
        /* NOTE: apart from IHDR the critical chunks (PLTE, IDAT and IEND) are set
         * up above not to do any length checks.
         *
         * The png_chunk_max check ensures that the variable length chunks are
         * always checked at this point for being within the system allocation
         * limits.
         */
        let max_length: c_uint = read_chunks[chunk_index as usize].max_length;

        let mut meets_limit: bool = false;

        match max_length {
            Limit => {
                /* png_read_chunk_header has already png_error'ed chunks with a
                 * length exceeding the 31-bit PNG limit, so just check the memory
                 * limit:
                 */
                if (length as png_alloc_size_t) <= png_chunk_max(png_ptr) {
                    meets_limit = true; /* goto MeetsLimit */
                } else {
                    errmsg = b"length exceeds libpng limit\0".as_ptr() as png_const_charp;
                }
            }

            NoCheck => {
                meets_limit = true;
            }

            _ => {
                if length <= max_length {
                    meets_limit = true; /* goto MeetsLimit */
                } else {
                    errmsg = b"too long\0".as_ptr() as png_const_charp;
                }
            }
        }

        if meets_limit
        /* MeetsLimit: */
        {
            handled = (read_chunks[chunk_index as usize].handler.unwrap())(
                png_ptr, info_ptr, length,
            );
        }
    }

    /* If there was an error or the chunk was simply skipped it is not counted as
     * 'seen'.
     */
    if errmsg != core::ptr::null() {
        if PNG_CHUNK_CRITICAL(chunk_name)
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
