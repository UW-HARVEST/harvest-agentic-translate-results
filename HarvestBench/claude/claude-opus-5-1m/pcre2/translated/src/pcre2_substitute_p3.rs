/* Translated from c_src/src/pcre2_substitute.c lines 742-1795 */

/*************************************************
*              Match and substitute              *
*************************************************/

/* This function applies a compiled re to a subject string and creates a new
string with substitutions. The first 7 arguments are the same as for
pcre2_match(). Either string length may be PCRE2_ZERO_TERMINATED.

Arguments:
  code            points to the compiled expression
  subject         points to the subject string
  length          length of subject string (may contain binary zeros)
  start_offset    where to start in the subject string
  options         option bits
  match_data      points to a match_data block, or is NULL
  context         points a PCRE2 context
  replacement     points to the replacement string
  rlength         length of replacement string
  buffer          where to put the substituted string
  blength         points to length of buffer; updated to length of string

Returns:          >= 0 number of substitutions made
                  < 0 an error code
                  PCRE2_ERROR_BADREPLACEMENT means invalid use of $
*/

/* The C source uses the macros CHECKMEMCPY(), CHECKCASECPY_DEFAULT(),
CHECKCASECPY_CALLOUT() and DELAYEDFORCECASE() (defined just above the function
in pcre2_substitute.c). They read and update a whole set of the function's local
variables and they contain "goto"s into the function's error handlers, so they
are reproduced here as macro_rules! macros defined *inside* the function body,
after the locals and inside the labelled blocks that stand for the C labels.

Here's the function */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substitute_8(
    code: *const pcre2_real_code,
    mut subject: PCRE2_SPTR,
    mut length: PCRE2_SIZE,
    mut start_offset: PCRE2_SIZE,
    mut options: u32,
    mut match_data: *mut pcre2_real_match_data,
    mcontext: *mut pcre2_real_match_context,
    mut replacement: PCRE2_SPTR,
    mut rlength: PCRE2_SIZE,
    buffer: *mut PCRE2_UCHAR,
    blength: *mut PCRE2_SIZE,
) -> c_int {
    let mut rc: c_int = 0;
    let mut subs: c_int;
    let mut ovector_count: u32;
    let mut goptions: u32 = 0;
    let mut suboptions: u32 = 0;
    let mut internal_match_data: *mut pcre2_real_match_data = std::ptr::null_mut();
    let mut escaped_literal: BOOL = FALSE;
    let mut overflowed: BOOL = FALSE;
    let mut use_existing_match: BOOL;
    let mut replacement_only: BOOL;
    let utf: BOOL = if ((*code).overall_options & PCRE2_UTF) != 0 {
        TRUE
    } else {
        FALSE
    };
    let partial: BOOL = if (options & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) != 0 {
        TRUE
    } else {
        FALSE
    };
    let mut temp: [PCRE2_UCHAR; 6] = [0; 6];
    let null_str: [PCRE2_UCHAR; 1] = [0xcd];
    let original_subject: PCRE2_SPTR = subject;
    let mut ptr: PCRE2_SPTR = std::ptr::null();
    let mut repend: PCRE2_SPTR = std::ptr::null();
    let mut extra_needed: PCRE2_SIZE = 0;
    let mut buff_offset: PCRE2_SIZE;
    let buff_length: PCRE2_SIZE;
    let mut lengthleft: PCRE2_SIZE;
    let mut fraglength: PCRE2_SIZE;
    let mut ovector: *mut PCRE2_SIZE;
    let mut ovecsave: [PCRE2_SIZE; 2] = [0, 0];
    let mut scb: pcre2_substitute_callout_block = pcre2_substitute_callout_block {
        version: 0,
        input: std::ptr::null(),
        output: std::ptr::null(),
        output_offsets: [0, 0],
        ovector: std::ptr::null_mut(),
        oveccount: 0,
        subscount: 0,
    };
    let mut sub_start_extra_needed: PCRE2_SIZE;
    let mut substitute_case_callout: pcre2_substitute_case_callout_fn = None;
    let mut substitute_case_callout_data: *mut c_void = std::ptr::null_mut();

    /* The case-forcing state is declared inside the global loop in C; it is
    hoisted out here (and re-initialized at the top of each iteration of the
    global loop, exactly as in C) so that the macros below can refer to it. */

    let mut forcecase: case_state;
    let mut casestart_offset: PCRE2_SIZE;
    let mut casestart_extra_needed: PCRE2_SIZE;

    /* General initialization */

    buff_offset = 0;
    buff_length = *blength;
    lengthleft = buff_length;
    *blength = PCRE2_UNSET;

    if !mcontext.is_null() {
        substitute_case_callout = (*mcontext).substitute_case_callout;
        substitute_case_callout_data = (*mcontext).substitute_case_callout_data;
    }

    /* Partial matching is supported, with limitations. We allow matching in partial
    mode, however, if a partial match is found, the substitution will fail with a
    PCRE2_ERROR_PARTIAL error. Additionally, outputting the after-match text is not
    allowed (PCRE2_ERROR_BADOPTION), and certain replacement items such as $' and $_
    are not supported (PCRE2_ERROR_PARTIALSUBS).

    This must come after setting *blength to PCRE2_UNSET, so as not to imply an
    offset in the replacement. */

    if partial != 0 && (options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY) == 0 {
        return PCRE2_ERROR_BADOPTION;
    }

    /* Validate length and find the end of the replacement. A NULL replacement of
    zero length is interpreted as an empty string. */

    if replacement.is_null() {
        if rlength != 0 {
            return PCRE2_ERROR_NULL;
        }
        replacement = null_str.as_ptr();
    }

    if rlength == PCRE2_ZERO_TERMINATED {
        rlength = _pcre2_strlen_8(replacement);
    }
    repend = replacement.add(rlength);

    /* A NULL subject of zero length is treated as an empty string. */

    if subject.is_null() {
        if length != 0 {
            return PCRE2_ERROR_NULL;
        }
        subject = null_str.as_ptr();
    }

    if length == PCRE2_ZERO_TERMINATED {
        length = _pcre2_strlen_8(subject);
    }

    /* Check for using a match that has already happened. Note that the subject
    pointer in the match data may be NULL after a no-match. */

    use_existing_match = if (options & PCRE2_SUBSTITUTE_MATCHED) != 0 {
        TRUE
    } else {
        FALSE
    };
    replacement_only = if (options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY) != 0 {
        TRUE
    } else {
        FALSE
    };

    if use_existing_match != 0 && match_data.is_null() {
        return PCRE2_ERROR_NULL;
    }

    /* If an existing match is being passed in, we should check that it matches
    the passed-in subject pointer, length, and match options. We don't currently
    have a use-case for someone to match on one subject, then try and use that
    match data on a different subject. In a UTF-encoded string, a simple change
    like replacing one character for another won't preserve the code unit offsets,
    so it's hard to see, in the general case, how it would be safe or useful to
    support swapping or mutating the subject string.

    Similarly, using different match options between the first (external) and
    subsequent (internal, global) matches is hard to justify. */

    if use_existing_match != 0 {
        /* Return early, as the rest of the match_data may not have been
        initialised. This duplicates and must be in sync with the check below that
        aborts substitution on any result other than success or no-match. */
        if (*match_data).rc < 0 && (*match_data).rc != PCRE2_ERROR_NOMATCH {
            return (*match_data).rc;
        }

        /* Not supported if the passed-in match was from the DFA interpreter. */
        if (*match_data).matchedby as u32 == PCRE2_MATCHEDBY_DFA_INTERPRETER {
            return PCRE2_ERROR_DFA_UFUNC;
        }

        if code != (*match_data).code {
            return PCRE2_ERROR_DIFFSUBSPATTERN;
        }

        /* We want the passed-in subject strings to match. This implies the effective
        length must match, and either: the pointers are equal (with strict matching
        of NULL against NULL); or, the special case of PCRE2_COPY_MATCHED_SUBJECT
        where we cannot compare pointers but we can verify the contents. */
        if length != (*match_data).subject_length
            || !(original_subject == (*match_data).subject
                || (((*match_data).flags as u32 & PCRE2_MD_COPIED_SUBJECT) != 0
                    && (length == 0
                        || memcmp(
                            subject as *const c_void,
                            (*match_data).subject as *const c_void,
                            CU2BYTES!(length),
                        ) == 0)))
        {
            return PCRE2_ERROR_DIFFSUBSSUBJECT;
        }

        if start_offset != (*match_data).start_offset {
            return PCRE2_ERROR_DIFFSUBSOFFSET;
        }

        if (options & !(SUBSTITUTE_OPTIONS | PCRE2_NO_UTF_CHECK))
            != ((*match_data).options & !PCRE2_NO_UTF_CHECK)
        {
            return PCRE2_ERROR_DIFFSUBSOPTIONS;
        }
    }

    /* If starting from an existing match, there must be an externally provided
    match data block. We create an internal match_data block in two cases: (a) an
    external one is not supplied (and we are not starting from an existing match);
    (b) an existing match is to be used for the first substitution. In the latter
    case, we copy the existing match into the internal block, except for any cached
    heap frame size and pointer. This ensures that no changes are made to the
    external match data block. */

    /* WARNING: In both cases below a general context is constructed "by hand"
    because calling pcre2_general_context_create() involves a memory allocation. If
    the contents of a general context control block are ever changed there will
    have to be changes below. */

    if match_data.is_null() {
        let mut gcontext: pcre2_real_general_context = pcre2_real_general_context {
            memctl: if mcontext.is_null() {
                (*code).memctl
            } else {
                (*mcontext).memctl
            },
        };
        internal_match_data = pcre2_match_data_create_from_pattern_8(code, &mut gcontext);
        match_data = internal_match_data;
        if internal_match_data.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }
    } else if use_existing_match != 0 {
        let pairs: c_int;
        let mut gcontext: pcre2_real_general_context = pcre2_real_general_context {
            memctl: if mcontext.is_null() {
                (*code).memctl
            } else {
                (*mcontext).memctl
            },
        };
        pairs = if ((*code).top_bracket as c_int + 1) < (*match_data).oveccount as c_int {
            (*code).top_bracket as c_int + 1
        } else {
            (*match_data).oveccount as c_int
        };
        internal_match_data =
            pcre2_match_data_create_8((*match_data).oveccount as u32, &mut gcontext);
        if internal_match_data.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }
        memcpy(
            internal_match_data as *mut c_void,
            match_data as *const c_void,
            offset_of!(pcre2_real_match_data, ovector)
                + 2 * (pairs as usize) * size_of::<PCRE2_SIZE>(),
        );
        (*internal_match_data).heapframes = std::ptr::null_mut();
        (*internal_match_data).heapframes_size = 0;
        /* Ensure that the subject is not freed when internal_match_data is */
        (*internal_match_data).flags &= !(PCRE2_MD_COPIED_SUBJECT as u8);
        match_data = internal_match_data;
    }

    /* If using an internal match data, there's no need to copy the subject. */

    if !internal_match_data.is_null() {
        options &= !PCRE2_COPY_MATCHED_SUBJECT;
    }

    /* Remember ovector details */

    ovector = pcre2_get_ovector_pointer_8(match_data);
    ovector_count = pcre2_get_ovector_count_8(match_data);

    /* Fixed things in the callout block */

    scb.version = 0;
    scb.input = subject;
    scb.output = buffer as PCRE2_SPTR;
    scb.ovector = ovector;

    'exit_all: {
        'ptrexit: {
            'badescape: {
                'bad: {
                    'toolargereplace: {
                        'caseerror: {
                            'noroom: {
                                /* ---- the C macros defined above pcre2_substitute() ---- */

                                /* This macro checks for space in the buffer before copying into
                                it. On overflow, either give an error immediately, or keep on,
                                accumulating the length. */

                                macro_rules! CHECKMEMCPY {
                                    ($from:expr, $length_:expr) => {{
                                        let chkmc_length: PCRE2_SIZE = $length_;
                                        if overflowed != 0 {
                                            if chkmc_length > !(0 as PCRE2_SIZE) - extra_needed {
                                                /* Integer overflow */
                                                break 'toolargereplace;
                                            }
                                            extra_needed += chkmc_length;
                                        } else if lengthleft < chkmc_length {
                                            if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0
                                            {
                                                break 'noroom;
                                            }
                                            overflowed = TRUE;
                                            extra_needed = chkmc_length - lengthleft;
                                        } else {
                                            memcpy(
                                                buffer.add(buff_offset) as *mut c_void,
                                                ($from) as *const c_void,
                                                CU2BYTES!(chkmc_length),
                                            );
                                            buff_offset += chkmc_length;
                                            lengthleft -= chkmc_length;
                                        }
                                    }};
                                }

                                /* This macro checks for space and copies characters with casing
                                modifications. On overflow, it behaves as for CHECKMEMCPY().

                                When substitute_case_callout is NULL, the source and destination
                                buffers must not overlap, because our default handler does not
                                support this. */

                                macro_rules! CHECKCASECPY_DEFAULT {
                                    ($from:expr, $length_:expr) => {{
                                        let chkcc_length: PCRE2_SIZE = ($length_) as PCRE2_SIZE;
                                        'chkcc: {
                                            let chkcc_rc: PCRE2_SIZE =
                                                default_substitute_case_callout(
                                                    $from,
                                                    chkcc_length,
                                                    buffer.add(buff_offset),
                                                    if overflowed != 0 { 0 } else { lengthleft },
                                                    &mut forcecase,
                                                    code,
                                                );
                                            if overflowed != 0 {
                                                if chkcc_rc > !(0 as PCRE2_SIZE) - extra_needed {
                                                    /* Integer overflow */
                                                    break 'toolargereplace;
                                                }
                                                extra_needed += chkcc_rc;
                                                break 'chkcc;
                                            }
                                            if lengthleft < chkcc_rc {
                                                if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH)
                                                    == 0
                                                {
                                                    break 'noroom;
                                                }
                                                overflowed = TRUE;
                                                extra_needed = chkcc_rc - lengthleft;
                                            } else {
                                                buff_offset += chkcc_rc;
                                                lengthleft -= chkcc_rc;
                                            }
                                        }
                                    }};
                                }

                                macro_rules! CHECKCASECPY_CALLOUT {
                                    ($length_:expr) => {{
                                        let chkcc_length: PCRE2_SIZE = ($length_) as PCRE2_SIZE;
                                        let chkcc_rc: PCRE2_SIZE = do_case_copy(
                                            buffer.add(buff_offset),
                                            chkcc_length,
                                            lengthleft,
                                            &mut forcecase,
                                            utf,
                                            substitute_case_callout,
                                            substitute_case_callout_data,
                                        );
                                        if chkcc_rc == !(0 as PCRE2_SIZE) {
                                            break 'caseerror;
                                        }
                                        if lengthleft < chkcc_rc {
                                            if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0
                                            {
                                                break 'noroom;
                                            }
                                            overflowed = TRUE;
                                            extra_needed = chkcc_rc - lengthleft;
                                        } else {
                                            buff_offset += chkcc_rc;
                                            lengthleft -= chkcc_rc;
                                        }
                                    }};
                                }

                                /* This macro does a delayed case transformation, for the
                                situation when we have a case-forcing callout. */

                                macro_rules! DELAYEDFORCECASE {
                                    () => {{
                                        let chars_outstanding: PCRE2_SIZE = (buff_offset
                                            - casestart_offset)
                                            + (extra_needed - casestart_extra_needed);
                                        if chars_outstanding > 0 {
                                            if overflowed != 0 {
                                                let guess: PCRE2_SIZE =
                                                    pessimistic_case_inflation(chars_outstanding);
                                                if guess > !(0 as PCRE2_SIZE) - extra_needed {
                                                    /* Integer overflow */
                                                    break 'toolargereplace;
                                                }
                                                extra_needed += guess;
                                            } else {
                                                /* Rewind the buffer */
                                                lengthleft += buff_offset - casestart_offset;
                                                buff_offset = casestart_offset;
                                                /* Care! In-place case transformation */
                                                CHECKCASECPY_CALLOUT!(chars_outstanding);
                                            }
                                        }
                                    }};
                                }

                                /* ---------------- body of pcre2_substitute() ---------------- */

                                /* Check UTF replacement string if necessary. */

                                if utf != 0 && (options & PCRE2_NO_UTF_CHECK) == 0 {
                                    rc = _pcre2_valid_utf_8(
                                        replacement,
                                        rlength,
                                        &mut (*match_data).startchar,
                                    );
                                    if rc != 0 {
                                        (*match_data).leftchar = 0;
                                        break 'exit_all;
                                    }
                                }

                                /* Save the substitute options and remove them from the match
                                options. */

                                suboptions = options & SUBSTITUTE_OPTIONS;
                                options &= !SUBSTITUTE_OPTIONS;

                                /* Error if the start match offset is greater than the length of
                                the subject. */

                                if start_offset > length {
                                    (*match_data).leftchar = 0;
                                    rc = PCRE2_ERROR_BADOFFSET;
                                    break 'exit_all;
                                }

                                /* Copy up to the start offset, unless only the replacement is
                                required. */

                                if replacement_only == 0 {
                                    CHECKMEMCPY!(subject, start_offset);
                                }

                                /* Loop for global substituting. If PCRE2_SUBSTITUTE_MATCHED is
                                set, the first match is taken from the match_data that was passed
                                in. */

                                subs = 0;
                                'global_loop: loop {
                                    let mut ptrstack: [PCRE2_SPTR; PTR_STACK_SIZE as usize] =
                                        [std::ptr::null(); PTR_STACK_SIZE as usize];
                                    let mut ptrstackptr: u32 = 0;
                                    forcecase = case_state {
                                        to_case: PCRE2_SUBSTITUTE_CASE_NONE as _,
                                        single_char: FALSE,
                                    };
                                    casestart_offset = 0;
                                    casestart_extra_needed = 0;

                                    if use_existing_match != 0 {
                                        rc = (*match_data).rc;
                                        use_existing_match = FALSE;
                                    } else {
                                        rc = pcre2_match_8(
                                            code,
                                            subject,
                                            length,
                                            start_offset,
                                            options | goptions,
                                            match_data,
                                            mcontext,
                                        );
                                    }

                                    if utf != 0 {
                                        options |= PCRE2_NO_UTF_CHECK; /* Only need to check once */
                                    }

                                    /* Any error other than no match returns the error code. No
                                    match breaks the global loop. */

                                    if rc == PCRE2_ERROR_NOMATCH {
                                        break 'global_loop;
                                    }

                                    if rc < 0 {
                                        break 'exit_all;
                                    }

                                    /* Handle a successful match. Matches that use \K to end
                                    before they start or start before the current point in the
                                    subject are not supported. */

                                    if *ovector.add(1) < *ovector.add(0)
                                        || *ovector.add(0) < start_offset
                                    {
                                        rc = PCRE2_ERROR_BADSUBSPATTERN;
                                        break 'exit_all;
                                    }

                                    /* Assert that our replacement loop is making progress,
                                    checked even in release builds. This should be impossible to
                                    hit, however, an infinite loop would be fairly catastrophic.

                                    "Progress" is measured as ovector[1] strictly advancing, or,
                                    an empty match after a non-empty match. */

                                    if subs > 0
                                        && !(*ovector.add(1) > ovecsave[1]
                                            || (*ovector.add(1) == *ovector.add(0)
                                                && ovecsave[1] > ovecsave[0]
                                                && *ovector.add(1) == ovecsave[1]))
                                    {
                                        rc = PCRE2_ERROR_INTERNAL_DUPMATCH;
                                        break 'exit_all;
                                    }

                                    ovecsave[0] = *ovector.add(0);
                                    ovecsave[1] = *ovector.add(1);

                                    /* Count substitutions with a paranoid check for integer
                                    overflow; surely no real call to this function would ever hit
                                    this! */

                                    if subs == c_int::MAX {
                                        rc = PCRE2_ERROR_TOOMANYREPLACE;
                                        break 'exit_all;
                                    }
                                    subs += 1;

                                    /* Copy the text leading up to the match (unless not
                                    required); remember where the insert begins and how many
                                    ovector pairs are set; and remember how much space we have
                                    requested in extra_needed. */

                                    if rc == 0 {
                                        rc = ovector_count as c_int;
                                    }
                                    fraglength = *ovector.add(0) - start_offset;
                                    if replacement_only == 0 {
                                        CHECKMEMCPY!(subject.add(start_offset), fraglength);
                                    }
                                    scb.output_offsets[0] = buff_offset;
                                    scb.oveccount = rc as u32;
                                    sub_start_extra_needed = extra_needed;

                                    /* Process the replacement string. If the entire replacement
                                    is literal, just copy it with length check. */

                                    ptr = replacement;
                                    if (suboptions & PCRE2_SUBSTITUTE_LITERAL) != 0 {
                                        CHECKMEMCPY!(ptr, rlength);
                                    }
                                    /* Within a non-literal replacement, which must be scanned
                                    character by character, local literal mode can be set by \Q,
                                    but only in extended mode when backslashes are being
                                    interpreted. In extended mode we must handle nested substrings
                                    that are to be reprocessed. */
                                    else {
                                        'replacement_loop: loop {
                                            let mut ch: u32 = 0;
                                            let mut chlen: c_uint;
                                            let mut group: c_int = 0;
                                            let mut special: u32 = 0;
                                            let mut text1_start: PCRE2_SPTR = std::ptr::null();
                                            let mut text1_end: PCRE2_SPTR = std::ptr::null();
                                            let mut text2_start: PCRE2_SPTR = std::ptr::null();
                                            let mut text2_end: PCRE2_SPTR = std::ptr::null();
                                            let mut name: [PCRE2_UCHAR; MAX_NAME_SIZE as usize + 1] =
                                                [0; MAX_NAME_SIZE as usize + 1];
                                            /* Declared inside the "$" block in C; hoisted so that
                                            the backslash branch can "goto GROUP_SUBSTITUTE". */
                                            let mut sublength: PCRE2_SIZE = 0;
                                            let mut subptr: PCRE2_SPTR = std::ptr::null();
                                            let mut subptrend: PCRE2_SPTR = std::ptr::null();
                                            /* Stands for the backward "goto GROUP_SUBSTITUTE". */
                                            let mut goto_group_substitute: BOOL = FALSE;

                                            'dispatch: loop {
                                                'loadliteral: {
                                                    'end_of_dollar: {
                                                        'subptr_substitute: {
                                                            'literal_substitute: {
                                                                'group_substitute: {
                                                                    if goto_group_substitute != 0 {
                                                                        break 'group_substitute;
                                                                    }

                                                                    /* If at the end of a nested
                                                                    substring, pop the stack. */

                                                                    if ptr >= repend {
                                                                        if ptrstackptr == 0 {
                                                                            /* End of replacement
                                                                            string */
                                                                            break 'replacement_loop;
                                                                        }
                                                                        ptrstackptr -= 1;
                                                                        repend = *ptrstack
                                                                            .as_ptr()
                                                                            .add(ptrstackptr
                                                                                as usize);
                                                                        ptrstackptr -= 1;
                                                                        ptr = *ptrstack
                                                                            .as_ptr()
                                                                            .add(ptrstackptr
                                                                                as usize);
                                                                        continue 'replacement_loop;
                                                                    }

                                                                    /* Handle the next character */

                                                                    if escaped_literal != 0 {
                                                                        if *ptr as u32
                                                                            == CHAR_BACKSLASH
                                                                            && ptr < repend.sub(1)
                                                                            && *ptr.add(1) as u32
                                                                                == CHAR_E
                                                                        {
                                                                            escaped_literal = FALSE;
                                                                            ptr = ptr.add(2);
                                                                            continue 'replacement_loop;
                                                                        }
                                                                        break 'loadliteral;
                                                                    }

                                                                    /* Not in literal mode. */

                                                                    if *ptr as u32
                                                                        == CHAR_DOLLAR_SIGN
                                                                    {
                                                                        let mut inparens: BOOL;
                                                                        let mut inangle: BOOL;
                                                                        let mut star: BOOL;
                                                                        let mut next: PCRE2_UCHAR;

                                                                        ptr = ptr.add(1);
                                                                        if ptr >= repend {
                                                                            break 'bad;
                                                                        }
                                                                        next = *ptr;
                                                                        if next as u32
                                                                            == CHAR_DOLLAR_SIGN
                                                                        {
                                                                            break 'loadliteral;
                                                                        }

                                                                        special = 0;
                                                                        text1_start =
                                                                            std::ptr::null();
                                                                        text1_end = std::ptr::null();
                                                                        text2_start =
                                                                            std::ptr::null();
                                                                        text2_end = std::ptr::null();
                                                                        group = -1;
                                                                        inparens = FALSE;
                                                                        inangle = FALSE;
                                                                        star = FALSE;
                                                                        subptr = std::ptr::null();
                                                                        subptrend = std::ptr::null();

                                                                        /* Special $ sequences, as
                                                                        supported by Perl,
                                                                        JavaScript, .NET and
                                                                        others. */
                                                                        if next as u32
                                                                            == CHAR_AMPERSAND
                                                                        {
                                                                            ptr = ptr.add(1);
                                                                            group = 0;
                                                                            break 'group_substitute;
                                                                        }
                                                                        if next as u32
                                                                            == CHAR_GRAVE_ACCENT
                                                                            || next as u32
                                                                                == CHAR_APOSTROPHE
                                                                        {
                                                                            ptr = ptr.add(1);

                                                                            /* (Sanity-check
                                                                            ovector before reading
                                                                            from it.) */
                                                                            rc = pcre2_substring_length_bynumber_8(
                                                                                match_data, 0,
                                                                                &mut sublength);
                                                                            if rc < 0 {
                                                                                break 'ptrexit;
                                                                            }

                                                                            if next as u32
                                                                                == CHAR_GRAVE_ACCENT
                                                                            {
                                                                                subptr = subject;
                                                                                subptrend = subject
                                                                                    .add(*ovector
                                                                                        .add(0));
                                                                            } else {
                                                                                if partial != 0 {
                                                                                    rc = PCRE2_ERROR_PARTIALSUBS;
                                                                                    break 'ptrexit;
                                                                                }

                                                                                subptr = subject
                                                                                    .add(*ovector
                                                                                        .add(1));
                                                                                subptrend =
                                                                                    subject.add(length);
                                                                            }

                                                                            break 'subptr_substitute;
                                                                        }
                                                                        if next as u32
                                                                            == CHAR_UNDERSCORE
                                                                        {
                                                                            /* Java, .NET support
                                                                            $_ for "entire input
                                                                            string". */
                                                                            ptr = ptr.add(1);

                                                                            if partial != 0 {
                                                                                rc = PCRE2_ERROR_PARTIALSUBS;
                                                                                break 'ptrexit;
                                                                            }

                                                                            subptr = subject;
                                                                            subptrend =
                                                                                subject.add(length);
                                                                            break 'subptr_substitute;
                                                                        }
                                                                        if next as u32 == CHAR_PLUS
                                                                            && !(ptr.add(1) < repend
                                                                                && *ptr.add(1)
                                                                                    as u32
                                                                                    == CHAR_LEFT_CURLY_BRACKET)
                                                                        {
                                                                            /* Perl supports $+ for
                                                                            "highest captured
                                                                            group" (not the same as
                                                                            $^N which is mainly only
                                                                            useful inside Perl's
                                                                            match callbacks). We
                                                                            also don't accept "$+{..."
                                                                            since that's Perl syntax
                                                                            for our ${name}. */
                                                                            ptr = ptr.add(1);
                                                                            if (*code).top_bracket
                                                                                == 0
                                                                            {
                                                                                /* Treat either as
                                                                                "no such group" or
                                                                                "all groups unset"
                                                                                based on the
                                                                                PCRE2_SUBSTITUTE_UNKNOWN_UNSET
                                                                                option. */
                                                                                if (suboptions
                                                                                    & PCRE2_SUBSTITUTE_UNKNOWN_UNSET)
                                                                                    == 0
                                                                                {
                                                                                    rc = PCRE2_ERROR_NOSUBSTRING;
                                                                                    break 'ptrexit;
                                                                                }
                                                                                group = 0;
                                                                            } else {
                                                                                /* If we have any
                                                                                capture groups, then
                                                                                the ovector needs to
                                                                                be large enough for
                                                                                all of them, or the
                                                                                result won't be
                                                                                accurate. */
                                                                                if ((*match_data)
                                                                                    .oveccount
                                                                                    as c_int)
                                                                                    < (*code)
                                                                                        .top_bracket
                                                                                        as c_int
                                                                                        + 1
                                                                                {
                                                                                    rc = PCRE2_ERROR_UNAVAILABLE;
                                                                                    break 'ptrexit;
                                                                                }
                                                                                group = (*code)
                                                                                    .top_bracket
                                                                                    as c_int;
                                                                                while group > 0 {
                                                                                    if *ovector.add(
                                                                                        2 * group
                                                                                            as usize,
                                                                                    ) != PCRE2_UNSET
                                                                                    {
                                                                                        break;
                                                                                    }
                                                                                    group -= 1;
                                                                                }
                                                                            }
                                                                            if group == 0 {
                                                                                if (suboptions
                                                                                    & PCRE2_SUBSTITUTE_UNSET_EMPTY)
                                                                                    != 0
                                                                                {
                                                                                    continue 'replacement_loop;
                                                                                }
                                                                                rc = PCRE2_ERROR_UNSET;
                                                                                break 'ptrexit;
                                                                            }
                                                                            break 'group_substitute;
                                                                        }

                                                                        if next as u32
                                                                            == CHAR_LEFT_CURLY_BRACKET
                                                                        {
                                                                            ptr = ptr.add(1);
                                                                            if ptr >= repend {
                                                                                break 'bad;
                                                                            }
                                                                            next = *ptr;
                                                                            inparens = TRUE;
                                                                        } else if next as u32
                                                                            == CHAR_LESS_THAN_SIGN
                                                                        {
                                                                            /* JavaScript
                                                                            compatibility syntax,
                                                                            $<name>. Processes only
                                                                            named groups (not
                                                                            numbered) and does not
                                                                            support extensions such
                                                                            as star (you can do
                                                                            ${name} and ${*name},
                                                                            but not $<*name>). */
                                                                            ptr = ptr.add(1);
                                                                            if ptr >= repend {
                                                                                break 'bad;
                                                                            }
                                                                            next = *ptr;
                                                                            inangle = TRUE;
                                                                        }

                                                                        if inangle == 0
                                                                            && next as u32
                                                                                == CHAR_ASTERISK
                                                                        {
                                                                            ptr = ptr.add(1);
                                                                            if ptr >= repend {
                                                                                break 'bad;
                                                                            }
                                                                            next = *ptr;
                                                                            star = TRUE;
                                                                        }

                                                                        if star == 0
                                                                            && inangle == 0
                                                                            && next as u32 >= CHAR_0
                                                                            && next as u32 <= CHAR_9
                                                                        {
                                                                            group = (next as u32
                                                                                - CHAR_0)
                                                                                as c_int;
                                                                            loop {
                                                                                ptr = ptr.add(1);
                                                                                if !(ptr < repend) {
                                                                                    break;
                                                                                }
                                                                                next = *ptr;
                                                                                if (next as u32)
                                                                                    < CHAR_0
                                                                                    || next as u32
                                                                                        > CHAR_9
                                                                                {
                                                                                    break;
                                                                                }
                                                                                group = group * 10
                                                                                    + (next as u32
                                                                                        - CHAR_0)
                                                                                        as c_int;

                                                                                /* A check for a
                                                                                number greater than
                                                                                the hightest
                                                                                captured group is
                                                                                sufficient here; no
                                                                                need for a separate
                                                                                overflow check. If
                                                                                unknown groups are to
                                                                                be treated as unset,
                                                                                just skip over any
                                                                                remaining digits and
                                                                                carry on. */

                                                                                if group
                                                                                    > (*code)
                                                                                        .top_bracket
                                                                                        as c_int
                                                                                {
                                                                                    if (suboptions
                                                                                        & PCRE2_SUBSTITUTE_UNKNOWN_UNSET)
                                                                                        != 0
                                                                                    {
                                                                                        loop {
                                                                                            ptr = ptr
                                                                                                .add(1);
                                                                                            if !(ptr
                                                                                                < repend
                                                                                                && *ptr
                                                                                                    as u32
                                                                                                    >= CHAR_0
                                                                                                && *ptr
                                                                                                    as u32
                                                                                                    <= CHAR_9)
                                                                                            {
                                                                                                break;
                                                                                            }
                                                                                        }
                                                                                        break;
                                                                                    } else {
                                                                                        rc = PCRE2_ERROR_NOSUBSTRING;
                                                                                        break 'ptrexit;
                                                                                    }
                                                                                }
                                                                            }
                                                                        } else {
                                                                            let name_len: PCRE2_SIZE;
                                                                            let name_start: PCRE2_SPTR =
                                                                                ptr;
                                                                            if read_name_subst(
                                                                                &mut ptr,
                                                                                repend,
                                                                                utf,
                                                                                (*code)
                                                                                    .tables
                                                                                    .add(ctypes_offset),
                                                                            ) == 0
                                                                            {
                                                                                break 'bad;
                                                                            }
                                                                            name_len = ptr
                                                                                .offset_from(
                                                                                    name_start,
                                                                                )
                                                                                as PCRE2_SIZE;
                                                                            memcpy(
                                                                                name.as_mut_ptr()
                                                                                    as *mut c_void,
                                                                                name_start
                                                                                    as *const c_void,
                                                                                CU2BYTES!(name_len),
                                                                            );
                                                                            *name
                                                                                .as_mut_ptr()
                                                                                .add(name_len) = 0;
                                                                        }

                                                                        next = 0; /* not used or updated after this point */

                                                                        /* In extended mode we
                                                                        recognize
                                                                        ${name:+set text:unset text}
                                                                        and ${name:-default text}. */

                                                                        if inparens != 0 {
                                                                            if (suboptions
                                                                                & PCRE2_SUBSTITUTE_EXTENDED)
                                                                                != 0
                                                                                && star == 0
                                                                                && ptr < repend.sub(2)
                                                                                && *ptr as u32
                                                                                    == CHAR_COLON
                                                                            {
                                                                                ptr = ptr.add(1);
                                                                                special =
                                                                                    *ptr as u32;
                                                                                if special
                                                                                    != CHAR_PLUS
                                                                                    && special
                                                                                        != CHAR_MINUS
                                                                                {
                                                                                    rc = PCRE2_ERROR_BADSUBSTITUTION;
                                                                                    break 'ptrexit;
                                                                                }

                                                                                ptr = ptr.add(1);
                                                                                text1_start = ptr;
                                                                                rc = find_text_end(
                                                                                    code,
                                                                                    &mut ptr,
                                                                                    repend,
                                                                                    if special
                                                                                        == CHAR_MINUS
                                                                                    {
                                                                                        TRUE
                                                                                    } else {
                                                                                        FALSE
                                                                                    },
                                                                                );
                                                                                if rc != 0 {
                                                                                    break 'ptrexit;
                                                                                }
                                                                                text1_end = ptr;

                                                                                if special
                                                                                    == CHAR_PLUS
                                                                                    && *ptr as u32
                                                                                        == CHAR_COLON
                                                                                {
                                                                                    ptr = ptr.add(1);
                                                                                    text2_start = ptr;
                                                                                    rc = find_text_end(
                                                                                        code,
                                                                                        &mut ptr,
                                                                                        repend,
                                                                                        TRUE,
                                                                                    );
                                                                                    if rc != 0 {
                                                                                        break 'ptrexit;
                                                                                    }
                                                                                    text2_end = ptr;
                                                                                }
                                                                            } else {
                                                                                if ptr >= repend
                                                                                    || *ptr as u32
                                                                                        != CHAR_RIGHT_CURLY_BRACKET
                                                                                {
                                                                                    rc = PCRE2_ERROR_REPMISSINGBRACE;
                                                                                    break 'ptrexit;
                                                                                }
                                                                            }

                                                                            ptr = ptr.add(1);
                                                                        }

                                                                        if inangle != 0 {
                                                                            if ptr >= repend
                                                                                || *ptr as u32
                                                                                    != CHAR_GREATER_THAN_SIGN
                                                                            {
                                                                                break 'bad;
                                                                            }
                                                                            ptr = ptr.add(1);
                                                                        }

                                                                        /* Have found a
                                                                        syntactically correct group
                                                                        number or name, or *name.
                                                                        Only *MARK is currently
                                                                        recognized. */

                                                                        if star != 0 {
                                                                            if _pcre2_strcmp_c8_8(
                                                                                name.as_ptr(),
                                                                                b"MARK\0".as_ptr()
                                                                                    as *const c_char,
                                                                            ) == 0
                                                                            {
                                                                                let mark: PCRE2_SPTR =
                                                                                    pcre2_get_mark_8(
                                                                                        match_data,
                                                                                    );
                                                                                if !mark.is_null() {
                                                                                    /* Peek
                                                                                    backwards one
                                                                                    code unit to
                                                                                    obtain the
                                                                                    length of the
                                                                                    mark. It can
                                                                                    (theoretically)
                                                                                    contain an
                                                                                    embedded NUL. */
                                                                                    fraglength =
                                                                                        *mark.sub(1)
                                                                                            as PCRE2_SIZE;
                                                                                    if forcecase
                                                                                        .to_case
                                                                                        != 0
                                                                                        && substitute_case_callout
                                                                                            .is_none()
                                                                                    {
                                                                                        CHECKCASECPY_DEFAULT!(
                                                                                            mark,
                                                                                            fraglength
                                                                                        );
                                                                                    } else {
                                                                                        CHECKMEMCPY!(
                                                                                            mark,
                                                                                            fraglength
                                                                                        );
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                break 'bad;
                                                                            }
                                                                            break 'end_of_dollar;
                                                                        }
                                                                        /* Substitute the contents
                                                                        of a group. We don't use
                                                                        substring_copy functions any
                                                                        more, in order to support
                                                                        case forcing. */
                                                                        else {
                                                                            break 'group_substitute;
                                                                        }
                                                                    }
                                                                    /* Handle an escape sequence in
                                                                    extended mode. We can use
                                                                    check_escape() to process \Q,
                                                                    \E, \c, \o, \x and \ followed by
                                                                    non-alphanumerics, but the
                                                                    case-forcing escapes are not
                                                                    supported in pcre2_compile() so
                                                                    must be recognized here. */
                                                                    else if (suboptions
                                                                        & PCRE2_SUBSTITUTE_EXTENDED)
                                                                        != 0
                                                                        && *ptr as u32
                                                                            == CHAR_BACKSLASH
                                                                    {
                                                                        let mut errorcode: c_int = 0;
                                                                        let mut new_forcecase: case_state =
                                                                            case_state {
                                                                                to_case: PCRE2_SUBSTITUTE_CASE_NONE
                                                                                    as _,
                                                                                single_char: FALSE,
                                                                            };
                                                                        /* Stands for the backward
                                                                        "goto SETFORCECASE". */
                                                                        let mut goto_setforcecase: BOOL =
                                                                            FALSE;

                                                                        'sfc: loop {
                                                                            if goto_setforcecase == 0
                                                                            {
                                                                                if ptr
                                                                                    < repend.sub(1)
                                                                                {
                                                                                    let nextch: u32 =
                                                                                        *ptr.add(1)
                                                                                            as u32;
                                                                                    if nextch
                                                                                        == CHAR_L
                                                                                    {
                                                                                        new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_LOWER as _;
                                                                                        new_forcecase.single_char = FALSE;
                                                                                        ptr = ptr.add(2);
                                                                                    } else if nextch
                                                                                        == CHAR_l
                                                                                    {
                                                                                        new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_LOWER as _;
                                                                                        new_forcecase.single_char = TRUE;
                                                                                        ptr = ptr.add(2);
                                                                                        if ptr.add(2) < repend
                                                                                            && *ptr as u32 == CHAR_BACKSLASH
                                                                                            && *ptr.add(1) as u32 == CHAR_U
                                                                                        {
                                                                                            /* Perl reverse-title-casing feature for \l\U */
                                                                                            new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST as _;
                                                                                            new_forcecase.single_char = FALSE;
                                                                                            ptr = ptr.add(2);
                                                                                        }
                                                                                    } else if nextch
                                                                                        == CHAR_U
                                                                                    {
                                                                                        new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_UPPER as _;
                                                                                        new_forcecase.single_char = FALSE;
                                                                                        ptr = ptr.add(2);
                                                                                    } else if nextch
                                                                                        == CHAR_u
                                                                                    {
                                                                                        new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_TITLE_FIRST as _;
                                                                                        new_forcecase.single_char = TRUE;
                                                                                        ptr = ptr.add(2);
                                                                                        if ptr.add(2) < repend
                                                                                            && *ptr as u32 == CHAR_BACKSLASH
                                                                                            && *ptr.add(1) as u32 == CHAR_L
                                                                                        {
                                                                                            /* Perl title-casing feature for \u\L */
                                                                                            new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_TITLE_FIRST as _;
                                                                                            new_forcecase.single_char = FALSE;
                                                                                            ptr = ptr.add(2);
                                                                                        }
                                                                                    }
                                                                                }

                                                                                if new_forcecase
                                                                                    .to_case
                                                                                    == 0
                                                                                {
                                                                                    ptr = ptr.add(1); /* Point after \ */
                                                                                    rc = _pcre2_check_escape_8(
                                                                                        &mut ptr,
                                                                                        repend,
                                                                                        &mut ch,
                                                                                        &mut errorcode,
                                                                                        (*code).overall_options,
                                                                                        (*code).extra_options,
                                                                                        (*code).top_bracket as u32,
                                                                                        FALSE,
                                                                                        std::ptr::null_mut(),
                                                                                    );
                                                                                    if errorcode != 0
                                                                                    {
                                                                                        break 'badescape;
                                                                                    }

                                                                                    if rc == ESC_E {
                                                                                        goto_setforcecase = TRUE;
                                                                                        continue 'sfc;
                                                                                    } else if rc
                                                                                        == ESC_Q
                                                                                    {
                                                                                        escaped_literal = TRUE;
                                                                                        continue 'replacement_loop;
                                                                                    } else if rc == 0
                                                                                        || rc
                                                                                            == ESC_b
                                                                                        || rc
                                                                                            == ESC_v
                                                                                    {
                                                                                        /* Data
                                                                                        character;
                                                                                        \b is
                                                                                        backspace and
                                                                                        \v is
                                                                                        vertical tab
                                                                                        in a
                                                                                        substitution */
                                                                                        if rc == ESC_b {
                                                                                            ch = CHAR_BS;
                                                                                        }
                                                                                        if rc == ESC_v {
                                                                                            ch = CHAR_VT;
                                                                                        }

                                                                                        if utf != 0 {
                                                                                            chlen = _pcre2_ord2utf_8(ch, temp.as_mut_ptr());
                                                                                        } else {
                                                                                            temp[0] = ch as PCRE2_UCHAR;
                                                                                            chlen = 1;
                                                                                        }

                                                                                        if forcecase.to_case != 0
                                                                                            && substitute_case_callout.is_none()
                                                                                        {
                                                                                            CHECKCASECPY_DEFAULT!(temp.as_ptr(), chlen as PCRE2_SIZE);
                                                                                        } else {
                                                                                            CHECKMEMCPY!(temp.as_ptr(), chlen as PCRE2_SIZE);
                                                                                        }
                                                                                        continue 'replacement_loop;
                                                                                    } else if rc
                                                                                        == ESC_g
                                                                                    {
                                                                                        let name_len: PCRE2_SIZE;
                                                                                        let name_start: PCRE2_SPTR;

                                                                                        /* Parse the \g<name> form (\g<number> already handled by check_escape) */
                                                                                        if ptr >= repend
                                                                                            || *ptr as u32 != CHAR_LESS_THAN_SIGN
                                                                                        {
                                                                                            break 'badescape;
                                                                                        }
                                                                                        ptr = ptr.add(1);

                                                                                        name_start = ptr;
                                                                                        if read_name_subst(
                                                                                            &mut ptr,
                                                                                            repend,
                                                                                            utf,
                                                                                            (*code).tables.add(ctypes_offset),
                                                                                        ) == 0
                                                                                        {
                                                                                            break 'badescape;
                                                                                        }
                                                                                        name_len = ptr.offset_from(name_start) as PCRE2_SIZE;

                                                                                        if ptr >= repend
                                                                                            || *ptr as u32 != CHAR_GREATER_THAN_SIGN
                                                                                        {
                                                                                            break 'badescape;
                                                                                        }
                                                                                        ptr = ptr.add(1);

                                                                                        special = 0;
                                                                                        group = -1;
                                                                                        memcpy(
                                                                                            name.as_mut_ptr() as *mut c_void,
                                                                                            name_start as *const c_void,
                                                                                            CU2BYTES!(name_len),
                                                                                        );
                                                                                        *name.as_mut_ptr().add(name_len) = 0;
                                                                                        goto_group_substitute = TRUE;
                                                                                        continue 'dispatch;
                                                                                    } else {
                                                                                        if rc < 0 {
                                                                                            special = 0;
                                                                                            group = -rc - 1;
                                                                                            goto_group_substitute = TRUE;
                                                                                            continue 'dispatch;
                                                                                        }
                                                                                        break 'badescape;
                                                                                    }
                                                                                }
                                                                            }

                                                                            /* SETFORCECASE: */

                                                                            /* If the
                                                                            substitute_case_callout
                                                                            is unset, our
                                                                            case-forcing is done
                                                                            immediately. If there is
                                                                            a callout however, then
                                                                            its action is delayed
                                                                            until all the characters
                                                                            have been collected.

                                                                            Apply the callout now,
                                                                            before we set the new
                                                                            casing mode. */

                                                                            if substitute_case_callout
                                                                                .is_some()
                                                                                && forcecase.to_case
                                                                                    != 0
                                                                            {
                                                                                DELAYEDFORCECASE!();
                                                                            }

                                                                            forcecase = new_forcecase;
                                                                            casestart_offset =
                                                                                buff_offset;
                                                                            casestart_extra_needed =
                                                                                extra_needed;
                                                                            continue 'replacement_loop;
                                                                        }
                                                                    }
                                                                    /* Handle a literal code unit */
                                                                    else {
                                                                        break 'loadliteral;
                                                                    }
                                                                }
                                                                /* GROUP_SUBSTITUTE: */

                                                                /* Find a number for a named group.
                                                                In case there are duplicate names,
                                                                search for the first one that is
                                                                set. If the name is not found when
                                                                PCRE2_SUBSTITUTE_UNKNOWN_EMPTY is
                                                                set, set the group number to a
                                                                non-existent group. */

                                                                if group < 0 {
                                                                    let mut first: PCRE2_SPTR =
                                                                        std::ptr::null();
                                                                    let mut last: PCRE2_SPTR =
                                                                        std::ptr::null();
                                                                    let mut entry: PCRE2_SPTR;
                                                                    rc = pcre2_substring_nametable_scan_8(
                                                                        code,
                                                                        name.as_ptr(),
                                                                        &mut first,
                                                                        &mut last,
                                                                    );
                                                                    if rc == PCRE2_ERROR_NOSUBSTRING
                                                                        && (suboptions
                                                                            & PCRE2_SUBSTITUTE_UNKNOWN_UNSET)
                                                                            != 0
                                                                    {
                                                                        group = (*code).top_bracket
                                                                            as c_int
                                                                            + 1;
                                                                    } else {
                                                                        if rc < 0 {
                                                                            break 'ptrexit;
                                                                        }
                                                                        entry = first;
                                                                        while entry <= last {
                                                                            let ng: u32 =
                                                                                GET2!(entry, 0);
                                                                            if ng < ovector_count {
                                                                                if group < 0 {
                                                                                    group =
                                                                                        ng as c_int;
                                                                                    /* First in ovector */
                                                                                }
                                                                                if *ovector.add(
                                                                                    ng as usize * 2,
                                                                                ) != PCRE2_UNSET
                                                                                {
                                                                                    group =
                                                                                        ng as c_int; /* First that is set */
                                                                                    break;
                                                                                }
                                                                            }
                                                                            entry =
                                                                                entry.add(rc as usize);
                                                                        }

                                                                        /* If group is still
                                                                        negative, it means we did not
                                                                        find a group that is in the
                                                                        ovector. Just set the first
                                                                        group. */

                                                                        if group < 0 {
                                                                            group = GET2!(first, 0)
                                                                                as c_int;
                                                                        }
                                                                    }
                                                                }

                                                                /* We now have a group that is
                                                                identified by number. Find the
                                                                length of the captured string. If a
                                                                group in a non-special substitution
                                                                is unset when
                                                                PCRE2_SUBSTITUTE_UNSET_EMPTY is set,
                                                                substitute nothing. */

                                                                rc = pcre2_substring_length_bynumber_8(
                                                                    match_data,
                                                                    group as u32,
                                                                    &mut sublength,
                                                                );
                                                                if rc < 0 {
                                                                    if rc == PCRE2_ERROR_NOSUBSTRING
                                                                        && (suboptions
                                                                            & PCRE2_SUBSTITUTE_UNKNOWN_UNSET)
                                                                            != 0
                                                                    {
                                                                        rc = PCRE2_ERROR_UNSET;
                                                                    }
                                                                    if rc != PCRE2_ERROR_UNSET {
                                                                        /* Non-unset errors */
                                                                        break 'ptrexit;
                                                                    }
                                                                    if special == 0 {
                                                                        /* Plain substitution */
                                                                        if (suboptions
                                                                            & PCRE2_SUBSTITUTE_UNSET_EMPTY)
                                                                            != 0
                                                                        {
                                                                            continue 'replacement_loop;
                                                                        }
                                                                        break 'ptrexit; /* Else error */
                                                                    }
                                                                }

                                                                /* If special is '+' we have a 'set'
                                                                and possibly an 'unset' text, both of
                                                                which are reprocessed when used. If
                                                                special is '-' we have a default text
                                                                for when the group is unset; it must
                                                                be reprocessed. */

                                                                if special != 0 {
                                                                    if special == CHAR_MINUS {
                                                                        if rc == 0 {
                                                                            break 'literal_substitute;
                                                                        }
                                                                        text2_start = text1_start;
                                                                        text2_end = text1_end;
                                                                    }

                                                                    if ptrstackptr
                                                                        >= PTR_STACK_SIZE as u32
                                                                    {
                                                                        break 'bad;
                                                                    }
                                                                    *ptrstack
                                                                        .as_mut_ptr()
                                                                        .add(ptrstackptr as usize) =
                                                                        ptr;
                                                                    ptrstackptr += 1;
                                                                    *ptrstack
                                                                        .as_mut_ptr()
                                                                        .add(ptrstackptr as usize) =
                                                                        repend;
                                                                    ptrstackptr += 1;

                                                                    if rc == 0 {
                                                                        ptr = text1_start;
                                                                        repend = text1_end;
                                                                    } else {
                                                                        ptr = text2_start;
                                                                        repend = text2_end;
                                                                    }
                                                                    continue 'replacement_loop;
                                                                }

                                                                /* Otherwise we have a literal
                                                                substitution of a group's contents. */
                                                            }
                                                            /* LITERAL_SUBSTITUTE: */
                                                            subptr = subject
                                                                .add(*ovector.add(group as usize * 2));
                                                            subptrend = subject.add(
                                                                *ovector.add(group as usize * 2 + 1),
                                                            );

                                                            /* Substitute a literal string, possibly
                                                            forcing alphabetic case. */
                                                        }
                                                        /* SUBPTR_SUBSTITUTE: */
                                                        if forcecase.to_case != 0
                                                            && substitute_case_callout.is_none()
                                                        {
                                                            CHECKCASECPY_DEFAULT!(
                                                                subptr,
                                                                subptrend.offset_from(subptr)
                                                                    as PCRE2_SIZE
                                                            );
                                                        } else {
                                                            CHECKMEMCPY!(
                                                                subptr,
                                                                subptrend.offset_from(subptr)
                                                                    as PCRE2_SIZE
                                                            );
                                                        }
                                                    } /* End of $ processing */
                                                    continue 'replacement_loop;
                                                }
                                                /* LOADLITERAL: */
                                                {
                                                    let ch_start: PCRE2_SPTR = ptr;
                                                    /* Get character value, increment pointer */
                                                    GETCHARINCTEST!(ch, ptr, utf);

                                                    if forcecase.to_case != 0
                                                        && substitute_case_callout.is_none()
                                                    {
                                                        CHECKCASECPY_DEFAULT!(
                                                            ch_start,
                                                            ptr.offset_from(ch_start) as PCRE2_SIZE
                                                        );
                                                    } else {
                                                        CHECKMEMCPY!(
                                                            ch_start,
                                                            ptr.offset_from(ch_start) as PCRE2_SIZE
                                                        );
                                                    }
                                                } /* End handling a literal code unit */
                                                continue 'replacement_loop;
                                            }
                                        } /* End of loop for scanning the replacement. */
                                    }

                                    /* If the substitute_case_callout is unset, our case-forcing is
                                    done immediately. If there is a callout however, then its action
                                    is delayed until all the characters have been collected.

                                    We now clean up any trailing section of the replacement for
                                    which we deferred the case-forcing. */

                                    if substitute_case_callout.is_some() && forcecase.to_case != 0 {
                                        DELAYEDFORCECASE!();
                                    }

                                    /* The replacement has been copied to the output, or its size
                                    has been remembered. Handle the callout if there is one. */

                                    if !mcontext.is_null()
                                        && (*mcontext).substitute_callout.is_some()
                                    {
                                        /* If we an actual (non-simulated) replacement, do the
                                        callout. */

                                        if overflowed == 0 {
                                            scb.subscount = subs as u32;
                                            scb.output_offsets[1] = buff_offset;
                                            rc = ((*mcontext).substitute_callout.unwrap())(
                                                &mut scb,
                                                (*mcontext).substitute_callout_data,
                                            );

                                            /* A non-zero return means cancel this substitution.
                                            Instead, copy the matched string fragment. */

                                            if rc != 0 {
                                                let newlength: PCRE2_SIZE = scb.output_offsets[1]
                                                    - scb.output_offsets[0];
                                                let oldlength: PCRE2_SIZE =
                                                    *ovector.add(1) - *ovector.add(0);

                                                buff_offset -= newlength;
                                                lengthleft += newlength;
                                                if replacement_only == 0 {
                                                    CHECKMEMCPY!(
                                                        subject.add(*ovector.add(0)),
                                                        oldlength
                                                    );
                                                }

                                                /* A negative return means do not do any more. */

                                                if rc < 0 {
                                                    suboptions &= !PCRE2_SUBSTITUTE_GLOBAL;
                                                }
                                            }
                                        }
                                        /* In this interesting case, we cannot do the callout, so
                                        it's hard to estimate the required buffer size. What callers
                                        want is to be able to make two calls to pcre2_substitute(),
                                        once with PCRE2_SUBSTITUTE_OVERFLOW_LENGTH to discover the
                                        buffer size, and then a second and final call. Older
                                        versions of PCRE2 violated this assumption, by proceding as
                                        if the callout had returned zero - but on the second call to
                                        pcre2_substitute() it could return non-zero and then
                                        overflow the buffer again. Callers probably don't want to
                                        keep on looping to incrementally discover the buffer
                                        size. */
                                        else {
                                            let newlength_buf: PCRE2_SIZE =
                                                buff_offset - scb.output_offsets[0];
                                            let newlength_extra: PCRE2_SIZE =
                                                extra_needed - sub_start_extra_needed;
                                            let newlength: PCRE2_SIZE = if newlength_extra
                                                > !(0 as PCRE2_SIZE) - newlength_buf
                                            {
                                                /* Integer overflow */
                                                !(0 as PCRE2_SIZE)
                                            } else {
                                                /* Cap the addition */
                                                newlength_buf + newlength_extra
                                            };
                                            let oldlength: PCRE2_SIZE =
                                                *ovector.add(1) - *ovector.add(0);

                                            /* Be pessimistic: request whichever buffer size is
                                            larger out of accepting or rejecting the
                                            substitution. */

                                            if oldlength > newlength {
                                                let additional: PCRE2_SIZE = oldlength - newlength;
                                                if additional > !(0 as PCRE2_SIZE) - extra_needed {
                                                    /* Integer overflow */
                                                    break 'toolargereplace;
                                                }
                                                extra_needed += additional;
                                            }

                                            /* Proceed as if the callout did not return a negative.
                                            A negative effectively rejects all future substitutions,
                                            but we want to examine them pessimistically. */
                                        }
                                    }

                                    /* Exit the global loop if we are not in global mode, or if
                                    pcre2_next_match() indicates we have reached the end of the
                                    subject. */

                                    if (suboptions & PCRE2_SUBSTITUTE_GLOBAL) == 0
                                        || pcre2_next_match_8(
                                            match_data,
                                            &mut start_offset,
                                            &mut goptions,
                                        ) == 0
                                    {
                                        start_offset = *ovector.add(1);
                                        break 'global_loop;
                                    }

                                    /* Verify that pcre2_next_match() has not done a bumpalong
                                    (because we have already returned PCRE2_ERROR_BADSUBSPATTERN
                                    for \K in lookarounds).

                                    We would otherwise have to memcpy the fragment spanning from
                                    ovector[1] to the new start_offset. */

                                    /* PCRE2_ASSERT(start_offset == ovector[1]); */
                                } /* End of global loop */

                                /* Copy the rest of the subject unless not required, and terminate
                                the output with a binary zero. */

                                if replacement_only == 0 {
                                    fraglength = length - start_offset;
                                    CHECKMEMCPY!(subject.add(start_offset), fraglength);
                                }

                                temp[0] = 0;
                                CHECKMEMCPY!(temp.as_ptr(), 1);

                                /* If overflowed is set it means the
                                PCRE2_SUBSTITUTE_OVERFLOW_LENGTH is set, and matching has carried on
                                after a full buffer, in order to compute the length needed.
                                Otherwise, an overflow generates an immediate error return. */

                                if overflowed != 0 {
                                    rc = PCRE2_ERROR_NOMEMORY;

                                    if extra_needed > !(0 as PCRE2_SIZE) - buff_length {
                                        /* Integer overflow */
                                        break 'toolargereplace;
                                    }
                                    *blength = buff_length + extra_needed;
                                }
                                /* After a successful execution, return the number of substitutions
                                and set the length of buffer used, excluding the trailing zero. */
                                else {
                                    rc = subs;
                                    *blength = buff_offset - 1;
                                }

                                break 'exit_all; /* Falls into EXIT in the C code */
                            }
                            /* NOROOM: */
                            rc = PCRE2_ERROR_NOMEMORY;
                            break 'exit_all;
                        }
                        /* CASEERROR: */
                        rc = PCRE2_ERROR_REPLACECASE;
                        break 'exit_all;
                    }
                    /* TOOLARGEREPLACE: */
                    rc = PCRE2_ERROR_TOOLARGEREPLACE;
                    break 'exit_all;
                }
                /* BAD: */
                rc = PCRE2_ERROR_BADREPLACEMENT;
                break 'ptrexit;
            }
            /* BADESCAPE: */
            rc = PCRE2_ERROR_BADREPESCAPE;
            /* Falls into PTREXIT */
        }
        /* PTREXIT: */
        *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
        /* Falls into EXIT */
    }

    /* EXIT: */
    if !internal_match_data.is_null() {
        pcre2_match_data_free_8(internal_match_data);
    } else {
        (*match_data).rc = rc;
    }
    rc
}

/* End of pcre2_substitute.c */
