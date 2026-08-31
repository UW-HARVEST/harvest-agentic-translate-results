//! Translation of `c_src/src/pcre2_match_next.c`.

#![allow(non_snake_case, non_upper_case_globals, unused_parens)]

use core::ffi::c_int;

use crate::chars::{CHAR_CR, CHAR_LF};
use crate::internal::*;

/* Advance the offset by one code unit, and return the new value.
It is only called when the offset is not at the end of the subject. */

unsafe fn do_bumpalong(
    match_data: *mut pcre2_real_match_data,
    offset: PCRE2_SIZE,
) -> PCRE2_SIZE {
    unsafe {
        let subject: PCRE2_SPTR = (*match_data).subject;
        let subject_length: PCRE2_SIZE = (*match_data).subject_length;
        let utf: BOOL = ((*(*match_data).code).overall_options & PCRE2_UTF != 0) as BOOL;

        /* Skip over CRLF as an atomic sequence, if CRLF is configured as a newline
        sequence. */

        if *subject.add(offset) as u32 == CHAR_CR
            && offset + 1 < subject_length
            && *subject.add(offset + 1) as u32 == CHAR_LF
        {
            match (*(*match_data).code).newline_convention as u32 {
                PCRE2_NEWLINE_CRLF | PCRE2_NEWLINE_ANY | PCRE2_NEWLINE_ANYCRLF => {
                    return offset + 2;
                }
                _ => {}
            }
        }

        /* Advance by one full character if in UTF mode. */

        if utf != FALSE {
            let mut next: PCRE2_SPTR = subject.add(offset + 1);
            let subject_end: PCRE2_SPTR = subject.add(subject_length);
            forwardchartest(&mut next, subject_end);
            return next.offset_from(subject) as PCRE2_SIZE;
        }

        offset + 1
    }
}

/*************************************************
*                Advance the match               *
*************************************************/

pub unsafe fn pcre2_next_match(
    match_data: *mut pcre2_real_match_data,
    pstart_offset: *mut PCRE2_SIZE,
    poptions: *mut u32,
) -> c_int {
    unsafe {
        let rc: c_int = (*match_data).rc;
        let start_offset: PCRE2_SIZE = (*match_data).start_offset;
        let ovector: *mut PCRE2_SIZE = (*match_data).ovector.as_mut_ptr();

        /* Match error, or no match: no further iteration possible. */

        if rc < 0 {
            return FALSE;
        }

        /* Match succeeded: get the start offset for the next match. */

        /* Special handling for patterns which contain \K in a lookaround. */

        if *ovector.add(0) != start_offset && *ovector.add(1) == start_offset {
            /* If the match end is at the end of the subject, we are done. */
            if start_offset >= (*match_data).subject_length {
                return FALSE;
            }

            /* Otherwise, bump along by one code unit, and do a normal search. */
            *pstart_offset = do_bumpalong(match_data, *ovector.add(1));
            *poptions = 0;
            return TRUE;
        }

        /* If the previous match was for an empty string, we are finished if we are
        at the end of the subject. */

        if *ovector.add(0) == *ovector.add(1) {
            /* If the match is at the end of the subject, we are done. */
            if *ovector.add(0) >= (*match_data).subject_length {
                return FALSE;
            }

            /* Otherwise, continue at this exact same point, setting the flag which
            ensures that we don't return the exact same empty match again. */
            *pstart_offset = *ovector.add(1);
            *poptions = PCRE2_NOTEMPTY_ATSTART;
            return TRUE;
        }

        /* Finally, a non-empty match where the end of the match is further on in
        the subject than start_offset, so we can continue and make progress. */

        *pstart_offset = *ovector.add(1);
        *poptions = 0;
        TRUE
    }
}

/* Exported as `pcre2_next_match_8`. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_next_match_8(
    match_data: *mut pcre2_real_match_data,
    pstart_offset: *mut PCRE2_SIZE,
    poptions: *mut u32,
) -> c_int {
    unsafe { pcre2_next_match(match_data, pstart_offset, poptions) }
}
