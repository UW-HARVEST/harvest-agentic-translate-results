//! Translation of the exported `pcre2_match()` function from `pcre2_match.c`.
//!
//! Configuration: `PCRE2_CODE_UNIT_WIDTH == 8`, `SUPPORT_UNICODE` enabled,
//! `SUPPORT_JIT` disabled. The `#ifdef SUPPORT_JIT` matching block is therefore
//! dead code and is omitted; `PCRE2_NO_JIT` is still an accepted option.

use core::ffi::{c_int, c_void};

use crate::internal::*;
use crate::match_local::*;

// The C code uses memchr() for the 8-bit first/required code unit searches.
unsafe extern "C" {
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
}

// File-local character constants (ASCII; EBCDIC is not configured).
const CHAR_CR: u8 = 0x0d;
const CHAR_NL: u8 = 0x0a;
const CHAR_NUL: u8 = 0x00;

// `PCRE2_CODE_UNIT_WIDTH` for this build.
const PCRE2_CODE_UNIT_WIDTH_8: u32 = 8;

// `PCRE2_MATCHEDBY_INTERPRETER` — first value of the matchedby enum.
const PCRE2_MATCHEDBY_INTERPRETER: u8 = 0;

/// `pcre2_match()` — match a compiled pattern against a subject string.
///
/// This is the interpreter entry point (JIT is not compiled in). It validates
/// arguments, checks UTF, sets up the match block, allocates the backtracking
/// frame vector, performs the start-of-match optimizations and the bumpalong
/// loop, calling the internal `match()` function for each starting position,
/// and finally fills in the match data / ovector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_8(
    code: *const pcre2_real_code,
    subject: PCRE2_SPTR,
    length: PCRE2_SIZE,
    start_offset: PCRE2_SIZE,
    options: u32,
    match_data: *mut pcre2_real_match_data,
    mcontext: *mut pcre2_real_match_context,
) -> c_int {
    unsafe {
        // Local variables mirroring the C declarations.
        let mut rc: c_int;
        let mut start_bits: *const u8 = core::ptr::null();
        let re = code;
        let original_options = options;
        let mut options = options;

        let anchored: BOOL;
        let firstline: BOOL;
        let mut has_first_cu: BOOL = FALSE;
        let mut has_req_cu: BOOL = FALSE;
        let startline: BOOL;

        // PCRE2_CODE_UNIT_WIDTH == 8: memchr caches.
        let mut memchr_found_first_cu: PCRE2_SPTR;
        let mut memchr_found_first_cu2: PCRE2_SPTR;

        let mut first_cu: PCRE2_UCHAR = 0;
        let mut first_cu2: PCRE2_UCHAR = 0;
        let mut req_cu: PCRE2_UCHAR = 0;
        let mut req_cu2: PCRE2_UCHAR = 0;

        let null_str: [PCRE2_UCHAR; 1] = [0xcd];
        let original_subject: PCRE2_SPTR = subject;
        let mut subject: PCRE2_SPTR = subject;
        let bumpalong_limit: PCRE2_SPTR;
        let mut end_subject: PCRE2_SPTR;
        let true_end_subject: PCRE2_SPTR;
        let mut start_match: PCRE2_SPTR;
        let mut req_cu_ptr: PCRE2_SPTR;
        let mut start_partial: PCRE2_SPTR;
        let mut match_partial: PCRE2_SPTR;

        // This flag is needed even when Unicode is not supported (it is used by
        // the IS_NEWLINE macro).
        let mut utf: BOOL = FALSE;

        // SUPPORT_UNICODE
        let ucp: BOOL;
        let allow_invalid: BOOL;
        let mut fragment_options: u32 = 0;

        let frame_size: PCRE2_SIZE;
        let mut heapframes_size: PCRE2_SIZE;

        // mb is a pointer to a match block because the IS_NEWLINE macro expects
        // NLBLOCK to be a pointer.
        let mut cb: pcre2_callout_block = core::mem::zeroed();
        let mut actual_match_block: match_block = core::mem::zeroed();
        let mb: *mut match_block = &mut actual_match_block;

        // Recognize NULL, length 0 as an empty string.
        if subject.is_null() && length == 0 {
            subject = null_str.as_ptr();
        }

        // Plausibility checks.
        if match_data.is_null() {
            return PCRE2_ERROR_NULL as c_int;
        }
        if code.is_null() || subject.is_null() {
            (*match_data).rc = PCRE2_ERROR_NULL as c_int;
            return (*match_data).rc;
        }
        if (options & !(PUBLIC_MATCH_OPTIONS)) != 0 {
            (*match_data).rc = PCRE2_ERROR_BADOPTION as c_int;
            return (*match_data).rc;
        }

        start_match = subject.add(start_offset);
        req_cu_ptr = start_match.sub(1);
        let mut length = length;
        if length == PCRE2_ZERO_TERMINATED {
            length = crate::string_utils::_pcre2_strlen_8(subject);
        }
        true_end_subject = subject.add(length);
        end_subject = true_end_subject;

        if start_offset > length {
            (*match_data).rc = PCRE2_ERROR_BADOFFSET as c_int;
            return (*match_data).rc;
        }

        // Check that the first field in the block is the magic number.
        if (*re).magic_number != MAGIC_NUMBER as u32 {
            (*match_data).rc = PCRE2_ERROR_BADMAGIC as c_int;
            return (*match_data).rc;
        }

        // Check the code unit width. PCRE2_CODE_UNIT_WIDTH/8 == 1.
        if ((*re).flags & PCRE2_MODE_MASK as u32) != (PCRE2_CODE_UNIT_WIDTH_8 / 8) {
            (*match_data).rc = PCRE2_ERROR_BADMODE as c_int;
            return (*match_data).rc;
        }

        // Transfer the (*NOTEMPTY)/(*NOTEMPTY_ATSTART) flag bits from the pattern
        // into the match-time options. FF and OO have adjacent bits.
        {
            const FF: u32 = (PCRE2_NOTEMPTY_SET as u32) | (PCRE2_NE_ATST_SET as u32);
            const OO: u32 = (PCRE2_NOTEMPTY as u32) | (PCRE2_NOTEMPTY_ATSTART as u32);
            options |= ((*re).flags & FF) / ((FF & (!FF).wrapping_add(1)) / (OO & (!OO).wrapping_add(1)));
        }

        // (JIT matching block omitted: SUPPORT_JIT is not defined.)

        // Initialize UTF/UCP parameters (SUPPORT_UNICODE).
        utf = if ((*re).overall_options & PCRE2_UTF as u32) != 0 { TRUE } else { FALSE };
        allow_invalid = if ((*re).overall_options & PCRE2_MATCH_INVALID_UTF as u32) != 0 {
            TRUE
        } else {
            FALSE
        };
        ucp = if ((*re).overall_options & PCRE2_UCP as u32) != 0 { TRUE } else { FALSE };

        // Convert the partial matching flags into an integer.
        (*mb).partial = if (options & PCRE2_PARTIAL_HARD as u32) != 0 {
            2
        } else if (options & PCRE2_PARTIAL_SOFT as u32) != 0 {
            1
        } else {
            0
        };

        // Partial matching and PCRE2_ENDANCHORED are not allowed together.
        if (*mb).partial != 0
            && (((*re).overall_options | options) & PCRE2_ENDANCHORED as u32) != 0
        {
            (*match_data).rc = PCRE2_ERROR_BADOPTION as c_int;
            return (*match_data).rc;
        }

        // It is an error to set an offset limit without setting the flag at
        // compile time.
        if !mcontext.is_null()
            && (*mcontext).offset_limit != PCRE2_UNSET
            && ((*re).overall_options & PCRE2_USE_OFFSET_LIMIT as u32) == 0
        {
            (*match_data).rc = PCRE2_ERROR_BADOFFSETLIMIT as c_int;
            return (*match_data).rc;
        }

        // If the match data block was previously used with
        // PCRE2_COPY_MATCHED_SUBJECT, free the memory that was obtained.
        if ((*match_data).flags & PCRE2_MD_COPIED_SUBJECT as u8) != 0 {
            ((*match_data).memctl.free.unwrap())(
                (*match_data).subject as *mut c_void,
                (*match_data).memctl.memory_data,
            );
            (*match_data).flags &= !(PCRE2_MD_COPIED_SUBJECT as u8);
        }
        (*match_data).subject = core::ptr::null();

        // Zero the error offset in case the first code unit is invalid UTF.
        (*match_data).startchar = 0;

        // ===================== Non-JIT matching ==========================

        // The default is to allow lookbehinds to the start of the subject. A UTF
        // check when there is a non-zero offset may change this.
        (*mb).check_subject = subject;

        // Check a UTF subject string for validity (SUPPORT_UNICODE). Since JIT is
        // not compiled in, jit_checked_utf is always FALSE.
        if utf != 0
            && ((options & PCRE2_NO_UTF_CHECK as u32) == 0 || allow_invalid != 0)
        {
            // PCRE2_CODE_UNIT_WIDTH != 32
            let mut skipped_bad_start: BOOL = FALSE;

            // For 8-bit UTF, check that the first code unit is a valid character
            // start. If handling invalid UTF, skip over such code units.
            // Otherwise, give an appropriate error.
            if allow_invalid != 0 {
                while start_match < end_subject && NOT_FIRSTCU(*start_match as u32) {
                    start_match = start_match.add(1);
                    skipped_bad_start = TRUE;
                }
            } else if start_match < end_subject && NOT_FIRSTCU(*start_match as u32) {
                if start_offset > 0 {
                    (*match_data).rc = PCRE2_ERROR_BADUTFOFFSET as c_int;
                    return (*match_data).rc;
                }
                // Isolated 0x80 byte.
                (*match_data).rc = PCRE2_ERROR_UTF8_ERR20 as c_int;
                return (*match_data).rc;
            }

            // The mb->check_subject field points to the start of UTF checking;
            // lookbehinds can go back no further than this.
            (*mb).check_subject = start_match;

            // Move back by the maximum lookbehind, unless we skipped bad code
            // units above.
            if skipped_bad_start == FALSE {
                let mut i = (*re).max_lookbehind as u32;
                while i > 0 && (*mb).check_subject > subject {
                    (*mb).check_subject = (*mb).check_subject.sub(1);
                    while (*mb).check_subject > subject
                        && (*(*mb).check_subject as u32 & 0xc0) == 0x80
                    {
                        (*mb).check_subject = (*mb).check_subject.sub(1);
                    }
                    i -= 1;
                }
            }

            // Validate the relevant portion of the subject. There's a loop in
            // case we encounter bad UTF in the characters preceding start_match
            // that we are scanning because of a lookbehind.
            loop {
                rc = crate::valid_utf::_pcre2_valid_utf_8(
                    (*mb).check_subject,
                    length - offset_diff((*mb).check_subject, subject),
                    &mut (*match_data).startchar,
                );

                if rc == 0 {
                    break; // Valid UTF string.
                }

                // Invalid UTF string. Adjust the offset to be an absolute offset
                // in the whole string.
                (*match_data).startchar += offset_diff((*mb).check_subject, subject);
                if allow_invalid == 0 || rc > 0 {
                    (*match_data).rc = rc;
                    return (*match_data).rc;
                }
                end_subject = subject.add((*match_data).startchar);

                // If the end precedes start_match, there is invalid UTF in the
                // extra code units we reversed over because of a lookbehind.
                if end_subject < start_match {
                    (*mb).check_subject = end_subject.add(1);
                    while (*mb).check_subject < start_match
                        && NOT_FIRSTCU(*(*mb).check_subject as u32)
                    {
                        (*mb).check_subject = (*mb).check_subject.add(1);
                    }
                    end_subject = true_end_subject;
                } else {
                    // Set the not end of line option, and do the match.
                    fragment_options = PCRE2_NOTEOL as u32;
                    break;
                }
            }
        }

        // A NULL match context means "use a default context", but we take the
        // memory control functions from the pattern.
        let mcontext: *mut pcre2_real_match_context = if mcontext.is_null() {
            (*mb).memctl = (*re).memctl;
            &raw mut crate::context::_pcre2_default_match_context_8
        } else {
            (*mb).memctl = (*mcontext).memctl;
            mcontext
        };

        anchored = if (((*re).overall_options | options) & PCRE2_ANCHORED as u32) != 0 {
            TRUE
        } else {
            FALSE
        };
        firstline = if anchored == FALSE
            && ((*re).overall_options & PCRE2_FIRSTLINE as u32) != 0
        {
            TRUE
        } else {
            FALSE
        };
        startline = if ((*re).flags & PCRE2_STARTLINE as u32) != 0 {
            TRUE
        } else {
            FALSE
        };
        bumpalong_limit = if (*mcontext).offset_limit == PCRE2_UNSET {
            true_end_subject
        } else {
            subject.add((*mcontext).offset_limit)
        };

        // Initialize and set up the fixed fields in the callout block.
        (*mb).cb = &mut cb;
        cb.version = 2;
        cb.subject = subject;
        cb.subject_length = offset_diff(end_subject, subject);
        cb.callout_flags = 0;

        // Fill in the remaining fields in the match block (moptions set later).
        (*mb).callout = (*mcontext).callout;
        (*mb).callout_data = (*mcontext).callout_data;

        (*mb).start_subject = subject;
        (*mb).start_offset = start_offset;
        (*mb).end_subject = end_subject;
        (*mb).true_end_subject = true_end_subject;
        (*mb).hasthen = if ((*re).flags & PCRE2_HASTHEN as u32) != 0 { TRUE } else { FALSE };
        (*mb).hasbsk = if ((*re).flags & PCRE2_HASBSK as u32) != 0 { TRUE } else { FALSE };
        (*mb).allowemptypartial = if (*re).max_lookbehind > 0
            || ((*re).flags & PCRE2_MATCH_EMPTY as u32) != 0
        {
            TRUE
        } else {
            FALSE
        };
        (*mb).allowlookaroundbsk =
            if ((*re).extra_options & PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK as u32) != 0 {
                TRUE
            } else {
                FALSE
            };
        (*mb).poptions = (*re).overall_options; // Pattern options.
        (*mb).ignore_skip_arg = 0;
        (*mb).mark = core::ptr::null();
        (*mb).nomatch_mark = core::ptr::null();

        // The name table follows the code.
        (*mb).name_table =
            (re as *const u8).add(core::mem::size_of::<pcre2_real_code>()) as PCRE2_SPTR;
        (*mb).name_count = (*re).name_count;
        (*mb).name_entry_size = (*re).name_entry_size;
        (*mb).start_code = (re as *const u8).add((*re).code_start) as PCRE2_SPTR;

        // Process the \R and newline settings.
        (*mb).bsr_convention = (*re).bsr_convention;
        (*mb).nltype = NLTYPE_FIXED as u32;
        match (*re).newline_convention as i64 {
            PCRE2_NEWLINE_CR => {
                (*mb).nllen = 1;
                (*mb).nl[0] = CHAR_CR as PCRE2_UCHAR;
            }
            PCRE2_NEWLINE_LF => {
                (*mb).nllen = 1;
                (*mb).nl[0] = CHAR_NL as PCRE2_UCHAR;
            }
            PCRE2_NEWLINE_NUL => {
                (*mb).nllen = 1;
                (*mb).nl[0] = CHAR_NUL as PCRE2_UCHAR;
            }
            PCRE2_NEWLINE_CRLF => {
                (*mb).nllen = 2;
                (*mb).nl[0] = CHAR_CR as PCRE2_UCHAR;
                (*mb).nl[1] = CHAR_NL as PCRE2_UCHAR;
            }
            PCRE2_NEWLINE_ANY => {
                (*mb).nltype = NLTYPE_ANY as u32;
            }
            PCRE2_NEWLINE_ANYCRLF => {
                (*mb).nltype = NLTYPE_ANYCRLF as u32;
            }
            _ => {
                (*match_data).rc = PCRE2_ERROR_INTERNAL as c_int;
                return (*match_data).rc;
            }
        }

        // Compute the frame size, padded for alignment.
        frame_size = (heapframe::OVECTOR_OFFSET
            + (*re).top_bracket as usize * 2 * core::mem::size_of::<PCRE2_SIZE>()
            + HEAPFRAME_ALIGNMENT
            - 1)
            & !(HEAPFRAME_ALIGNMENT - 1);

        // Limits set in the pattern override the match context only if smaller.
        (*mb).heap_limit = if (*mcontext).heap_limit < (*re).limit_heap {
            (*mcontext).heap_limit
        } else {
            (*re).limit_heap
        };
        (*mb).match_limit = if (*mcontext).match_limit < (*re).limit_match {
            (*mcontext).match_limit
        } else {
            (*re).limit_match
        };
        (*mb).match_limit_depth = if (*mcontext).depth_limit < (*re).limit_depth {
            (*mcontext).depth_limit
        } else {
            (*re).limit_depth
        };

        // Set the initial frame vector size (at least 10 frames, minimum
        // START_FRAMES_SIZE). If greater than the heap limit, get as large a
        // vector as possible.
        heapframes_size = frame_size * 10;
        if heapframes_size < START_FRAMES_SIZE as usize {
            heapframes_size = START_FRAMES_SIZE as usize;
        }
        if heapframes_size / 1024 > (*mb).heap_limit as usize {
            let max_size: PCRE2_SIZE = 1024 * (*mb).heap_limit as usize;
            if max_size < frame_size {
                (*match_data).rc = PCRE2_ERROR_HEAPLIMIT as c_int;
                return (*match_data).rc;
            }
            heapframes_size = max_size;
        }

        // Reuse an existing frame vector if large enough; otherwise free any
        // pre-existing vector and get a new one.
        if (*match_data).heapframes_size < heapframes_size {
            ((*match_data).memctl.free.unwrap())(
                (*match_data).heapframes as *mut c_void,
                (*match_data).memctl.memory_data,
            );
            (*match_data).heapframes = ((*match_data).memctl.malloc.unwrap())(
                heapframes_size,
                (*match_data).memctl.memory_data,
            ) as *mut heapframe;
            if (*match_data).heapframes.is_null() {
                (*match_data).heapframes_size = 0;
                (*match_data).rc = PCRE2_ERROR_NOMEMORY as c_int;
                return (*match_data).rc;
            }
            (*match_data).heapframes_size = heapframes_size;
        }

        // Write to the ovector within the first frame to mark every capture
        // unset and to avoid uninitialized reads when copied to a new frame.
        core::ptr::write_bytes(
            ((*match_data).heapframes as *mut u8).add(heapframe::OVECTOR_OFFSET),
            0xff,
            frame_size - heapframe::OVECTOR_OFFSET,
        );

        // Pointers to the individual character tables.
        (*mb).lcc = (*re).tables.add(lcc_offset as usize);
        (*mb).fcc = (*re).tables.add(fcc_offset as usize);
        (*mb).ctypes = (*re).tables.add(ctypes_offset as usize);

        // Set up the first code unit to match, if available. If there's no first
        // code unit there may be a bitmap of possible first characters.
        if ((*re).flags & PCRE2_FIRSTSET as u32) != 0 {
            has_first_cu = TRUE;
            first_cu = (*re).first_codeunit as PCRE2_UCHAR;
            first_cu2 = first_cu;
            if ((*re).flags & PCRE2_FIRSTCASELESS as u32) != 0 {
                first_cu2 = TABLE_GET(first_cu as u32, (*mb).fcc, first_cu as u32) as PCRE2_UCHAR;
                // PCRE2_CODE_UNIT_WIDTH == 8
                if first_cu > 127 && ucp != 0 && utf == 0 {
                    first_cu2 = UCD_OTHERCASE(first_cu as u32) as PCRE2_UCHAR;
                }
            }
        } else if startline == FALSE && ((*re).flags & PCRE2_FIRSTMAPSET as u32) != 0 {
            start_bits = (*re).start_bitmap.as_ptr();
        }

        // There may also be a "last known required character" set.
        if ((*re).flags & PCRE2_LASTSET as u32) != 0 {
            has_req_cu = TRUE;
            req_cu = (*re).last_codeunit as PCRE2_UCHAR;
            req_cu2 = req_cu;
            if ((*re).flags & PCRE2_LASTCASELESS as u32) != 0 {
                req_cu2 = TABLE_GET(req_cu as u32, (*mb).fcc, req_cu as u32) as PCRE2_UCHAR;
                // PCRE2_CODE_UNIT_WIDTH == 8
                if req_cu > 127 && ucp != 0 && utf == 0 {
                    req_cu2 = UCD_OTHERCASE(req_cu as u32) as PCRE2_UCHAR;
                }
            }
        }

        // ================================================================

        // IS_NEWLINE / WAS_NEWLINE macros, with NLBLOCK = mb,
        // PSSTART = start_subject, PSEND = end_subject.
        macro_rules! IS_NEWLINE {
            ($p:expr) => {{
                let p_: PCRE2_SPTR = $p;
                if (*mb).nltype != NLTYPE_FIXED as u32 {
                    p_ < end_subject
                        && crate::newline::_pcre2_is_newline_8(
                            p_,
                            (*mb).nltype,
                            end_subject,
                            &mut (*mb).nllen,
                            utf,
                        ) != 0
                } else {
                    let nllen = (*mb).nllen as usize;
                    (p_ as usize) <= (end_subject as usize).wrapping_sub(nllen)
                        && *p_ == (*mb).nl[0]
                        && ((*mb).nllen == 1 || *p_.add(1) == (*mb).nl[1])
                }
            }};
        }
        macro_rules! WAS_NEWLINE {
            ($p:expr) => {{
                let p_: PCRE2_SPTR = $p;
                if (*mb).nltype != NLTYPE_FIXED as u32 {
                    p_ > (*mb).start_subject
                        && crate::newline::_pcre2_was_newline_8(
                            p_,
                            (*mb).nltype,
                            (*mb).start_subject,
                            &mut (*mb).nllen,
                            utf,
                        ) != 0
                } else {
                    let nllen = (*mb).nllen as usize;
                    (p_ as usize) >= ((*mb).start_subject as usize).wrapping_add(nllen)
                        && *p_.sub(nllen) == (*mb).nl[0]
                        && ((*mb).nllen == 1 || *p_.sub(nllen).add(1) == (*mb).nl[1])
                }
            }};
        }

        // The final return code from match() (or a break in the bumpalong loop).
        // Loop for handling unanchored repeated matching attempts; for anchored
        // regexes the loop runs just once.
        //
        // 'FRAGMENT_RESTART:' is implemented as a labelled outer loop that we
        // 'continue' to restart the bumpalong.
        'fragment_restart: loop {
            start_partial = core::ptr::null();
            match_partial = core::ptr::null();
            (*mb).hitend = FALSE;

            // PCRE2_CODE_UNIT_WIDTH == 8
            memchr_found_first_cu = core::ptr::null();
            memchr_found_first_cu2 = core::ptr::null();

            // The bumpalong loop.
            'bumpalong: loop {
                let mut new_start_match: PCRE2_SPTR;

                // ------------- Start of match optimizations ------------

                if ((*re).optimization_flags & PCRE2_OPTIM_START_OPTIMIZE as u32) != 0 {
                    // firstline: constrain the start to the first line.
                    if firstline != 0 {
                        let mut t: PCRE2_SPTR = start_match;
                        if utf != 0 {
                            while t < end_subject && !IS_NEWLINE!(t) {
                                t = t.add(1);
                                // ACROSSCHAR(t < end_subject, t, t++)
                                while t < end_subject && (*t as u32 & 0xc0) == 0x80 {
                                    t = t.add(1);
                                }
                            }
                        } else {
                            while t < end_subject && !IS_NEWLINE!(t) {
                                t = t.add(1);
                            }
                        }
                        end_subject = t;
                    }

                    // Anchored: check the first code unit if one is recorded.
                    if anchored != 0 {
                        if has_first_cu != 0 || !start_bits.is_null() {
                            let mut ok: bool = start_match < end_subject;
                            if ok {
                                let mut c: PCRE2_UCHAR = *start_match;
                                ok = has_first_cu != 0 && (c == first_cu || c == first_cu2);
                                if !ok && !start_bits.is_null() {
                                    ok = (*start_bits.add((c / 8) as usize)
                                        & (1u8 << (c & 7)))
                                        != 0;
                                }
                            }
                            if !ok {
                                rc = MATCH_NOMATCH;
                                break 'bumpalong;
                            }
                        }
                    }
                    // Not anchored. Advance to a unique first code unit if any.
                    else {
                        if has_first_cu != 0 {
                            if first_cu != first_cu2 {
                                // Caseless. 8-bit: use memchr() twice.
                                let mut pp1: PCRE2_SPTR;
                                let mut pp2: PCRE2_SPTR;
                                let searchlength: PCRE2_SIZE =
                                    offset_diff(end_subject, start_match);

                                if memchr_found_first_cu.is_null()
                                    || start_match > memchr_found_first_cu
                                {
                                    pp1 = memchr(
                                        start_match as *const c_void,
                                        first_cu as c_int,
                                        searchlength,
                                    ) as PCRE2_SPTR;
                                    memchr_found_first_cu =
                                        if pp1.is_null() { end_subject } else { pp1 };
                                } else {
                                    pp1 = if memchr_found_first_cu == end_subject {
                                        core::ptr::null()
                                    } else {
                                        memchr_found_first_cu
                                    };
                                }

                                if memchr_found_first_cu2.is_null()
                                    || start_match > memchr_found_first_cu2
                                {
                                    pp2 = memchr(
                                        start_match as *const c_void,
                                        first_cu2 as c_int,
                                        searchlength,
                                    ) as PCRE2_SPTR;
                                    memchr_found_first_cu2 =
                                        if pp2.is_null() { end_subject } else { pp2 };
                                } else {
                                    pp2 = if memchr_found_first_cu2 == end_subject {
                                        core::ptr::null()
                                    } else {
                                        memchr_found_first_cu2
                                    };
                                }

                                if pp1.is_null() {
                                    start_match = if pp2.is_null() { end_subject } else { pp2 };
                                } else {
                                    start_match =
                                        if pp2.is_null() || pp1 < pp2 { pp1 } else { pp2 };
                                }
                            } else {
                                // Caseful. 8-bit: single memchr().
                                start_match = memchr(
                                    start_match as *const c_void,
                                    first_cu as c_int,
                                    offset_diff(end_subject, start_match),
                                )
                                    as PCRE2_SPTR;
                                if start_match.is_null() {
                                    start_match = end_subject;
                                }
                            }

                            // If we can't find the first code unit, having
                            // reached the true end of the subject, break the
                            // bumpalong loop, except when doing partial matching.
                            if (*mb).partial == 0 && start_match >= (*mb).end_subject {
                                rc = MATCH_NOMATCH;
                                break 'bumpalong;
                            }
                        }
                        // No first code unit: advance to just after a linebreak
                        // for a multiline match if required.
                        else if startline != 0 {
                            if start_match > (*mb).start_subject.add(start_offset) {
                                if utf != 0 {
                                    while start_match < end_subject
                                        && !WAS_NEWLINE!(start_match)
                                    {
                                        start_match = start_match.add(1);
                                        // ACROSSCHAR(start_match < end_subject, ...)
                                        while start_match < end_subject
                                            && (*start_match as u32 & 0xc0) == 0x80
                                        {
                                            start_match = start_match.add(1);
                                        }
                                    }
                                } else {
                                    while start_match < end_subject
                                        && !WAS_NEWLINE!(start_match)
                                    {
                                        start_match = start_match.add(1);
                                    }
                                }

                                // If we have just passed a CR and the newline
                                // option is ANY or ANYCRLF, and we are now at LF,
                                // advance by one more code unit.
                                if *start_match.sub(1) == CHAR_CR as PCRE2_UCHAR
                                    && ((*mb).nltype == NLTYPE_ANY as u32
                                        || (*mb).nltype == NLTYPE_ANYCRLF as u32)
                                    && start_match < end_subject
                                    && *start_match == CHAR_NL as PCRE2_UCHAR
                                {
                                    start_match = start_match.add(1);
                                }
                            }
                        }
                        // Advance to a non-unique first code unit from the bitmap.
                        else if !start_bits.is_null() {
                            while start_match < end_subject {
                                let c: u32 = *start_match as u32;
                                if (*start_bits.add((c / 8) as usize) & (1u8 << (c & 7))) != 0 {
                                    break;
                                }
                                start_match = start_match.add(1);
                            }

                            if (*mb).partial == 0 && start_match >= (*mb).end_subject {
                                rc = MATCH_NOMATCH;
                                break 'bumpalong;
                            }
                        }
                    } // End first code unit handling.

                    // Restore fudged end_subject.
                    end_subject = (*mb).end_subject;

                    // The following two optimizations must be disabled for
                    // partial matching.
                    if (*mb).partial == 0 {
                        let mut p: PCRE2_SPTR;

                        // The minimum matching length is a lower bound.
                        if offset_diff(end_subject, start_match) < (*re).minlength as usize {
                            rc = MATCH_NOMATCH;
                            break 'bumpalong;
                        }

                        // req_cu presence check.
                        p = start_match.add(if has_first_cu != 0 { 1 } else { 0 });
                        if has_req_cu != 0 && p > req_cu_ptr {
                            let check_length: PCRE2_SIZE =
                                offset_diff(end_subject, start_match);

                            if check_length < REQ_CU_MAX as usize
                                || (anchored == FALSE
                                    && check_length < REQ_CU_MAX as usize * 1000)
                            {
                                if req_cu != req_cu2 {
                                    // Caseless. 8-bit code units.
                                    let pp: PCRE2_SPTR = p;
                                    p = memchr(
                                        pp as *const c_void,
                                        req_cu as c_int,
                                        offset_diff(end_subject, pp),
                                    ) as PCRE2_SPTR;
                                    if p.is_null() {
                                        p = memchr(
                                            pp as *const c_void,
                                            req_cu2 as c_int,
                                            offset_diff(end_subject, pp),
                                        )
                                            as PCRE2_SPTR;
                                        if p.is_null() {
                                            p = end_subject;
                                        }
                                    }
                                } else {
                                    // Caseful. 8-bit code units.
                                    p = memchr(
                                        p as *const c_void,
                                        req_cu as c_int,
                                        offset_diff(end_subject, p),
                                    ) as PCRE2_SPTR;
                                    if p.is_null() {
                                        p = end_subject;
                                    }
                                }

                                // If we can't find the required code unit, break.
                                if p >= end_subject {
                                    rc = MATCH_NOMATCH;
                                    break 'bumpalong;
                                }

                                // Save where we found it.
                                req_cu_ptr = p;
                            }
                        }
                    }
                }

                // ---------- End of start of match optimizations ----------

                // Give no match if we have passed the bumpalong limit.
                if start_match > bumpalong_limit {
                    rc = MATCH_NOMATCH;
                    break 'bumpalong;
                }

                // Run the match.
                cb.start_match = offset_diff(start_match, subject);
                cb.callout_flags |= PCRE2_CALLOUT_STARTMATCH as u32;

                (*mb).start_used_ptr = start_match;
                (*mb).last_used_ptr = start_match;
                (*mb).moptions = options | fragment_options;
                (*mb).match_call_count = 0;
                (*mb).end_offset_top = 0;
                (*mb).skip_arg_count = 0;

                rc = crate::match_core::match_(
                    start_match,
                    (*mb).start_code,
                    (*re).top_bracket,
                    frame_size,
                    match_data,
                    mb,
                );

                if (*mb).hitend != 0 && start_partial.is_null() {
                    start_partial = (*mb).start_used_ptr;
                    match_partial = start_match;
                }

                match rc {
                    // MATCH_SKIP_ARG: a MARK matching the SKIP's arg was not
                    // found; re-do the match at the same point, ignoring
                    // SKIP-with-argument.
                    MATCH_SKIP_ARG => {
                        new_start_match = start_match;
                        (*mb).ignore_skip_arg = (*mb).skip_arg_count;
                    }

                    // MATCH_SKIP: next starting point is explicit; if not greater
                    // than the current match, fall through to NOMATCH handling.
                    MATCH_SKIP => {
                        if (*mb).verb_skip_ptr > start_match {
                            new_start_match = (*mb).verb_skip_ptr;
                        } else {
                            // Fall through (NOMATCH/PRUNE/THEN).
                            (*mb).ignore_skip_arg = 0;
                            new_start_match = start_match.add(1);
                            if utf != 0 {
                                // ACROSSCHAR(new_start_match < end_subject, ...)
                                while new_start_match < end_subject
                                    && (*new_start_match as u32 & 0xc0) == 0x80
                                {
                                    new_start_match = new_start_match.add(1);
                                }
                            }
                        }
                    }

                    // NOMATCH and PRUNE advance by one character. THEN at this
                    // level acts like PRUNE. Unset ignore SKIP-with-argument.
                    MATCH_NOMATCH | MATCH_PRUNE | MATCH_THEN => {
                        (*mb).ignore_skip_arg = 0;
                        new_start_match = start_match.add(1);
                        if utf != 0 {
                            while new_start_match < end_subject
                                && (*new_start_match as u32 & 0xc0) == 0x80
                            {
                                new_start_match = new_start_match.add(1);
                            }
                        }
                    }

                    // COMMIT disables the bumpalong, otherwise like NOMATCH.
                    // (C: `goto ENDLOOP`, which is immediately after this loop.)
                    MATCH_COMMIT => {
                        rc = MATCH_NOMATCH;
                        break 'bumpalong;
                    }

                    // Any other return is a match or an error.
                    // (C: `goto ENDLOOP`.)
                    _ => {
                        break 'bumpalong;
                    }
                }

                // Reset the code to MATCH_NOMATCH for subsequent checking.
                rc = MATCH_NOMATCH;

                // PCRE2_FIRSTLINE: if we have just failed to match starting at a
                // newline, do not continue.
                if firstline != 0 && IS_NEWLINE!(start_match) {
                    break 'bumpalong;
                }

                // Advance to new matching position.
                start_match = new_start_match;

                // Break if anchored or past the end of the subject.
                if anchored != 0 || start_match > end_subject {
                    break 'bumpalong;
                }

                // Skip a LF after a CR at a match boundary under certain newline
                // conventions when the pattern has no explicit \r or \n.
                if start_match > subject.add(start_offset)
                    && *start_match.sub(1) == CHAR_CR as PCRE2_UCHAR
                    && start_match < end_subject
                    && *start_match == CHAR_NL as PCRE2_UCHAR
                    && ((*re).flags & PCRE2_HASCRORLF as u32) == 0
                    && ((*mb).nltype == NLTYPE_ANY as u32
                        || (*mb).nltype == NLTYPE_ANYCRLF as u32
                        || (*mb).nllen == 2)
                {
                    start_match = start_match.add(1);
                }

                (*mb).mark = core::ptr::null(); // Reset for next match attempt.
            } // End of bumpalong loop.

            // ===================== ENDLOOP handling =======================

            // If end_subject != true_end_subject, we are handling invalid UTF and
            // have just processed a non-terminal fragment. Carry on to the next
            // fragment on no match or partial match (SUPPORT_UNICODE).
            if utf != 0
                && end_subject != true_end_subject
                && (rc == MATCH_NOMATCH || rc == PCRE2_ERROR_PARTIAL as c_int)
            {
                loop {
                    // Advance past the first bad code unit, then skip invalid
                    // character starting code units (8-bit).
                    start_match = end_subject.add(1);
                    while start_match < true_end_subject && NOT_FIRSTCU(*start_match as u32) {
                        start_match = start_match.add(1);
                    }

                    // If we've hit the end, there isn't another non-empty
                    // fragment, so give up.
                    if start_match >= true_end_subject {
                        rc = MATCH_NOMATCH; // In case it was partial.
                        match_partial = core::ptr::null();
                        break;
                    }

                    // Check the rest of the subject.
                    (*mb).check_subject = start_match;
                    rc = crate::valid_utf::_pcre2_valid_utf_8(
                        start_match,
                        length - offset_diff(start_match, subject),
                        &mut (*match_data).startchar,
                    );

                    // The rest of the subject is valid UTF.
                    if rc == 0 {
                        end_subject = true_end_subject;
                        (*mb).end_subject = end_subject;
                        fragment_options = PCRE2_NOTBOL as u32;
                        continue 'fragment_restart;
                    }
                    // A subsequent UTF error; if the next fragment is non-empty,
                    // set up to process it. Otherwise, let the loop advance.
                    else if rc < 0 {
                        end_subject = start_match.add((*match_data).startchar);
                        (*mb).end_subject = end_subject;
                        if end_subject > start_match {
                            fragment_options = (PCRE2_NOTBOL | PCRE2_NOTEOL) as u32;
                            continue 'fragment_restart;
                        }
                    }
                }
            }

            break 'fragment_restart rc;
        }; // End of 'fragment_restart loop; `rc` now holds the final result.

        // ==================================================================
        // Fill in fields that are always returned in the match data.
        (*match_data).code = re;
        (*match_data).mark = (*mb).mark;
        (*match_data).matchedby = PCRE2_MATCHEDBY_INTERPRETER as u8;
        (*match_data).options = original_options;

        // Handle a fully successful match.
        if rc == MATCH_MATCH {
            (*match_data).rc = if (*mb).end_offset_top as c_int
                >= 2 * (*match_data).oveccount as c_int
            {
                0
            } else {
                (*mb).end_offset_top as c_int / 2 + 1
            };
            (*match_data).subject_length = length;
            (*match_data).start_offset = start_offset;
            (*match_data).startchar = offset_diff(start_match, subject);
            (*match_data).leftchar = offset_diff((*mb).start_used_ptr, subject);
            (*match_data).rightchar = offset_diff(
                if (*mb).last_used_ptr > (*mb).end_match_ptr {
                    (*mb).last_used_ptr
                } else {
                    (*mb).end_match_ptr
                },
                subject,
            );
            if (options & PCRE2_COPY_MATCHED_SUBJECT as u32) != 0 {
                if length != 0 {
                    (*match_data).subject = ((*match_data).memctl.malloc.unwrap())(
                        CU2BYTES(length),
                        (*match_data).memctl.memory_data,
                    ) as PCRE2_SPTR;
                    if (*match_data).subject.is_null() {
                        (*match_data).rc = PCRE2_ERROR_NOMEMORY as c_int;
                        return (*match_data).rc;
                    }
                    core::ptr::copy_nonoverlapping(
                        subject,
                        (*match_data).subject as *mut PCRE2_UCHAR,
                        CU2BYTES(length),
                    );
                } else {
                    (*match_data).subject = core::ptr::null();
                }
                (*match_data).flags |= PCRE2_MD_COPIED_SUBJECT as u8;
            } else {
                (*match_data).subject = original_subject;
            }

            return (*match_data).rc;
        }

        // Control gets here if there has been a partial match, an error, or if
        // the overall match attempt failed at all permitted starting positions.
        // Any mark data is in the nomatch_mark field.
        (*match_data).mark = (*mb).nomatch_mark;

        // For anything other than nomatch or partial match, just return the code.
        if rc != MATCH_NOMATCH && rc != PCRE2_ERROR_PARTIAL as c_int {
            (*match_data).rc = rc;
        }
        // Handle a partial match.
        else if !match_partial.is_null() {
            (*match_data).subject = original_subject;
            (*match_data).subject_length = length;
            (*match_data).start_offset = start_offset;
            *(*match_data).ovec().add(0) = offset_diff(match_partial, subject);
            *(*match_data).ovec().add(1) = offset_diff(end_subject, subject);
            (*match_data).startchar = offset_diff(match_partial, subject);
            (*match_data).leftchar = offset_diff(start_partial, subject);
            (*match_data).rightchar = offset_diff(end_subject, subject);
            (*match_data).rc = PCRE2_ERROR_PARTIAL as c_int;
        }
        // Else this is the classic nomatch case.
        else {
            (*match_data).subject = original_subject;
            (*match_data).subject_length = length;
            (*match_data).start_offset = start_offset;
            (*match_data).rc = PCRE2_ERROR_NOMATCH as c_int;
        }

        (*match_data).rc
    }
}

// Helper: pointer difference in code units (bytes in 8-bit mode).
#[inline(always)]
unsafe fn offset_diff(hi: PCRE2_SPTR, lo: PCRE2_SPTR) -> PCRE2_SIZE {
    (hi as usize) - (lo as usize)
}
