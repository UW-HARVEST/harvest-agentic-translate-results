//! pngset.c lines 1156-2057: tIME/tRNS/sPLT/unknown-chunk/limit setters and
//! png_check_keyword.
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_tIME(
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
        png_warning(png_ptr, c"Ignoring invalid time value".as_ptr());

        return;
    }

    (*info_ptr).mod_time = *mod_time;
    (*info_ptr).valid |= PNG_INFO_tIME;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_tRNS(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    trans_alpha: png_const_bytep,
    num_trans: c_int,
    trans_color: png_const_color_16p,
) {
    let mut trans_alpha = trans_alpha;
    let mut num_trans = num_trans;

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
                safe_trans.as_mut_ptr() as *mut u8,
                trans_alpha as *const u8,
                num_trans as usize,
            );
        }

        trans_alpha = safe_trans.as_ptr();

        png_free_data(png_ptr, info_ptr, PNG_FREE_TRNS, 0);

        if num_trans > 0 && num_trans <= PNG_MAX_PALETTE_LENGTH {
            /* Allocate info_ptr's copy of the transparency data.
             * Initialize all entries to fully opaque (0xff), then overwrite
             * the first num_trans entries with the actual values.
             */
            (*info_ptr).trans_alpha =
                png_malloc(png_ptr, PNG_MAX_PALETTE_LENGTH as usize) as png_bytep;
            memset(
                (*info_ptr).trans_alpha as *mut u8,
                0xff,
                PNG_MAX_PALETTE_LENGTH as usize,
            );
            memcpy(
                (*info_ptr).trans_alpha as *mut u8,
                trans_alpha as *const u8,
                num_trans as usize,
            );
            (*info_ptr).free_me |= PNG_FREE_TRNS;
            (*info_ptr).valid |= PNG_INFO_tRNS;

            /* Allocate an independent copy for png_struct, so that the
             * lifetime of png_ptr->trans_alpha is decoupled from the
             * lifetime of info_ptr->trans_alpha.  Previously these two
             * pointers were aliased, which caused a use-after-free if
             * png_free_data freed info_ptr->trans_alpha while
             * png_ptr->trans_alpha was still in use by the row transform
             * functions (e.g. png_do_expand_palette).
             */
            png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
            (*png_ptr).trans_alpha = core::ptr::null_mut();
            (*png_ptr).trans_alpha =
                png_malloc(png_ptr, PNG_MAX_PALETTE_LENGTH as usize) as png_bytep;
            memset(
                (*png_ptr).trans_alpha as *mut u8,
                0xff,
                PNG_MAX_PALETTE_LENGTH as usize,
            );
            memcpy(
                (*png_ptr).trans_alpha as *mut u8,
                trans_alpha as *const u8,
                num_trans as usize,
            );
        } else {
            png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
            (*png_ptr).trans_alpha = core::ptr::null_mut();
        }
    }

    if !trans_color.is_null() {
        if ((*info_ptr).bit_depth as c_int) < 16 {
            let sample_max: c_int = (1 << ((*info_ptr).bit_depth as c_int)) - 1;

            if ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY
                && (*trans_color).gray as c_int > sample_max)
                || ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB
                    && ((*trans_color).red as c_int > sample_max
                        || (*trans_color).green as c_int > sample_max
                        || (*trans_color).blue as c_int > sample_max))
            {
                png_warning(
                    png_ptr,
                    c"tRNS chunk has out-of-range samples for bit_depth".as_ptr(),
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sPLT(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    entries: png_const_sPLT_tp,
    nentries: c_int,
) /*
  *  entries        - array of png_sPLT_t structures
  *                   to be added to the list of palettes
  *                   in the info structure.
  *
  *  nentries       - number of palette structures to be
  *                   added.
  */
{
    let mut np: png_sPLT_tp;
    let old_spalettes: png_sPLT_tp;
    let mut entries = entries;
    let mut nentries = nentries;

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
            c"too many sPLT chunks".as_ptr(),
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    /* Defer freeing the old array until after the copy loop below,
     * in case entries aliases info_ptr->splt_palettes (getter-to-setter).
     */
    old_spalettes = (*info_ptr).splt_palettes;

    (*info_ptr).splt_palettes = np;
    (*info_ptr).free_me |= PNG_FREE_SPLT;

    np = np.add((*info_ptr).splt_palettes_num as usize);

    'do_loop: loop {
        'continue_loop: {
            let length: usize;

            /* Skip invalid input entries */
            if (*entries).name.is_null() || (*entries).entries.is_null() {
                /* png_handle_sPLT doesn't do this, so this is an app error */
                png_app_error(png_ptr, c"png_set_sPLT: invalid sPLT".as_ptr());
                /* Just skip the invalid entry */
                break 'continue_loop;
            }

            (*np).depth = (*entries).depth;

            /* In the event of out-of-memory just return - there's no point keeping
             * on trying to add sPLT chunks.
             */
            length = strlen((*entries).name) + 1;
            (*np).name = png_malloc_base(png_ptr, length) as png_charp;

            if (*np).name.is_null() {
                break 'do_loop;
            }

            memcpy(
                (*np).name as *mut u8,
                (*entries).name as *const u8,
                length,
            );

            /* IMPORTANT: we have memory now that won't get freed if something else
             * goes wrong; this code must free it.  png_malloc_array produces no
             * warnings; use a png_chunk_report (below) if there is an error.
             */
            (*np).entries = png_malloc_array(
                png_ptr,
                (*entries).nentries,
                core::mem::size_of::<png_sPLT_entry>(),
            ) as png_sPLT_entryp;

            if (*np).entries.is_null() {
                png_free(png_ptr, (*np).name as png_voidp);
                (*np).name = core::ptr::null_mut();
                break 'do_loop;
            }

            (*np).nentries = (*entries).nentries;
            /* This multiply can't overflow because png_malloc_array has already
             * checked it when doing the allocation.
             */
            memcpy(
                (*np).entries as *mut u8,
                (*entries).entries as *const u8,
                ((*entries).nentries as c_uint as usize)
                    * core::mem::size_of::<png_sPLT_entry>(),
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
            break 'do_loop;
        }
    }

    png_free(png_ptr, old_spalettes as png_voidp);

    if nentries > 0 {
        png_chunk_report(
            png_ptr,
            c"sPLT out of memory".as_ptr(),
            PNG_CHUNK_WRITE_ERROR,
        );
    }
}

pub unsafe fn check_location(png_ptr: png_const_structrp, location: c_int) -> png_byte {
    let mut location = location;

    location &= (PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT) as c_int;

    /* New in 1.6.0; copy the location and check it.  This is an API
     * change; previously the app had to use the
     * png_set_unknown_chunk_location API below for each chunk.
     */
    if location == 0 && ((*png_ptr).mode & PNG_IS_READ_STRUCT) == 0 {
        /* Write struct, so unknown chunks come from the app */
        png_app_warning(
            png_ptr,
            c"png_set_unknown_chunks now expects a valid location".as_ptr(),
        );
        /* Use the old behavior */
        location = ((*png_ptr).mode & (PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT))
            as png_byte as c_int;
    }

    /* This need not be an internal error - if the app calls
     * png_set_unknown_chunks on a read pointer it must get the location right.
     */
    if location == 0 {
        png_error(png_ptr, c"invalid location in png_set_unknown_chunks".as_ptr());
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
pub unsafe extern "C-unwind" fn png_set_unknown_chunks(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    unknowns: png_const_unknown_chunkp,
    num_unknowns: c_int,
) {
    let mut np: png_unknown_chunkp;
    let old_unknowns: png_unknown_chunkp;
    let mut unknowns = unknowns;
    let mut num_unknowns = num_unknowns;

    if png_ptr.is_null() || info_ptr.is_null() || num_unknowns <= 0 || unknowns.is_null() {
        return;
    }

    /* Check for the failure cases where support has been disabled at compile
     * time.  This code is hardly ever compiled - it's here because
     * STORE_UNKNOWN_CHUNKS is set by both read and write code (compiling in this
     * code) but may be meaningless if the read or write handling of unknown
     * chunks is not compiled in.
     */

    /* Prior to 1.6.0 this code used png_malloc_warn; however, this meant that
     * unknown critical chunks could be lost with just a warning resulting in
     * undefined behavior.  Now png_chunk_report is used to provide behavior
     * appropriate to read or write.
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
            c"too many unknown chunks".as_ptr(),
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    /* Defer freeing the old array until after the copy loop below,
     * in case unknowns aliases info_ptr->unknown_chunks (getter-to-setter).
     */
    old_unknowns = (*info_ptr).unknown_chunks;

    (*info_ptr).unknown_chunks = np; /* safe because it is initialized */
    (*info_ptr).free_me |= PNG_FREE_UNKN;

    np = np.add((*info_ptr).unknown_chunks_num as usize);

    /* Increment unknown_chunks_num each time round the loop to protect the
     * just-allocated chunk data.
     */
    while num_unknowns > 0 {
        'continue_loop: {
            memcpy(
                (*np).name.as_mut_ptr() as *mut u8,
                (*unknowns).name.as_ptr() as *const u8,
                5,
            );
            (*np).name[5 - 1] = b'\0';
            (*np).location = check_location(png_ptr, (*unknowns).location as c_int);

            if (*unknowns).size == 0 {
                (*np).data = core::ptr::null_mut();
                (*np).size = 0;
            } else {
                (*np).data = png_malloc_base(png_ptr, (*unknowns).size) as png_bytep;

                if (*np).data.is_null() {
                    png_chunk_report(
                        png_ptr,
                        c"unknown chunk: out of memory".as_ptr(),
                        PNG_CHUNK_WRITE_ERROR,
                    );
                    /* But just skip storing the unknown chunk */
                    break 'continue_loop;
                }

                memcpy(
                    (*np).data as *mut u8,
                    (*unknowns).data as *const u8,
                    (*unknowns).size,
                );
                (*np).size = (*unknowns).size;
            }

            /* These increments are skipped on out-of-memory for the data - the
             * unknown chunk entry gets overwritten if the png_chunk_report returns.
             * This is correct in the read case (the chunk is just dropped.)
             */
            np = np.add(1);
            (*info_ptr).unknown_chunks_num += 1;
        }

        num_unknowns -= 1;
        unknowns = unknowns.add(1);
    }

    png_free(png_ptr, old_unknowns as png_voidp);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_unknown_chunk_location(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    chunk: c_int,
    location: c_int,
) {
    let mut location = location;

    /* This API is pretty pointless in 1.6.0 because the location can be set
     * before the call to png_set_unknown_chunks.
     *
     * TODO: add a png_app_warning in 1.7
     */
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && chunk >= 0
        && chunk < (*info_ptr).unknown_chunks_num
    {
        if (location & (PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT) as c_int) == 0 {
            png_app_error(png_ptr, c"invalid unknown chunk location".as_ptr());
            /* Fake out the pre 1.6.0 behavior: */
            if ((location as c_uint) & PNG_HAVE_IDAT) != 0
            /* undocumented! */
            {
                location = PNG_AFTER_IDAT as c_int;
            } else {
                location = PNG_HAVE_IHDR as c_int; /* also undocumented */
            }
        }

        (*(*info_ptr).unknown_chunks.add(chunk as usize)).location =
            check_location(png_ptr, location);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_permit_mng_features(
    png_ptr: png_structrp,
    mng_features: png_uint_32,
) -> png_uint_32 {
    if png_ptr.is_null() {
        return 0;
    }

    (*png_ptr).mng_features_permitted = mng_features & PNG_ALL_MNG_FEATURES;

    (*png_ptr).mng_features_permitted
}

pub unsafe fn add_one_chunk(
    list: png_bytep,
    count: c_uint,
    add: png_const_bytep,
    keep: c_int,
) -> c_uint {
    let mut i: c_uint;
    let mut list = list;
    let mut count = count;

    /* Utility function: update the 'keep' state of a chunk if it is already in
     * the list, otherwise add it to the list.
     */
    i = 0;
    while i < count {
        if memcmp(list as *const u8, add as *const u8, 4) == 0 {
            *list.add(4) = keep as png_byte;

            return count;
        }

        i += 1;
        list = list.add(5);
    }

    if keep != PNG_HANDLE_CHUNK_AS_DEFAULT {
        count += 1;
        memcpy(list as *mut u8, add as *const u8, 4);
        *list.add(4) = keep as png_byte;
    }

    count
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_keep_unknown_chunks(
    png_ptr: png_structrp,
    keep: c_int,
    chunk_list: png_const_bytep,
    num_chunks_in: c_int,
) {
    let mut new_list: png_bytep;
    let mut num_chunks: c_uint;
    let mut old_num_chunks: c_uint;
    let mut chunk_list = chunk_list;

    if png_ptr.is_null() {
        return;
    }

    if keep < 0 || keep >= PNG_HANDLE_CHUNK_LAST {
        png_app_error(
            png_ptr,
            c"png_set_keep_unknown_chunks: invalid keep".as_ptr(),
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

        chunk_list = chunks_to_ignore.as_ptr();
        num_chunks = (105 as c_uint) / 5u32 /*SAFE*/;
    } else
    /* num_chunks_in > 0 */
    {
        if chunk_list.is_null() {
            /* Prior to 1.6.0 this was silently ignored, now it is an app_error
             * which can be switched off.
             */
            png_app_error(
                png_ptr,
                c"png_set_keep_unknown_chunks: no chunk list".as_ptr(),
            );

            return;
        }

        num_chunks = num_chunks_in as c_uint;
    }

    old_num_chunks = (*png_ptr).num_chunk_list;
    if (*png_ptr).chunk_list.is_null() {
        old_num_chunks = 0;
    }

    /* Since num_chunks is always restricted to UINT_MAX/5 this can't overflow.
     */
    if num_chunks.wrapping_add(old_num_chunks) > c_uint::MAX / 5 {
        png_app_error(
            png_ptr,
            c"png_set_keep_unknown_chunks: too many chunks".as_ptr(),
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
            (5u32.wrapping_mul(num_chunks.wrapping_add(old_num_chunks))) as usize,
        ) as png_bytep;

        if old_num_chunks > 0 {
            memcpy(
                new_list as *mut u8,
                (*png_ptr).chunk_list as *const u8,
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
    if !new_list.is_null() {
        let mut inlist: png_const_bytep;
        let mut outlist: png_bytep;
        let mut i: c_uint;

        i = 0;
        while i < num_chunks {
            old_num_chunks = add_one_chunk(
                new_list,
                old_num_chunks,
                chunk_list.add(5 * i as usize),
                keep,
            );
            i += 1;
        }

        /* Now remove any spurious 'default' entries. */
        num_chunks = 0;
        i = 0;
        outlist = new_list;
        inlist = new_list as png_const_bytep;
        while i < old_num_chunks {
            if *inlist.add(4) != 0 {
                if outlist as png_const_bytep != inlist {
                    memcpy(outlist as *mut u8, inlist as *const u8, 5);
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_read_user_chunk_fn(
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_rows(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    row_pointers: png_bytepp,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    if !(*info_ptr).row_pointers.is_null() && ((*info_ptr).row_pointers != row_pointers) {
        png_free_data(png_ptr, info_ptr, PNG_FREE_ROWS, 0);
    }

    (*info_ptr).row_pointers = row_pointers;

    if !row_pointers.is_null() {
        (*info_ptr).valid |= PNG_INFO_IDAT;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_compression_buffer_size(
    png_ptr: png_structrp,
    size: usize,
) {
    let mut size = size;

    if png_ptr.is_null() {
        return;
    }

    if size == 0 || size > PNG_UINT_31_MAX as usize {
        png_error(png_ptr, c"invalid compression buffer size".as_ptr());
    }

    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 {
        (*png_ptr).IDAT_read_size = size as png_uint_32; /* checked above */
        return;
    }

    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) == 0 {
        if (*png_ptr).zowner != 0 {
            png_warning(
                png_ptr,
                c"Compression buffer size cannot be changed because it is in use".as_ptr(),
            );

            return;
        }

        /* Some compilers complain that this is always false.  However, it
         * can be true when integer overflow happens.
         */
        if size > zlib::ZLIB_IO_MAX as usize {
            png_warning(
                png_ptr,
                c"Compression buffer size limited to system maximum".as_ptr(),
            );
            size = zlib::ZLIB_IO_MAX as usize; /* must fit */
        }

        if size < 6 {
            /* Deflate will potentially go into an infinite loop on a SYNC_FLUSH
             * if this is permitted.
             */
            png_warning(
                png_ptr,
                c"Compression buffer size cannot be reduced below 6".as_ptr(),
            );

            return;
        }

        if (*png_ptr).zbuffer_size as usize != size {
            png_free_buffer_list(png_ptr, core::ptr::addr_of_mut!((*png_ptr).zbuffer_list));
            (*png_ptr).zbuffer_size = size as uInt;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_invalid(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mask: c_int,
) {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        (*info_ptr).valid &= (!mask) as c_uint;
    }
}

/* This function was added to libpng 1.2.6 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_user_limits(
    png_ptr: png_structrp,
    user_width_max: png_uint_32,
    user_height_max: png_uint_32,
) {
    /* Images with dimensions larger than these limits will be
     * rejected by png_set_IHDR().  To accept any PNG datastream
     * regardless of dimensions, set both limits to 0x7fffffff.
     */
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).user_width_max = user_width_max;
    (*png_ptr).user_height_max = user_height_max;
}

/* This function was added to libpng 1.4.0 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_chunk_cache_max(
    png_ptr: png_structrp,
    user_chunk_cache_max: png_uint_32,
) {
    if !png_ptr.is_null() {
        (*png_ptr).user_chunk_cache_max = user_chunk_cache_max;
    }
}

/* This function was added to libpng 1.4.1 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_chunk_malloc_max(
    png_ptr: png_structrp,
    user_chunk_malloc_max: png_alloc_size_t,
) {
    /* pngstruct::user_chunk_malloc_max is initialized to a non-zero value in
     * png.c.  This API supports '0' for unlimited, make sure the correct
     * (unlimited) value is set here to avoid a need to check for 0 everywhere
     * the parameter is used.
     */
    if !png_ptr.is_null() {
        if user_chunk_malloc_max == 0 {
            /* unlimited */
            (*png_ptr).user_chunk_malloc_max = PNG_SIZE_MAX;
        } else {
            (*png_ptr).user_chunk_malloc_max = user_chunk_malloc_max;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_benign_errors(png_ptr: png_structrp, allowed: c_int) {
    /* If allowed is 1, png_benign_error() is treated as a warning.
     *
     * If allowed is 0, png_benign_error() is treated as an error (which
     * is the default behavior if png_set_benign_errors() is not called).
     */

    if allowed != 0 {
        (*png_ptr).flags |=
            PNG_FLAG_BENIGN_ERRORS_WARN | PNG_FLAG_APP_WARNINGS_WARN | PNG_FLAG_APP_ERRORS_WARN;
    } else {
        (*png_ptr).flags &= !(PNG_FLAG_BENIGN_ERRORS_WARN
            | PNG_FLAG_APP_WARNINGS_WARN
            | PNG_FLAG_APP_ERRORS_WARN);
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
pub unsafe extern "C-unwind" fn png_set_check_for_invalid_index(
    png_ptr: png_structrp,
    allowed: c_int,
) {
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
pub unsafe extern "C-unwind" fn png_check_keyword(
    png_ptr: png_structrp,
    key: png_const_charp,
    new_key: png_bytep,
) -> png_uint_32 {
    let orig_key: png_const_charp = key;
    let mut key_len: png_uint_32 = 0;
    let mut bad_character: c_int = 0;
    let mut space: c_int = 1;
    let mut key = key;
    let mut new_key = new_key;

    if key.is_null() {
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
        png_warning(png_ptr, c"keyword truncated".as_ptr());
    } else if bad_character != 0 {
        let mut p: png_warning_parameters =
            [[0; PNG_WARNING_PARAMETER_SIZE]; PNG_WARNING_PARAMETER_COUNT];

        png_warning_parameter(p.as_mut_ptr(), 1, orig_key);
        png_warning_parameter_signed(p.as_mut_ptr(), 2, PNG_NUMBER_FORMAT_02x, bad_character);

        png_formatted_warning(
            png_ptr,
            p.as_mut_ptr(),
            c"keyword \"@1\": bad character '0x@2'".as_ptr(),
        );
    }

    key_len
}
