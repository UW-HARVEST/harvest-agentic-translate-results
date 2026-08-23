/* pngset.c lines 1156..1556 */

/* png_set_tIME */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_tIME(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mod_time: png_const_timep,
) {
    if png_ptr == core::ptr::null_mut()
        || info_ptr == core::ptr::null_mut()
        || mod_time == core::ptr::null()
        || ((*png_ptr).mode & PNG_WROTE_tIME) != 0
    {
        return;
    }

    if (*mod_time).month as c_int == 0
        || (*mod_time).month as c_int > 12
        || (*mod_time).day as c_int == 0
        || (*mod_time).day as c_int > 31
        || (*mod_time).hour as c_int > 23
        || (*mod_time).minute as c_int > 59
        || (*mod_time).second as c_int > 60
    {
        png_warning(
            png_ptr,
            b"Ignoring invalid time value\0".as_ptr() as png_const_charp,
        );

        return;
    }

    (*info_ptr).mod_time = *mod_time;
    (*info_ptr).valid |= PNG_INFO_tIME;
}

/* png_set_tRNS */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_tRNS(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    mut trans_alpha: png_const_bytep,
    mut num_trans: c_int,
    trans_color: png_const_color_16p,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    if trans_alpha != core::ptr::null() {
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
            /* Allocate info_ptr's copy of the transparency data.
             * Initialize all entries to fully opaque (0xff), then overwrite
             * the first num_trans entries with the actual values.
             */
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

    if trans_color != core::ptr::null() {
        if ((*info_ptr).bit_depth as c_int) < 16 {
            let sample_max: c_int = ((1 as c_int) << ((*info_ptr).bit_depth as c_int)) - 1;

            if ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY
                && (*trans_color).gray as c_int > sample_max)
                || ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB
                    && ((*trans_color).red as c_int > sample_max
                        || (*trans_color).green as c_int > sample_max
                        || (*trans_color).blue as c_int > sample_max))
            {
                png_warning(
                    png_ptr,
                    b"tRNS chunk has out-of-range samples for bit_depth\0".as_ptr()
                        as png_const_charp,
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

/* png_set_sPLT
 *
 *  entries        - array of png_sPLT_t structures
 *                   to be added to the list of palettes
 *                   in the info structure.
 *
 *  nentries       - number of palette structures to be
 *                   added.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sPLT(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mut entries: png_const_sPLT_tp,
    mut nentries: c_int,
) {
    let mut np: png_sPLT_tp;
    let old_spalettes: png_sPLT_tp;

    if png_ptr == core::ptr::null_mut()
        || info_ptr == core::ptr::null_mut()
        || nentries <= 0
        || entries == core::ptr::null()
    {
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

    if np == core::ptr::null_mut() {
        /* Out of memory or too many chunks */
        png_chunk_report(
            png_ptr,
            b"too many sPLT chunks\0".as_ptr() as png_const_charp,
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

    np = np.offset((*info_ptr).splt_palettes_num as isize);

    'main: loop {
        'cont: {
            let length: usize;

            /* Skip invalid input entries */
            if (*entries).name == core::ptr::null_mut()
                || (*entries).entries == core::ptr::null_mut()
            {
                /* png_handle_sPLT doesn't do this, so this is an app error */
                png_app_error(
                    png_ptr,
                    b"png_set_sPLT: invalid sPLT\0".as_ptr() as png_const_charp,
                );
                /* Just skip the invalid entry */
                break 'cont;
            }

            (*np).depth = (*entries).depth;

            /* In the event of out-of-memory just return - there's no point keeping
             * on trying to add sPLT chunks.
             */
            length = strlen((*entries).name) + 1;
            (*np).name = png_malloc_base(png_ptr, length as png_alloc_size_t) as png_charp;

            if (*np).name == core::ptr::null_mut() {
                break 'main;
            }

            memcpy(
                (*np).name as *mut c_void,
                (*entries).name as *const c_void,
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

            if (*np).entries == core::ptr::null_mut() {
                png_free(png_ptr, (*np).name as png_voidp);
                (*np).name = core::ptr::null_mut();
                break 'main;
            }

            (*np).nentries = (*entries).nentries;
            /* This multiply can't overflow because png_malloc_array has already
             * checked it when doing the allocation.
             */
            memcpy(
                (*np).entries as *mut c_void,
                (*entries).entries as *const c_void,
                ((*entries).nentries as c_uint as usize)
                    .wrapping_mul(core::mem::size_of::<png_sPLT_entry>()),
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
            b"sPLT out of memory\0".as_ptr() as png_const_charp,
            PNG_CHUNK_WRITE_ERROR,
        );
    }
}

/* check_location */
unsafe fn check_location(png_ptr: png_const_structrp, mut location: c_int) -> png_byte {
    location &= (PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT) as c_int;

    /* New in 1.6.0; copy the location and check it.  This is an API
     * change; previously the app had to use the
     * png_set_unknown_chunk_location API below for each chunk.
     */
    if location == 0 && ((*png_ptr).mode & PNG_IS_READ_STRUCT) == 0 {
        /* Write struct, so unknown chunks come from the app */
        png_app_warning(
            png_ptr,
            b"png_set_unknown_chunks now expects a valid location\0".as_ptr() as png_const_charp,
        );
        /* Use the old behavior */
        location = ((*png_ptr).mode & (PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT))
            as png_byte as c_int;
    }

    /* This need not be an internal error - if the app calls
     * png_set_unknown_chunks on a read pointer it must get the location right.
     */
    if location == 0 {
        png_error(
            png_ptr,
            b"invalid location in png_set_unknown_chunks\0".as_ptr() as png_const_charp,
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

/* png_set_unknown_chunks */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_unknown_chunks(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mut unknowns: png_const_unknown_chunkp,
    mut num_unknowns: c_int,
) {
    let mut np: png_unknown_chunkp;
    let old_unknowns: png_unknown_chunkp;

    if png_ptr == core::ptr::null_mut()
        || info_ptr == core::ptr::null_mut()
        || num_unknowns <= 0
        || unknowns == core::ptr::null()
    {
        return;
    }

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

    if np == core::ptr::null_mut() {
        png_chunk_report(
            png_ptr,
            b"too many unknown chunks\0".as_ptr() as png_const_charp,
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

    np = np.offset((*info_ptr).unknown_chunks_num as isize);

    /* Increment unknown_chunks_num each time round the loop to protect the
     * just-allocated chunk data.
     */
    while num_unknowns > 0 {
        'cont: {
            memcpy(
                core::ptr::addr_of_mut!((*np).name) as *mut c_void,
                core::ptr::addr_of!((*unknowns).name) as *const c_void,
                core::mem::size_of::<[png_byte; 5]>(),
            );
            (*np).name[core::mem::size_of::<[png_byte; 5]>() - 1] = 0; /* '\0' */
            (*np).location = check_location(png_ptr, (*unknowns).location as c_int);

            if (*unknowns).size == 0 {
                (*np).data = core::ptr::null_mut();
                (*np).size = 0;
            } else {
                (*np).data =
                    png_malloc_base(png_ptr, (*unknowns).size as png_alloc_size_t) as png_bytep;

                if (*np).data == core::ptr::null_mut() {
                    png_chunk_report(
                        png_ptr,
                        b"unknown chunk: out of memory\0".as_ptr() as png_const_charp,
                        PNG_CHUNK_WRITE_ERROR,
                    );
                    /* But just skip storing the unknown chunk */
                    break 'cont;
                }

                memcpy(
                    (*np).data as *mut c_void,
                    (*unknowns).data as *const c_void,
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

/* png_set_unknown_chunk_location */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_unknown_chunk_location(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    chunk: c_int,
    mut location: c_int,
) {
    /* This API is pretty pointless in 1.6.0 because the location can be set
     * before the call to png_set_unknown_chunks.
     *
     * TODO: add a png_app_warning in 1.7
     */
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && chunk >= 0
        && chunk < (*info_ptr).unknown_chunks_num
    {
        if (location & (PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT) as c_int) == 0 {
            png_app_error(
                png_ptr,
                b"invalid unknown chunk location\0".as_ptr() as png_const_charp,
            );
            /* Fake out the pre 1.6.0 behavior: */
            if ((location as c_uint) & PNG_HAVE_IDAT) != 0
            /* undocumented! */
            {
                location = PNG_AFTER_IDAT as c_int;
            } else {
                location = PNG_HAVE_IHDR as c_int; /* also undocumented */
            }
        }

        (*(*info_ptr).unknown_chunks.offset(chunk as isize)).location =
            check_location(png_ptr, location);
    }
}
