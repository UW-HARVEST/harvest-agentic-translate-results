// Translated from pcre2_match_next.c
use crate::internal::*;
use crate::pcre2_pub::*;
use core::ffi::c_int;

/* Advance the offset by one code unit, and return the new value. */

unsafe fn do_bumpalong(
    match_data: *mut pcre2_real_match_data,
    offset: PCRE2_SIZE,
) -> PCRE2_SIZE {
    let subject: PCRE2_SPTR = (*match_data).subject;
    let subject_length: PCRE2_SIZE = (*match_data).subject_length;
    let utf: BOOL = (((*(*match_data).code).overall_options & PCRE2_UTF) != 0) as BOOL;

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

        /* FORWARDCHARTEST(next, subject_end) */
        while next < subject_end && (*next & 0xc0) == 0x80 {
            next = next.add(1);
        }
        return next.offset_from(subject) as PCRE2_SIZE;
    }

    offset + 1
}

/*************************************************
*                Advance the match               *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_next_match_8(
    match_data: *mut pcre2_real_match_data,
    pstart_offset: *mut PCRE2_SIZE,
    poptions: *mut u32,
) -> c_int {
    let rc: c_int = (*match_data).rc;
    let start_offset: PCRE2_SIZE = (*match_data).start_offset;
    let ovector: *mut PCRE2_SIZE = core::ptr::addr_of_mut!((*match_data).ovector) as *mut PCRE2_SIZE;

    if rc < 0 {
        return FALSE;
    }

    if *ovector.add(0) != start_offset && *ovector.add(1) == start_offset {
        if start_offset >= (*match_data).subject_length {
            return FALSE;
        }

        *pstart_offset = do_bumpalong(match_data, *ovector.add(1));
        *poptions = 0;
        return TRUE;
    }

    if *ovector.add(0) == *ovector.add(1) {
        if *ovector.add(0) >= (*match_data).subject_length {
            return FALSE;
        }

        *pstart_offset = *ovector.add(1);
        *poptions = PCRE2_NOTEMPTY_ATSTART;
        return TRUE;
    }

    *pstart_offset = *ovector.add(1);
    *poptions = 0;
    TRUE
}
