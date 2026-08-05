// pngrutil.c part B1a (lines ~2759-3217): png_handle_unknown, the read_chunks
// table, png_chunk_index_from_name and png_handle_chunk.

// PNG_CHUNK_ANCILLARY(c) == (1 & ((c) >> 29)) ; CRITICAL == !ANCILLARY (pngpriv.h)
#[inline]
fn PNG_CHUNK_ANCILLARY(c: png_uint_32) -> bool {
    (1 & (c >> 29)) != 0
}
#[inline]
fn PNG_CHUNK_CRITICAL(c: png_uint_32) -> bool {
    !PNG_CHUNK_ANCILLARY(c)
}

// png_chunk_max(png_ptr) with PNG_SET_USER_LIMITS_SUPPORTED (active):
//   ((png_ptr)->user_chunk_malloc_max)
#[inline]
unsafe fn png_chunk_max(png_ptr: png_const_structrp) -> png_alloc_size_t {
    (*png_ptr).user_chunk_malloc_max
}

// png_file_has_chunk / png_file_add_chunk (pngstruct.h) implemented via the
// chunk flag bit: 0x80000000U >> (31 - i).
#[inline]
fn png_chunk_flag_from_index(i: png_index) -> png_uint_32 {
    0x80000000u32 >> (31 - i)
}
#[inline]
unsafe fn png_file_has_chunk(png_ptr: png_const_structrp, i: png_index) -> bool {
    ((*png_ptr).chunks & png_chunk_flag_from_index(i)) != 0
}
#[inline]
unsafe fn png_file_add_chunk(png_ptr: png_structrp, i: png_index) {
    (*png_ptr).chunks |= png_chunk_flag_from_index(i);
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

    /* PNG_READ_UNKNOWN_CHUNKS_SUPPORTED and PNG_HANDLE_AS_UNKNOWN_SUPPORTED are
     * both defined, so 'keep' is passed in from the caller (no lookup here).
     */

    /* PNG_READ_USER_CHUNKS_SUPPORTED: the user callback takes precedence over
     * the chunk keep value, but the keep value is still required to validate a
     * save of a critical chunk.
     */
    if (*png_ptr).read_user_chunk_fn.is_some() {
        if png_cache_unknown_chunk(png_ptr, length) != 0 {
            /* Callback to user unknown chunk handler */
            let ret: c_int = ((*png_ptr).read_user_chunk_fn.unwrap())(
                png_ptr,
                &mut (*png_ptr).unknown_chunk,
            );

            if ret < 0 {
                /* handled_error */
                png_chunk_error(png_ptr, c"error in user chunk".as_ptr());
            } else if ret == 0 {
                /* If the keep value is 'default' or 'never' override it, but
                 * still error out on critical chunks unless the keep value is
                 * 'always'.
                 */
                if keep < PNG_HANDLE_CHUNK_IF_SAFE {
                    /* PNG_SET_UNKNOWN_CHUNKS_SUPPORTED */
                    if (*png_ptr).unknown_default < PNG_HANDLE_CHUNK_IF_SAFE {
                        png_chunk_warning(png_ptr, c"Saving unknown chunk:".as_ptr());
                        png_app_warning(
                            png_ptr,
                            c"forcing save of an unhandled chunk; please call png_set_keep_unknown_chunks".as_ptr(),
                        );
                    }
                    keep = PNG_HANDLE_CHUNK_IF_SAFE;
                }
            } else {
                /* chunk was handled */
                handled = handled_ok;
                /* Critical chunks can be safely discarded at this point. */
                keep = PNG_HANDLE_CHUNK_NEVER;
            }
        } else {
            keep = PNG_HANDLE_CHUNK_NEVER; /* insufficient memory */
        }
    } else {
        /* PNG_SAVE_UNKNOWN_CHUNKS_SUPPORTED */
        /* keep is currently just the per-chunk setting, if there was no setting
         * change it to the global default now then obtain the cache of the
         * chunk if required, if not simply skip the chunk.
         */
        if keep == PNG_HANDLE_CHUNK_AS_DEFAULT {
            keep = (*png_ptr).unknown_default;
        }

        if keep == PNG_HANDLE_CHUNK_ALWAYS
            || (keep == PNG_HANDLE_CHUNK_IF_SAFE
                && PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name))
        {
            if png_cache_unknown_chunk(png_ptr, length) == 0 {
                keep = PNG_HANDLE_CHUNK_NEVER;
            }
        } else {
            png_crc_finish(png_ptr, length);
        }
    }

    /* PNG_STORE_UNKNOWN_CHUNKS_SUPPORTED: now store the chunk in the chunk list
     * if appropriate, and if the limits permit it.
     */
    if keep == PNG_HANDLE_CHUNK_ALWAYS
        || (keep == PNG_HANDLE_CHUNK_IF_SAFE
            && PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name))
    {
        /* PNG_USER_LIMITS_SUPPORTED */
        match (*png_ptr).user_chunk_cache_max {
            2 => {
                (*png_ptr).user_chunk_cache_max = 1;
                png_chunk_benign_error(png_ptr, c"no space in chunk cache".as_ptr());
                /* FALLTHROUGH to case 1 */
                /* case 1: NOTE: prior to 1.6.0 this case resulted in an unknown
                 * critical chunk being skipped, now there will be a hard error
                 * below.
                 */
            }
            1 => {
                /* break */
            }
            0 => {
                /* no limit: store the chunk. */
                png_set_unknown_chunks(png_ptr, info_ptr, &mut (*png_ptr).unknown_chunk, 1);
                handled = handled_saved;
            }
            _ => {
                /* not at limit */
                (*png_ptr).user_chunk_cache_max -= 1;
                /* FALLTHROUGH to case 0: store the chunk. */
                png_set_unknown_chunks(png_ptr, info_ptr, &mut (*png_ptr).unknown_chunk, 1);
                handled = handled_saved;
            }
        }
    }

    /* Regardless of the error handling below the cached data (if any) can be
     * freed now.
     */
    if !(*png_ptr).unknown_chunk.data.is_null() {
        png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
    }
    (*png_ptr).unknown_chunk.data = ptr::null_mut();

    /* Check for unhandled critical chunks */
    if handled < handled_saved && PNG_CHUNK_CRITICAL((*png_ptr).chunk_name) {
        png_chunk_error(png_ptr, c"unhandled critical chunk".as_ptr());
    }

    handled
}

