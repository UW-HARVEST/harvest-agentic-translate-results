//! Translation of `pcre2_pattern_info.c`.
//!
//! `pcre2_pattern_info()` returns info about a compiled pattern, and
//! `pcre2_callout_enumerate()` walks the compiled code invoking a callback for
//! each callout.

use crate::internal::*;
use core::ffi::{c_int, c_void};
use core::mem::{offset_of, size_of};

/// `LINK_SIZE` for this configuration (2).
const LINK_SIZE: usize = LINK_SIZE_U;

/// `PCRE2_CODE_UNIT_WIDTH / 8` — in 8-bit mode this is 1.
const CODE_UNIT_WIDTH_FLAG: u32 = 1;

// ---------------------------------------------------------------------------
// Return info about compiled pattern
// ---------------------------------------------------------------------------

/// `pcre2_pattern_info()`.
///
/// Returns 0 when data returned, > 0 when a length is requested (NULL `where`),
/// and < 0 on error or unset value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_pattern_info_8(
    code: *const pcre2_code,
    what: u32,
    where_: *mut c_void,
) -> c_int {
    unsafe {
        let re = code as *const pcre2_real_code;

        if where_.is_null() {
            // Requests field length.
            match what as i64 {
                PCRE2_INFO_ALLOPTIONS
                | PCRE2_INFO_ARGOPTIONS
                | PCRE2_INFO_BACKREFMAX
                | PCRE2_INFO_BSR
                | PCRE2_INFO_CAPTURECOUNT
                | PCRE2_INFO_DEPTHLIMIT
                | PCRE2_INFO_EXTRAOPTIONS
                | PCRE2_INFO_FIRSTCODETYPE
                | PCRE2_INFO_FIRSTCODEUNIT
                | PCRE2_INFO_HASBACKSLASHC
                | PCRE2_INFO_HASCRORLF
                | PCRE2_INFO_HEAPLIMIT
                | PCRE2_INFO_JCHANGED
                | PCRE2_INFO_LASTCODETYPE
                | PCRE2_INFO_LASTCODEUNIT
                | PCRE2_INFO_MATCHEMPTY
                | PCRE2_INFO_MATCHLIMIT
                | PCRE2_INFO_MAXLOOKBEHIND
                | PCRE2_INFO_MINLENGTH
                | PCRE2_INFO_NAMEENTRYSIZE
                | PCRE2_INFO_NAMECOUNT
                | PCRE2_INFO_NEWLINE => return size_of::<u32>() as c_int,

                PCRE2_INFO_FIRSTBITMAP => return size_of::<*const u8>() as c_int,

                PCRE2_INFO_JITSIZE | PCRE2_INFO_SIZE | PCRE2_INFO_FRAMESIZE => {
                    return size_of::<usize>() as c_int;
                }

                PCRE2_INFO_NAMETABLE => return size_of::<PCRE2_SPTR>() as c_int,

                _ => {}
            }
        }

        if re.is_null() {
            return PCRE2_ERROR_NULL as c_int;
        }

        // Check the magic number.
        if (*re).magic_number != MAGIC_NUMBER as u32 {
            return PCRE2_ERROR_BADMAGIC as c_int;
        }

        // Check that this pattern was compiled in the correct bit mode.
        if ((*re).flags & CODE_UNIT_WIDTH_FLAG) == 0 {
            return PCRE2_ERROR_BADMODE as c_int;
        }

        match what as i64 {
            PCRE2_INFO_ALLOPTIONS => {
                *(where_ as *mut u32) = (*re).overall_options;
            }

            PCRE2_INFO_ARGOPTIONS => {
                *(where_ as *mut u32) = (*re).compile_options;
            }

            PCRE2_INFO_BACKREFMAX => {
                *(where_ as *mut u32) = (*re).top_backref as u32;
            }

            PCRE2_INFO_BSR => {
                *(where_ as *mut u32) = (*re).bsr_convention as u32;
            }

            PCRE2_INFO_CAPTURECOUNT => {
                *(where_ as *mut u32) = (*re).top_bracket as u32;
            }

            PCRE2_INFO_DEPTHLIMIT => {
                *(where_ as *mut u32) = (*re).limit_depth;
                if (*re).limit_depth == u32::MAX {
                    return PCRE2_ERROR_UNSET as c_int;
                }
            }

            PCRE2_INFO_EXTRAOPTIONS => {
                *(where_ as *mut u32) = (*re).extra_options;
            }

            PCRE2_INFO_FIRSTCODETYPE => {
                *(where_ as *mut u32) = if ((*re).flags & PCRE2_FIRSTSET as u32) != 0 {
                    1
                } else if ((*re).flags & PCRE2_STARTLINE as u32) != 0 {
                    2
                } else {
                    0
                };
            }

            PCRE2_INFO_FIRSTCODEUNIT => {
                *(where_ as *mut u32) = if ((*re).flags & PCRE2_FIRSTSET as u32) != 0 {
                    (*re).first_codeunit
                } else {
                    0
                };
            }

            PCRE2_INFO_FIRSTBITMAP => {
                *(where_ as *mut *const u8) = if ((*re).flags & PCRE2_FIRSTMAPSET as u32) != 0 {
                    &raw const (*re).start_bitmap[0]
                } else {
                    core::ptr::null()
                };
            }

            PCRE2_INFO_FRAMESIZE => {
                *(where_ as *mut usize) = offset_of!(heapframe, ovector)
                    + (*re).top_bracket as usize * 2 * size_of::<PCRE2_SIZE>();
            }

            PCRE2_INFO_HASBACKSLASHC => {
                *(where_ as *mut u32) = (((*re).flags & PCRE2_HASBKC as u32) != 0) as u32;
            }

            PCRE2_INFO_HASCRORLF => {
                *(where_ as *mut u32) = (((*re).flags & PCRE2_HASCRORLF as u32) != 0) as u32;
            }

            PCRE2_INFO_HEAPLIMIT => {
                *(where_ as *mut u32) = (*re).limit_heap;
                if (*re).limit_heap == u32::MAX {
                    return PCRE2_ERROR_UNSET as c_int;
                }
            }

            PCRE2_INFO_JCHANGED => {
                *(where_ as *mut u32) = (((*re).flags & PCRE2_JCHANGED as u32) != 0) as u32;
            }

            PCRE2_INFO_JITSIZE => {
                // SUPPORT_JIT is off, so this is always 0.
                *(where_ as *mut usize) = if !(*re).executable_jit.is_null() {
                    crate::jit::_pcre2_jit_get_size_8((*re).executable_jit)
                } else {
                    0
                };
            }

            PCRE2_INFO_LASTCODETYPE => {
                *(where_ as *mut u32) = (((*re).flags & PCRE2_LASTSET as u32) != 0) as u32;
            }

            PCRE2_INFO_LASTCODEUNIT => {
                *(where_ as *mut u32) = if ((*re).flags & PCRE2_LASTSET as u32) != 0 {
                    (*re).last_codeunit
                } else {
                    0
                };
            }

            PCRE2_INFO_MATCHEMPTY => {
                *(where_ as *mut u32) = (((*re).flags & PCRE2_MATCH_EMPTY as u32) != 0) as u32;
            }

            PCRE2_INFO_MATCHLIMIT => {
                *(where_ as *mut u32) = (*re).limit_match;
                if (*re).limit_match == u32::MAX {
                    return PCRE2_ERROR_UNSET as c_int;
                }
            }

            PCRE2_INFO_MAXLOOKBEHIND => {
                *(where_ as *mut u32) = (*re).max_lookbehind as u32;
            }

            PCRE2_INFO_MINLENGTH => {
                *(where_ as *mut u32) = (*re).minlength as u32;
            }

            PCRE2_INFO_NAMEENTRYSIZE => {
                *(where_ as *mut u32) = (*re).name_entry_size as u32;
            }

            PCRE2_INFO_NAMECOUNT => {
                *(where_ as *mut u32) = (*re).name_count as u32;
            }

            PCRE2_INFO_NAMETABLE => {
                *(where_ as *mut PCRE2_SPTR) =
                    (re as *const u8).add(size_of::<pcre2_real_code>()) as PCRE2_SPTR;
            }

            PCRE2_INFO_NEWLINE => {
                *(where_ as *mut u32) = (*re).newline_convention as u32;
            }

            PCRE2_INFO_SIZE => {
                *(where_ as *mut usize) = (*re).blocksize;
            }

            _ => return PCRE2_ERROR_BADOPTION as c_int,
        }

        0
    }
}

