//! Translation of `c_src/src/pcre2_pattern_info.c`.

#![allow(non_snake_case, non_upper_case_globals, unused_parens)]

use core::ffi::{c_int, c_void};

use crate::internal::*;
use crate::opcodes::*;

/* ------------------------------------------------------------------ *
 *        Return info about compiled pattern                           *
 * ------------------------------------------------------------------ */

/*
Arguments:
  code          points to compiled code
  what          what information is required
  where         where to put the information; if NULL, return length

Returns:        0 when data returned
                > 0 when length requested
                < 0 on error or unset value
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_pattern_info_8(
    code: *const pcre2_real_code,
    what: u32,
    where_: *mut c_void,
) -> c_int {
    unsafe {
        let re = code;

        if where_.is_null()
        /* Requests field length */
        {
            match what {
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
                | PCRE2_INFO_NEWLINE => return core::mem::size_of::<u32>() as c_int,

                PCRE2_INFO_FIRSTBITMAP => {
                    return core::mem::size_of::<*const u8>() as c_int;
                }

                PCRE2_INFO_JITSIZE | PCRE2_INFO_SIZE | PCRE2_INFO_FRAMESIZE => {
                    return core::mem::size_of::<usize>() as c_int;
                }

                PCRE2_INFO_NAMETABLE => {
                    return core::mem::size_of::<PCRE2_SPTR>() as c_int;
                }

                _ => {}
            }
        }

        if re.is_null() {
            return PCRE2_ERROR_NULL;
        }

        /* Check that the first field in the block is the magic number. If it is not,
        return with PCRE2_ERROR_BADMAGIC. */

        if (*re).magic_number != MAGIC_NUMBER {
            return PCRE2_ERROR_BADMAGIC;
        }

        /* Check that this pattern was compiled in the correct bit mode */

        if ((*re).flags & (PCRE2_CODE_UNIT_WIDTH / 8)) == 0 {
            return PCRE2_ERROR_BADMODE;
        }

        match what {
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
                    return PCRE2_ERROR_UNSET;
                }
            }

            PCRE2_INFO_EXTRAOPTIONS => {
                *(where_ as *mut u32) = (*re).extra_options;
            }

            PCRE2_INFO_FIRSTCODETYPE => {
                *(where_ as *mut u32) = if ((*re).flags & PCRE2_FIRSTSET) != 0 {
                    1
                } else if ((*re).flags & PCRE2_STARTLINE) != 0 {
                    2
                } else {
                    0
                };
            }

            PCRE2_INFO_FIRSTCODEUNIT => {
                *(where_ as *mut u32) = if ((*re).flags & PCRE2_FIRSTSET) != 0 {
                    (*re).first_codeunit
                } else {
                    0
                };
            }

            PCRE2_INFO_FIRSTBITMAP => {
                *(where_ as *mut *const u8) = if ((*re).flags & PCRE2_FIRSTMAPSET) != 0 {
                    &(*re).start_bitmap[0] as *const u8
                } else {
                    core::ptr::null()
                };
            }

            PCRE2_INFO_FRAMESIZE => {
                *(where_ as *mut usize) = HEAPFRAME_OVECTOR_OFFSET
                    + (*re).top_bracket as usize * 2 * core::mem::size_of::<PCRE2_SIZE>();
            }

            PCRE2_INFO_HASBACKSLASHC => {
                *(where_ as *mut u32) = (((*re).flags & PCRE2_HASBKC) != 0) as u32;
            }

            PCRE2_INFO_HASCRORLF => {
                *(where_ as *mut u32) = (((*re).flags & PCRE2_HASCRORLF) != 0) as u32;
            }

            PCRE2_INFO_HEAPLIMIT => {
                *(where_ as *mut u32) = (*re).limit_heap;
                if (*re).limit_heap == u32::MAX {
                    return PCRE2_ERROR_UNSET;
                }
            }

            PCRE2_INFO_JCHANGED => {
                *(where_ as *mut u32) = (((*re).flags & PCRE2_JCHANGED) != 0) as u32;
            }

            PCRE2_INFO_JITSIZE => {
                /* SUPPORT_JIT is not defined. */
                *(where_ as *mut usize) = 0;
            }

            PCRE2_INFO_LASTCODETYPE => {
                *(where_ as *mut u32) = (((*re).flags & PCRE2_LASTSET) != 0) as u32;
            }

            PCRE2_INFO_LASTCODEUNIT => {
                *(where_ as *mut u32) = if ((*re).flags & PCRE2_LASTSET) != 0 {
                    (*re).last_codeunit
                } else {
                    0
                };
            }

            PCRE2_INFO_MATCHEMPTY => {
                *(where_ as *mut u32) = (((*re).flags & PCRE2_MATCH_EMPTY) != 0) as u32;
            }

            PCRE2_INFO_MATCHLIMIT => {
                *(where_ as *mut u32) = (*re).limit_match;
                if (*re).limit_match == u32::MAX {
                    return PCRE2_ERROR_UNSET;
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
                *(where_ as *mut PCRE2_SPTR) = (re as *const u8)
                    .add(core::mem::size_of::<pcre2_real_code>())
                    as PCRE2_SPTR;
            }

            PCRE2_INFO_NEWLINE => {
                *(where_ as *mut u32) = (*re).newline_convention as u32;
            }

            PCRE2_INFO_SIZE => {
                *(where_ as *mut usize) = (*re).blocksize;
            }

            _ => return PCRE2_ERROR_BADOPTION,
        }

        0
    }
}