/* APNG handling: png_handle_acTL/fcTL/fdAT are NULL (handled as unknown). */

/* 1.6.47 table driven interface to chunk handling. read_chunks describes the
 * PNG standard rules for reading known chunks.
 */

// NoCheck / Limit sentinels (pngrutil.c)
const NoCheck: png_uint_32 = 0x801;
const Limit: png_uint_32 = 0x802;
const LKMin: png_uint_32 = 3 + LZ77Min as png_uint_32;

// PNG_HAVE_ position flags used in the table.
const hIHDR: png_uint_32 = PNG_HAVE_IHDR;
const hPLTE: png_uint_32 = PNG_HAVE_PLTE;
const hIDAT: png_uint_32 = PNG_HAVE_IDAT;
const hCOL: png_uint_32 = PNG_HAVE_PLTE | PNG_HAVE_IDAT;
const aIDAT: png_uint_32 = PNG_AFTER_IDAT;

type ReadChunkHandler =
    Option<unsafe fn(png_structrp, png_inforp, png_uint_32) -> png_handle_result_code>;

struct ReadChunkEntry {
    handler: ReadChunkHandler,
    max_length: png_uint_32, /* :12 */
    min_length: png_uint_32, /* :8 */
    pos_before: png_uint_32, /* :4 */
    pos_after: png_uint_32,  /* :4 */
    multiple: png_uint_32,   /* :1 */
}