// ---------------------------------------------------------------------------
// Callout enumerator
// ---------------------------------------------------------------------------

/// `pcre2_callout_enumerate()`.
///
/// Returns 0 when successfully completed, < 0 on local error, and any nonzero
/// value returned by the callback (for a callback error).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_callout_enumerate_8(
    code: *const pcre2_code,
    callback: Option<
        unsafe extern "C" fn(*mut pcre2_callout_enumerate_block, *mut c_void) -> c_int,
    >,
    callout_data: *mut c_void,
) -> c_int {
    unsafe {
        let re = code as *const pcre2_real_code;
        let mut cb: pcre2_callout_enumerate_block = core::mem::zeroed();

        if re.is_null() {
            return PCRE2_ERROR_NULL as c_int;
        }

        // SUPPORT_UNICODE is on.
        let utf = ((*re).overall_options & PCRE2_UTF as u32) != 0;

        // Check the magic number.
        if (*re).magic_number != MAGIC_NUMBER as u32 {
            return PCRE2_ERROR_BADMAGIC as c_int;
        }

        // Check that this pattern was compiled in the correct bit mode.
        if ((*re).flags & CODE_UNIT_WIDTH_FLAG) == 0 {
            return PCRE2_ERROR_BADMODE as c_int;
        }

        cb.version = 0;
        let mut cc: PCRE2_SPTR = (re as *const u8).add((*re).code_start) as PCRE2_SPTR;

        let op_lengths = &crate::tables::_pcre2_OP_lengths;

        loop {
            let op = *cc as u32;
            match op {
                OP_END => return 0,

                OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_STAR | OP_MINSTAR | OP_PLUS
                | OP_MINPLUS | OP_QUERY | OP_MINQUERY | OP_UPTO | OP_MINUPTO | OP_EXACT
                | OP_POSSTAR | OP_POSPLUS | OP_POSQUERY | OP_POSUPTO | OP_STARI | OP_MINSTARI
                | OP_PLUSI | OP_MINPLUSI | OP_QUERYI | OP_MINQUERYI | OP_UPTOI | OP_MINUPTOI
                | OP_EXACTI | OP_POSSTARI | OP_POSPLUSI | OP_POSQUERYI | OP_POSUPTOI
                | OP_NOTSTAR | OP_NOTMINSTAR | OP_NOTPLUS | OP_NOTMINPLUS | OP_NOTQUERY
                | OP_NOTMINQUERY | OP_NOTUPTO | OP_NOTMINUPTO | OP_NOTEXACT | OP_NOTPOSSTAR
                | OP_NOTPOSPLUS | OP_NOTPOSQUERY | OP_NOTPOSUPTO | OP_NOTSTARI | OP_NOTMINSTARI
                | OP_NOTPLUSI | OP_NOTMINPLUSI | OP_NOTQUERYI | OP_NOTMINQUERYI | OP_NOTUPTOI
                | OP_NOTMINUPTOI | OP_NOTEXACTI | OP_NOTPOSSTARI | OP_NOTPOSPLUSI
                | OP_NOTPOSQUERYI | OP_NOTPOSUPTOI => {
                    cc = cc.add(op_lengths[op as usize] as usize);
                    // SUPPORT_UNICODE.
                    if utf && HAS_EXTRALEN(*cc.sub(1) as u32) {
                        cc = cc.add(GET_EXTRALEN(*cc.sub(1) as u32) as usize);
                    }
                }

                OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
                | OP_TYPEMINQUERY | OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT
                | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS | OP_TYPEPOSQUERY | OP_TYPEPOSUPTO => {
                    cc = cc.add(op_lengths[op as usize] as usize);
                    // SUPPORT_UNICODE.
                    if *cc.sub(1) as u32 == OP_PROP || *cc.sub(1) as u32 == OP_NOTPROP {
                        cc = cc.add(2);
                    }
                }

                // SUPPORT_WIDE_CHARS (defined with SUPPORT_UNICODE in 8-bit mode).
                OP_XCLASS | OP_ECLASS => {
                    cc = cc.add(GET(cc, 1) as usize);
                }

                OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                    cc = cc.add(op_lengths[op as usize] as usize + *cc.add(1) as usize);
                }

                OP_CALLOUT => {
                    cb.pattern_position = GET(cc, 1) as PCRE2_SIZE;
                    cb.next_item_length = GET(cc, 1 + LINK_SIZE) as PCRE2_SIZE;
                    cb.callout_number = *cc.add(1 + 2 * LINK_SIZE) as u32;
                    cb.callout_string_offset = 0;
                    cb.callout_string_length = 0;
                    cb.callout_string = core::ptr::null();
                    let rc = callback.unwrap()(&mut cb, callout_data);
                    if rc != 0 {
                        return rc;
                    }
                    cc = cc.add(op_lengths[op as usize] as usize);
                }

                OP_CALLOUT_STR => {
                    cb.pattern_position = GET(cc, 1) as PCRE2_SIZE;
                    cb.next_item_length = GET(cc, 1 + LINK_SIZE) as PCRE2_SIZE;
                    cb.callout_number = 0;
                    cb.callout_string_offset = GET(cc, 1 + 3 * LINK_SIZE) as PCRE2_SIZE;
                    cb.callout_string_length =
                        (GET(cc, 1 + 2 * LINK_SIZE) as PCRE2_SIZE) - (1 + 4 * LINK_SIZE) - 2;
                    cb.callout_string = cc.add((1 + 4 * LINK_SIZE) + 1);
                    let rc = callback.unwrap()(&mut cb, callout_data);
                    if rc != 0 {
                        return rc;
                    }
                    cc = cc.add(GET(cc, 1 + 2 * LINK_SIZE) as usize);
                }

                _ => {
                    cc = cc.add(op_lengths[op as usize] as usize);
                }
            }
        }
    }
}

// End of pcre2_pattern_info.c
