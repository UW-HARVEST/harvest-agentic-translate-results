// pngrutil.c - utilities to read a PNG file
//
// Chunk 6: png_cache_unknown_chunk .. png_handle_chunk

use crate::*;

/* Utility function for png_handle_unknown; set up png_ptr::unknown_chunk */
unsafe fn png_cache_unknown_chunk(png_ptr: png_structrp, length: png_uint_32) -> c_int {
    let limit: png_alloc_size_t = png_chunk_max(png_ptr);

    if !(*png_ptr).unknown_chunk.data.is_null() {
        png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
        (*png_ptr).unknown_chunk.data = core::ptr::null_mut();
    }

    if (length as png_alloc_size_t) <= limit {
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
        png_chunk_benign_error(png_ptr, cstr!("unknown chunk exceeds memory limits"));
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
    keep: c_int,
) -> png_handle_result_code {
    let mut keep: c_int = keep;
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
                png_ptr as png_structp,
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
                png_chunk_error(png_ptr, cstr!("error in user chunk"));
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
                        png_chunk_warning(png_ptr, cstr!("Saving unknown chunk:"));
                        png_app_warning(
                            png_ptr,
                            cstr!(
                                "forcing save of an unhandled chunk; please call png_set_keep_unknown_chunks"
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
                png_chunk_benign_error(png_ptr, cstr!("no space in chunk cache"));
                /* FALLTHROUGH */
                /* case 1: */
                /* NOTE: prior to 1.6.0 this case resulted in an unknown critical
                 * chunk being skipped, now there will be a hard error below.
                 */
                /* break */
            }

            1 => {
                /* NOTE: prior to 1.6.0 this case resulted in an unknown critical
                 * chunk being skipped, now there will be a hard error below.
                 */
            }

            0 =>
            /* no limit */
            {
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

            _ =>
            /* default: not at limit */
            {
                (*png_ptr).user_chunk_cache_max -= 1;
                /* FALLTHROUGH */
                /* case 0: no limit */

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
        png_chunk_error(png_ptr, cstr!("unhandled critical chunk"));
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
 *
 * (png_handle_IDAT is NULL for the same kind of reason: IDAT is handled by the
 * row reading code, not by a chunk handler.)
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
type png_chunk_handler_fn =
    unsafe extern "C" fn(png_structrp, png_inforp, png_uint_32) -> png_handle_result_code;

#[derive(Copy, Clone)]
struct png_read_chunk_rules {
    /* A chunk-specific 'handler', NULL if the chunk is not supported in this
     * build.
     */
    handler: Option<png_chunk_handler_fn>,

    /* Crushing these values helps on modern 32-bit architectures because the
     * pointer and the following bit fields both end up requiring 32 bits.
     * Typically this will halve the table size.  On 64-bit architectures the
     * table entries will typically be 8 bytes.
     *
     * In C these are bit fields; every value below is within the width of its
     * field so no truncation occurs and plain integers behave identically.
     */
    max_length: png_uint_32, /* :12  Length min, max in bytes */
    min_length: png_uint_32, /* :8 */
    /* Length errors on critical chunks have special handling to preserve the
     * existing behaviour in libpng 1.6.  Ancillary chunks are checked below
     * and produce a 'benign' error.
     */
    pos_before: png_uint_32, /* :4  PNG_HAVE_ values chunk must precede */
    pos_after: png_uint_32,  /* :4  PNG_HAVE_ values chunk must follow */
    /* NOTE: PLTE, tRNS and bKGD require special handling which depends on
     * the colour type of the base image.
     */
    multiple: png_uint_32, /* :1  Multiple occurrences permitted */
                           /* This is enabled for PLTE because PLTE may, in practice, be optional */
}

/* Definitions as in the C source but done indirectly by #define so that
 * PNG_KNOWN_CHUNKS can be used safely to build the table in order.
 */
const NoCheck: c_uint = 0x801;
const Limit: c_uint = 0x802;
const LZ77Min: png_uint_32 = 2 + 5 + 4;
const LKMin: png_uint_32 = 3 + LZ77Min; /* Minimum length of keyword+LZ77 */

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

/* The entries are in PNG_KNOWN_CHUNKS order, i.e. indexed by PNG_INDEX_cHNK.
 *      cHNK  max_len,   min, before, after, multiple
 */
static read_chunks: [png_read_chunk_rules; PNG_INDEX_unknown as usize] = [
    /* IHDR: CDIHDR 13U, 13U, hIHDR, 0, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_IHDR),
        max_length: 13,
        min_length: 13,
        pos_before: hIHDR,
        pos_after: 0,
        multiple: 0,
    },
    /* PLTE: CDPLTE NoCheck, 0U, 0, hIHDR, 1
     * PLTE errors are only critical for colour-map images, consequently the
     * handler does all the checks.
     */
    png_read_chunk_rules {
        handler: Some(png_handle_PLTE),
        max_length: NoCheck,
        min_length: 0,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* IDAT: CDIDAT NoCheck, 0U, aIDAT, hIHDR, 1 (png_handle_IDAT is NULL) */
    png_read_chunk_rules {
        handler: None,
        max_length: NoCheck,
        min_length: 0,
        pos_before: aIDAT,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* IEND: CDIEND NoCheck, 0U, 0, aIDAT, 0
     * Historically data was allowed in IEND
     */
    png_read_chunk_rules {
        handler: Some(png_handle_IEND),
        max_length: NoCheck,
        min_length: 0,
        pos_before: 0,
        pos_after: aIDAT,
        multiple: 0,
    },
    /* acTL: CDacTL 8U, 8U, hIDAT, hIHDR, 0 (png_handle_acTL is NULL) */
    png_read_chunk_rules {
        handler: None,
        max_length: 8,
        min_length: 8,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* bKGD: CDbKGD 6U, 1U, hIDAT, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_bKGD),
        max_length: 6,
        min_length: 1,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* cHRM: CDcHRM 32U, 32U, hCOL, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_cHRM),
        max_length: 32,
        min_length: 32,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* cICP: CDcICP 4U, 4U, hCOL, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_cICP),
        max_length: 4,
        min_length: 4,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* cLLI: CDcLLI 8U, 8U, hCOL, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_cLLI),
        max_length: 8,
        min_length: 8,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* eXIf: CDeXIf Limit, 4U, 0, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_eXIf),
        max_length: Limit,
        min_length: 4,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* fcTL: CDfcTL 25U, 26U, 0, hIHDR, 1 (png_handle_fcTL is NULL) */
    png_read_chunk_rules {
        handler: None,
        max_length: 25,
        min_length: 26,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* fdAT: CDfdAT Limit, 4U, hIDAT, hIHDR, 1 (png_handle_fdAT is NULL) */
    png_read_chunk_rules {
        handler: None,
        max_length: Limit,
        min_length: 4,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* gAMA: CDgAMA 4U, 4U, hCOL, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_gAMA),
        max_length: 4,
        min_length: 4,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* hIST: CDhIST 1024U, 0U, hPLTE, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_hIST),
        max_length: 1024,
        min_length: 0,
        pos_before: hPLTE,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* iCCP: CDiCCP NoCheck, LKMin, hCOL, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_iCCP),
        max_length: NoCheck,
        min_length: LKMin,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* iTXt: CDiTXt NoCheck, 6U, 0, hIHDR, 1
     * Allocates 'length+1'; checked in the handler
     */
    png_read_chunk_rules {
        handler: Some(png_handle_iTXt),
        max_length: NoCheck,
        min_length: 6,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* mDCV: CDmDCV 24U, 24U, hCOL, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_mDCV),
        max_length: 24,
        min_length: 24,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* oFFs: CDoFFs 9U, 9U, hIDAT, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_oFFs),
        max_length: 9,
        min_length: 9,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* pCAL: CDpCAL NoCheck, 14U, hIDAT, hIHDR, 0
     * Allocates 'length+1'; checked in the handler
     */
    png_read_chunk_rules {
        handler: Some(png_handle_pCAL),
        max_length: NoCheck,
        min_length: 14,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* pHYs: CDpHYs 9U, 9U, hIDAT, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_pHYs),
        max_length: 9,
        min_length: 9,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* sBIT: CDsBIT 4U, 1U, hCOL, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_sBIT),
        max_length: 4,
        min_length: 1,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* sCAL: CDsCAL Limit, 4U, hIDAT, hIHDR, 0
     * Allocates 'length+1'; checked in the handler
     */
    png_read_chunk_rules {
        handler: Some(png_handle_sCAL),
        max_length: Limit,
        min_length: 4,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* sPLT: CDsPLT NoCheck, 3U, hIDAT, hIHDR, 1
     * Allocates 'length+1'; checked in the handler
     */
    png_read_chunk_rules {
        handler: Some(png_handle_sPLT),
        max_length: NoCheck,
        min_length: 3,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* sRGB: CDsRGB 1U, 1U, hCOL, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_sRGB),
        max_length: 1,
        min_length: 1,
        pos_before: hCOL,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* tEXt: CDtEXt NoCheck, 2U, 0, hIHDR, 1
     * Allocates 'length+1'; checked in the handler
     */
    png_read_chunk_rules {
        handler: Some(png_handle_tEXt),
        max_length: NoCheck,
        min_length: 2,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 1,
    },
    /* tIME: CDtIME 7U, 7U, 0, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_tIME),
        max_length: 7,
        min_length: 7,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* tRNS: CDtRNS 256U, 0U, hIDAT, hIHDR, 0 */
    png_read_chunk_rules {
        handler: Some(png_handle_tRNS),
        max_length: 256,
        min_length: 0,
        pos_before: hIDAT,
        pos_after: hIHDR,
        multiple: 0,
    },
    /* zTXt: CDzTXt Limit, LKMin, 0, hIHDR, 1 */
    png_read_chunk_rules {
        handler: Some(png_handle_zTXt),
        max_length: Limit,
        min_length: LKMin,
        pos_before: 0,
        pos_after: hIHDR,
        multiple: 1,
    },
];

unsafe fn png_chunk_index_from_name(chunk_name: png_uint_32) -> c_int {
    /* For chunk png_cHNK return PNG_INDEX_cHNK.  Return PNG_INDEX_unknown if
     * chunk_name is not known.  Notice that in a particular build "known" does
     * not necessarily mean "supported", although the inverse applies.
     */
    match chunk_name {
        crate::pngconsts::png_IHDR => PNG_INDEX_IHDR,
        crate::pngconsts::png_PLTE => PNG_INDEX_PLTE,
        crate::pngconsts::png_IDAT => PNG_INDEX_IDAT,
        crate::pngconsts::png_IEND => PNG_INDEX_IEND,
        crate::pngconsts::png_acTL => PNG_INDEX_acTL,
        crate::pngconsts::png_bKGD => PNG_INDEX_bKGD,
        crate::pngconsts::png_cHRM => PNG_INDEX_cHRM,
        crate::pngconsts::png_cICP => PNG_INDEX_cICP,
        crate::pngconsts::png_cLLI => PNG_INDEX_cLLI,
        crate::pngconsts::png_eXIf => PNG_INDEX_eXIf,
        crate::pngconsts::png_fcTL => PNG_INDEX_fcTL,
        crate::pngconsts::png_fdAT => PNG_INDEX_fdAT,
        crate::pngconsts::png_gAMA => PNG_INDEX_gAMA,
        crate::pngconsts::png_hIST => PNG_INDEX_hIST,
        crate::pngconsts::png_iCCP => PNG_INDEX_iCCP,
        crate::pngconsts::png_iTXt => PNG_INDEX_iTXt,
        crate::pngconsts::png_mDCV => PNG_INDEX_mDCV,
        crate::pngconsts::png_oFFs => PNG_INDEX_oFFs,
        crate::pngconsts::png_pCAL => PNG_INDEX_pCAL,
        crate::pngconsts::png_pHYs => PNG_INDEX_pHYs,
        crate::pngconsts::png_sBIT => PNG_INDEX_sBIT,
        crate::pngconsts::png_sCAL => PNG_INDEX_sCAL,
        crate::pngconsts::png_sPLT => PNG_INDEX_sPLT,
        crate::pngconsts::png_sRGB => PNG_INDEX_sRGB,
        crate::pngconsts::png_tEXt => PNG_INDEX_tEXt,
        crate::pngconsts::png_tIME => PNG_INDEX_tIME,
        crate::pngconsts::png_tRNS => PNG_INDEX_tRNS,
        crate::pngconsts::png_zTXt => PNG_INDEX_zTXt,

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
    if chunk_index == PNG_INDEX_unknown
        || read_chunks[chunk_index as usize].handler.is_none()
    {
        handled = png_handle_unknown(png_ptr, info_ptr, length, PNG_HANDLE_CHUNK_AS_DEFAULT);
    }
    /* First check the position.   The first check is historical; the stream must
     * start with IHDR and anything else causes libpng to give up immediately.
     */
    else if chunk_index != PNG_INDEX_IHDR && ((*png_ptr).mode & PNG_HAVE_IHDR) == 0 {
        png_chunk_error(png_ptr, cstr!("missing IHDR")); /* NORETURN */
    }
    /* Before all the pos_before chunks, after all the pos_after chunks. */
    else if ((*png_ptr).mode & read_chunks[chunk_index as usize].pos_before) != 0
        || ((*png_ptr).mode & read_chunks[chunk_index as usize].pos_after)
            != read_chunks[chunk_index as usize].pos_after
    {
        errmsg = cstr!("out of place");
    }
    /* Now check for duplicates: duplicated critical chunks also produce a
     * full error.
     */
    else if read_chunks[chunk_index as usize].multiple == 0
        && png_file_has_chunk(png_ptr, chunk_index)
    {
        errmsg = cstr!("duplicate");
    } else if length < read_chunks[chunk_index as usize].min_length {
        errmsg = cstr!("too short");
    } else {
        /* NOTE: apart from IHDR the critical chunks (PLTE, IDAT and IEND) are set
         * up above not to do any length checks.
         *
         * The png_chunk_max check ensures that the variable length chunks are
         * always checked at this point for being within the system allocation
         * limits.
         */
        let max_length: c_uint = read_chunks[chunk_index as usize].max_length as c_uint;

        /* 'goto MeetsLimit' out of the switch below. */
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
                    errmsg = cstr!("length exceeds libpng limit");
                }
            }

            NoCheck => {
                /* MeetsLimit: */
                meets_limit = true;
            }

            _ => {
                if length <= max_length {
                    meets_limit = true; /* goto MeetsLimit */
                } else {
                    errmsg = cstr!("too long");
                }
            }
        }

        if meets_limit {
            /* MeetsLimit: */
            handled = (read_chunks[chunk_index as usize].handler.unwrap())(
                png_ptr, info_ptr, length,
            );
        }
    }

    /* If there was an error or the chunk was simply skipped it is not counted as
     * 'seen'.
     */
    if !errmsg.is_null() {
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
