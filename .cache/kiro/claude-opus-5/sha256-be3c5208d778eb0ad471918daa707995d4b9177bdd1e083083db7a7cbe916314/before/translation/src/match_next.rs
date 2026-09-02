//! Translation of `pcre2_match_next.c`.

use crate::internal::*;

// EBCDIC is not configured, so CHAR_CR / CHAR_LF take their ASCII values.
const CHAR_CR: u8 = 0x0d;
const CHAR_LF: u8 = 0x0a;

/// Advance the offset by one code unit, and return the new value. It is only
/// called when the offset is not at the end of the subject.
///
/// Mirrors the file-local `do_bumpalong()`.
unsafe fn do_bumpalong(match_data: *mut pcre2_match_data, offset: PCRE2_SIZE) -> PCRE2_SIZE {
    unsafe {
        let subject = (*match_data).subject;
        let subject_length = (*match_data).subject_length;
        let utf = ((*(*match_data).code).overall_options & PCRE2_UTF as u32) != 0;

        // Skip over CRLF as an atomic sequence, if CRLF is configured as a
        // newline sequence.
        if *subject.add(offset) == CHAR_CR
            && offset + 1 < subject_length
            && *subject.add(offset + 1) == CHAR_LF
        {
            match (*(*match_data).code).newline_convention as i64 {
                PCRE2_NEWLINE_CRLF | PCRE2_NEWLINE_ANY | PCRE2_NEWLINE_ANYCRLF => {
                    return offset + 2;
                }
                _ => {}
            }
        }

        // Advance by one full character if in UTF mode.
        if utf {
            let mut next: PCRE2_SPTR = subject.add(offset + 1);
            let subject_end: PCRE2_SPTR = subject.add(subject_length);

            let _ = subject_end; // Suppress warning; matches the C `(void)` cast.
            FORWARDCHARTEST(&mut next, subject_end);
            return next.offset_from(subject) as PCRE2_SIZE;
        }

        offset + 1
    }
}

// ---------------------------------------------------------------------------
// Advance the match
// ---------------------------------------------------------------------------

/// `pcre2_next_match()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_next_match_8(
    match_data: *mut pcre2_match_data,
    pstart_offset: *mut PCRE2_SIZE,
    poptions: *mut u32,
) -> BOOL {
    unsafe {
        let rc = (*match_data).rc;
        let start_offset = (*match_data).start_offset;
        let ovector = (*match_data).ovec();

        // Match error, or no match: no further iteration possible.
        if rc < 0 {
            return FALSE;
        }

        // Match succeeded: get the start offset for the next match.
        //
        // Although \K can affect the position of ovector[0], there are no ways
        // to do anything surprising with ovector[1], which must always be
        // >= start_offset.
        debug_assert!(*ovector.add(1) >= start_offset);

        // Special handling for patterns which contain \K in a lookaround, which
        // enables the match start to be pushed back to before the starting
        // search offset (ovector[0] < start_offset) or after the match ends
        // (ovector[0] > ovector[1]). This is not a problem if
        // ovector[1] > start_offset, because in this case we can just attempt
        // the next match at ovector[1]: we are making progress.
        //
        // However, if we have ovector[1] == start_offset, then we have a very
        // rare case which must be handled specially, because it's a non-empty
        // match which nonetheless fails to make progress through the subject.
        if *ovector.add(0) != start_offset && *ovector.add(1) == start_offset {
            // If the match end is at the end of the subject, we are done.
            if start_offset >= (*match_data).subject_length {
                return FALSE;
            }

            // Otherwise, bump along by one code unit, and do a normal search.
            *pstart_offset = do_bumpalong(match_data, *ovector.add(1));
            *poptions = 0;
            return TRUE;
        }

        // If the previous match was for an empty string, we are finished if we
        // are at the end of the subject. Otherwise, arrange to run another match
        // at the same point to see if a non-empty match can be found.
        if *ovector.add(0) == *ovector.add(1) {
            // If the match is at the end of the subject, we are done.
            if *ovector.add(0) >= (*match_data).subject_length {
                return FALSE;
            }

            // Otherwise, continue at this exact same point, but we must set the
            // flag which ensures that we don't return the exact same empty match
            // again.
            *pstart_offset = *ovector.add(1);
            *poptions = PCRE2_NOTEMPTY_ATSTART as u32;
            return TRUE;
        }

        // Finally, we must be in the happy state of a non-empty match, where the
        // end of the match is further on in the subject than start_offset, so we
        // are easily able to continue and make progress.
        *pstart_offset = *ovector.add(1);
        *poptions = 0;
        TRUE
    }
}
