//! Translation of c_src/src/pngset.c lines 1156..2057
use crate::prelude::*;

/* PNG_tIME_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_tIME(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mod_time: png_const_timep,
) {
    if png_ptr.is_null()
        || info_ptr.is_null()
        || mod_time.is_null()
        || ((*png_ptr).mode & PNG_WROTE_tIME) != 0
    {
        return;
    }

    if (*mod_time).month == 0
        || (*mod_time).month > 12
        || (*mod_time).day == 0
        || (*mod_time).day > 31
        || (*mod_time).hour > 23
        || (*mod_time).minute > 59
        || (*mod_time).second > 60
    {
        png_warning(png_ptr, cstr(b"Ignoring invalid time value\0"));

        return;
    }

    (*info_ptr).mod_time = *mod_time;
    (*info_ptr).valid |= PNG_INFO_tIME;
}

/* PNG_tRNS_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_tRNS(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    mut trans_alpha: png_const_bytep,
    mut num_trans: c_int,
    trans_color: png_const_color_16p,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    if !trans_alpha.is_null() {
        /* Snapshot the caller's trans_alpha before freeing, in case it
         * points to info_ptr->trans_alpha (getter-to-setter aliasing).
         */
        let mut safe_trans: [png_byte; PNG_MAX_PALETTE_LENGTH as usize] =
            [0; PNG_MAX_PALETTE_LENGTH as usize];

        if num_trans > 0 && num_trans <= PNG_MAX_PALETTE_LENGTH {
            memcpy(
                safe_trans.as_mut_ptr() as *mut c_void,
                trans_alpha as *const c_void,
                num_trans as usize,
            );
        }

        trans_alpha = safe_trans.as_ptr();

        png_free_data(png_ptr, info_ptr, PNG_FREE_TRNS, 0);

        if num_trans > 0 && num_trans <= PNG_MAX_PALETTE_LENGTH {
            /* Allocate info_ptr's copy of the transparency data. */
            (*info_ptr).trans_alpha =
                png_malloc(png_ptr, PNG_MAX_PALETTE_LENGTH as png_alloc_size_t) as png_bytep;
            memset(
                (*info_ptr).trans_alpha as *mut c_void,
                0xff,
                PNG_MAX_PALETTE_LENGTH as usize,
            );
            memcpy(
                (*info_ptr).trans_alpha as *mut c_void,
                trans_alpha as *const c_void,
                num_trans as usize,
            );
            (*info_ptr).free_me |= PNG_FREE_TRNS;
            (*info_ptr).valid |= PNG_INFO_tRNS;

            /* Allocate an independent copy for png_struct. */
            png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
            (*png_ptr).trans_alpha = core::ptr::null_mut();
            (*png_ptr).trans_alpha =
                png_malloc(png_ptr, PNG_MAX_PALETTE_LENGTH as png_alloc_size_t) as png_bytep;
            memset(
                (*png_ptr).trans_alpha as *mut c_void,
                0xff,
                PNG_MAX_PALETTE_LENGTH as usize,
            );
            memcpy(
                (*png_ptr).trans_alpha as *mut c_void,
                trans_alpha as *const c_void,
                num_trans as usize,
            );
        } else {
            png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
            (*png_ptr).trans_alpha = core::ptr::null_mut();
        }
    }

    if !trans_color.is_null() {
        /* PNG_WARNINGS_SUPPORTED */
        if (*info_ptr).bit_depth < 16 {
            let sample_max: c_int = (1 << (*info_ptr).bit_depth) - 1;

            if ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY
                && (*trans_color).gray as c_int > sample_max)
                || ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB
                    && ((*trans_color).red as c_int > sample_max
                        || (*trans_color).green as c_int > sample_max
                        || (*trans_color).blue as c_int > sample_max))
            {
                png_warning(
                    png_ptr,
                    cstr(b"tRNS chunk has out-of-range samples for bit_depth\0"),
                );
            }
        }

        (*info_ptr).trans_color = *trans_color;

        if num_trans == 0 {
            num_trans = 1;
        }
    }

    (*info_ptr).num_trans = num_trans as png_uint_16;

    if num_trans != 0 {
        (*info_ptr).free_me |= PNG_FREE_TRNS;
        (*info_ptr).valid |= PNG_INFO_tRNS;
    }
}

