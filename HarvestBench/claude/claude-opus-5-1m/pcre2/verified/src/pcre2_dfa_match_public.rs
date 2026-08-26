/* This function matches a compiled pattern against a subject string, using an
alternate matching algorithm that finds all matches at once.

Arguments:
  code          points to the compiled pattern
  subject       subject string
  length        length of subject string
  startoffset   where to start matching in the subject
  options       option bits
  match_data    points to a match data structure
  gcontext      points to a match context
  workspace     pointer to workspace
  wscount       size of workspace

Returns:        > 0 => number of match offset pairs placed in offsets
                = 0 => offsets overflowed; longest matches are present
                 -1 => failed to match
               < -1 => some kind of unexpected problem
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_dfa_match_8(
    code: *const pcre2_real_code,
    subject: PCRE2_SPTR,
    length: PCRE2_SIZE,
    start_offset: PCRE2_SIZE,
    options: u32,
    match_data: *mut pcre2_real_match_data,
    mcontext: *mut pcre2_real_match_context,
    workspace: *mut c_int,
    wscount: PCRE2_SIZE,
) -> c_int {
    let mut subject: PCRE2_SPTR = subject;
    let mut length: PCRE2_SIZE = length;
    let mut options: u32 = options;

    let mut rc: c_int = 0;

    let re: *const pcre2_real_code = code;
    let original_options: u32 = options;

    let null_str: [PCRE2_UCHAR; 1] = [0xcd];
    let original_subject: PCRE2_SPTR = subject;
    let mut start_match: PCRE2_SPTR = core::ptr::null();
    let mut end_subject: PCRE2_SPTR = core::ptr::null();
    let mut bumpalong_limit: PCRE2_SPTR = core::ptr::null();
    let mut req_cu_ptr: PCRE2_SPTR = core::ptr::null();

    let mut utf: BOOL = FALSE;
    let mut anchored: BOOL = FALSE;
    let mut startline: BOOL = FALSE;
    let mut firstline: BOOL = FALSE;
    let mut has_first_cu: BOOL = FALSE;
    let mut has_req_cu: BOOL = FALSE;

    let mut memchr_found_first_cu: PCRE2_SPTR = core::ptr::null();
    let mut memchr_found_first_cu2: PCRE2_SPTR = core::ptr::null();

    let mut first_cu: PCRE2_UCHAR = 0;
    let mut first_cu2: PCRE2_UCHAR = 0;
    let mut req_cu: PCRE2_UCHAR = 0;
    let mut req_cu2: PCRE2_UCHAR = 0;

    let mut start_bits: *const u8 = core::ptr::null();

    /* We need to have mb pointing to a match block, because the IS_NEWLINE macro
    is used below, and it expects NLBLOCK to be defined as a pointer. */

    let mut cb: pcre2_callout_block = core::mem::zeroed();
    let mut actual_match_block: dfa_match_block = core::mem::zeroed();
    let mb: *mut dfa_match_block = &mut actual_match_block;

    /* Set up a starting block of memory for use during recursive calls to
    internal_dfa_match(). By putting this on the stack, it minimizes resource use
    in the case when it is not needed. If this is too small, more memory is
    obtained from the heap. At the start of each block is an anchor structure.*/

    let mut base_recursion_workspace: [c_int; RWS_BASE_SIZE] = core::mem::zeroed();
    let rws: *mut RWS_anchor = base_recursion_workspace.as_mut_ptr() as *mut RWS_anchor;
    (*rws).next = core::ptr::null_mut();
    (*rws).size = RWS_BASE_SIZE as u32;
    (*rws).free = (RWS_BASE_SIZE - RWS_ANCHOR_SIZE) as u32;

    /* Recognize NULL, length 0 as an empty string. */

    if subject.is_null() && length == 0 {
        subject = null_str.as_ptr();
    }

    /* Plausibility checks */

    if match_data.is_null() {
        return PCRE2_ERROR_NULL;
    }

    'exit: {
        if re.is_null() || subject.is_null() || workspace.is_null() {
            rc = PCRE2_ERROR_NULL;
            break 'exit;
        }
        if (options & !PUBLIC_DFA_MATCH_OPTIONS) != 0 {
            rc = PCRE2_ERROR_BADOPTION;
            break 'exit;
        }

        if length == PCRE2_ZERO_TERMINATED {
            length = _pcre2_strlen_8(subject);
        }

        if wscount < 20 {
            rc = PCRE2_ERROR_DFA_WSSIZE;
            break 'exit;
        }
        if start_offset > length {
            rc = PCRE2_ERROR_BADOFFSET;
            break 'exit;
        }

        /* Partial matching and PCRE2_ENDANCHORED are currently not allowed at the
        same time. */

        if (options & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) != 0
            && (((*re).overall_options | options) & PCRE2_ENDANCHORED) != 0
        {
            rc = PCRE2_ERROR_BADOPTION;
            break 'exit;
        }

        /* Invalid UTF support is not available for DFA matching. */

        if ((*re).overall_options & PCRE2_MATCH_INVALID_UTF) != 0 {
            rc = PCRE2_ERROR_DFA_UINVALID_UTF;
            break 'exit;
        }

        /* Check that the first field in the block is the magic number. If it is
        not, return with PCRE2_ERROR_BADMAGIC. */

        if (*re).magic_number != MAGIC_NUMBER {
            rc = PCRE2_ERROR_BADMAGIC;
            break 'exit;
        }

        /* Check the code unit width. */

        if ((*re).flags & PCRE2_MODE_MASK) != 1
        /* PCRE2_CODE_UNIT_WIDTH/8 */
        {
            rc = PCRE2_ERROR_BADMODE;
            break 'exit;
        }

        /* PCRE2_NOTEMPTY and PCRE2_NOTEMPTY_ATSTART are match-time flags in the
        options variable for this function. Users of PCRE2 who are not calling the
        function directly would like to have a way of setting these flags, in the
        same way that they can set pcre2_compile() flags like
        PCRE2_NO_AUTO_POSSESS with constructions like (*NO_AUTOPOSSESS). To enable
        this, (*NOTEMPTY) and (*NOTEMPTY_ATSTART) set bits in the pattern's "flag"
        function which can now be transferred to the options for this function. The
        bits are guaranteed to be adjacent, but do not have the same values. This
        bit of Boolean trickery assumes that the match-time bits are not more
        significant than the flag bits. */

        {
            const FF: u32 = PCRE2_NOTEMPTY_SET | PCRE2_NE_ATST_SET;
            const OO: u32 = PCRE2_NOTEMPTY | PCRE2_NOTEMPTY_ATSTART;
            options |= ((*re).flags & FF) / ((FF & (!FF + 1)) / (OO & (!OO + 1)));
        }

        /* If restarting after a partial match, do some sanity checks on the
        contents of the workspace. */

        if (options & PCRE2_DFA_RESTART) != 0 {
            if (*workspace & (-2i32)) != 0
                || *workspace.add(1) < 1
                || *workspace.add(1) > ((wscount - 2) / INTS_PER_STATEBLOCK as usize) as c_int
            {
                rc = PCRE2_ERROR_DFA_BADRESTART;
                break 'exit;
            }
        }

        /* Set some local values */

        utf = (((*re).overall_options & PCRE2_UTF) != 0) as BOOL;
        start_match = subject.add(start_offset);
        end_subject = subject.add(length);
        req_cu_ptr = start_match.wrapping_offset(-1);
        anchored = ((options & (PCRE2_ANCHORED | PCRE2_DFA_RESTART)) != 0
            || ((*re).overall_options & PCRE2_ANCHORED) != 0) as BOOL;

        /* The "must be at the start of a line" flags are used in a loop when
        finding where to start. */

        startline = (((*re).flags & PCRE2_STARTLINE) != 0) as BOOL;
        firstline = (anchored == 0 && ((*re).overall_options & PCRE2_FIRSTLINE) != 0) as BOOL;
        bumpalong_limit = end_subject;

        /* Initialize and set up the fixed fields in the callout block, with a
        pointer in the match block. */

        (*mb).cb = &mut cb;
        cb.version = 2;
        cb.subject = subject;
        cb.subject_length = end_subject.offset_from(subject) as PCRE2_SIZE;
        cb.callout_flags = 0;
        cb.capture_top = 1; /* No capture support */
        cb.capture_last = 0;
        cb.mark = core::ptr::null(); /* No (*MARK) support */

        /* Get data from the match context, if present, and fill in the remaining
        fields in the match block. It is an error to set an offset limit without
        setting the flag at compile time. */

        if mcontext.is_null() {
            (*mb).callout = None;
            (*mb).memctl = (*re).memctl;
            (*mb).match_limit = _pcre2_default_match_context_8.match_limit;
            (*mb).match_limit_depth = _pcre2_default_match_context_8.depth_limit;
            (*mb).heap_limit = _pcre2_default_match_context_8.heap_limit;
        } else {
            if (*mcontext).offset_limit != PCRE2_UNSET {
                if ((*re).overall_options & PCRE2_USE_OFFSET_LIMIT) == 0 {
                    rc = PCRE2_ERROR_BADOFFSETLIMIT;
                    break 'exit;
                }
                bumpalong_limit = subject.add((*mcontext).offset_limit);
            }
            (*mb).callout = (*mcontext).callout;
            (*mb).callout_data = (*mcontext).callout_data;
            (*mb).memctl = (*mcontext).memctl;
            (*mb).match_limit = (*mcontext).match_limit;
            (*mb).match_limit_depth = (*mcontext).depth_limit;
            (*mb).heap_limit = (*mcontext).heap_limit;
        }

        if (*mb).match_limit > (*re).limit_match {
            (*mb).match_limit = (*re).limit_match;
        }

        if (*mb).match_limit_depth > (*re).limit_depth {
            (*mb).match_limit_depth = (*re).limit_depth;
        }

        if (*mb).heap_limit > (*re).limit_heap {
            (*mb).heap_limit = (*re).limit_heap;
        }

        (*mb).start_code = (re as *const u8).add((*re).code_start) as PCRE2_SPTR;
        (*mb).tables = (*re).tables;
        (*mb).start_subject = subject;
        (*mb).end_subject = end_subject;
        (*mb).start_offset = start_offset;
        (*mb).allowemptypartial =
            ((*re).max_lookbehind > 0 || ((*re).flags & PCRE2_MATCH_EMPTY) != 0) as BOOL;
        (*mb).moptions = options;
        (*mb).poptions = (*re).overall_options;
        (*mb).match_call_count = 0;
        (*mb).heap_used = 0;

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
                rc = PCRE2_ERROR_INTERNAL;
                break 'exit;
            }
        }

        /* Check a UTF string for validity if required. For 8-bit and 16-bit
        strings, we must also check that a starting offset does not point into the
        middle of a multiunit character. We check only the portion of the subject
        that is going to be inspected during matching - from the offset minus the
        maximum back reference to the given length. This saves time when a small
        part of a large subject is being matched by the use of a starting offset.
        Note that the maximum lookbehind is a number of characters, not code
        units. */

        if utf != 0 && (options & PCRE2_NO_UTF_CHECK) == 0 {
            let mut check_subject: PCRE2_SPTR = start_match; /* start_match includes offset */

            if start_offset > 0 {
                let mut i: c_uint;
                if start_match < end_subject && NOT_FIRSTCU!(*start_match) {
                    rc = PCRE2_ERROR_BADUTFOFFSET;
                    break 'exit;
                }
                i = (*re).max_lookbehind as c_uint;
                while i > 0 && check_subject > subject {
                    check_subject = check_subject.sub(1);
                    while check_subject > subject && (*check_subject & 0xc0) == 0x80 {
                        check_subject = check_subject.sub(1);
                    }
                    i -= 1;
                }
            }

            /* Validate the relevant portion of the subject. After an error, adjust
            the offset to be an absolute offset in the whole string. */

            rc = _pcre2_valid_utf_8(
                check_subject,
                length - check_subject.offset_from(subject) as PCRE2_SIZE,
                &mut (*match_data).startchar,
            );
            if rc != 0 {
                (*match_data).startchar += check_subject.offset_from(subject) as PCRE2_SIZE;
                break 'exit;
            }
        }

        /* Set up the first code unit to match, if available. If there's no first
        code unit there may be a bitmap of possible first characters. */

        if ((*re).flags & PCRE2_FIRSTSET) != 0 {
            has_first_cu = TRUE;
            first_cu = (*re).first_codeunit as PCRE2_UCHAR;
            first_cu2 = first_cu;
            if ((*re).flags & PCRE2_FIRSTCASELESS) != 0 {
                first_cu2 = TABLE_GET!(first_cu, (*mb).tables.add(fcc_offset), first_cu);
                if first_cu > 127 && utf == 0 && ((*re).overall_options & PCRE2_UCP) != 0 {
                    first_cu2 = UCD_OTHERCASE(first_cu as u32) as PCRE2_UCHAR;
                }
            }
        } else if startline == 0 && ((*re).flags & PCRE2_FIRSTMAPSET) != 0 {
            start_bits = (*re).start_bitmap.as_ptr();
        }

        /* There may be a "last known required code unit" set. */

        if ((*re).flags & PCRE2_LASTSET) != 0 {
            has_req_cu = TRUE;
            req_cu = (*re).last_codeunit as PCRE2_UCHAR;
            req_cu2 = req_cu;
            if ((*re).flags & PCRE2_LASTCASELESS) != 0 {
                req_cu2 = TABLE_GET!(req_cu, (*mb).tables.add(fcc_offset), req_cu);
                if req_cu > 127 && utf == 0 && ((*re).overall_options & PCRE2_UCP) != 0 {
                    req_cu2 = UCD_OTHERCASE(req_cu as u32) as PCRE2_UCHAR;
                }
            }
        }

        /* If the match data block was previously used with
        PCRE2_COPY_MATCHED_SUBJECT, free the memory that was obtained. */

        if ((*match_data).flags as u32 & PCRE2_MD_COPIED_SUBJECT) != 0 {
            ((*match_data).memctl.free.unwrap())(
                (*match_data).subject as *mut c_void,
                (*match_data).memctl.memory_data,
            );
            (*match_data).flags &= !(PCRE2_MD_COPIED_SUBJECT as u8);
        }

        /* Fill in fields that are always returned in the match data. */

        (*match_data).code = re;
        (*match_data).subject = core::ptr::null(); /* Default for match error */
        (*match_data).mark = core::ptr::null();
        (*match_data).matchedby = PCRE2_MATCHEDBY_DFA_INTERPRETER as u8;
        (*match_data).options = original_options;

        /* Call the main matching function, looping for a non-anchored regex after a
        failed match. If not restarting, perform certain optimizations at the start
        of a match. */

        'bumpalong: loop {
            /* ----------------- Start of match optimizations ---------------- */

            /* There are some optimizations that avoid running the match if a known
            starting point is not found, or if a known later code unit is not
            present. However, there is an option (settable at compile time) that
            disables these, for testing and for ensuring that all callouts do
            actually occur. The optimizations must also be avoided when restarting a
            DFA match. */

            if ((*re).optimization_flags & PCRE2_OPTIM_START_OPTIMIZE) != 0
                && (options & PCRE2_DFA_RESTART) == 0
            {
                /* If firstline is TRUE, the start of the match is constrained to the
                first line of a multiline string. That is, the match must be before or
                at the first newline following the start of matching. Temporarily
                adjust end_subject so that we stop the optimization scans for a first
                code unit immediately after the first character of a newline (the first
                code unit can legitimately be a newline). If the match fails at the
                newline, later code breaks this loop. */

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

                /* Anchored: check the first code unit if one is recorded. This may
                seem pointless but it can help in detecting a no match case without
                scanning for the required code unit. */

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
                            break 'bumpalong;
                        }
                    }
                }
                /* Not anchored. Advance to a unique first code unit if there is
                one. */
                else {
                    if has_first_cu != 0 {
                        if first_cu != first_cu2
                        /* Caseless */
                        {
                            /* In 8-bit mode, the use of memchr() gives a big speed up,
                            even though we have to call it twice in order to find the
                            earliest occurrence of the code unit in either of its cases.
                            Caching is used to remember the positions of previously found
                            code units. This can make a huge difference when the strings
                            are very long and only one case is actually present. */

                            let mut pp1: PCRE2_SPTR = core::ptr::null();
                            let mut pp2: PCRE2_SPTR = core::ptr::null();
                            let searchlength: PCRE2_SIZE =
                                end_subject.offset_from(start_match) as PCRE2_SIZE;

                            /* If we haven't got a previously found position for first_cu,
                            or if the current starting position is later, we need to do a
                            search. If the code unit is not found, set it to the end. */

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

                            /* Set the start to the end of the subject if neither case was
                            found. Otherwise, use the earlier found point. */

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

                        /* If we can't find the required code unit, having reached the
                        true end of the subject, break the bumpalong loop, to force a
                        match failure, except when doing partial matching, when we let
                        the next cycle run at the end of the subject. To see why,
                        consider the pattern /(?<=abc)def/, which partially matches
                        "abc", even though the string does not contain the starting
                        character "d". If we have not reached the true end of the subject
                        (PCRE2_FIRSTLINE caused end_subject to be temporarily modified)
                        we also let the cycle run, because the matching string is
                        legitimately allowed to start with the first code unit of a
                        newline. */

                        if ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) == 0
                            && start_match >= (*mb).end_subject
                        {
                            break 'bumpalong;
                        }
                    }
                    /* If there's no first code unit, advance to just after a linebreak
                    for a multiline match if required. */
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
                            ANYCRLF, and we are now at a LF, advance the match position by
                            one more code unit. */

                            if *start_match.offset(-1) as u32 == CHAR_CR
                                && ((*mb).nltype == NLTYPE_ANY || (*mb).nltype == NLTYPE_ANYCRLF)
                                && start_match < end_subject
                                && *start_match as u32 == CHAR_NL
                            {
                                start_match = start_match.add(1);
                            }
                        }
                    }
                    /* If there's no first code unit or a requirement for a multiline
                    line start, advance to a non-unique first code unit if any have been
                    identified. The bitmap contains only 256 bits. */
                    else if !start_bits.is_null() {
                        while start_match < end_subject {
                            let c: u32 = *start_match as u32;
                            if (*start_bits.add((c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0 {
                                break;
                            }
                            start_match = start_match.add(1);
                        }

                        /* See comment above in first_cu checking about the next line. */

                        if ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) == 0
                            && start_match >= (*mb).end_subject
                        {
                            break 'bumpalong;
                        }
                    }
                } /* End of first code unit handling */

                /* Restore fudged end_subject */

                end_subject = (*mb).end_subject;

                /* The following two optimizations are disabled for partial
                matching. */

                if ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) == 0 {
                    let mut p: PCRE2_SPTR;

                    /* The minimum matching length is a lower bound; no actual string of
                    that length may actually match the pattern. Although the value is,
                    strictly, in characters, we treat it as code units to avoid spending
                    too much time in this optimization. */

                    if end_subject.offset_from(start_match) < (*re).minlength as isize {
                        break 'bumpalong; /* goto NOMATCH_EXIT */
                    }

                    /* If req_cu is set, we know that that code unit must appear in the
                    subject for the match to succeed. If the first code unit is set,
                    req_cu must be later in the subject; otherwise the test starts at the
                    match point. This optimization can save a huge amount of backtracking
                    in patterns with nested unlimited repeats that aren't going to match.
                    Writing separate code for cased/caseless versions makes it go faster,
                    as does using an autoincrement and backing off on a match. As in the
                    case of the first code unit, using memchr() in the 8-bit library gives
                    a big speed up. Unlike the first_cu check above, we do not need to
                    call memchr() twice in the caseless case because we only need to check
                    for the presence of the character in either case, not find the first
                    occurrence.

                    The search can be skipped if the code unit was found later than the
                    current starting point in a previous iteration of the bumpalong loop.

                    HOWEVER: when the subject string is very, very long, searching to its
                    end can take a long time, and give bad performance on quite ordinary
                    patterns. This showed up when somebody was matching something like
                    /^\d+C/ on a 32-megabyte string... so we don't do this when the string
                    is sufficiently long, but it's worth searching a lot more for
                    unanchored patterns. */

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

                            /* If we can't find the required code unit, break the matching
                            loop, forcing a match failure. */

                            if p >= end_subject {
                                break 'bumpalong;
                            }

                            /* If we have found the required code unit, save the point where
                            we found it, so that we don't search again next time round the
                            loop if the start hasn't passed this code unit yet. */

                            req_cu_ptr = p;
                        }
                    }
                }
            }

            /* ------------ End of start of match optimizations ------------ */

            /* Give no match if we have passed the bumpalong limit. */

            if start_match > bumpalong_limit {
                break 'bumpalong;
            }

            /* OK, now we can do the business */

            (*mb).start_used_ptr = start_match;
            (*mb).last_used_ptr = start_match;
            (*mb).recursive = core::ptr::null_mut();

            rc = internal_dfa_match(
                mb,                                    /* fixed match data */
                (*mb).start_code,                      /* this subexpression's code */
                start_match,                           /* where we currently are */
                start_offset,                          /* start offset in subject */
                (*match_data).ovector.as_mut_ptr(),    /* offset vector */
                (*match_data).oveccount as u32 * 2,    /* actual size of same */
                workspace,                             /* workspace vector */
                wscount as c_int,                      /* size of same */
                0,                                     /* function recurse level */
                base_recursion_workspace.as_mut_ptr(), /* initial workspace for recursion */
            );

            /* Anything other than "no match" means we are done, always; otherwise,
            carry on only if not anchored. */

            if rc != PCRE2_ERROR_NOMATCH || anchored != 0 {
                if rc == PCRE2_ERROR_NOMATCH {
                    break 'bumpalong; /* goto NOMATCH_EXIT */
                }

                if rc == PCRE2_ERROR_PARTIAL && (*match_data).oveccount > 0 {
                    *(*match_data).ovector.as_mut_ptr().add(0) =
                        start_match.offset_from(subject) as PCRE2_SIZE;
                    *(*match_data).ovector.as_mut_ptr().add(1) =
                        end_subject.offset_from(subject) as PCRE2_SIZE;
                }

                if rc >= 0 || rc == PCRE2_ERROR_PARTIAL {
                    (*match_data).subject_length = length;
                    (*match_data).start_offset = start_offset;
                    (*match_data).leftchar =
                        (*mb).start_used_ptr.offset_from(subject) as PCRE2_SIZE;
                    (*match_data).rightchar =
                        (*mb).last_used_ptr.offset_from(subject) as PCRE2_SIZE;
                    (*match_data).startchar = start_match.offset_from(subject) as PCRE2_SIZE;
                }

                if rc >= 0 && (options & PCRE2_COPY_MATCHED_SUBJECT) != 0 {
                    if length != 0 {
                        (*match_data).subject = ((*match_data).memctl.malloc.unwrap())(
                            CU2BYTES!(length),
                            (*match_data).memctl.memory_data,
                        ) as PCRE2_SPTR;
                        if (*match_data).subject.is_null() {
                            rc = PCRE2_ERROR_NOMEMORY;
                            break 'exit;
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
                } else if rc >= 0 || rc == PCRE2_ERROR_PARTIAL {
                    (*match_data).subject = original_subject;
                }
                break 'exit;
            }

            /* Advance to the next subject character unless we are at the end of a
            line and firstline is set. */

            if firstline != 0 && (IS_NEWLINE!(start_match, mb, (*mb).end_subject, utf)) {
                break 'bumpalong;
            }
            start_match = start_match.add(1);
            if utf != 0 {
                ACROSSCHAR!(
                    start_match < end_subject,
                    start_match,
                    start_match = start_match.add(1)
                );
            }
            if start_match > end_subject {
                break 'bumpalong;
            }

            /* If we have just passed a CR and we are now at a LF, and the pattern
            does not contain any explicit matches for \r or \n, and the newline
            option is CRLF or ANY or ANYCRLF, advance the match position by one more
            character. */

            if *start_match.offset(-1) as u32 == CHAR_CR
                && start_match < end_subject
                && *start_match as u32 == CHAR_NL
                && ((*re).flags & PCRE2_HASCRORLF) == 0
                && ((*mb).nltype == NLTYPE_ANY
                    || (*mb).nltype == NLTYPE_ANYCRLF
                    || (*mb).nllen == 2)
            {
                start_match = start_match.add(1);
            }
        } /* "Bumpalong" loop */

        /* NOMATCH_EXIT: */
        (*match_data).subject = original_subject;
        (*match_data).subject_length = length;
        (*match_data).start_offset = start_offset;
        rc = PCRE2_ERROR_NOMATCH;
    }

    /* EXIT: */
    while !(*rws).next.is_null() {
        let next: *mut RWS_anchor = (*rws).next;
        (*rws).next = (*next).next;
        ((*mb).memctl.free.unwrap())(next as *mut c_void, (*mb).memctl.memory_data);
    }

    (*match_data).rc = rc;
    rc
}
