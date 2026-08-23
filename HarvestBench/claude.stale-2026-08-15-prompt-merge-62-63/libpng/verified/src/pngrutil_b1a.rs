// pngrutil.c part B1a (lines 2759-3225)

/* From pngpriv.h:
 *
 * Result of a call to png_handle_chunk made to handle the current chunk
 * png_struct::chunk_name on read.  Always informational, either the stream is
 * read for the next chunk or the routine will call png_error.
 *
 * NOTE: order is important internally.  handled_saved and above are regarded
 * as handling the chunk.
 */
pub type png_handle_result_code = c_int;
pub const handled_error: png_handle_result_code = 0; /* bad crc or known and bad format or too long */
pub const handled_discarded: png_handle_result_code = 1; /* not saved in the unknown chunk list */
pub const handled_saved: png_handle_result_code = 2; /* saved in the unknown chunk list */
pub const handled_ok: png_handle_result_code = 3; /* known, supported and handled without error */

/* Chunk index values as an enum (pngstruct.h), PNG_INDEX_unknown is also a
 * count of the number of chunks.  PNG_INDEX_bKGD, PNG_INDEX_cICP,
 * PNG_INDEX_mDCV and PNG_INDEX_tRNS are declared in pngrutil_a2.rs.
 */
pub type png_index = c_int;
const PNG_INDEX_IHDR: c_int = 0;
const PNG_INDEX_PLTE: c_int = 1;
const PNG_INDEX_IDAT: c_int = 2;
const PNG_INDEX_IEND: c_int = 3;
const PNG_INDEX_acTL: c_int = 4;
/* PNG_INDEX_bKGD == 5 (pngrutil_a2.rs) */
const PNG_INDEX_cHRM: c_int = 6;
/* PNG_INDEX_cICP == 7 (pngrutil_a2.rs) */
const PNG_INDEX_cLLI: c_int = 8;
const PNG_INDEX_eXIf: c_int = 9;
const PNG_INDEX_fcTL: c_int = 10;
const PNG_INDEX_fdAT: c_int = 11;
const PNG_INDEX_gAMA: c_int = 12;
const PNG_INDEX_hIST: c_int = 13;
const PNG_INDEX_iCCP: c_int = 14;
const PNG_INDEX_iTXt: c_int = 15;
/* PNG_INDEX_mDCV == 16 (pngrutil_a2.rs) */
const PNG_INDEX_oFFs: c_int = 17;
const PNG_INDEX_pCAL: c_int = 18;
const PNG_INDEX_pHYs: c_int = 19;
const PNG_INDEX_sBIT: c_int = 20;
const PNG_INDEX_sCAL: c_int = 21;
const PNG_INDEX_sPLT: c_int = 22;
const PNG_INDEX_sRGB: c_int = 23;
const PNG_INDEX_tEXt: c_int = 24;
const PNG_INDEX_tIME: c_int = 25;
/* PNG_INDEX_tRNS == 26 (pngrutil_a2.rs) */
const PNG_INDEX_zTXt: c_int = 27;
const PNG_INDEX_unknown: c_int = 28;

/* png_chunk_flag_from_index(i) == (0x80000000U >> (31 - (i))) (pngstruct.h) */
#[inline]
fn png_chunk_flag_from_index(i: c_int) -> png_uint_32 {
    0x80000000u32 >> (31 - i)
}

/* png_file_has_chunk(png_ptr, i) (pngstruct.h) */
#[inline]
unsafe fn png_file_has_chunk(png_ptr: png_const_structrp, i: c_int) -> bool {
    ((*png_ptr).chunks & png_chunk_flag_from_index(i)) != 0
}

/* png_file_add_chunk(png_ptr, i) (pngstruct.h) */
#[inline]
unsafe fn png_file_add_chunk(png_ptr: png_structrp, i: c_int) {
    (*png_ptr).chunks |= png_chunk_flag_from_index(i);
}

