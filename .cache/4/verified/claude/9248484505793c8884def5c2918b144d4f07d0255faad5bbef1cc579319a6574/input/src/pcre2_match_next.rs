// Translated from c_src/src/pcre2_match_next.c
use crate::internal::*;

/* Advance the offset by one code unit, and return the new value.
It is only called when the offset is not at the end of the subject. */

unsafe fn do_bumpalong(match_data: *mut pcre2_real_match_data, offset: PCRE2_SIZE) -> PCRE2_SIZE {
    let subject: PCRE2_SPTR = (*match_data).subject;
    let subject_length: PCRE2_SIZE = (*match_data).subject_length;
    let utf: BOOL = if ((*(*match_data).code).overall_options & PCRE2_UTF) != 0 {
        TRUE
    } else {
        FALSE
    };

    /* Skip over CRLF as an atomic sequence, if CRLF is configured as a newline
    sequence. */

    if *subject.add(offset) as u32 == CHAR_CR
        && offset + 1 < subject_length
        && *subject.add(offset + 1) as u32 == CHAR_LF
    {
        let nl: u32 = (*(*match_data).code).newline_convention as u32;
        if nl == PCRE2_NEWLINE_CRLF || nl == PCRE2_NEWLINE_ANY || nl == PCRE2_NEWLINE_ANYCRLF {
            return offset + 2;
        }
    }

    /* Advance by one full character if in UTF mode. */

    if utf != 0 {
        let mut next: PCRE2_SPTR = subject.add(offset).add(1);
        let subject_end: PCRE2_SPTR = subject.add(subject_length);

        FORWARDCHARTEST!(next, subject_end);
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
    offset: *mut PCRE2_SIZE,
    options: *mut u32,
) -> c_int {
    let rc: c_int = (*match_data).rc;
    let start_offset: PCRE2_SIZE = (*match_data).start_offset;
    let ovector: *mut PCRE2_SIZE = (*match_data).ovector.as_mut_ptr();

    /* Match error, or no match: no further iteration possible. In previous versions
    of PCRE2, we recommended that clients use a strategy which involved retrying in
    certain cases after PCRE2_ERROR_NOMATCH, but this is no longer required. */

    if rc < 0 {
        return FALSE;
    }

    /* Match succeeded: get the start offset for the next match */

    /* Although \K can affect the position of ovector[0], there are no ways to do
    anything surprising with ovector[1], which must always be >= start_offset. */

    /* PCRE2_ASSERT(ovector[1] >= start_offset); -- a no-op without PCRE2_DEBUG */

    /* Special handling for patterns which contain \K in a lookaround, which enables
    the match start to be pushed back to before the starting search offset
    (ovector[0] < start_offset) or after the match ends (ovector[0] > ovector[1]).
    This is not a problem if ovector[1] > start_offset, because in this case, we can
    just attempt the next match at ovector[1]: we are making progress, which is all
    that we require.

    However, if we have ovector[1] == start_offset, then we have a very rare case
    which must be handled specially, because it's a non-empty match which
    nonetheless fails to make progress through the subject. */

    if *ovector.add(0) != start_offset && *ovector.add(1) == start_offset {
        /* If the match end is at the end of the subject, we are done. */

        if start_offset >= (*match_data).subject_length {
            return FALSE;
        }

        /* Otherwise, bump along by one code unit, and do a normal search. */

        *offset = do_bumpalong(match_data, *ovector.add(1));
        *options = 0;
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

        *offset = *ovector.add(1);
        *options = PCRE2_NOTEMPTY_ATSTART;
        return TRUE;
    }

    /* Finally, we must be in the happy state of a non-empty match, where the end of
    the match is further on in the subject than start_offset, so we are easily able
    to continue and make progress. */

    *offset = *ovector.add(1);
    *options = 0;
    TRUE
}

/* End of pcre2_match_next.c */