// The read_chunks table, in PNG_KNOWN_CHUNKS order (index 0..PNG_INDEX_unknown).
// Each row: { handler, max_len, min, before, after, multiple }.
static read_chunks: [ReadChunkEntry; PNG_INDEX_unknown as usize] = [
    // IHDR: CDIHDR 13, 13, hIHDR, 0, 0
    ReadChunkEntry { handler: Some(png_handle_IHDR), max_length: 13, min_length: 13, pos_before: hIHDR, pos_after: 0, multiple: 0 },
    // PLTE: CDPLTE NoCheck, 0, 0, hIHDR, 1
    ReadChunkEntry { handler: Some(png_handle_PLTE), max_length: NoCheck, min_length: 0, pos_before: 0, pos_after: hIHDR, multiple: 1 },
    // IDAT: handler NULL; CDIDAT NoCheck, 0, aIDAT, hIHDR, 1
    ReadChunkEntry { handler: None, max_length: NoCheck, min_length: 0, pos_before: aIDAT, pos_after: hIHDR, multiple: 1 },
    // IEND: CDIEND NoCheck, 0, 0, aIDAT, 0
    ReadChunkEntry { handler: Some(png_handle_IEND), max_length: NoCheck, min_length: 0, pos_before: 0, pos_after: aIDAT, multiple: 0 },
    // acTL: handler NULL; CDacTL 8, 8, hIDAT, hIHDR, 0
    ReadChunkEntry { handler: None, max_length: 8, min_length: 8, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    // bKGD: CDbKGD 6, 1, hIDAT, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_bKGD), max_length: 6, min_length: 1, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    // cHRM: CDcHRM 32, 32, hCOL, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_cHRM), max_length: 32, min_length: 32, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    // cICP: CDcICP 4, 4, hCOL, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_cICP), max_length: 4, min_length: 4, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    // cLLI: CDcLLI 8, 8, hCOL, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_cLLI), max_length: 8, min_length: 8, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    // eXIf: CDeXIf Limit, 4, 0, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_eXIf), max_length: Limit, min_length: 4, pos_before: 0, pos_after: hIHDR, multiple: 0 },
    // fcTL: handler NULL; CDfcTL 25, 26, 0, hIHDR, 1
    ReadChunkEntry { handler: None, max_length: 25, min_length: 26, pos_before: 0, pos_after: hIHDR, multiple: 1 },
    // fdAT: handler NULL; CDfdAT Limit, 4, hIDAT, hIHDR, 1
    ReadChunkEntry { handler: None, max_length: Limit, min_length: 4, pos_before: hIDAT, pos_after: hIHDR, multiple: 1 },
    // gAMA: CDgAMA 4, 4, hCOL, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_gAMA), max_length: 4, min_length: 4, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    // hIST: CDhIST 1024, 0, hPLTE, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_hIST), max_length: 1024, min_length: 0, pos_before: hPLTE, pos_after: hIHDR, multiple: 0 },
    // iCCP: CDiCCP NoCheck, LKMin, hCOL, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_iCCP), max_length: NoCheck, min_length: LKMin, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    // iTXt: CDiTXt NoCheck, 6, 0, hIHDR, 1
    ReadChunkEntry { handler: Some(png_handle_iTXt), max_length: NoCheck, min_length: 6, pos_before: 0, pos_after: hIHDR, multiple: 1 },
    // mDCV: CDmDCV 24, 24, hCOL, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_mDCV), max_length: 24, min_length: 24, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    // oFFs: CDoFFs 9, 9, hIDAT, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_oFFs), max_length: 9, min_length: 9, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    // pCAL: CDpCAL NoCheck, 14, hIDAT, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_pCAL), max_length: NoCheck, min_length: 14, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    // pHYs: CDpHYs 9, 9, hIDAT, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_pHYs), max_length: 9, min_length: 9, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    // sBIT: CDsBIT 4, 1, hCOL, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_sBIT), max_length: 4, min_length: 1, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    // sCAL: CDsCAL Limit, 4, hIDAT, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_sCAL), max_length: Limit, min_length: 4, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    // sPLT: CDsPLT NoCheck, 3, hIDAT, hIHDR, 1
    ReadChunkEntry { handler: Some(png_handle_sPLT), max_length: NoCheck, min_length: 3, pos_before: hIDAT, pos_after: hIHDR, multiple: 1 },
    // sRGB: CDsRGB 1, 1, hCOL, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_sRGB), max_length: 1, min_length: 1, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    // tEXt: CDtEXt NoCheck, 2, 0, hIHDR, 1
    ReadChunkEntry { handler: Some(png_handle_tEXt), max_length: NoCheck, min_length: 2, pos_before: 0, pos_after: hIHDR, multiple: 1 },
    // tIME: CDtIME 7, 7, 0, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_tIME), max_length: 7, min_length: 7, pos_before: 0, pos_after: hIHDR, multiple: 0 },
    // tRNS: CDtRNS 256, 0, hIDAT, hIHDR, 0
    ReadChunkEntry { handler: Some(png_handle_tRNS), max_length: 256, min_length: 0, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    // zTXt: CDzTXt Limit, LKMin, 0, hIHDR, 1
    ReadChunkEntry { handler: Some(png_handle_zTXt), max_length: Limit, min_length: LKMin, pos_before: 0, pos_after: hIHDR, multiple: 1 },
];

fn png_chunk_index_from_name(chunk_name: png_uint_32) -> png_index {
    /* For chunk png_cHNK return PNG_INDEX_cHNK. Return PNG_INDEX_unknown if
     * chunk_name is not known.
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
    let chunk_name: png_uint_32 = (*png_ptr).chunk_name;
    let chunk_index: png_index = png_chunk_index_from_name(chunk_name);

    let mut handled: png_handle_result_code = handled_error;
    let mut errmsg: png_const_charp = ptr::null();

    /* Is this a known chunk? If not there are no checks performed here. */
    if chunk_index == PNG_INDEX_unknown
        || read_chunks[chunk_index as usize].handler.is_none()
    {
        handled = png_handle_unknown(png_ptr, info_ptr, length, PNG_HANDLE_CHUNK_AS_DEFAULT);
    }
    /* First check the position. The stream must start with IHDR. */
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
    /* Now check for duplicates. */
    else if read_chunks[chunk_index as usize].multiple == 0
        && png_file_has_chunk(png_ptr, chunk_index)
    {
        errmsg = c"duplicate".as_ptr();
    } else if length < read_chunks[chunk_index as usize].min_length {
        errmsg = c"too short".as_ptr();
    } else {
        let max_length: png_uint_32 = read_chunks[chunk_index as usize].max_length;

        // Emulate the C switch with goto MeetsLimit.
        let mut meets_limit = false;
        if max_length == Limit {
            if length as png_alloc_size_t <= png_chunk_max(png_ptr) {
                meets_limit = true;
            } else {
                errmsg = c"length exceeds libpng limit".as_ptr();
            }
        } else if max_length == NoCheck {
            meets_limit = true;
        } else {
            /* default */
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

    /* If there was an error or the chunk was simply skipped it is not counted
     * as 'seen'.
     */
    if !errmsg.is_null() {
        if PNG_CHUNK_CRITICAL(chunk_name) {
            /* stop immediately */
            png_chunk_error(png_ptr, errmsg);
        } else {
            /* ancillary chunk: skip the data */
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