/* Test on flag values as defined in the spec (section 5.4), pngpriv.h:
 * PNG_CHUNK_ANCILLARY(c) == (1 & ((c) >> 29))
 * PNG_CHUNK_CRITICAL(c)  == (!PNG_CHUNK_ANCILLARY(c))
 */
#[inline]
fn PNG_CHUNK_ANCILLARY(c: png_uint_32) -> bool {
    (1 & (c >> 29)) != 0
}

#[inline]
fn PNG_CHUNK_CRITICAL(c: png_uint_32) -> bool {
    !PNG_CHUNK_ANCILLARY(c)
}

/* png_chunk_max(png_ptr) (pngpriv.h); PNG_SET_USER_LIMITS_SUPPORTED is defined
 * so this is the run-time limit.
 */
#[inline]
unsafe fn png_chunk_max(png_ptr: png_const_structrp) -> png_alloc_size_t {
    (*png_ptr).user_chunk_malloc_max
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

    /* png_debug(1, "in png_handle_unknown"); */

    /* PNG_READ_UNKNOWN_CHUNKS_SUPPORTED
     *
     * NOTE: this code is based on the code in libpng-1.4.12 except for fixing
     * the bug which meant that setting a non-default behavior for a specific
     * chunk would be ignored (the default was always used unless a user
     * callback was installed).
     *
     * 'keep' is the value from the png_chunk_unknown_handling, the setting for
     * this specific chunk_name, if PNG_HANDLE_AS_UNKNOWN_SUPPORTED, if not it
     * will always be PNG_HANDLE_CHUNK_AS_DEFAULT and it needs to be set here.
     * This is just an optimization to avoid multiple calls to the lookup
     * function.
     *
     * PNG_HANDLE_AS_UNKNOWN_SUPPORTED is defined, so 'keep' is already the
     * per-chunk setting.
     *
     * One of the following methods will read the chunk or skip it (at least one
     * of these is always defined because this is the only way to switch on
     * PNG_READ_UNKNOWN_CHUNKS_SUPPORTED)
     */
    /* PNG_READ_USER_CHUNKS_SUPPORTED
     * The user callback takes precedence over the chunk keep value, but the
     * keep value is still required to validate a save of a critical chunk.
     */
    if (*png_ptr).read_user_chunk_fn.is_some() {
        if png_cache_unknown_chunk(png_ptr, length) != 0 {
            /* Callback to user unknown chunk handler */
            let ret: c_int = ((*png_ptr).read_user_chunk_fn.unwrap())(
                png_ptr,
                &mut (*png_ptr).unknown_chunk,
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
            if ret < 0 {
                /* handled_error */
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
                    /* PNG_SET_UNKNOWN_CHUNKS_SUPPORTED */
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
            } else {
                /* chunk was handled */
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
        /* PNG_SAVE_UNKNOWN_CHUNKS_SUPPORTED
         *
         * keep is currently just the per-chunk setting, if there was no
         * setting change it to the global default now (not that this may
         * still be AS_DEFAULT) then obtain the cache of the chunk if required,
         * if not simply skip the chunk.
         */
        if keep == PNG_HANDLE_CHUNK_AS_DEFAULT {
            keep = (*png_ptr).unknown_default;
        }

        if keep == PNG_HANDLE_CHUNK_ALWAYS
            || (keep == PNG_HANDLE_CHUNK_IF_SAFE && PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name))
        {
            if png_cache_unknown_chunk(png_ptr, length) == 0 {
                keep = PNG_HANDLE_CHUNK_NEVER;
            }
        } else {
            png_crc_finish(png_ptr, length);
        }
    }

    /* PNG_STORE_UNKNOWN_CHUNKS_SUPPORTED
     * Now store the chunk in the chunk list if appropriate, and if the limits
     * permit it.
     */
    if keep == PNG_HANDLE_CHUNK_ALWAYS
        || (keep == PNG_HANDLE_CHUNK_IF_SAFE && PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name))
    {
        /* PNG_USER_LIMITS_SUPPORTED */
        let cache_max = (*png_ptr).user_chunk_cache_max;

        if cache_max == 2 {
            (*png_ptr).user_chunk_cache_max = 1;
            png_chunk_benign_error(png_ptr, c"no space in chunk cache".as_ptr());
            /* FALLTHROUGH to case 1 */
            /* NOTE: prior to 1.6.0 this case resulted in an unknown critical
             * chunk being skipped, now there will be a hard error below.
             */
        } else if cache_max == 1 {
            /* NOTE: prior to 1.6.0 this case resulted in an unknown critical
             * chunk being skipped, now there will be a hard error below.
             */
        } else {
            if cache_max != 0 {
                /* default: not at limit */
                (*png_ptr).user_chunk_cache_max -= 1;
                /* FALLTHROUGH */
            }

            /* case 0: no limit
             *
             * Here when the limit isn't reached or when limits are compiled
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
    (*png_ptr).unknown_chunk.data = ptr::null_mut();

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
 *
 * On read the IDAT chunk is always handled specially, even if marked for
 * unknown handling (this is allowed), so (pngrutil.c line 1082):
 *
 *   #define png_handle_IDAT NULL
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
    unsafe fn(png_structrp, png_inforp, png_uint_32) -> png_handle_result_code;

struct png_read_chunk_info {
    /* A chunk-specific 'handler', NULL if the chunk is not supported in this
     * build.
     */
    handler: Option<png_chunk_handler_fn>,

    /* Crushing these values helps on modern 32-bit architectures because the
     * pointer and the following bit fields both end up requiring 32 bits.
     * Typically this will halve the table size.  On 64-bit architectures the
     * table entries will typically be 8 bytes.
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

const NoCheck: png_uint_32 = 0x801; /* Do not check the maximum length */
const Limit: png_uint_32 = 0x802; /* Limit to png_chunk_max bytes */
const LKMin: png_uint_32 = 3 + LZ77Min as png_uint_32; /* Minimum length of keyword+LZ77 */

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

/* Each entry lists the values for the parameters **after** the first,
 * 'handler', function.  'handler' is NULL when the chunk has no compiled in
 * support.  The entries are in PNG_KNOWN_CHUNKS order.
 */
macro_rules! chunk_entry {
    ($handler:expr, $max:expr, $min:expr, $before:expr, $after:expr, $multiple:expr) => {
        png_read_chunk_info {
            handler: $handler,
            max_length: $max,
            min_length: $min,
            pos_before: $before,
            pos_after: $after,
            multiple: $multiple,
        }
    };
}

static read_chunks: [png_read_chunk_info; PNG_INDEX_unknown as usize] = [
    /*       cHNK      handler                max_len,   min, before, after, multiple */
    /* IHDR */ chunk_entry!(Some(png_handle_IHDR), 13, 13, hIHDR, 0, 0),
    /* PLTE: PLTE errors are only critical for colour-map images, consequently
     * the handler does all the checks.
     */
    /* PLTE */ chunk_entry!(Some(png_handle_PLTE), NoCheck, 0, 0, hIHDR, 1),
    /* IDAT */ chunk_entry!(None, NoCheck, 0, aIDAT, hIHDR, 1),
    /* IEND: historically data was allowed in IEND */
    /* IEND */ chunk_entry!(Some(png_handle_IEND), NoCheck, 0, 0, aIDAT, 0),
    /* acTL */ chunk_entry!(None, 8, 8, hIDAT, hIHDR, 0),
    /* bKGD */ chunk_entry!(Some(png_handle_bKGD), 6, 1, hIDAT, hIHDR, 0),
    /* cHRM */ chunk_entry!(Some(png_handle_cHRM), 32, 32, hCOL, hIHDR, 0),
    /* cICP */ chunk_entry!(Some(png_handle_cICP), 4, 4, hCOL, hIHDR, 0),
    /* cLLI */ chunk_entry!(Some(png_handle_cLLI), 8, 8, hCOL, hIHDR, 0),
    /* eXIf */ chunk_entry!(Some(png_handle_eXIf), Limit, 4, 0, hIHDR, 0),
    /* fcTL */ chunk_entry!(None, 25, 26, 0, hIHDR, 1),
    /* fdAT */ chunk_entry!(None, Limit, 4, hIDAT, hIHDR, 1),
    /* gAMA */ chunk_entry!(Some(png_handle_gAMA), 4, 4, hCOL, hIHDR, 0),
    /* hIST */ chunk_entry!(Some(png_handle_hIST), 1024, 0, hPLTE, hIHDR, 0),
    /* iCCP */ chunk_entry!(Some(png_handle_iCCP), NoCheck, LKMin, hCOL, hIHDR, 0),
    /* iTXt: allocates 'length+1'; checked in the handler */
    /* iTXt */ chunk_entry!(Some(png_handle_iTXt), NoCheck, 6, 0, hIHDR, 1),
    /* mDCV */ chunk_entry!(Some(png_handle_mDCV), 24, 24, hCOL, hIHDR, 0),
    /* Supported chunks from PNG extensions 1.5.0, NYI so limit */
    /* oFFs */ chunk_entry!(Some(png_handle_oFFs), 9, 9, hIDAT, hIHDR, 0),
    /* pCAL: allocates 'length+1'; checked in the handler */
    /* pCAL */ chunk_entry!(Some(png_handle_pCAL), NoCheck, 14, hIDAT, hIHDR, 0),
    /* pHYs */ chunk_entry!(Some(png_handle_pHYs), 9, 9, hIDAT, hIHDR, 0),
    /* sBIT */ chunk_entry!(Some(png_handle_sBIT), 4, 1, hCOL, hIHDR, 0),
    /* sCAL: allocates 'length+1'; checked in the handler */
    /* sCAL */ chunk_entry!(Some(png_handle_sCAL), Limit, 4, hIDAT, hIHDR, 0),
    /* sPLT: allocates 'length+1'; checked in the handler */
    /* sPLT */ chunk_entry!(Some(png_handle_sPLT), NoCheck, 3, hIDAT, hIHDR, 1),
    /* sRGB */ chunk_entry!(Some(png_handle_sRGB), 1, 1, hCOL, hIHDR, 0),
    /* tEXt: allocates 'length+1'; checked in the handler */
    /* tEXt */ chunk_entry!(Some(png_handle_tEXt), NoCheck, 2, 0, hIHDR, 1),
    /* tIME */ chunk_entry!(Some(png_handle_tIME), 7, 7, 0, hIHDR, 0),
    /* tRNS */ chunk_entry!(Some(png_handle_tRNS), 256, 0, hIDAT, hIHDR, 0),
    /* zTXt */ chunk_entry!(Some(png_handle_zTXt), Limit, LKMin, 0, hIHDR, 1),
];

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
    let mut errmsg: png_const_charp = ptr::null();

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
    else if (((*png_ptr).mode & read_chunks[chunk_index as usize].pos_before) != 0)
        || (((*png_ptr).mode & read_chunks[chunk_index as usize].pos_after)
            != read_chunks[chunk_index as usize].pos_after)
    {
        errmsg = c"out of place".as_ptr();
    }
    /* Now check for duplicates: duplicated critical chunks also produce a
     * full error.
     */
    else if read_chunks[chunk_index as usize].multiple == 0
        && png_file_has_chunk(png_ptr, chunk_index)
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
        let max_length: c_uint = read_chunks[chunk_index as usize].max_length;
        let mut meets_limit = false;

        if max_length == Limit {
            /* png_read_chunk_header has already png_error'ed chunks with a
             * length exceeding the 31-bit PNG limit, so just check the memory
             * limit:
             */
            if (length as png_alloc_size_t) <= png_chunk_max(png_ptr) {
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
        } else {
            /* ancillary chunk */
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