/* ------------------------------------------------------------------ *
 *              Callout enumerator                                     *
 * ------------------------------------------------------------------ */

/*
Arguments:
  code          points to compiled code
  callback      function called for each callout block
  callout_data  user data passed to the callback

Returns:        0 when successfully completed
                < 0 on local error
               != 0 for callback error
*/

pub type CalloutEnumerateFn =
    Option<unsafe extern "C" fn(*mut pcre2_callout_enumerate_block, *mut c_void) -> c_int>;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_callout_enumerate_8(
    code: *const pcre2_real_code,
    callback: CalloutEnumerateFn,
    callout_data: *mut c_void,
) -> c_int {
    unsafe {
        let re = code;
        let mut cb: pcre2_callout_enumerate_block = core::mem::zeroed();
        let mut cc: PCRE2_SPTR;
        let utf: BOOL;

        if re.is_null() {
            return PCRE2_ERROR_NULL;
        }

        utf = (((*re).overall_options & PCRE2_UTF) != 0) as BOOL;

        /* Check that the first field in the block is the magic number. If it is not,
        return with PCRE2_ERROR_BADMAGIC. */

        if (*re).magic_number != MAGIC_NUMBER {
            return PCRE2_ERROR_BADMAGIC;
        }

        /* Check that this pattern was compiled in the correct bit mode */

        if ((*re).flags & (PCRE2_CODE_UNIT_WIDTH / 8)) == 0 {
            return PCRE2_ERROR_BADMODE;
        }

        cb.version = 0;
        cc = (re as *const u8).add((*re).code_start) as PCRE2_SPTR;

        loop {
            let rc: c_int;
            match *cc {
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
                    cc = cc.add(OP_LENGTHS[*cc as usize] as usize);
                    if utf != 0 && has_extralen(*cc.sub(1) as u32) {
                        cc = cc.add(get_extralen(*cc.sub(1) as u32) as usize);
                    }
                }

                OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
                | OP_TYPEMINQUERY | OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT
                | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS | OP_TYPEPOSQUERY | OP_TYPEPOSUPTO => {
                    cc = cc.add(OP_LENGTHS[*cc as usize] as usize);
                    if *cc.sub(1) == OP_PROP || *cc.sub(1) == OP_NOTPROP {
                        cc = cc.add(2);
                    }
                }

                /* SUPPORT_WIDE_CHARS is defined */
                OP_XCLASS | OP_ECLASS => {
                    cc = cc.add(get(cc, 1) as usize);
                }

                OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                    cc = cc.add(OP_LENGTHS[*cc as usize] as usize + *cc.add(1) as usize);
                }

                OP_CALLOUT => {
                    cb.pattern_position = get(cc, 1) as PCRE2_SIZE;
                    cb.next_item_length = get(cc, 1 + LINK_SIZE) as PCRE2_SIZE;
                    cb.callout_number = *cc.add(1 + 2 * LINK_SIZE) as u32;
                    cb.callout_string_offset = 0;
                    cb.callout_string_length = 0;
                    cb.callout_string = core::ptr::null();
                    rc = (callback.unwrap())(&mut cb, callout_data);
                    if rc != 0 {
                        return rc;
                    }
                    cc = cc.add(OP_LENGTHS[*cc as usize] as usize);
                }

                OP_CALLOUT_STR => {
                    cb.pattern_position = get(cc, 1) as PCRE2_SIZE;
                    cb.next_item_length = get(cc, 1 + LINK_SIZE) as PCRE2_SIZE;
                    cb.callout_number = 0;
                    cb.callout_string_offset = get(cc, 1 + 3 * LINK_SIZE) as PCRE2_SIZE;
                    cb.callout_string_length = (get(cc, 1 + 2 * LINK_SIZE)
                        - (1 + 4 * LINK_SIZE) as c_int
                        - 2) as PCRE2_SIZE;
                    cb.callout_string = cc.add((1 + 4 * LINK_SIZE) + 1);
                    rc = (callback.unwrap())(&mut cb, callout_data);
                    if rc != 0 {
                        return rc;
                    }
                    cc = cc.add(get(cc, 1 + 2 * LINK_SIZE) as usize);
                }

                _ => {
                    cc = cc.add(OP_LENGTHS[*cc as usize] as usize);
                }
            }
        }
    }
}