/* PNG_sPLT_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sPLT(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mut entries: png_const_sPLT_tp,
    mut nentries: c_int,
) {
    let mut np: png_sPLT_tp;
    let old_spalettes: png_sPLT_tp;

    if png_ptr.is_null() || info_ptr.is_null() || nentries <= 0 || entries.is_null() {
        return;
    }

    /* Use the internal realloc function, which checks for all the possible
     * overflows.  Notice that the parameters are (int) and (size_t)
     */
    np = png_realloc_array(
        png_ptr,
        (*info_ptr).splt_palettes as png_const_voidp,
        (*info_ptr).splt_palettes_num,
        nentries,
        core::mem::size_of::<png_sPLT_t>(),
    ) as png_sPLT_tp;

    if np.is_null() {
        /* Out of memory or too many chunks */
        png_chunk_report(
            png_ptr,
            cstr(b"too many sPLT chunks\0"),
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    /* Defer freeing the old array until after the copy loop below. */
    old_spalettes = (*info_ptr).splt_palettes;

    (*info_ptr).splt_palettes = np;
    (*info_ptr).free_me |= PNG_FREE_SPLT;

    np = np.add((*info_ptr).splt_palettes_num as usize);

    /* do { BODY } while (--nentries);  A C `continue` in BODY jumps to the
     * `--nentries` test, skipping the ++np/++entries advance, so it is
     * expressed here as `break 'body`.  A C `break` leaves the loop entirely.
     */
    'outer: loop {
        'body: {
            let length: usize;

            /* Skip invalid input entries */
            if (*entries).name.is_null() || (*entries).entries.is_null() {
                /* png_handle_sPLT doesn't do this, so this is an app error */
                png_app_error(png_ptr, cstr(b"png_set_sPLT: invalid sPLT\0"));
                /* Just skip the invalid entry */
                break 'body;
            }

            (*np).depth = (*entries).depth;

            /* In the event of out-of-memory just return. */
            length = strlen((*entries).name) + 1;
            (*np).name = png_malloc_base(png_ptr, length) as png_charp;

            if (*np).name.is_null() {
                break 'outer;
            }

            memcpy(
                (*np).name as *mut c_void,
                (*entries).name as *const c_void,
                length,
            );

            (*np).entries = png_malloc_array(
                png_ptr,
                (*entries).nentries,
                core::mem::size_of::<png_sPLT_entry>(),
            ) as png_sPLT_entryp;

            if (*np).entries.is_null() {
                png_free(png_ptr, (*np).name as png_voidp);
                (*np).name = core::ptr::null_mut();
                break 'outer;
            }

            (*np).nentries = (*entries).nentries;
            /* This multiply can't overflow. */
            memcpy(
                (*np).entries as *mut c_void,
                (*entries).entries as *const c_void,
                (*entries).nentries as c_uint as usize * core::mem::size_of::<png_sPLT_entry>(),
            );

            /* Note that 'continue' skips the advance of the out pointer and out
             * count, so an invalid entry is not added.
             */
            (*info_ptr).valid |= PNG_INFO_sPLT;
            (*info_ptr).splt_palettes_num += 1;
            np = np.add(1);
            entries = entries.add(1);
        }

        nentries -= 1;
        if nentries == 0 {
            break;
        }
    }

    png_free(png_ptr, old_spalettes as png_voidp);

    if nentries > 0 {
        png_chunk_report(
            png_ptr,
            cstr(b"sPLT out of memory\0"),
            PNG_CHUNK_WRITE_ERROR,
        );
    }
}

