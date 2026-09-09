//! Translated from pcre2_match_next.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

/* Advance the offset by one code unit, and return the new value.
It is only called when the offset is not at the end of the subject. */

pub(crate) unsafe fn do_bumpalong(
    match_data: *mut pcre2_real_match_data,
    offset: PCRE2_SIZE,
) -> PCRE2_SIZE {
    let subject: PCRE2_SPTR = (*match_data).subject;
    let subject_length: PCRE2_SIZE = (*match_data).subject_length;
    let utf: BOOL = (((*(*match_data).code).overall_options & PCRE2_UTF) != 0) as BOOL;

    /* Skip over CRLF as an atomic sequence, if CRLF is configured as a newline
    sequence. */

    if *subject.add(offset) == 0x0d /* CHAR_CR */
        && offset + 1 < subject_length
        && *subject.add(offset + 1) == 0x0a
    /* CHAR_LF */
    {
        match (*(*match_data).code).newline_convention as u32 {
            PCRE2_NEWLINE_CRLF | PCRE2_NEWLINE_ANY | PCRE2_NEWLINE_ANYCRLF => {
                return offset + 2;
            }
            _ => {}
        }
    }

    /* Advance by one full character if in UTF mode. */

    if utf != 0 {
        let mut next: PCRE2_SPTR = subject.add(offset).add(1);
        let subject_end: PCRE2_SPTR = subject.add(subject_length);

        FORWARDCHARTEST!(next, subject_end);
        return (next as usize) - (subject as usize);
    }

    offset + 1
}

/*************************************************
*                Advance the match               *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_next_match_8(match_data: *mut pcre2_real_match_data, lengthptr: *mut PCRE2_SIZE, optionsptr: *mut u32) -> i32 {
    let rc: i32 = (*match_data).rc;
    let start_offset: PCRE2_SIZE = (*match_data).start_offset;
    let ovector: *mut PCRE2_SIZE = (*match_data).ovector.as_mut_ptr();

    /* Match error, or no match: no further iteration possible. */

    if rc < 0 {
        return FALSE;
    }

    /* Match succeeded: get the start offset for the next match */

    /* PCRE2_ASSERT(ovector[1] >= start_offset); */

    /* Special handling for patterns which contain \K in a lookaround. */

    if *ovector.add(0) != start_offset && *ovector.add(1) == start_offset {
        /* If the match end is at the end of the subject, we are done. */

        if start_offset >= (*match_data).subject_length {
            return FALSE;
        }

        /* Otherwise, bump along by one code unit, and do a normal search. */

        *lengthptr = do_bumpalong(match_data, *ovector.add(1));
        *optionsptr = 0;
        return TRUE;
    }

    /* If the previous match was for an empty string, we are finished if we are at
    the end of the subject. Otherwise, arrange to run another match at the same
    point to see if a non-empty match can be found. */

    if *ovector.add(0) == *ovector.add(1) {
        /* If the match is at the end of the subject, we are done. */

        if *ovector.add(0) >= (*match_data).subject_length {
            return FALSE;
        }

        /* Otherwise, continue at this exact same point, but we must set the flag
        which ensures that we don't return the exact same empty match again. */

        *lengthptr = *ovector.add(1);
        *optionsptr = PCRE2_NOTEMPTY_ATSTART;
        return TRUE;
    }

    /* Finally, we must be in the happy state of a non-empty match. */

    *lengthptr = *ovector.add(1);
    *optionsptr = 0;
    TRUE
}
