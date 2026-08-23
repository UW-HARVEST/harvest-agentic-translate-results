/* This function applies a compiled pattern to a subject string and picks out
portions of the string if it matches. Two elements in the vector are set for
each substring: the offsets to the start and end of the substring.

Arguments:
  code            points to the compiled expression
  subject         points to the subject string
  length          length of subject string (may contain binary zeros)
  start_offset    where to start in the subject string
  options         option bits
  match_data      points to a match_data block
  mcontext        points a PCRE2 context

Returns:          > 0 => success; value is the number of ovector pairs filled
                  = 0 => success, but ovector is not big enough
                  = -1 => failed to match (PCRE2_ERROR_NOMATCH)
                  = -2 => partial match (PCRE2_ERROR_PARTIAL)
                  < -2 => some kind of unexpected problem
*/

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
    /* The C function modifies these arguments. */

    let mut subject: PCRE2_SPTR = subject;
    let mut length: PCRE2_SIZE = length;
    let mut options: u32 = options;
    let mut mcontext: *mut pcre2_real_match_context = mcontext;

    let mut rc: c_int = 0;
    let mut start_bits: *const u8 = core::ptr::null();
    let re: *const pcre2_real_code = code;
    let original_options: u32 = options;

    let anchored: BOOL;
    let firstline: BOOL;
    let mut has_first_cu: BOOL = FALSE;
    let mut has_req_cu: BOOL = FALSE;
    let startline: BOOL;

    let mut memchr_found_first_cu: PCRE2_SPTR;
    let mut memchr_found_first_cu2: PCRE2_SPTR;

    let mut first_cu: PCRE2_UCHAR = 0;
    let mut first_cu2: PCRE2_UCHAR = 0;
    let mut req_cu: PCRE2_UCHAR = 0;
    let mut req_cu2: PCRE2_UCHAR = 0;

    let null_str: [PCRE2_UCHAR; 1] = [0xcd];
    let original_subject: PCRE2_SPTR = subject;
    let bumpalong_limit: PCRE2_SPTR;
    let mut end_subject: PCRE2_SPTR;
    let true_end_subject: PCRE2_SPTR;
    let mut start_match: PCRE2_SPTR;
    let mut req_cu_ptr: PCRE2_SPTR;
    let mut start_partial: PCRE2_SPTR;
    let mut match_partial: PCRE2_SPTR;

    /* This flag is needed even when Unicode is not supported for convenience
    (it is used by the IS_NEWLINE macro). */

    let mut utf: BOOL = FALSE;

    let mut ucp: BOOL = FALSE;
    let allow_invalid: BOOL;
    let mut fragment_options: u32 = 0;

    let frame_size: PCRE2_SIZE;
    let mut heapframes_size: PCRE2_SIZE;

    /* We need to have mb as a pointer to a match block, because the IS_NEWLINE
    macro is used below, and it expects NLBLOCK to be defined as a pointer. */

    let mut cb: pcre2_callout_block = core::mem::zeroed();
    let mut actual_match_block: match_block = core::mem::zeroed();
    let mb: *mut match_block = &mut actual_match_block as *mut match_block;

    /* Stands in for the C "return match_data->rc = xxx;" idiom. */

    macro_rules! RETURN_RC {
        ($rc:expr) => {{
            (*match_data).rc = $rc;
            return (*match_data).rc;
        }};
    }

    /* Recognize NULL, length 0 as an empty string. */

    if subject.is_null() && length == 0 {
        subject = null_str.as_ptr();
    }

    /* Plausibility checks */

    if match_data.is_null() {
        return PCRE2_ERROR_NULL;
    }
    if code.is_null() || subject.is_null() {
        RETURN_RC!(PCRE2_ERROR_NULL);
    }
    if (options & !PUBLIC_MATCH_OPTIONS) != 0 {
        RETURN_RC!(PCRE2_ERROR_BADOPTION);
    }

    start_match = subject.add(start_offset);
    req_cu_ptr = start_match.wrapping_sub(1);
    if length == PCRE2_ZERO_TERMINATED {
        length = _pcre2_strlen_8(subject);
    }
    end_subject = subject.add(length);
    true_end_subject = end_subject;

    if start_offset > length {
        RETURN_RC!(PCRE2_ERROR_BADOFFSET);
    }

    /* Check that the first field in the block is the magic number. */

    if (*re).magic_number != MAGIC_NUMBER {
        RETURN_RC!(PCRE2_ERROR_BADMAGIC);
    }

    /* Check the code unit width. */

    if ((*re).flags & PCRE2_MODE_MASK) != 1
    /* PCRE2_CODE_UNIT_WIDTH/8 */
    {
        RETURN_RC!(PCRE2_ERROR_BADMODE);
    }

    /* PCRE2_NOTEMPTY and PCRE2_NOTEMPTY_ATSTART are match-time flags in the
    options variable for this function. Users of PCRE2 who are not calling the
    function directly would like to have a way of setting these flags, in the same
    way that they can set pcre2_compile() flags like PCRE2_NO_AUTO_POSSESS with
    constructions like (*NO_AUTOPOSSESS). To enable this, (*NOTEMPTY) and
    (*NOTEMPTY_ATSTART) set bits in the pattern's "flag" function which we now
    transfer to the options for this function. */

    {
        const FF: u32 = PCRE2_NOTEMPTY_SET | PCRE2_NE_ATST_SET;
        const OO: u32 = PCRE2_NOTEMPTY | PCRE2_NOTEMPTY_ATSTART;
        options |= ((*re).flags & FF)
            / ((FF & (!FF).wrapping_add(1)) / (OO & (!OO).wrapping_add(1)));
    }

    /* Initialize UTF/UCP parameters. */

    utf = (((*re).overall_options & PCRE2_UTF) != 0) as BOOL;
    allow_invalid = (((*re).overall_options & PCRE2_MATCH_INVALID_UTF) != 0) as BOOL;
    ucp = (((*re).overall_options & PCRE2_UCP) != 0) as BOOL;

    /* Convert the partial matching flags into an integer. */

    (*mb).partial = if (options & PCRE2_PARTIAL_HARD) != 0 {
        2
    } else if (options & PCRE2_PARTIAL_SOFT) != 0 {
        1
    } else {
        0
    };

    /* Partial matching and PCRE2_ENDANCHORED are currently not allowed at the same
    time. */

    if (*mb).partial != 0 && (((*re).overall_options | options) & PCRE2_ENDANCHORED) != 0 {
        RETURN_RC!(PCRE2_ERROR_BADOPTION);
    }

    /* It is an error to set an offset limit without setting the flag at compile
    time. */

    if !mcontext.is_null()
        && (*mcontext).offset_limit != PCRE2_UNSET
        && ((*re).overall_options & PCRE2_USE_OFFSET_LIMIT) == 0
    {
        RETURN_RC!(PCRE2_ERROR_BADOFFSETLIMIT);
    }

    /* If the match data block was previously used with PCRE2_COPY_MATCHED_SUBJECT,
    free the memory that was obtained. Set the field to NULL for match error
    cases. */

    if ((*match_data).flags as u32 & PCRE2_MD_COPIED_SUBJECT) != 0 {
        ((*match_data).memctl.free.unwrap())(
            (*match_data).subject as *mut c_void,
            (*match_data).memctl.memory_data,
        );
        (*match_data).flags &= !(PCRE2_MD_COPIED_SUBJECT as u8);
    }
    (*match_data).subject = core::ptr::null();

    /* Zero the error offset in case the first code unit is invalid UTF. */

    (*match_data).startchar = 0;

    /* ============================= JIT matching ==============================

    SUPPORT_JIT is not defined in this build, so there is no JIT matching code
    here at all.

    ========================= End of JIT matching ========================== */

    /* Proceed with non-JIT matching. The default is to allow lookbehinds to the
    start of the subject. A UTF check when there is a non-zero offset may change
    this. */

    (*mb).check_subject = subject;

    /* Check a UTF subject string for validity, and handle support for invalid UTF
    strings. We check only the portion of the subject that might be inspected
    during matching - from the offset minus the maximum lookbehind to the given
    length. Note also that support for invalid UTF forces a check, overriding the
    setting of PCRE2_NO_CHECK_UTF. */

    if utf != 0 && ((options & PCRE2_NO_UTF_CHECK) == 0 || allow_invalid != 0) {
        let mut skipped_bad_start: BOOL = FALSE;

        /* For 8-bit and 16-bit UTF, check that the first code unit is a valid
        character start. If we are handling invalid UTF, just skip over such code
        units. Otherwise, give an appropriate error. */

        if allow_invalid != 0 {
            while start_match < end_subject && NOT_FIRSTCU!(*start_match) {
                start_match = start_match.add(1);
                skipped_bad_start = TRUE;
            }
        } else if start_match < end_subject && NOT_FIRSTCU!(*start_match) {
            if start_offset > 0 {
                RETURN_RC!(PCRE2_ERROR_BADUTFOFFSET);
            }
            RETURN_RC!(PCRE2_ERROR_UTF8_ERR20); /* Isolated 0x80 byte */
        }

        /* The mb->check_subject field points to the start of UTF checking;
        lookbehinds can go back no further than this. */

        (*mb).check_subject = start_match;

        /* Move back by the maximum lookbehind, just in case it happens at the very
        start of matching, but don't do this if we skipped bad 8-bit or 16-bit code
        units above. */

        if skipped_bad_start == 0 {
            let mut i: c_uint = (*re).max_lookbehind as c_uint;
            while i > 0 && (*mb).check_subject > subject {
                (*mb).check_subject = (*mb).check_subject.sub(1);
                while (*mb).check_subject > subject && (*(*mb).check_subject & 0xc0) == 0x80 {
                    (*mb).check_subject = (*mb).check_subject.sub(1);
                }
                i -= 1;
            }
        }

        /* Validate the relevant portion of the subject. There's a loop in case we
        encounter bad UTF in the characters preceding start_match which we are
        scanning because of a lookbehind. */

        loop {
            rc = _pcre2_valid_utf_8(
                (*mb).check_subject,
                length - ((*mb).check_subject.offset_from(subject) as PCRE2_SIZE),
                &raw mut (*match_data).startchar,
            );

            if rc == 0 {
                break; /* Valid UTF string */
            }

            /* Invalid UTF string. Adjust the offset to be an absolute offset in the
            whole string. If we are handling invalid UTF strings, set end_subject to
            stop before the bad code unit, and set the options to "not end of line".
            Otherwise return the error. */

            (*match_data).startchar += (*mb).check_subject.offset_from(subject) as PCRE2_SIZE;
            if allow_invalid == 0 || rc > 0 {
                RETURN_RC!(rc);
            }
            end_subject = subject.add((*match_data).startchar);

            /* If the end precedes start_match, it means there is invalid UTF in the
            extra code units we reversed over because of a lookbehind. Advance past the
            first bad code unit, and then skip invalid character starting code units in
            8-bit and 16-bit modes, and try again with the original end point. */

            if end_subject < start_match {
                (*mb).check_subject = end_subject.add(1);
                while (*mb).check_subject < start_match && NOT_FIRSTCU!(*(*mb).check_subject) {
                    (*mb).check_subject = (*mb).check_subject.add(1);
                }
                end_subject = true_end_subject;
            }
            /* Otherwise, set the not end of line option, and do the match. */
            else {
                fragment_options = PCRE2_NOTEOL;
                break;
            }
        }
    }

    /* A NULL match context means "use a default context", but we take the memory
    control functions from the pattern. */

    if mcontext.is_null() {
        mcontext = (&raw mut _pcre2_default_match_context_8) as *mut pcre2_real_match_context;
        (*mb).memctl = (*re).memctl;
    } else {
        (*mb).memctl = (*mcontext).memctl;
    }

    anchored = ((((*re).overall_options | options) & PCRE2_ANCHORED) != 0) as BOOL;
    firstline = (anchored == 0 && ((*re).overall_options & PCRE2_FIRSTLINE) != 0) as BOOL;
    startline = (((*re).flags & PCRE2_STARTLINE) != 0) as BOOL;
    bumpalong_limit = if (*mcontext).offset_limit == PCRE2_UNSET {
        true_end_subject
    } else {
        subject.add((*mcontext).offset_limit)
    };

    /* Initialize and set up the fixed fields in the callout block, with a pointer
    in the match block. */

    (*mb).cb = &mut cb as *mut pcre2_callout_block;
    cb.version = 2;
    cb.subject = subject;
    cb.subject_length = end_subject.offset_from(subject) as PCRE2_SIZE;
    cb.callout_flags = 0;

    /* Fill in the remaining fields in the match block, except for moptions, which
    gets set later. */

    (*mb).callout = (*mcontext).callout;
    (*mb).callout_data = (*mcontext).callout_data;

    (*mb).start_subject = subject;
    (*mb).start_offset = start_offset;
    (*mb).end_subject = end_subject;
    (*mb).true_end_subject = true_end_subject;
    (*mb).hasthen = (((*re).flags & PCRE2_HASTHEN) != 0) as BOOL;
    (*mb).hasbsk = (((*re).flags & PCRE2_HASBSK) != 0) as BOOL;
    (*mb).allowemptypartial =
        ((*re).max_lookbehind > 0 || ((*re).flags & PCRE2_MATCH_EMPTY) != 0) as BOOL;
    (*mb).allowlookaroundbsk =
        (((*re).extra_options & PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK) != 0) as BOOL;
    (*mb).poptions = (*re).overall_options; /* Pattern options */
    (*mb).ignore_skip_arg = 0;
    (*mb).nomatch_mark = core::ptr::null(); /* In case never set */
    (*mb).mark = (*mb).nomatch_mark;

    /* The name table is needed for finding all the numbers associated with a
    given name, for condition testing. The code follows the name table. */

    (*mb).name_table = (re as *const u8).add(size_of::<pcre2_real_code>());
    (*mb).name_count = (*re).name_count;
    (*mb).name_entry_size = (*re).name_entry_size;
    (*mb).start_code = (re as *const u8).add((*re).code_start);

    /* Process the \R and newline settings. */

    (*mb).bsr_convention = (*re).bsr_convention;
    (*mb).nltype = NLTYPE_FIXED;
    match (*re).newline_convention as u32 {
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
            (*mb).nltype = NLTYPE_ANY;
        }

        PCRE2_NEWLINE_ANYCRLF => {
            (*mb).nltype = NLTYPE_ANYCRLF;
        }

        _ => {
            RETURN_RC!(PCRE2_ERROR_INTERNAL);
        }
    }

    /* The backtracking frames have fixed data at the front, and a PCRE2_SIZE
    vector at the end, whose size depends on the number of capturing parentheses in
    the pattern. We must pad the frame_size for alignment to ensure subsequent
    frames are as aligned as heapframe. */

    frame_size = (offset_of!(heapframe, ovector)
        + (*re).top_bracket as PCRE2_SIZE * 2 * size_of::<PCRE2_SIZE>()
        + HEAPFRAME_ALIGNMENT
        - 1)
        & !(HEAPFRAME_ALIGNMENT - 1);

    /* Limits set in the pattern override the match context only if they are
    smaller. */

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

    /* If a pattern has very many capturing parentheses, the frame size may be very
    large. Set the initial frame vector size to ensure that there are at least 10
    available frames, but enforce a minimum of START_FRAMES_SIZE. If this is
    greater than the heap limit, get as large a vector as possible. */

    heapframes_size = frame_size * 10;
    if heapframes_size < START_FRAMES_SIZE {
        heapframes_size = START_FRAMES_SIZE;
    }
    if heapframes_size / 1024 > (*mb).heap_limit as PCRE2_SIZE {
        let max_size: PCRE2_SIZE = 1024 * (*mb).heap_limit as PCRE2_SIZE;
        if max_size < frame_size {
            RETURN_RC!(PCRE2_ERROR_HEAPLIMIT);
        }
        heapframes_size = max_size;
    }

    /* If an existing frame vector in the match_data block is large enough, we can
    use it. Otherwise, free any pre-existing vector and get a new one. */

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
            RETURN_RC!(PCRE2_ERROR_NOMEMORY);
        }
        (*match_data).heapframes_size = heapframes_size;
    }

    /* Write to the ovector within the first frame to mark every capture unset and
    to avoid uninitialized memory read errors when it is copied to a new frame. */

    memset(
        ((*match_data).heapframes as *mut u8).add(offset_of!(heapframe, ovector)) as *mut c_void,
        0xff,
        frame_size - offset_of!(heapframe, ovector),
    );

    /* Pointers to the individual character tables */

    (*mb).lcc = (*re).tables.add(lcc_offset);
    (*mb).fcc = (*re).tables.add(fcc_offset);
    (*mb).ctypes = (*re).tables.add(ctypes_offset);

    /* Set up the first code unit to match, if available. If there's no first code
    unit there may be a bitmap of possible first characters. */

    if ((*re).flags & PCRE2_FIRSTSET) != 0 {
        has_first_cu = TRUE;
        first_cu2 = (*re).first_codeunit as PCRE2_UCHAR;
        first_cu = first_cu2;
        if ((*re).flags & PCRE2_FIRSTCASELESS) != 0 {
            first_cu2 = TABLE_GET!(first_cu, (*mb).fcc, first_cu);
            if first_cu > 127 && ucp != 0 && utf == 0 {
                first_cu2 = UCD_OTHERCASE(first_cu as u32) as PCRE2_UCHAR;
            }
        }
    } else if startline == 0 && ((*re).flags & PCRE2_FIRSTMAPSET) != 0 {
        start_bits = (*re).start_bitmap.as_ptr();
    }

    /* There may also be a "last known required character" set. */

    if ((*re).flags & PCRE2_LASTSET) != 0 {
        has_req_cu = TRUE;
        req_cu2 = (*re).last_codeunit as PCRE2_UCHAR;
        req_cu = req_cu2;
        if ((*re).flags & PCRE2_LASTCASELESS) != 0 {
            req_cu2 = TABLE_GET!(req_cu, (*mb).fcc, req_cu);
            if req_cu > 127 && ucp != 0 && utf == 0 {
                req_cu2 = UCD_OTHERCASE(req_cu as u32) as PCRE2_UCHAR;
            }
        }
    }

    /* ==========================================================================*/

    /* Loop for handling unanchored repeated matching attempts; for anchored regexs
    the loop runs just once. */

    'fragment_restart: loop {
        start_partial = core::ptr::null();
        match_partial = core::ptr::null();
        (*mb).hitend = FALSE;

        memchr_found_first_cu = core::ptr::null();
        memchr_found_first_cu2 = core::ptr::null();

        'bumpalong: loop {
            let mut new_start_match: PCRE2_SPTR = core::ptr::null();

            /* ----------------- Start of match optimizations ---------------- */

            /* There are some optimizations that avoid running the match if a known
            starting point is not found, or if a known later code unit is not present.
            However, there is an option (settable at compile time) that disables these,
            for testing and for ensuring that all callouts do actually occur. */

            if ((*re).optimization_flags & PCRE2_OPTIM_START_OPTIMIZE) != 0 {
                /* If firstline is TRUE, the start of the match is constrained to the first
                line of a multiline string. Temporarily adjust end_subject so that we stop
                the scans for a first code unit at a newline. If the match fails at the
                newline, later code breaks the loop. */

                if firstline != 0 {
                    let mut t: PCRE2_SPTR = start_match;
                    if utf != 0 {
                        while t < end_subject && !(IS_NEWLINE!(t, mb, (*mb).end_subject, utf)) {
                            t = t.add(1);
                            ACROSSCHAR!(t < end_subject, t, t = t.add(1));
                        }
                    } else {
                        while t < end_subject && !(IS_NEWLINE!(t, mb, (*mb).end_subject, utf)) {
                            t = t.add(1);
                        }
                    }
                    end_subject = t;
                }

                /* Anchored: check the first code unit if one is recorded. This may seem
                pointless but it can help in detecting a no match case without scanning for
                the required code unit. */

                if anchored != 0 {
                    if has_first_cu != 0 || !start_bits.is_null() {
                        let mut ok: BOOL = (start_match < end_subject) as BOOL;
                        if ok != 0 {
                            let c: PCRE2_UCHAR = *start_match;
                            ok = (has_first_cu != 0 && (c == first_cu || c == first_cu2)) as BOOL;
                            if ok == 0 && !start_bits.is_null() {
                                ok = ((*start_bits.add((c / 8) as usize) as u32
                                    & (1u32 << (c & 7)))
                                    != 0) as BOOL;
                            }
                        }
                        if ok == 0 {
                            rc = MATCH_NOMATCH;
                            break 'bumpalong;
                        }
                    }
                }
                /* Not anchored. Advance to a unique first code unit if there is one. */
                else {
                    if has_first_cu != 0 {
                        if first_cu != first_cu2
                        /* Caseless */
                        {
                            /* In 8-bit mode, the use of memchr() gives a big speed up, even
                            though we have to call it twice in order to find the earliest
                            occurrence of the code unit in either of its cases. Caching is used
                            to remember the positions of previously found code units. */

                            let mut pp1: PCRE2_SPTR;
                            let mut pp2: PCRE2_SPTR;
                            let searchlength: PCRE2_SIZE =
                                end_subject.offset_from(start_match) as PCRE2_SIZE;

                            /* If we haven't got a previously found position for first_cu, or if
                            the current starting position is later, we need to do a search. If
                            the code unit is not found, set it to the end. */

                            if memchr_found_first_cu.is_null()
                                || start_match > memchr_found_first_cu
                            {
                                pp1 = memchr(
                                    start_match as *const c_void,
                                    first_cu as c_int,
                                    searchlength,
                                ) as PCRE2_SPTR;
                                memchr_found_first_cu = if pp1.is_null() { end_subject } else { pp1 };
                            }
                            /* If the start is before a previously found position, use the
                            previous position, or NULL if a previous search failed. */
                            else {
                                pp1 = if memchr_found_first_cu == end_subject {
                                    core::ptr::null()
                                } else {
                                    memchr_found_first_cu
                                };
                            }

                            /* Do the same thing for the other case. */

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

                            /* Set the start to the end of the subject if neither case was found.
                            Otherwise, use the earlier found point. */

                            if pp1.is_null() {
                                start_match = if pp2.is_null() { end_subject } else { pp2 };
                            } else {
                                start_match = if pp2.is_null() || pp1 < pp2 { pp1 } else { pp2 };
                            }
                        }
                        /* The caseful case is much simpler. */
                        else {
                            start_match = memchr(
                                start_match as *const c_void,
                                first_cu as c_int,
                                end_subject.offset_from(start_match) as PCRE2_SIZE,
                            ) as PCRE2_SPTR;
                            if start_match.is_null() {
                                start_match = end_subject;
                            }
                        }

                        /* If we can't find the required first code unit, having reached the
                        true end of the subject, break the bumpalong loop, to force a match
                        failure, except when doing partial matching, when we let the next cycle
                        run at the end of the subject. */

                        if (*mb).partial == 0 && start_match >= (*mb).end_subject {
                            rc = MATCH_NOMATCH;
                            break 'bumpalong;
                        }
                    }
                    /* If there's no first code unit, advance to just after a linebreak for a
                    multiline match if required. */
                    else if startline != 0 {
                        if start_match > (*mb).start_subject.add(start_offset) {
                            if utf != 0 {
                                while start_match < end_subject
                                    && !(WAS_NEWLINE!(
                                        start_match,
                                        mb,
                                        (*mb).start_subject,
                                        utf
                                    ))
                                {
                                    start_match = start_match.add(1);
                                    ACROSSCHAR!(
                                        start_match < end_subject,
                                        start_match,
                                        start_match = start_match.add(1)
                                    );
                                }
                            } else {
                                while start_match < end_subject
                                    && !(WAS_NEWLINE!(
                                        start_match,
                                        mb,
                                        (*mb).start_subject,
                                        utf
                                    ))
                                {
                                    start_match = start_match.add(1);
                                }
                            }

                            /* If we have just passed a CR and the newline option is ANY or
                            ANYCRLF, and we are now at a LF, advance the match position by one
                            more code unit. */

                            if *start_match.offset(-1) as u32 == CHAR_CR
                                && ((*mb).nltype == NLTYPE_ANY || (*mb).nltype == NLTYPE_ANYCRLF)
                                && start_match < end_subject
                                && *start_match as u32 == CHAR_NL
                            {
                                start_match = start_match.add(1);
                            }
                        }
                    }
                    /* If there's no first code unit or a requirement for a multiline line
                    start, advance to a non-unique first code unit if any have been
                    identified. The bitmap contains only 256 bits. */
                    else if !start_bits.is_null() {
                        while start_match < end_subject {
                            let c: u32 = *start_match as u32;
                            if (*start_bits.add((c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0 {
                                break;
                            }
                            start_match = start_match.add(1);
                        }

                        /* See comment above in first_cu checking about the next few lines. */

                        if (*mb).partial == 0 && start_match >= (*mb).end_subject {
                            rc = MATCH_NOMATCH;
                            break 'bumpalong;
                        }
                    }
                } /* End first code unit handling */

                /* Restore fudged end_subject */

                end_subject = (*mb).end_subject;

                /* The following two optimizations must be disabled for partial matching. */

                if (*mb).partial == 0 {
                    let mut p: PCRE2_SPTR;

                    /* The minimum matching length is a lower bound; no string of that length
                    may actually match the pattern. Although the value is, strictly, in
                    characters, we treat it as code units to avoid spending too much time in
                    this optimization. */

                    if end_subject.offset_from(start_match) < (*re).minlength as isize {
                        rc = MATCH_NOMATCH;
                        break 'bumpalong;
                    }

                    /* If req_cu is set, we know that that code unit must appear in the
                    subject for the (non-partial) match to succeed. If the first code unit is
                    set, req_cu must be later in the subject; otherwise the test starts at
                    the match point. This optimization can save a huge amount of backtracking
                    in patterns with nested unlimited repeats that aren't going to match.

                    The search can be skipped if the code unit was found later than the
                    current starting point in a previous iteration of the bumpalong loop.

                    HOWEVER: when the subject string is very, very long, searching to its end
                    can take a long time, and give bad performance on quite ordinary
                    anchored patterns... so we don't do this when the string is sufficiently
                    long, but it's worth searching a lot more for unanchored patterns. */

                    p = start_match.add(if has_first_cu != 0 { 1 } else { 0 });
                    if has_req_cu != 0 && p > req_cu_ptr {
                        let check_length: PCRE2_SIZE =
                            end_subject.offset_from(start_match) as PCRE2_SIZE;

                        if check_length < REQ_CU_MAX
                            || (anchored == 0 && check_length < REQ_CU_MAX * 1000)
                        {
                            if req_cu != req_cu2
                            /* Caseless */
                            {
                                let pp: PCRE2_SPTR = p;
                                p = memchr(
                                    pp as *const c_void,
                                    req_cu as c_int,
                                    end_subject.offset_from(pp) as PCRE2_SIZE,
                                ) as PCRE2_SPTR;
                                if p.is_null() {
                                    p = memchr(
                                        pp as *const c_void,
                                        req_cu2 as c_int,
                                        end_subject.offset_from(pp) as PCRE2_SIZE,
                                    ) as PCRE2_SPTR;
                                    if p.is_null() {
                                        p = end_subject;
                                    }
                                }
                            }
                            /* The caseful case */
                            else {
                                p = memchr(
                                    p as *const c_void,
                                    req_cu as c_int,
                                    end_subject.offset_from(p) as PCRE2_SIZE,
                                ) as PCRE2_SPTR;
                                if p.is_null() {
                                    p = end_subject;
                                }
                            }

                            /* If we can't find the required code unit, break the bumpalong loop,
                            forcing a match failure. */

                            if p >= end_subject {
                                rc = MATCH_NOMATCH;
                                break 'bumpalong;
                            }

                            /* If we have found the required code unit, save the point where we
                            found it, so that we don't search again next time round the bumpalong
                            loop if the start hasn't yet passed this code unit. */

                            req_cu_ptr = p;
                        }
                    }
                }
            }

            /* ------------ End of start of match optimizations ------------ */

            /* Give no match if we have passed the bumpalong limit. */

            if start_match > bumpalong_limit {
                rc = MATCH_NOMATCH;
                break 'bumpalong;
            }

            /* OK, we can now run the match. If "hitend" is set afterwards, remember the
            first starting point for which a partial match was found. */

            cb.start_match = start_match.offset_from(subject) as PCRE2_SIZE;
            cb.callout_flags |= PCRE2_CALLOUT_STARTMATCH;

            (*mb).start_used_ptr = start_match;
            (*mb).last_used_ptr = start_match;
            (*mb).moptions = options | fragment_options;
            (*mb).match_call_count = 0;
            (*mb).end_offset_top = 0;
            (*mb).skip_arg_count = 0;

            rc = r#match(
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

            'sw: {
                /* If MATCH_SKIP_ARG reaches this level it means that a MARK that matched
                the SKIP's arg was not found. In this circumstance, Perl ignores the SKIP
                entirely. The only way we can do that is to re-do the match at the same
                point, with a flag to force SKIP with an argument to be ignored. */

                if rc == MATCH_SKIP_ARG {
                    new_start_match = start_match;
                    (*mb).ignore_skip_arg = (*mb).skip_arg_count;
                    break 'sw;
                }

                /* SKIP passes back the next starting point explicitly, but if it is no
                greater than the match we have just done, treat it as NOMATCH. */

                if rc == MATCH_SKIP {
                    if (*mb).verb_skip_ptr > start_match {
                        new_start_match = (*mb).verb_skip_ptr;
                        break 'sw;
                    }
                    /* Fall through */
                } else if !(rc == MATCH_NOMATCH || rc == MATCH_PRUNE || rc == MATCH_THEN) {
                    /* COMMIT disables the bumpalong, but otherwise behaves as NOMATCH. */

                    if rc == MATCH_COMMIT {
                        rc = MATCH_NOMATCH;
                        break 'bumpalong; /* goto ENDLOOP */
                    }

                    /* Any other return is either a match, or some kind of error. */

                    break 'bumpalong; /* goto ENDLOOP */
                }

                /* NOMATCH and PRUNE advance by one character. THEN at this level acts
                exactly like PRUNE. Unset ignore SKIP-with-argument. */

                (*mb).ignore_skip_arg = 0;
                new_start_match = start_match.add(1);
                if utf != 0 {
                    ACROSSCHAR!(
                        new_start_match < end_subject,
                        new_start_match,
                        new_start_match = new_start_match.add(1)
                    );
                }
            }

            /* Control reaches here for the various types of "no match at this point"
            result. Reset the code to MATCH_NOMATCH for subsequent checking. */

            rc = MATCH_NOMATCH;

            /* If PCRE2_FIRSTLINE is set, the match must happen before or at the first
            newline in the subject (though it may continue over the newline). Therefore,
            if we have just failed to match, starting at a newline, do not continue. */

            if firstline != 0 && IS_NEWLINE!(start_match, mb, (*mb).end_subject, utf) {
                break 'bumpalong;
            }

            /* Advance to new matching position */

            start_match = new_start_match;

            /* Break the loop if the pattern is anchored or if we have passed the end of
            the subject. */

            if anchored != 0 || start_match > end_subject {
                break 'bumpalong;
            }

            /* If we have just passed a CR and we are now at a LF, and the pattern does
            not contain any explicit matches for \r or \n, and the newline option is CRLF
            or ANY or ANYCRLF, advance the match position by one more code unit. In
            normal matching start_match will aways be greater than the first position at
            this stage, but a failed *SKIP can cause a return at the same point, which is
            why the first test exists. */

            if start_match > subject.add(start_offset)
                && *start_match.offset(-1) as u32 == CHAR_CR
                && start_match < end_subject
                && *start_match as u32 == CHAR_NL
                && ((*re).flags & PCRE2_HASCRORLF) == 0
                && ((*mb).nltype == NLTYPE_ANY
                    || (*mb).nltype == NLTYPE_ANYCRLF
                    || (*mb).nllen == 2)
            {
                start_match = start_match.add(1);
            }

            (*mb).mark = core::ptr::null(); /* Reset for start of next match attempt */
        } /* End of for(;;) "bumpalong" loop */

        /* ==========================================================================*/

        /* When we reach here, one of the following stopping conditions is true:

        (1) The match succeeded, either completely, or partially;

        (2) The pattern is anchored or the match was failed after (*COMMIT);

        (3) We are past the end of the subject or the bumpalong limit;

        (4) PCRE2_FIRSTLINE is set and we have failed to match at a newline, because
            this option requests that a match occur at or before the first newline in
            the subject.

        (5) Some kind of error occurred.
        */

        /* ENDLOOP: */

        /* If end_subject != true_end_subject, it means we are handling invalid UTF,
        and have just processed a non-terminal fragment. If this resulted in no match
        or a partial match we must carry on to the next fragment (a partial match is
        returned to the caller only at the very end of the subject). A loop is used to
        avoid trying to match against empty fragments; if the pattern can match an
        empty string it would have done so already. */

        if utf != 0
            && end_subject != true_end_subject
            && (rc == MATCH_NOMATCH || rc == PCRE2_ERROR_PARTIAL)
        {
            loop {
                /* Advance past the first bad code unit, and then skip invalid character
                starting code units in 8-bit and 16-bit modes. */

                start_match = end_subject.add(1);

                while start_match < true_end_subject && NOT_FIRSTCU!(*start_match) {
                    start_match = start_match.add(1);
                }

                /* If we have hit the end of the subject, there isn't another non-empty
                fragment, so give up. */

                if start_match >= true_end_subject {
                    rc = MATCH_NOMATCH; /* In case it was partial */
                    match_partial = core::ptr::null();
                    break;
                }

                /* Check the rest of the subject */

                (*mb).check_subject = start_match;
                rc = _pcre2_valid_utf_8(
                    start_match,
                    length - (start_match.offset_from(subject) as PCRE2_SIZE),
                    &raw mut (*match_data).startchar,
                );

                /* The rest of the subject is valid UTF. */

                if rc == 0 {
                    end_subject = true_end_subject;
                    (*mb).end_subject = end_subject;
                    fragment_options = PCRE2_NOTBOL;
                    continue 'fragment_restart;
                }
                /* A subsequent UTF error has been found; if the next fragment is
                non-empty, set up to process it. Otherwise, let the loop advance. */
                else if rc < 0 {
                    end_subject = start_match.add((*match_data).startchar);
                    (*mb).end_subject = end_subject;
                    if end_subject > start_match {
                        fragment_options = PCRE2_NOTBOL | PCRE2_NOTEOL;
                        continue 'fragment_restart;
                    }
                }
            }
        }

        break;
    }

    /* Fill in fields that are always returned in the match data. */

    (*match_data).code = re;
    (*match_data).mark = (*mb).mark;
    (*match_data).matchedby = PCRE2_MATCHEDBY_INTERPRETER as u8;
    (*match_data).options = original_options;

    /* Handle a fully successful match. Set the return code to the number of
    captured strings, or 0 if there were too many to fit into the ovector, and then
    set the remaining returned values before returning. Make a copy of the subject
    string if requested. */

    if rc == MATCH_MATCH {
        (*match_data).rc = if ((*mb).end_offset_top as c_int) >= 2 * (*match_data).oveccount as c_int
        {
            0
        } else {
            ((*mb).end_offset_top as c_int) / 2 + 1
        };
        (*match_data).subject_length = length;
        (*match_data).start_offset = start_offset;
        (*match_data).startchar = start_match.offset_from(subject) as PCRE2_SIZE;
        (*match_data).leftchar = (*mb).start_used_ptr.offset_from(subject) as PCRE2_SIZE;
        (*match_data).rightchar = (if (*mb).last_used_ptr > (*mb).end_match_ptr {
            (*mb).last_used_ptr
        } else {
            (*mb).end_match_ptr
        })
        .offset_from(subject) as PCRE2_SIZE;
        if (options & PCRE2_COPY_MATCHED_SUBJECT) != 0 {
            if length != 0 {
                (*match_data).subject = ((*match_data).memctl.malloc.unwrap())(
                    CU2BYTES!(length),
                    (*match_data).memctl.memory_data,
                ) as PCRE2_SPTR;
                if (*match_data).subject.is_null() {
                    RETURN_RC!(PCRE2_ERROR_NOMEMORY);
                }
                memcpy(
                    (*match_data).subject as *mut c_void,
                    subject as *const c_void,
                    CU2BYTES!(length),
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

    /* Control gets here if there has been a partial match, an error, or if the
    overall match attempt has failed at all permitted starting positions. Any mark
    data is in the nomatch_mark field. */

    (*match_data).mark = (*mb).nomatch_mark;

    /* For anything other than nomatch or partial match, just return the code. */

    if rc != MATCH_NOMATCH && rc != PCRE2_ERROR_PARTIAL {
        (*match_data).rc = rc;
    }
    /* Handle a partial match. If a "soft" partial match was requested, searching
    for a complete match will have continued, and the value of rc at this point
    will be MATCH_NOMATCH. For a "hard" partial match, it will already be
    PCRE2_ERROR_PARTIAL. */
    else if !match_partial.is_null() {
        (*match_data).subject = original_subject;
        (*match_data).subject_length = length;
        (*match_data).start_offset = start_offset;
        *(*match_data).ovector.as_mut_ptr().add(0) =
            match_partial.offset_from(subject) as PCRE2_SIZE;
        *(*match_data).ovector.as_mut_ptr().add(1) = end_subject.offset_from(subject) as PCRE2_SIZE;
        (*match_data).startchar = match_partial.offset_from(subject) as PCRE2_SIZE;
        (*match_data).leftchar = start_partial.offset_from(subject) as PCRE2_SIZE;
        (*match_data).rightchar = end_subject.offset_from(subject) as PCRE2_SIZE;
        (*match_data).rc = PCRE2_ERROR_PARTIAL;
    }
    /* Else this is the classic nomatch case. */
    else {
        (*match_data).subject = original_subject;
        (*match_data).subject_length = length;
        (*match_data).start_offset = start_offset;
        (*match_data).rc = PCRE2_ERROR_NOMATCH;
    }

    (*match_data).rc
}