/* PNG_STORE_UNKNOWN_CHUNKS_SUPPORTED */
pub unsafe extern "C" fn check_location(
    png_ptr: png_const_structrp,
    mut location: c_int,
) -> png_byte {
    location &= PNG_HAVE_IHDR as c_int | PNG_HAVE_PLTE as c_int | PNG_AFTER_IDAT as c_int;

    /* New in 1.6.0; copy the location and check it. */
    if location == 0 && ((*png_ptr).mode & PNG_IS_READ_STRUCT) == 0 {
        /* Write struct, so unknown chunks come from the app */
        png_app_warning(
            png_ptr,
            cstr(b"png_set_unknown_chunks now expects a valid location\0"),
        );
        /* Use the old behavior */
        location = ((*png_ptr).mode & (PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT)) as png_byte
            as c_int;
    }

    /* This need not be an internal error. */
    if location == 0 {
        png_error(
            png_ptr,
            cstr(b"invalid location in png_set_unknown_chunks\0"),
        );
    }

    /* Now reduce the location to the top-most set bit by removing each least
     * significant bit in turn.
     */
    while location != (location & location.wrapping_neg()) {
        location &= !(location & location.wrapping_neg());
    }

    /* The cast is safe because 'location' is a bit mask and only the low four
     * bits are significant.
     */
    location as png_byte
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_unknown_chunks(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mut unknowns: png_const_unknown_chunkp,
    mut num_unknowns: c_int,
) {
    let mut np: png_unknown_chunkp;
    let old_unknowns: png_unknown_chunkp;

    if png_ptr.is_null() || info_ptr.is_null() || num_unknowns <= 0 || unknowns.is_null() {
        return;
    }

    /* The compile-time-disabled read/write support checks are skipped:
     * both PNG_READ_UNKNOWN_CHUNKS_SUPPORTED and
     * PNG_WRITE_UNKNOWN_CHUNKS_SUPPORTED are defined in this build.
     */

    np = png_realloc_array(
        png_ptr,
        (*info_ptr).unknown_chunks as png_const_voidp,
        (*info_ptr).unknown_chunks_num,
        num_unknowns,
        core::mem::size_of::<png_unknown_chunk>(),
    ) as png_unknown_chunkp;

    if np.is_null() {
        png_chunk_report(
            png_ptr,
            cstr(b"too many unknown chunks\0"),
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    /* Defer freeing the old array until after the copy loop below. */
    old_unknowns = (*info_ptr).unknown_chunks;

    (*info_ptr).unknown_chunks = np; /* safe because it is initialized */
    (*info_ptr).free_me |= PNG_FREE_UNKN;

    np = np.add((*info_ptr).unknown_chunks_num as usize);

    /* Increment unknown_chunks_num each time round the loop to protect the
     * just-allocated chunk data.
     */
    while num_unknowns > 0 {
        memcpy(
            (*np).name.as_mut_ptr() as *mut c_void,
            (*unknowns).name.as_ptr() as *const c_void,
            core::mem::size_of_val(&(*np).name),
        );
        (*np).name[core::mem::size_of_val(&(*np).name) - 1] = b'\0';
        (*np).location = check_location(png_ptr, (*unknowns).location as c_int);

        if (*unknowns).size == 0 {
            (*np).data = core::ptr::null_mut();
            (*np).size = 0;
        } else {
            (*np).data = png_malloc_base(png_ptr, (*unknowns).size) as png_bytep;

            if (*np).data.is_null() {
                png_chunk_report(
                    png_ptr,
                    cstr(b"unknown chunk: out of memory\0"),
                    PNG_CHUNK_WRITE_ERROR,
                );
                /* But just skip storing the unknown chunk */
                num_unknowns -= 1;
                unknowns = unknowns.add(1);
                continue;
            }

            memcpy(
                (*np).data as *mut c_void,
                (*unknowns).data as *const c_void,
                (*unknowns).size,
            );
            (*np).size = (*unknowns).size;
        }

        /* These increments are skipped on out-of-memory for the data. */
        np = np.add(1);
        (*info_ptr).unknown_chunks_num += 1;

        num_unknowns -= 1;
        unknowns = unknowns.add(1);
    }

    png_free(png_ptr, old_unknowns as png_voidp);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_unknown_chunk_location(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    chunk: c_int,
    mut location: c_int,
) {
    /* This API is pretty pointless in 1.6.0. */
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && chunk >= 0
        && chunk < (*info_ptr).unknown_chunks_num
    {
        if (location & (PNG_HAVE_IHDR as c_int | PNG_HAVE_PLTE as c_int | PNG_AFTER_IDAT as c_int))
            == 0
        {
            png_app_error(png_ptr, cstr(b"invalid unknown chunk location\0"));
            /* Fake out the pre 1.6.0 behavior: */
            if (location as c_uint & PNG_HAVE_IDAT) != 0 {
                /* undocumented! */
                location = PNG_AFTER_IDAT as c_int;
            } else {
                location = PNG_HAVE_IHDR as c_int; /* also undocumented */
            }
        }

        (*(*info_ptr).unknown_chunks.add(chunk as usize)).location =
            check_location(png_ptr, location);
    }
}

/* PNG_MNG_FEATURES_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_permit_mng_features(
    png_ptr: png_structrp,
    mng_features: png_uint_32,
) -> png_uint_32 {
    if png_ptr.is_null() {
        return 0;
    }

    (*png_ptr).mng_features_permitted = mng_features & PNG_ALL_MNG_FEATURES;

    (*png_ptr).mng_features_permitted
}

/* PNG_HANDLE_AS_UNKNOWN_SUPPORTED */
pub unsafe extern "C" fn add_one_chunk(
    mut list: png_bytep,
    count: c_uint,
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

    let mut count = count;
    if keep != PNG_HANDLE_CHUNK_AS_DEFAULT {
        count += 1;
        memcpy(list as *mut c_void, add as *const c_void, 4);
        *list.add(4) = keep as png_byte;
    }

    count
}

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

    if png_ptr.is_null() {
        return;
    }

    if keep < 0 || keep >= PNG_HANDLE_CHUNK_LAST {
        png_app_error(
            png_ptr,
            cstr(b"png_set_keep_unknown_chunks: invalid keep\0"),
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
        static CHUNKS_TO_IGNORE: [png_byte; 105] = [
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

        chunk_list = CHUNKS_TO_IGNORE.as_ptr();
        num_chunks = (core::mem::size_of_val(&CHUNKS_TO_IGNORE) / 5) as c_uint;
    } else {
        /* num_chunks_in > 0 */
        if chunk_list.is_null() {
            /* Prior to 1.6.0 this was silently ignored, now it is an app_error. */
            png_app_error(
                png_ptr,
                cstr(b"png_set_keep_unknown_chunks: no chunk list\0"),
            );

            return;
        }

        num_chunks = num_chunks_in as c_uint;
    }

    old_num_chunks = (*png_ptr).num_chunk_list;
    if (*png_ptr).chunk_list.is_null() {
        old_num_chunks = 0;
    }

    /* Since num_chunks is always restricted to UINT_MAX/5 this can't overflow. */
    if num_chunks + old_num_chunks > c_uint::MAX / 5 {
        png_app_error(
            png_ptr,
            cstr(b"png_set_keep_unknown_chunks: too many chunks\0"),
        );

        return;
    }

    /* If these chunks are being reset to the default then no more memory is
     * required.
     */
    if keep != 0 {
        new_list = png_malloc(
            png_ptr,
            (5 * (num_chunks + old_num_chunks)) as png_alloc_size_t,
        ) as png_bytep;

        if old_num_chunks > 0 {
            memcpy(
                new_list as *mut c_void,
                (*png_ptr).chunk_list as *const c_void,
                (5 * old_num_chunks) as usize,
            );
        }
    } else if old_num_chunks > 0 {
        new_list = (*png_ptr).chunk_list;
    } else {
        new_list = core::ptr::null_mut();
    }

    /* Add the new chunks together with each one's handling code. */
    if !new_list.is_null() {
        let mut inlist: png_const_bytep;
        let mut outlist: png_bytep;
        let mut i: c_uint;

        i = 0;
        while i < num_chunks {
            old_num_chunks = add_one_chunk(
                new_list,
                old_num_chunks,
                chunk_list.add((5 * i) as usize),
                keep,
            );
            i += 1;
        }

        /* Now remove any spurious 'default' entries. */
        num_chunks = 0;
        i = 0;
        inlist = new_list;
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
        if !(*png_ptr).chunk_list.is_null() {
            png_free(png_ptr, (*png_ptr).chunk_list as png_voidp);
        }

        (*png_ptr).chunk_list = new_list;
    }
}

/* PNG_READ_USER_CHUNKS_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_read_user_chunk_fn(
    png_ptr: png_structrp,
    user_chunk_ptr: png_voidp,
    read_user_chunk_fn: png_user_chunk_ptr,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).read_user_chunk_fn = read_user_chunk_fn;
    (*png_ptr).user_chunk_ptr = user_chunk_ptr;
}

/* PNG_INFO_IMAGE_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_rows(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    row_pointers: png_bytepp,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    if !(*info_ptr).row_pointers.is_null() && (*info_ptr).row_pointers != row_pointers {
        png_free_data(png_ptr, info_ptr, PNG_FREE_ROWS, 0);
    }

    (*info_ptr).row_pointers = row_pointers;

    if !row_pointers.is_null() {
        (*info_ptr).valid |= PNG_INFO_IDAT;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_buffer_size(png_ptr: png_structrp, mut size: usize) {
    if png_ptr.is_null() {
        return;
    }

    if size == 0 || size > PNG_UINT_31_MAX as usize {
        png_error(png_ptr, cstr(b"invalid compression buffer size\0"));
    }

    /* PNG_SEQUENTIAL_READ_SUPPORTED */
    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 {
        (*png_ptr).IDAT_read_size = size as png_uint_32; /* checked above */
        return;
    }

    /* PNG_WRITE_SUPPORTED */
    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) == 0 {
        if (*png_ptr).zowner != 0 {
            png_warning(
                png_ptr,
                cstr(b"Compression buffer size cannot be changed because it is in use\0"),
            );

            return;
        }

        /* __COVERITY__ is not defined. */
        if size > ZLIB_IO_MAX as usize {
            png_warning(
                png_ptr,
                cstr(b"Compression buffer size limited to system maximum\0"),
            );
            size = ZLIB_IO_MAX as usize; /* must fit */
        }

        if size < 6 {
            /* Deflate will potentially go into an infinite loop on a SYNC_FLUSH
             * if this is permitted.
             */
            png_warning(
                png_ptr,
                cstr(b"Compression buffer size cannot be reduced below 6\0"),
            );

            return;
        }

        if (*png_ptr).zbuffer_size as usize != size {
            png_free_buffer_list(png_ptr, &mut (*png_ptr).zbuffer_list);
            (*png_ptr).zbuffer_size = size as uInt;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_invalid(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mask: c_int,
) {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        (*info_ptr).valid &= (!mask) as c_uint;
    }
}

/* PNG_SET_USER_LIMITS_SUPPORTED */
/* This function was added to libpng 1.2.6 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_user_limits(
    png_ptr: png_structrp,
    user_width_max: png_uint_32,
    user_height_max: png_uint_32,
) {
    /* Images with dimensions larger than these limits will be
     * rejected by png_set_IHDR().
     */
    if png_ptr.is_null() {
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
    if !png_ptr.is_null() {
        (*png_ptr).user_chunk_cache_max = user_chunk_cache_max;
    }
}

/* This function was added to libpng 1.4.1 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_chunk_malloc_max(
    png_ptr: png_structrp,
    user_chunk_malloc_max: png_alloc_size_t,
) {
    if !png_ptr.is_null() {
        if user_chunk_malloc_max == 0 {
            /* unlimited */
            /* PNG_MAX_MALLOC_64K is NOT defined. */
            (*png_ptr).user_chunk_malloc_max = PNG_SIZE_MAX;
        } else {
            (*png_ptr).user_chunk_malloc_max = user_chunk_malloc_max;
        }
    }
}

/* PNG_BENIGN_ERRORS_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_benign_errors(png_ptr: png_structrp, allowed: c_int) {
    /* If allowed is 1, png_benign_error() is treated as a warning.
     * If allowed is 0, png_benign_error() is treated as an error.
     */
    if allowed != 0 {
        (*png_ptr).flags |=
            PNG_FLAG_BENIGN_ERRORS_WARN | PNG_FLAG_APP_WARNINGS_WARN | PNG_FLAG_APP_ERRORS_WARN;
    } else {
        (*png_ptr).flags &=
            !(PNG_FLAG_BENIGN_ERRORS_WARN | PNG_FLAG_APP_WARNINGS_WARN | PNG_FLAG_APP_ERRORS_WARN);
    }
}

/* PNG_CHECK_FOR_INVALID_INDEX_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_check_for_invalid_index(png_ptr: png_structrp, allowed: c_int) {
    if allowed > 0 {
        (*png_ptr).num_palette_max = 0;
    } else {
        (*png_ptr).num_palette_max = -1;
    }
}

/* PNG_TEXT_SUPPORTED || PNG_pCAL_SUPPORTED || PNG_iCCP_SUPPORTED || PNG_sPLT_SUPPORTED */
/* Check that the tEXt or zTXt keyword is valid per PNG 1.0 specification. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_check_keyword(
    png_ptr: png_structrp,
    key: png_const_charp,
    mut new_key: png_bytep,
) -> png_uint_32 {
    /* PNG_WARNINGS_SUPPORTED */
    let orig_key: png_const_charp = key;
    let mut key = key;
    let mut key_len: png_uint_32 = 0;
    let mut bad_character: c_int = 0;
    let mut space: c_int = 1;

    if key.is_null() {
        *new_key = 0;
        return 0;
    }

    while *key != 0 && key_len < 79 {
        let ch: png_byte = *key as png_byte;
        key = key.add(1);

        if (ch > 32 && ch <= 126) || (ch >= 161/*&& ch <= 255*/) {
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
            if ch != 32 {
                bad_character = ch as c_int;
            }
        } else if bad_character == 0 {
            bad_character = ch as c_int; /* just skip it, record the first error */
        }
    }

    if key_len > 0 && space != 0 {
        /* trailing space */
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

    /* PNG_WARNINGS_SUPPORTED */
    /* Try to only output one warning per keyword: */
    if *key != 0 {
        /* keyword too long */
        png_warning(png_ptr, cstr(b"keyword truncated\0"));
    } else if bad_character != 0 {
        /* PNG_WARNING_PARAMETERS(p) */
        let mut p: [png_warning_parameters_row; PNG_WARNING_PARAMETER_COUNT] =
            [[0; PNG_WARNING_PARAMETER_SIZE]; PNG_WARNING_PARAMETER_COUNT];

        png_warning_parameter(p.as_mut_ptr(), 1, orig_key);
        png_warning_parameter_signed(p.as_mut_ptr(), 2, PNG_NUMBER_FORMAT_02x, bad_character);

        png_formatted_warning(
            png_ptr,
            p.as_mut_ptr(),
            cstr(b"keyword \"@1\": bad character '0x@2'\0"),
        );
    }

    key_len
}
