/* Translated from c_src/src/pcre2_compile.c lines 6067-8573 */

/*************************************************
*           Compile one branch                   *
*************************************************/

/* Scan the parsed pattern, compiling it into the a vector of PCRE2_UCHAR. If
the options are changed during the branch, the pointer is used to change the
external options bits. This function is always called in the real compile
phase, and also in the pre-compile phase, in order to find out the amount of
memory needed for the compiled pattern.

Arguments:
  optionsptr        pointer to the option bits
  xoptionsptr       pointer to the extra option bits
  codeptr           points to the pointer to the current code point
  pptrptr           points to the current parsed pattern pointer
  errorcodeptr      points to error code variable
  firstcuptr        place to put the first required code unit
  firstcuflagsptr   place to put the first code unit flags
  reqcuptr          place to put the last required code unit
  reqcuflagsptr     place to put the last required code unit flags
  bcptr             points to current branch chain
  open_caps         points to current capitem
  cb                contains pointers to tables etc.
  lengthptr         NULL during the real compile phase
                    points to length accumulator during pre-compile phase

Returns:            0 There has been an error
                   +1 Success, this branch must match at least one character
                   -1 Success, this branch may match an empty string
*/

unsafe fn compile_branch(
    optionsptr: *mut u32,
    xoptionsptr: *mut u32,
    codeptr: *mut *mut PCRE2_UCHAR,
    pptrptr: *mut *mut u32,
    errorcodeptr: *mut c_int,
    firstcuptr: *mut u32,
    firstcuflagsptr: *mut u32,
    reqcuptr: *mut u32,
    reqcuflagsptr: *mut u32,
    bcptr: *mut branch_chain,
    open_caps: *mut open_capitem,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    let mut bravalue: c_int = 0;
    let mut okreturn: c_int = -1;
    let mut group_return: c_int = 0;
    let mut repeat_min: u32 = 0;
    let mut repeat_max: u32 = 0; /* To please picky compilers */
    let mut greedy_default: u32;
    let mut greedy_non_default: u32;
    let mut repeat_type: u32 = 0;
    let mut op_type: u32 = 0;
    let mut options: u32 = *optionsptr; /* May change dynamically */
    let mut xoptions: u32 = *xoptionsptr; /* May change dynamically */
    let mut firstcu: u32;
    let mut reqcu: u32;
    let mut zeroreqcu: u32;
    let mut zerofirstcu: u32;
    let mut pptr: *mut u32 = *pptrptr;
    let mut meta: u32;
    let mut meta_arg: u32;
    let mut firstcuflags: u32;
    let mut reqcuflags: u32;
    let mut zeroreqcuflags: u32;
    let mut zerofirstcuflags: u32;
    let mut req_caseopt: u32;
    let mut reqvary: u32 = 0;
    let mut tempreqvary: u32 = 0;
    /* Some opcodes, such as META_CAPTURE_NUMBER or META_CAPTURE_NAME,
    depends on the previous value of offset. */
    let mut offset: PCRE2_SIZE = 0;
    let mut length_prevgroup: PCRE2_SIZE = 0;
    let mut code: *mut PCRE2_UCHAR = *codeptr;
    let mut last_code: *mut PCRE2_UCHAR = code;
    let orig_code: *mut PCRE2_UCHAR = code;
    let mut tempcode: *mut PCRE2_UCHAR = std::ptr::null_mut();
    let mut previous: *mut PCRE2_UCHAR = std::ptr::null_mut();
    let mut op_previous: PCRE2_UCHAR = 0;
    let mut groupsetfirstcu: BOOL = FALSE;
    let mut had_accept: BOOL = FALSE;
    let mut matched_char: BOOL = FALSE;
    let mut previous_matched_char: BOOL = FALSE;
    let mut reset_caseful: BOOL = FALSE;

    /* We can fish out the UTF setting once and for all into a BOOL, but we must
    not do this for other options (e.g. PCRE2_EXTENDED) that may change dynamically
    as we process the pattern. */

    let utf: BOOL = ((options & PCRE2_UTF) != 0) as BOOL;
    let ucp: BOOL = ((options & PCRE2_UCP) != 0) as BOOL;

    /* Set up the default and non-default settings for greediness */

    greedy_default = ((options & PCRE2_UNGREEDY) != 0) as u32;
    greedy_non_default = greedy_default ^ 1;

    /* Initialize no first unit, no required unit. REQ_UNSET means "no char
    matching encountered yet". It gets changed to REQ_NONE if we hit something that
    matches a non-fixed first unit; reqcu just remains unset if we never find one.

    When we hit a repeat whose minimum is zero, we may have to adjust these values
    to take the zero repeat into account. This is implemented by setting them to
    zerofirstcu and zeroreqcu when such a repeat is encountered. The individual
    item types that can be repeated set these backoff variables appropriately. */

    firstcu = 0;
    reqcu = 0;
    zerofirstcu = 0;
    zeroreqcu = 0;
    firstcuflags = REQ_UNSET as u32;
    reqcuflags = REQ_UNSET as u32;
    zerofirstcuflags = REQ_UNSET as u32;
    zeroreqcuflags = REQ_UNSET as u32;

    /* The variable req_caseopt contains either the REQ_CASELESS bit or zero,
    according to the current setting of the caseless flag. The REQ_CASELESS value
    leaves the lower 28 bit empty. It is added into the firstcu or reqcu variables
    to record the case status of the value. This is used only for ASCII characters.
    */

    req_caseopt = if (options & PCRE2_CASELESS) != 0 {
        REQ_CASELESS as u32
    } else {
        0
    };

    /* Switch on next META item until the end of the branch */

    loop {
        let mut possessive_quantifier: BOOL = FALSE;
        let mut note_group_empty: BOOL;
        let mut mclength: u32 = 0;
        let mut skipunits: u32;
        let mut subreqcu: u32 = 0;
        let mut subfirstcu: u32 = 0;
        let mut groupnumber: u32;
        let mut verbarglen: u32;
        let mut verbculen: u32;
        let mut subreqcuflags: u32 = 0;
        let mut subfirstcuflags: u32 = 0;
        let mut oc: *mut open_capitem;
        let mut mcbuffer: [PCRE2_UCHAR; 8] = [0; 8];
        /* Hoisted out of the inner blocks so that they survive the labelled-block
        boundaries that replace the C gotos. */
        let mut prop_type: c_int = 0;
        let mut prop_value: c_int = 0;
        let mut oldcode: *mut PCRE2_UCHAR = std::ptr::null_mut();

        /* Get next META item in the pattern and its potential argument. */

        meta = META_CODE!(*pptr);
        meta_arg = META_DATA!(*pptr);

        /* If we are in the pre-compile phase, accumulate the length used for the
        previous cycle of this loop, unless the next item is a quantifier. */

        if !lengthptr.is_null() {
            /* LCOV_EXCL_START */
            if code >= (*cb).start_workspace.add((*cb).workspace_size) {
                /* PCRE2_DEBUG_UNREACHABLE(); */
                *errorcodeptr = ERR52; /* Over-ran workspace - internal error */
                (*cb).erroroffset = 0;
                return 0;
            }
            /* LCOV_EXCL_STOP */

            if code
                > (*cb)
                    .start_workspace
                    .add((*cb).workspace_size - WORK_SIZE_SAFETY_MARGIN as usize)
            /* Check for overrun */
            {
                *errorcodeptr = ERR86; /* Pattern too complicated */
                (*cb).erroroffset = 0;
                return 0;
            }

            /* There is at least one situation where code goes backwards: this is the
            case of a zero quantifier after a class (e.g. [ab]{0}). When the quantifier
            is processed, the whole class is eliminated. However, it is created first,
            so we have to allow memory for it. Therefore, don't ever reduce the length
            at this point. */

            if code < last_code {
                code = last_code;
            }

            /* If the next thing is not a quantifier, we add the length of the previous
            item into the total, and reset the code pointer to the start of the
            workspace. Otherwise leave the previous item available to be quantified. */

            if meta < META_ASTERISK || meta > META_MINMAX_QUERY {
                if (OFLOW_MAX as PCRE2_SIZE).wrapping_sub(*lengthptr)
                    < code.offset_from(orig_code) as PCRE2_SIZE
                {
                    *errorcodeptr = ERR20; /* Integer overflow */
                    (*cb).erroroffset = 0;
                    return 0;
                }
                *lengthptr += code.offset_from(orig_code) as PCRE2_SIZE;
                if *lengthptr > MAX_PATTERN_SIZE {
                    *errorcodeptr = ERR20; /* Pattern is too large */
                    (*cb).erroroffset = 0;
                    return 0;
                }
                code = orig_code;
            }

            /* Remember where this code item starts so we can catch the "backwards"
            case above next time round. */

            last_code = code;
        }

        /* Process the next parsed pattern item. If it is not a quantifier, remember
        where it starts so that it can be quantified when a quantifier follows.
        Checking for the legality of quantifiers happens in parse_regex(), except for
        a quantifier after an assertion that is a condition. */

        if meta < META_ASTERISK || meta > META_MINMAX_QUERY {
            previous = code;
            if matched_char != 0 && had_accept == 0 {
                okreturn = 1;
            }
        }

        previous_matched_char = matched_char;
        matched_char = FALSE;
        note_group_empty = FALSE;
        skipunits = 0; /* Default value for most subgroups */

        /* switch(meta) -- the C labels inside the switch become nested labelled
        blocks; the innermost block corresponds to the earliest label. Breaking out
        of the outermost block ('switch_break) is the C `break` of the switch. */

        'switch_break: {
            'class_caseless_char: {
                'normal_char_set: {
                    'normal_char: {
                        'handle_numerical_recursion: {
                            'handle_single_reference: {
                                'end_repeat: {
                                    'after_switchb: {
                                        'after_prop: {
                                            'output_single_repeat: {
                                                'repeat: {
                                                    'group_process: {
                                                        'group_process_note_empty: {
                                                            'verb_arg: {
                                                                'class_end_processing: {

/* ------------------------------------------------------------------------- */
/* The switch dispatch. Every C `case` body lives here; the shared code that
follows a C label lives after the corresponding block closes. */
/* ------------------------------------------------------------------------- */

if meta == META_END || meta == META_ALT || meta == META_KET {
    /* ===================================================================*/
    /* The branch terminates at pattern end or | or ) */

    *firstcuptr = firstcu;
    *firstcuflagsptr = firstcuflags;
    *reqcuptr = reqcu;
    *reqcuflagsptr = reqcuflags;
    *codeptr = code;
    *pptrptr = pptr;
    return okreturn;
}
/* ===================================================================*/
/* Handle single-character metacharacters. In multiline mode, ^ disables
the setting of any following char as a first character. */
else if meta == META_CIRCUMFLEX {
    if (options & PCRE2_MULTILINE) != 0 {
        if firstcuflags == REQ_UNSET as u32 {
            firstcuflags = REQ_NONE as u32;
            zerofirstcuflags = REQ_NONE as u32;
        }
        *code = OP_CIRCM as u8;
        code = code.add(1);
    } else {
        *code = OP_CIRC as u8;
        code = code.add(1);
    }
    break 'switch_break;
} else if meta == META_DOLLAR {
    *code = if (options & PCRE2_MULTILINE) != 0 {
        OP_DOLLM as u8
    } else {
        OP_DOLL as u8
    };
    code = code.add(1);
    break 'switch_break;
}
/* There can never be a first char if '.' is first, whatever happens about
repeats. The value of reqcu doesn't change either. */
else if meta == META_DOT {
    matched_char = TRUE;
    if firstcuflags == REQ_UNSET as u32 {
        firstcuflags = REQ_NONE as u32;
    }
    zerofirstcu = firstcu;
    zerofirstcuflags = firstcuflags;
    zeroreqcu = reqcu;
    zeroreqcuflags = reqcuflags;
    *code = if (options & PCRE2_DOTALL) != 0 {
        OP_ALLANY as u8
    } else {
        OP_ANY as u8
    };
    code = code.add(1);
    break 'switch_break;
}
/* ===================================================================*/
/* Empty character classes are allowed if PCRE2_ALLOW_EMPTY_CLASS is set.
Otherwise, an initial ']' is taken as a data character. When empty classes
are allowed, [] must generate an empty class - we have no dedicated opcode
to optimise the representation, but it's a rare case (the '(*FAIL)'
construct would be a clearer way for a pattern author to represent a
non-matching branch, but it does have different semantics to '[]' if both
are followed by a quantifier). The empty-negated [^] matches any character,
so is useful: generate OP_ALLANY for this. */
else if meta == META_CLASS_EMPTY || meta == META_CLASS_EMPTY_NOT {
    matched_char = TRUE;
    if meta == META_CLASS_EMPTY_NOT {
        *code = OP_ALLANY as u8;
        code = code.add(1);
    } else {
        *code = OP_CLASS as u8;
        code = code.add(1);
        memset(code as *mut c_void, 0, 32);
        code = code.add(32);
    }

    if firstcuflags == REQ_UNSET as u32 {
        firstcuflags = REQ_NONE as u32;
    }
    zerofirstcu = firstcu;
    zerofirstcuflags = firstcuflags;
    break 'switch_break;
}
/* ===================================================================*/
/* Non-empty character class. If the included characters are all < 256, we
build a 32-byte bitmap of the permitted characters, except in the special
case where there is only one such character. For negated classes, we build
the map as usual, then invert it at the end. However, we use a different
opcode so that data characters > 255 can be handled correctly.

If the class contains characters outside the 0-255 range, a different
opcode is compiled. It may optionally have a bit map for characters < 256,
but those above are explicitly listed afterwards. A flag code unit tells
whether the bitmap is present, and whether this is a negated class or
not. */
else if meta == META_CLASS_NOT || meta == META_CLASS {
    matched_char = TRUE;

    /* Check for complex extended classes and handle them separately. */

    if (*pptr & CLASS_IS_ECLASS) != 0 {
        if _pcre2_compile_class_nested_8(
            options,
            xoptions,
            &mut pptr,
            &mut code,
            errorcodeptr,
            cb,
            lengthptr,
        ) == 0
        {
            return 0;
        }
        break 'class_end_processing;
    }

    /* We can optimize the case of a single character in a class by generating
    OP_CHAR or OP_CHARI if it's positive, or OP_NOT or OP_NOTI if it's
    negative. In the negative case there can be no first char if this item is
    first, whatever repeat count may follow. In the case of reqcu, save the
    previous value for reinstating. */

    if *pptr.add(1) < META_END && *pptr.add(2) == META_CLASS_END {
        let c: u32 = *pptr.add(1);

        pptr = pptr.add(2); /* Move on to class end */
        if meta == META_CLASS
        /* A positive one-char class can be */
        {
            /* handled as a normal literal character. */
            meta = c; /* Set up the character */
            break 'normal_char_set;
        }

        /* Handle a negative one-character class */

        zeroreqcu = reqcu;
        zeroreqcuflags = reqcuflags;
        if firstcuflags == REQ_UNSET as u32 {
            firstcuflags = REQ_NONE as u32;
        }
        zerofirstcu = firstcu;
        zerofirstcuflags = firstcuflags;

        /* For caseless UTF or UCP mode, check whether this character has more
        than one other case. If so, generate a special OP_NOTPROP item instead of
        OP_NOTI. When restricted by PCRE2_EXTRA_CASELESS_RESTRICT, ignore any
        caseless set that starts with an ASCII character. If the character is
        affected by the special Turkish rules, hardcode the not-matching
        characters using a caseset. */

        if (utf != 0 || ucp != 0) && (options & PCRE2_CASELESS) != 0 {
            let mut caseset: u32;

            if (xoptions & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                == PCRE2_EXTRA_TURKISH_CASING
                && UCD_ANY_I(c)
            {
                caseset = _pcre2_ucd_turkish_dotted_i_caseset_8
                    + (if UCD_DOTTED_I(c) { 0 } else { 3 });
            } else {
                caseset = UCD_CASESET(c);
                if caseset != 0
                    && (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                    && *_pcre2_ucd_caseless_sets_8.as_ptr().add(caseset as usize) < 128
                {
                    caseset = 0; /* Ignore the caseless set if it's restricted. */
                }
            }

            if caseset != 0 {
                *code = OP_NOTPROP as u8;
                code = code.add(1);
                *code = PT_CLIST as u8;
                code = code.add(1);
                *code = caseset as u8;
                code = code.add(1);
                break 'switch_break; /* We are finished with this class */
            }
        }

        /* Char has only one other (usable) case, or UCP not available */

        *code = if (options & PCRE2_CASELESS) != 0 {
            OP_NOTI as u8
        } else {
            OP_NOT as u8
        };
        code = code.add(1);
        code = code.add(PUTCHAR!(c, code, utf) as usize);
        break 'switch_break; /* We are finished with this class */
    } /* End of 1-char optimization */

    /* Handle character classes that contain more than just one literal
    character. If there are exactly two characters in a positive class, see if
    they are case partners. This can be optimized to generate a caseless single
    character match (which also sets first/required code units if relevant).
    When casing restrictions apply, ignore a caseless set if both characters
    are ASCII. When Turkish casing applies, an 'i' does not match its normal
    Unicode "othercase". */

    if meta == META_CLASS
        && *pptr.add(1) < META_END
        && *pptr.add(2) < META_END
        && *pptr.add(3) == META_CLASS_END
    {
        let c: u32 = *pptr.add(1);

        if (UCD_CASESET(c) == 0
            || ((xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                && c < 128
                && *pptr.add(2) < 128))
            && !((xoptions & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                == PCRE2_EXTRA_TURKISH_CASING
                && UCD_ANY_I(c))
        {
            let d: u32;

            if (utf != 0 || ucp != 0) && c > 127 {
                d = UCD_OTHERCASE(c);
            } else {
                d = TABLE_GET!(c, (*cb).fcc, c) as u32;
            }

            if c != d && *pptr.add(2) == d {
                pptr = pptr.add(3); /* Move on to class end */
                meta = c;
                if (options & PCRE2_CASELESS) == 0 {
                    reset_caseful = TRUE;
                    options |= PCRE2_CASELESS;
                    req_caseopt = REQ_CASELESS as u32;
                }
                break 'class_caseless_char;
            }
        }
    }

    /* Now emit the OP_CLASS/OP_NCLASS/OP_XCLASS/OP_ALLANY opcode. */

    pptr = _pcre2_compile_class_not_nested_8(
        options,
        xoptions,
        pptr.add(1),
        &mut code,
        (meta == META_CLASS_NOT) as BOOL,
        std::ptr::null_mut(),
        errorcodeptr,
        cb,
        lengthptr,
    );
    if pptr.is_null() {
        return 0;
    }
    /* PCRE2_ASSERT(*pptr == META_CLASS_END); */

    break 'class_end_processing;
}
/* ===================================================================*/
/* Deal with (*VERB)s. */

/* Check for open captures before ACCEPT and close those that are within
the same assertion level, also converting ACCEPT to ASSERT_ACCEPT in an
assertion. In the first pass, just accumulate the length required;
otherwise hitting (*ACCEPT) inside many nested parentheses can cause
workspace overflow. Do not set firstcu after *ACCEPT. */
else if meta == META_ACCEPT {
    (*cb).had_accept = TRUE;
    had_accept = TRUE;
    oc = open_caps;
    while !oc.is_null() && (*oc).assert_depth >= (*cb).assert_depth {
        if !lengthptr.is_null() {
            *lengthptr += 1 + IMM2_SIZE;
        } else {
            *code = OP_CLOSE as u8;
            code = code.add(1);
            PUT2INC!(code, 0, (*oc).number as u32);
        }
        oc = (*oc).next;
    }
    *code = if (*cb).assert_depth > 0 {
        OP_ASSERT_ACCEPT as u8
    } else {
        OP_ACCEPT as u8
    };
    code = code.add(1);
    if firstcuflags == REQ_UNSET as u32 {
        firstcuflags = REQ_NONE as u32;
    }
    break 'switch_break;
} else if meta == META_PRUNE || meta == META_SKIP {
    (*cb).had_pruneorskip = TRUE;
    /* Fall through */
    *code = *verbops.as_ptr().add(((meta - META_MARK) >> 16) as usize) as u8;
    code = code.add(1);
    break 'switch_break;
} else if meta == META_COMMIT || meta == META_FAIL {
    *code = *verbops.as_ptr().add(((meta - META_MARK) >> 16) as usize) as u8;
    code = code.add(1);
    break 'switch_break;
} else if meta == META_THEN {
    (*cb).external_flags |= PCRE2_HASTHEN;
    *code = OP_THEN as u8;
    code = code.add(1);
    break 'switch_break;
}
/* Handle verbs with arguments. Arguments can be very long, especially in
16- and 32-bit modes, and can overflow the workspace in the first pass.
However, the argument length is constrained to be small enough to fit in
one code unit. This check happens in parse_regex(). In the first pass,
instead of putting the argument into memory, we just update the length
counter and set up an empty argument. */
else if meta == META_THEN_ARG {
    (*cb).external_flags |= PCRE2_HASTHEN;
    break 'verb_arg;
} else if meta == META_PRUNE_ARG || meta == META_SKIP_ARG {
    (*cb).had_pruneorskip = TRUE;
    /* Fall through */
    break 'verb_arg;
} else if meta == META_MARK || meta == META_COMMIT_ARG {
    break 'verb_arg;
}
/* ===================================================================*/
/* Handle options change. The new setting must be passed back for use in
subsequent branches. Reset the greedy defaults and the case value for
firstcu and reqcu. */
else if meta == META_OPTIONS {
    pptr = pptr.add(1);
    options = *pptr;
    *optionsptr = options;
    pptr = pptr.add(1);
    xoptions = *pptr;
    *xoptionsptr = xoptions;
    greedy_default = ((options & PCRE2_UNGREEDY) != 0) as u32;
    greedy_non_default = greedy_default ^ 1;
    req_caseopt = if (options & PCRE2_CASELESS) != 0 {
        REQ_CASELESS as u32
    } else {
        0
    };
    break 'switch_break;
}
/* ===================================================================*/
/* Handle scan substring. Scan substring assertion starts with META_SCS,
which recursively calls compile_branch. The first opcode processed by
this recursive call is always META_OFFSET. */
else if meta == META_OFFSET {
    if !lengthptr.is_null() {
        pptr = _pcre2_compile_parse_scan_substr_args8(pptr, errorcodeptr, cb, lengthptr);
        if pptr.is_null() {
            return 0;
        }
        break 'switch_break;
    }

    loop {
        let mut count: c_int;
        let mut index: c_int;
        let ng: *mut named_group;

        let mc: u32 = META_CODE!(*pptr);

        if mc == META_OFFSET {
            pptr = pptr.add(1);
            SKIPOFFSET!(pptr);
            continue;
        } else if mc == META_CAPTURE_NAME {
            ng = (*cb).named_groups.add(*pptr.add(1) as usize);
            pptr = pptr.add(2);
            count = 0;
            index = 0;

            if _pcre2_compile_find_dupname_details8(
                (*ng).name,
                (*ng).length as u32,
                &mut index,
                &mut count,
                errorcodeptr,
                cb,
            ) == 0
            {
                return 0;
            }

            *code.add(0) = OP_DNCREF as u8;
            PUT2!(code, 1, index);
            PUT2!(code, 1 + IMM2_SIZE, count);
            code = code.add(1 + 2 * IMM2_SIZE);
            continue;
        } else if mc == META_CAPTURE_NUMBER {
            pptr = pptr.add(2);
            if *pptr.offset(-1) == 0 {
                continue;
            }

            *code.add(0) = OP_CREF as u8;
            PUT2!(code, 1, *pptr.offset(-1));
            code = code.add(1 + IMM2_SIZE);
            continue;
        } else {
            /* default: break out of the inner switch */
        }

        break;
    }
    pptr = pptr.sub(1);
    break 'switch_break;
} else if meta == META_SCS {
    bravalue = OP_ASSERT_SCS as c_int;
    (*cb).assert_depth += 1;
    break 'group_process;
}
/* ===================================================================*/
/* Handle conditional subpatterns. The case of (?(Rdigits) is ambiguous
because it could be a numerical check on recursion, or a name check on a
group's being set. The pre-pass sets up META_COND_RNUMBER as a name so that
we can handle it either way. We first try for a name; if not found, process
the number. */
else if meta == META_COND_RNUMBER   /* (?(Rdigits) */
    || meta == META_COND_NAME       /* (?(name) or (?'name') or ?(<name>) */
    || meta == META_COND_RNAME
/* (?(R&name) - test for recursion */
{
    bravalue = OP_COND as c_int;

    if !lengthptr.is_null() {
        let mut i: u32;
        let name: PCRE2_SPTR;
        let ng: *mut named_group;
        let start_pptr: *mut u32 = pptr;
        pptr = pptr.add(1);
        let length: u32 = *pptr;

        GETPLUSOFFSET!(offset, pptr);
        name = (*cb).start_pattern.add(offset);

        /* In the first pass, the names generated in the pre-pass are available,
        but the main name table has not yet been created. Scan the list of names
        generated in the pre-pass in order to get a number and whether or not
        this name is duplicated. If it is not duplicated, we can handle it as a
        numerical group. */

        ng = _pcre2_compile_find_named_group8(name, length, cb);

        if ng.is_null() {
            /* If the name was not found we have a bad reference, unless we are
            dealing with R<digits>, which is treated as a recursion test by
            number. */

            groupnumber = 0;
            if meta == META_COND_RNUMBER {
                i = 1;
                while i < length {
                    groupnumber = groupnumber * 10 + (*name.add(i as usize) as u32 - CHAR_0);
                    if groupnumber > MAX_GROUP_NUMBER as u32 {
                        *errorcodeptr = ERR61;
                        (*cb).erroroffset = offset + i as PCRE2_SIZE;
                        return 0;
                    }
                    i += 1;
                }
            }

            if meta != META_COND_RNUMBER || groupnumber > (*cb).bracount {
                *errorcodeptr = ERR15;
                (*cb).erroroffset = offset;
                return 0;
            }

            /* (?Rdigits) treated as a recursion reference by number. A value of
            zero (which is the result of both (?R) and (?R0)) means "any", and is
            translated into RREF_ANY (which is 0xffff). */

            if groupnumber == 0 {
                groupnumber = RREF_ANY;
            }
            /* PCRE2_ASSERT(start_pptr[0] == META_COND_RNUMBER); */
            *start_pptr.add(1) = groupnumber;
            skipunits = 1 + IMM2_SIZE as u32;
            break 'group_process_note_empty;
        }

        /* From here on, we know we have a name (not a number),
        so treat META_COND_RNUMBER the same as META_COND_NAME. */
        if meta == META_COND_RNUMBER {
            meta = META_COND_NAME;
        }

        if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) == 0 {
            /* Found a non-duplicated name. Since it is a global,
            it is enough to update it in the pre-processing phase. */
            if (*ng).number > (*cb).top_backref {
                (*cb).top_backref = (*ng).number;
            }

            *start_pptr.add(0) = meta;
            *start_pptr.add(1) = (*ng).number;

            skipunits = 1 + IMM2_SIZE as u32;
            break 'group_process_note_empty;
        }

        /* We have a duplicated name. In the compile pass we have to search the
        main table in order to get the index and count values. */

        *start_pptr.add(0) = meta | 1;
        *start_pptr.add(1) = ng.offset_from((*cb).named_groups) as u32;

        /* A duplicated name was found. Note that if an R<digits> name is found
        (META_COND_RNUMBER), it is a reference test, not a recursion test. */
        skipunits = 1 + 2 * IMM2_SIZE as u32;
    } else {
        /* Otherwise lengthptr equals to NULL,
        which is the second phase of compilation. */
        let mut count: c_int;
        let mut index: c_int;
        let ng: *mut named_group;

        /* Generate code using the data
        collected in the pre-processing phase. */

        if meta == META_COND_RNUMBER {
            *code.add(1 + LINK_SIZE) = OP_RREF as u8;
            PUT2!(code, 2 + LINK_SIZE, *pptr.add(1));
            skipunits = 1 + IMM2_SIZE as u32;
            pptr = pptr.add(1 + SIZEOFFSET);
            break 'group_process_note_empty;
        }

        if meta_arg == 0 {
            *code.add(1 + LINK_SIZE) = if meta == META_COND_RNAME {
                OP_RREF as u8
            } else {
                OP_CREF as u8
            };
            PUT2!(code, 2 + LINK_SIZE, *pptr.add(1));
            skipunits = 1 + IMM2_SIZE as u32;
            pptr = pptr.add(1 + SIZEOFFSET);
            break 'group_process_note_empty;
        }

        ng = (*cb).named_groups.add(*pptr.add(1) as usize);
        count = 0; /* Values for first pass (avoids compiler warning) */
        index = 0;

        /* The failed case is an internal error. */
        if _pcre2_compile_find_dupname_details8(
            (*ng).name,
            (*ng).length as u32,
            &mut index,
            &mut count,
            errorcodeptr,
            cb,
        ) == 0
        {
            return 0;
        }

        /* A duplicated name was found. Note that if an R<digits> name is found
        (META_COND_RNUMBER), it is a reference test, not a recursion test. */

        *code.add(1 + LINK_SIZE) = if meta == META_COND_RNAME {
            OP_DNRREF as u8
        } else {
            OP_DNCREF as u8
        };

        /* Insert appropriate data values. */
        PUT2!(code, 2 + LINK_SIZE, index);
        PUT2!(code, 2 + LINK_SIZE + IMM2_SIZE, count);
        skipunits = 1 + 2 * IMM2_SIZE as u32;
        pptr = pptr.add(1 + SIZEOFFSET);
    }

    /* PCRE2_ASSERT(meta != META_CAPTURE_NAME); */
    break 'group_process_note_empty;
}
/* The DEFINE condition is always false. Its internal groups may never
be called, so matched_char must remain false, hence the jump to
GROUP_PROCESS rather than GROUP_PROCESS_NOTE_EMPTY. */
else if meta == META_COND_DEFINE {
    bravalue = OP_COND as c_int;
    GETPLUSOFFSET!(offset, pptr);
    *code.add(1 + LINK_SIZE) = OP_DEFINE as u8;
    skipunits = 1;
    break 'group_process;
}
/* Conditional test of a group's being set. */
else if meta == META_COND_NUMBER {
    bravalue = OP_COND as c_int;
    GETPLUSOFFSET!(offset, pptr);

    pptr = pptr.add(1);
    groupnumber = *pptr;
    if groupnumber > (*cb).bracount {
        *errorcodeptr = ERR15;
        (*cb).erroroffset = offset;
        return 0;
    }
    if groupnumber > (*cb).top_backref {
        (*cb).top_backref = groupnumber;
    }

    /* Point at initial ( for too many branches error */
    offset -= 2;
    *code.add(1 + LINK_SIZE) = OP_CREF as u8;
    skipunits = 1 + IMM2_SIZE as u32;
    PUT2!(code, 2 + LINK_SIZE, groupnumber);
    break 'group_process_note_empty;
}
/* Test for the PCRE2 version. */
else if meta == META_COND_VERSION {
    bravalue = OP_COND as c_int;
    if *pptr.add(1) > 0 {
        *code.add(1 + LINK_SIZE) = if (PCRE2_MAJOR as u32) > *pptr.add(2)
            || (PCRE2_MAJOR as u32 == *pptr.add(2) && PCRE2_MINOR as u32 >= *pptr.add(3))
        {
            OP_TRUE as u8
        } else {
            OP_FALSE as u8
        };
    } else {
        *code.add(1 + LINK_SIZE) =
            if PCRE2_MAJOR as u32 == *pptr.add(2) && PCRE2_MINOR as u32 == *pptr.add(3) {
                OP_TRUE as u8
            } else {
                OP_FALSE as u8
            };
    }
    skipunits = 1;
    pptr = pptr.add(3);
    break 'group_process_note_empty;
}
/* The condition is an assertion, possibly preceded by a callout. */
else if meta == META_COND_ASSERT {
    bravalue = OP_COND as c_int;
    break 'group_process_note_empty;
}
/* ===================================================================*/
/* Handle all kinds of nested bracketed groups. The non-capturing,
non-conditional cases are here; others come to GROUP_PROCESS via goto. */
else if meta == META_LOOKAHEAD {
    bravalue = OP_ASSERT as c_int;
    (*cb).assert_depth += 1;
    break 'group_process;
} else if meta == META_LOOKAHEAD_NA {
    bravalue = OP_ASSERT_NA as c_int;
    (*cb).assert_depth += 1;
    break 'group_process;
}
/* Optimize (?!) to (*FAIL) unless it is quantified - which is a weird
thing to do, but Perl allows all assertions to be quantified, and when
they contain capturing parentheses there may be a potential use for
this feature. Not that that applies to a quantified (?!) but we allow
it for uniformity. */
else if meta == META_LOOKAHEADNOT {
    if *pptr.add(1) == META_KET
        && (*pptr.add(2) < META_ASTERISK || *pptr.add(2) > META_MINMAX_QUERY)
    {
        *code = OP_FAIL as u8;
        code = code.add(1);
        pptr = pptr.add(1);
    } else {
        bravalue = OP_ASSERT_NOT as c_int;
        (*cb).assert_depth += 1;
        break 'group_process;
    }
    break 'switch_break;
} else if meta == META_LOOKBEHIND {
    bravalue = OP_ASSERTBACK as c_int;
    (*cb).assert_depth += 1;
    break 'group_process;
} else if meta == META_LOOKBEHINDNOT {
    bravalue = OP_ASSERTBACK_NOT as c_int;
    (*cb).assert_depth += 1;
    break 'group_process;
} else if meta == META_LOOKBEHIND_NA {
    bravalue = OP_ASSERTBACK_NA as c_int;
    (*cb).assert_depth += 1;
    break 'group_process;
} else if meta == META_ATOMIC {
    bravalue = OP_ONCE as c_int;
    break 'group_process_note_empty;
} else if meta == META_SCRIPT_RUN {
    bravalue = OP_SCRIPT_RUN as c_int;
    break 'group_process_note_empty;
} else if meta == META_NOCAPTURE {
    bravalue = OP_BRA as c_int;
    /* Fall through to GROUP_PROCESS_NOTE_EMPTY */
    break 'group_process_note_empty;
}
/* ===================================================================*/
/* Handle named backreferences and recursions. */
else if meta == META_BACKREF_BYNAME || meta == META_RECURSE_BYNAME {
    {
        let mut count: c_int;
        let mut index: c_int;
        let name: PCRE2_SPTR;
        let ng: *mut named_group;
        pptr = pptr.add(1);
        let length: u32 = *pptr;

        GETPLUSOFFSET!(offset, pptr);
        name = (*cb).start_pattern.add(offset);

        /* In the first pass, the names generated in the pre-pass are available,
        but the main name table has not yet been created. Scan the list of names
        generated in the pre-pass in order to get a number and whether or not
        this name is duplicated. */

        ng = _pcre2_compile_find_named_group8(name, length, cb);

        if ng.is_null() {
            /* If the name was not found we have a bad reference. */
            *errorcodeptr = ERR15;
            (*cb).erroroffset = offset;
            return 0;
        }

        groupnumber = (*ng).number;

        /* For a recursion, that's all that is needed. We can now go to
        the code that handles numerical recursion, applying it to the first
        group with the given name. */

        if meta == META_RECURSE_BYNAME {
            meta_arg = groupnumber;
            break 'handle_numerical_recursion;
        }

        /* For a back reference, update the back reference map and the
        maximum back reference. */

        (*cb).backref_map |= if groupnumber < 32 {
            1u32 << groupnumber
        } else {
            1
        };
        if groupnumber > (*cb).top_backref {
            (*cb).top_backref = groupnumber;
        }

        /* If a back reference name is not duplicated, we can handle it as
        a numerical reference. */

        if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) == 0 {
            meta_arg = groupnumber;
            break 'handle_single_reference;
        }

        /* If a back reference name is duplicated, we generate a different
        opcode to a numerical back reference. In the second pass we must
        search for the index and count in the final name table. */

        count = 0; /* Values for first pass (avoids compiler warning) */
        index = 0;
        if lengthptr.is_null()
            && _pcre2_compile_find_dupname_details8(
                name,
                length,
                &mut index,
                &mut count,
                errorcodeptr,
                cb,
            ) == 0
        {
            return 0;
        }

        if firstcuflags == REQ_UNSET as u32 {
            firstcuflags = REQ_NONE as u32;
        }
        *code = if (options & PCRE2_CASELESS) != 0 {
            OP_DNREFI as u8
        } else {
            OP_DNREF as u8
        };
        code = code.add(1);
        PUT2INC!(code, 0, index);
        PUT2INC!(code, 0, count);
        if (options & PCRE2_CASELESS) != 0 {
            *code = ((if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0 {
                REFI_FLAG_CASELESS_RESTRICT
            } else {
                0
            }) | (if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0 {
                REFI_FLAG_TURKISH_CASING
            } else {
                0
            })) as u8;
            code = code.add(1);
        }
    }
    break 'switch_break;
}
/* ===================================================================*/
/* Handle a numerical callout. */
else if meta == META_CALLOUT_NUMBER {
    *code.add(0) = OP_CALLOUT as u8;
    PUT!(code, 1, *pptr.add(1)); /* Offset to next pattern item */
    PUT!(code, 1 + LINK_SIZE, *pptr.add(2)); /* Length of next pattern item */
    *code.add(1 + 2 * LINK_SIZE) = *pptr.add(3) as u8;
    pptr = pptr.add(3);
    code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(OP_CALLOUT as usize) as usize);
    break 'switch_break;
}
/* ===================================================================*/
/* Handle a callout with a string argument. In the pre-pass we just compute
the length without generating anything. The length in pptr[3] includes both
delimiters; in the actual compile only the first one is copied, but a
terminating zero is added. Any doubled delimiters within the string make
this an overestimate, but it is not worth bothering about. */
else if meta == META_CALLOUT_STRING {
    if !lengthptr.is_null() {
        *lengthptr += (*pptr.add(3)).wrapping_add(1 + 4 * LINK_SIZE as u32) as PCRE2_SIZE;
        pptr = pptr.add(3);
        SKIPOFFSET!(pptr);
    }
    /* In the real compile we can copy the string. The starting delimiter is
    included so that the client can discover it if they want. We also pass the
    start offset to help a script language give better error messages. */
    else {
        let mut pp: PCRE2_SPTR;
        let mut delimiter: u32;
        let mut length: u32 = *pptr.add(3);
        let mut callout_string: *mut PCRE2_UCHAR = code.add(1 + 4 * LINK_SIZE);

        *code.add(0) = OP_CALLOUT_STR as u8;
        PUT!(code, 1, *pptr.add(1)); /* Offset to next pattern item */
        PUT!(code, 1 + LINK_SIZE, *pptr.add(2)); /* Length of next pattern item */

        pptr = pptr.add(3);
        GETPLUSOFFSET!(offset, pptr); /* Offset to string in pattern */
        pp = (*cb).start_pattern.add(offset);
        *callout_string = *pp;
        delimiter = *callout_string as u32;
        callout_string = callout_string.add(1);
        pp = pp.add(1);
        if delimiter == CHAR_LEFT_CURLY_BRACKET {
            delimiter = CHAR_RIGHT_CURLY_BRACKET;
        }
        PUT!(code, 1 + 3 * LINK_SIZE, (offset + 1) as c_int); /* One after delimiter */

        /* The syntax of the pattern was checked in the parsing scan. The length
        includes both delimiters, but we have passed the opening one just above,
        so we reduce length before testing it. The test is for > 1 because we do
        not want to copy the final delimiter. This also ensures that pp[1] is
        accessible. */

        loop {
            length -= 1;
            if !(length > 1) {
                break;
            }
            if *pp as u32 == delimiter && *pp.add(1) as u32 == delimiter {
                *callout_string = delimiter as u8;
                callout_string = callout_string.add(1);
                pp = pp.add(2);
                length -= 1;
            } else {
                *callout_string = *pp;
                callout_string = callout_string.add(1);
                pp = pp.add(1);
            }
        }
        *callout_string = CHAR_NUL as u8;
        callout_string = callout_string.add(1);

        /* Set the length of the entire item, the advance to its end. */

        PUT!(
            code,
            1 + 2 * LINK_SIZE,
            callout_string.offset_from(code) as c_int
        );
        code = callout_string;
    }
    break 'switch_break;
}
/* ===================================================================*/
/* Handle repetition. The different types are all sorted out in the parsing
pass. */
else if meta == META_MINMAX_PLUS || meta == META_MINMAX_QUERY || meta == META_MINMAX {
    pptr = pptr.add(1);
    repeat_min = *pptr;
    pptr = pptr.add(1);
    repeat_max = *pptr;
    break 'repeat;
} else if meta == META_ASTERISK || meta == META_ASTERISK_PLUS || meta == META_ASTERISK_QUERY {
    repeat_min = 0;
    repeat_max = REPEAT_UNLIMITED as u32;
    break 'repeat;
} else if meta == META_PLUS || meta == META_PLUS_PLUS || meta == META_PLUS_QUERY {
    repeat_min = 1;
    repeat_max = REPEAT_UNLIMITED as u32;
    break 'repeat;
} else if meta == META_QUERY || meta == META_QUERY_PLUS || meta == META_QUERY_QUERY {
    repeat_min = 0;
    repeat_max = 1;
    /* Fall through to REPEAT */
    break 'repeat;
}
/* ===================================================================*/
/* Handle a 32-bit data character with a value greater than META_END. */
else if meta == META_BIGVALUE {
    pptr = pptr.add(1);
    break 'normal_char;
}
/* ===============================================================*/
/* Handle a back reference by number, which is the meta argument. The
pattern offsets for back references to group numbers less than 10 are held
in a special vector, to avoid using more than two parsed pattern elements
in 64-bit environments. We only need the offset to the first occurrence,
because if that doesn't fail, subsequent ones will also be OK. */
else if meta == META_BACKREF {
    if meta_arg < 10 {
        offset = *(*cb).small_ref_offset.as_ptr().add(meta_arg as usize);
    } else {
        GETPLUSOFFSET!(offset, pptr);
    }

    if meta_arg > (*cb).bracount {
        (*cb).erroroffset = offset;
        *errorcodeptr = ERR15; /* Non-existent subpattern */
        return 0;
    }

    /* Fall through to HANDLE_SINGLE_REFERENCE */
    break 'handle_single_reference;
}
/* ===============================================================*/
/* Handle recursion by inserting the number of the called group (which is
the meta argument) after OP_RECURSE. At the end of compiling the pattern is
scanned and these numbers are replaced by offsets within the pattern. It is
done like this to avoid problems with forward references and adjusting
offsets when groups are duplicated and moved (as discovered in previous
implementations). Note that a recursion does not have a set first
character. */
else if meta == META_RECURSE {
    GETPLUSOFFSET!(offset, pptr);
    if meta_arg > (*cb).bracount {
        (*cb).erroroffset = offset;
        *errorcodeptr = ERR15; /* Non-existent subpattern */
        return 0;
    }
    /* Fall through to HANDLE_NUMERICAL_RECURSION */
    break 'handle_numerical_recursion;
}
/* ===============================================================*/
/* Handle capturing parentheses; the number is the meta argument. */
else if meta == META_CAPTURE {
    bravalue = OP_CBRA as c_int;
    skipunits = IMM2_SIZE as u32;
    PUT2!(code, 1 + LINK_SIZE, meta_arg);
    (*cb).lastcapture = meta_arg;
    break 'group_process_note_empty;
}
/* ===============================================================*/
/* Handle escape sequence items. For ones like \d, the ESC_values are
arranged to be the same as the corresponding OP_values in the default case
when PCRE2_UCP is not set (which is the only case in which they will appear
here).

Note: \Q and \E are never seen here, as they were dealt with in
parse_pattern(). Neither are numerical back references or recursions, which
were turned into META_BACKREF or META_RECURSE items, respectively. \k and
\g, when followed by names, are turned into META_BACKREF_BYNAME or
META_RECURSE_BYNAME. */
else if meta == META_ESCAPE {
    /* We can test for escape sequences that consume a character because their
    values lie between ESC_b and ESC_Z; this may have to change if any new ones
    are ever created. For these sequences, we disable the setting of a first
    character if it hasn't already been set. */

    if meta_arg > ESC_b as u32 && meta_arg < ESC_Z as u32 {
        matched_char = TRUE;
        if firstcuflags == REQ_UNSET as u32 {
            firstcuflags = REQ_NONE as u32;
        }
    }

    /* Set values to reset to if this is followed by a zero repeat. */

    zerofirstcu = firstcu;
    zerofirstcuflags = firstcuflags;
    zeroreqcu = reqcu;
    zeroreqcuflags = reqcuflags;

    /* If Unicode is not supported, \P and \p are not allowed and are
    faulted at parse time, so will never appear here. */

    if meta_arg == ESC_P as u32 || meta_arg == ESC_p as u32 {
        pptr = pptr.add(1);
        let mut ptype: u32 = *pptr >> 16;
        let mut pdata: u32 = *pptr & 0xffff;

        /* In caseless matching, particular characteristics Lu, Ll, and Lt get
        converted to the general characteristic L&. That is, upper, lower, and
        title case letters are all conflated. */

        if (options & PCRE2_CASELESS) != 0
            && ptype == PT_PC
            && (pdata == ucp_Lu || pdata == ucp_Ll || pdata == ucp_Lt)
        {
            ptype = PT_LAMP;
            pdata = 0;
        }

        /* The special case of \p{Any} is compiled to OP_ALLANY and \P{Any}
        is compiled to [] so as to benefit from the auto-anchoring code. */

        if ptype == PT_ANY {
            if meta_arg == ESC_P as u32 {
                *code = OP_CLASS as u8;
                code = code.add(1);
                memset(code as *mut c_void, 0, 32);
                code = code.add(32);
            } else {
                *code = OP_ALLANY as u8;
                code = code.add(1);
            }
        } else {
            *code = if meta_arg == ESC_p as u32 {
                OP_PROP as u8
            } else {
                OP_NOTPROP as u8
            };
            code = code.add(1);
            *code = ptype as u8;
            code = code.add(1);
            *code = pdata as u8;
            code = code.add(1);
        }
        break 'switch_break; /* End META_ESCAPE */
    }

    /* \K is forbidden in lookarounds since 10.38 because that's what Perl has
    done. However, there's an option, in case anyone was relying on it. */

    if (*cb).assert_depth > 0
        && meta_arg == ESC_K as u32
        && (xoptions & PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK) == 0
    {
        *errorcodeptr = ERR99;
        return 0;
    }

    /* For the rest (including \X when Unicode is supported - if not it's
    faulted at parse time), the OP value is the escape value when PCRE2_UCP is
    not set; if it is set, most of them do not show up here because they are
    converted into Unicode property tests in parse_regex().

    In non-UTF mode, and for both 32-bit modes, we turn \C into OP_ALLANY
    instead of OP_ANYBYTE so that it works in DFA mode and in lookbehinds.
    There are special UCP codes for \B and \b which are used in UCP mode unless
    "word" matching is being forced to ASCII.

    Note that \b and \B do a one-character lookbehind, and \A also behaves as
    if it does. */

    if meta_arg == ESC_C as u32 {
        (*cb).external_flags |= PCRE2_HASBKC; /* Record */
        if utf == 0 {
            meta_arg = OP_ALLANY;
        }
    } else if meta_arg == ESC_B as u32 || meta_arg == ESC_b as u32 {
        if (options & PCRE2_UCP) != 0 && (xoptions & PCRE2_EXTRA_ASCII_BSW) == 0 {
            meta_arg = if meta_arg == ESC_B as u32 {
                OP_NOT_UCP_WORD_BOUNDARY
            } else {
                OP_UCP_WORD_BOUNDARY
            };
        }
        /* Fall through */
        if (*cb).max_lookbehind == 0 {
            (*cb).max_lookbehind = 1;
        }
    } else if meta_arg == ESC_A as u32 {
        if (*cb).max_lookbehind == 0 {
            (*cb).max_lookbehind = 1;
        }
    } else if meta_arg == ESC_K as u32 {
        (*cb).external_flags |= PCRE2_HASBSK; /* Record */
    }

    *code = meta_arg as u8;
    code = code.add(1);
    break 'switch_break; /* End META_ESCAPE */
}
/* ===================================================================*/
/* Handle an unrecognized meta value. A parsed pattern value less than
META_END is a literal. Otherwise we have a problem. */
else {
    /* LCOV_EXCL_START */
    if meta >= META_END {
        /* PCRE2_DEBUG_UNREACHABLE(); */
        *errorcodeptr = ERR89; /* Internal error - unrecognized. */
        return 0;
    }
    /* LCOV_EXCL_STOP */

    /* Fall through to NORMAL_CHAR */
    break 'normal_char;
}
/* ------------------------- end of switch dispatch ------------------------ */
                                                                } /* 'class_end_processing */

    /* CLASS_END_PROCESSING: */

    /* If this class is the first thing in the branch, there can be no first
    char setting, whatever the repeat count. Any reqcu setting must remain
    unchanged after any kind of repeat. */

    if firstcuflags == REQ_UNSET as u32 {
        firstcuflags = REQ_NONE as u32;
    }
    zerofirstcu = firstcu;
    zerofirstcuflags = firstcuflags;
    zeroreqcu = reqcu;
    zeroreqcuflags = reqcuflags;
    break 'switch_break; /* End of class processing */
                                                            } /* 'verb_arg */

    /* VERB_ARG: */

    *code = *verbops.as_ptr().add(((meta - META_MARK) >> 16) as usize) as u8;
    code = code.add(1);
    /* The length is in characters. */
    pptr = pptr.add(1);
    verbarglen = *pptr;
    verbculen = 0;
    tempcode = code;
    code = code.add(1);
    {
        let mut i: c_int = 0;
        while i < verbarglen as c_int {
            pptr = pptr.add(1);
            meta = *pptr;
            if utf != 0 {
                mclength = _pcre2_ord2utf_8(meta, mcbuffer.as_mut_ptr());
            } else {
                mclength = 1;
                mcbuffer[0] = meta as u8;
            }
            if !lengthptr.is_null() {
                *lengthptr += mclength as PCRE2_SIZE;
            } else {
                memcpy(
                    code as *mut c_void,
                    mcbuffer.as_ptr() as *const c_void,
                    mclength as usize,
                );
                code = code.add(mclength as usize);
                verbculen += mclength;
            }
            i += 1;
        }
    }

    *tempcode = verbculen as u8; /* Fill in the code unit length */
    *code = 0; /* Terminating zero */
    code = code.add(1);
    break 'switch_break;
                                                        } /* 'group_process_note_empty */

    /* Process nested bracketed regex. The nesting depth is maintained for the
    benefit of the stackguard function. The test for too deep nesting is now
    done in parse_regex(). Assertion and DEFINE groups come to GROUP_PROCESS;
    others come to GROUP_PROCESS_NOTE_EMPTY, to indicate that we need to take
    note of whether or not they may match an empty string. */

    /* GROUP_PROCESS_NOTE_EMPTY: */
    note_group_empty = TRUE;

    /* Falls through into GROUP_PROCESS */
                                                    } /* 'group_process */

    /* GROUP_PROCESS: */
    (*cb).parens_depth += 1;
    *code = bravalue as u8;
    pptr = pptr.add(1);
    tempcode = code;
    tempreqvary = (*cb).req_varyopt; /* Save value before group */
    length_prevgroup = 0; /* Initialize for pre-compile phase */

    group_return = compile_regex(
        options,   /* The options state */
        xoptions,  /* The extra options state */
        &mut tempcode, /* Where to put code (updated) */
        &mut pptr, /* Input pointer (updated) */
        errorcodeptr, /* Where to put an error message */
        skipunits, /* Skip over bracket number */
        &mut subfirstcu, /* For possible first char */
        &mut subfirstcuflags,
        &mut subreqcu, /* For possible last char */
        &mut subreqcuflags,
        bcptr,     /* Current branch chain */
        open_caps, /* Pointer to capture stack */
        cb,        /* Compile data block */
        if lengthptr.is_null() {
            std::ptr::null_mut()
        } else {
            &mut length_prevgroup
        },
    );
    if group_return == 0 {
        return 0; /* Error */
    }

    (*cb).parens_depth -= 1;

    /* If that was a non-conditional significant group (not an assertion, not a
    DEFINE) that matches at least one character, then the current item matches
    a character. Conditionals are handled below. */

    if note_group_empty != 0 && bravalue != OP_COND as c_int && group_return > 0 {
        matched_char = TRUE;
    }

    /* If we've just compiled an assertion, pop the assert depth. */

    if bravalue >= OP_ASSERT as c_int && bravalue <= OP_ASSERT_SCS as c_int {
        (*cb).assert_depth -= 1;
    }

    /* At the end of compiling, code is still pointing to the start of the
    group, while tempcode has been updated to point past the end of the group.
    The parsed pattern pointer (pptr) is on the closing META_KET.

    If this is a conditional bracket, check that there are no more than
    two branches in the group, or just one if it's a DEFINE group. We do this
    in the real compile phase, not in the pre-pass, where the whole group may
    not be available. */

    if bravalue == OP_COND as c_int && lengthptr.is_null() {
        let mut tc: *mut PCRE2_UCHAR = code;
        let mut condcount: c_int = 0;

        loop {
            condcount += 1;
            tc = tc.add(GET!(tc, 1) as usize);
            if !(*tc as u32 != OP_KET) {
                break;
            }
        }

        /* A DEFINE group is never obeyed inline (the "condition" is always
        false). It must have only one branch. Having checked this, change the
        opcode to OP_FALSE. */

        if *code.add(LINK_SIZE + 1) as u32 == OP_DEFINE {
            if condcount > 1 {
                (*cb).erroroffset = offset;
                *errorcodeptr = ERR54;
                return 0;
            }
            *code.add(LINK_SIZE + 1) = OP_FALSE as u8;
            bravalue = OP_DEFINE as c_int; /* A flag to suppress char handling below */
        }
        /* A "normal" conditional group. If there is just one branch, we must not
        make use of its firstcu or reqcu, because this is equivalent to an
        empty second branch. Also, it may match an empty string. If there are two
        branches, this item must match a character if the group must. */
        else {
            if condcount > 2 {
                (*cb).erroroffset = offset;
                *errorcodeptr = ERR27;
                return 0;
            }
            if condcount == 1 {
                subfirstcuflags = REQ_NONE as u32;
                subreqcuflags = REQ_NONE as u32;
            } else if group_return > 0 {
                matched_char = TRUE;
            }
        }
    }

    /* In the pre-compile phase, update the length by the length of the group,
    less the brackets at either end. Then reduce the compiled code to just a
    set of non-capturing brackets so that it doesn't use much memory if it is
    duplicated by a quantifier.*/

    if !lengthptr.is_null() {
        if (OFLOW_MAX as PCRE2_SIZE).wrapping_sub(*lengthptr)
            < length_prevgroup
                .wrapping_sub(2)
                .wrapping_sub(2 * LINK_SIZE)
        {
            *errorcodeptr = ERR20;
            return 0;
        }
        *lengthptr = (*lengthptr).wrapping_add(
            length_prevgroup
                .wrapping_sub(2)
                .wrapping_sub(2 * LINK_SIZE),
        );
        code = code.add(1); /* This already contains bravalue */
        PUTINC!(code, 0, (1 + LINK_SIZE) as u32);
        *code = OP_KET as u8;
        code = code.add(1);
        PUTINC!(code, 0, (1 + LINK_SIZE) as u32);
        break 'switch_break; /* No need to waste time with special character handling */
    }

    /* Otherwise update the main code pointer to the end of the group. */

    code = tempcode;

    /* For a DEFINE group, required and first character settings are not
    relevant. */

    if bravalue == OP_DEFINE as c_int {
        break 'switch_break;
    }

    /* Handle updating of the required and first code units for other types of
    group. Update for normal brackets of all kinds, and conditions with two
    branches (see code above). If the bracket is followed by a quantifier with
    zero repeat, we have to back off. Hence the definition of zeroreqcu and
    zerofirstcu outside the main loop so that they can be accessed for the back
    off. */

    zeroreqcu = reqcu;
    zeroreqcuflags = reqcuflags;
    zerofirstcu = firstcu;
    zerofirstcuflags = firstcuflags;
    groupsetfirstcu = FALSE;

    if bravalue >= OP_ONCE as c_int
    /* Not an assertion */
    {
        /* If we have not yet set a firstcu in this branch, take it from the
        subpattern, remembering that it was set here so that a repeat of more
        than one can replicate it as reqcu if necessary. If the subpattern has
        no firstcu, set "none" for the whole branch. In both cases, a zero
        repeat forces firstcu to "none". */

        if firstcuflags == REQ_UNSET as u32 && subfirstcuflags != REQ_UNSET as u32 {
            if subfirstcuflags < REQ_NONE as u32 {
                firstcu = subfirstcu;
                firstcuflags = subfirstcuflags;
                groupsetfirstcu = TRUE;
            } else {
                firstcuflags = REQ_NONE as u32;
            }
            zerofirstcuflags = REQ_NONE as u32;
        }
        /* If firstcu was previously set, convert the subpattern's firstcu
        into reqcu if there wasn't one, using the vary flag that was in
        existence beforehand. */
        else if subfirstcuflags < REQ_NONE as u32 && subreqcuflags >= REQ_NONE as u32 {
            subreqcu = subfirstcu;
            subreqcuflags = subfirstcuflags | tempreqvary;
        }

        /* If the subpattern set a required code unit (or set a first code unit
        that isn't really the first code unit - see above), set it. */

        if subreqcuflags < REQ_NONE as u32 {
            reqcu = subreqcu;
            reqcuflags = subreqcuflags;
        }
    }
    /* For a forward assertion, we take the reqcu, if set, provided that the
    group has also set a firstcu. This can be helpful if the pattern that
    follows the assertion doesn't set a different char. For example, it's
    useful for /(?=abcde).+/. We can't set firstcu for an assertion, however
    because it leads to incorrect effect for patterns such as /(?=a)a.+/ when
    the "real" "a" would then become a reqcu instead of a firstcu. This is
    overcome by a scan at the end if there's no firstcu, looking for an
    asserted first char. A similar effect for patterns like /(?=.*X)X$/ means
    we must only take the reqcu when the group also set a firstcu. Otherwise,
    in that example, 'X' ends up set for both. */
    else if (bravalue == OP_ASSERT as c_int || bravalue == OP_ASSERT_NA as c_int)
        && subreqcuflags < REQ_NONE as u32
        && subfirstcuflags < REQ_NONE as u32
    {
        reqcu = subreqcu;
        reqcuflags = subreqcuflags;
    }

    break 'switch_break; /* End of nested group handling */
                                                } /* 'repeat */

    /* REPEAT: */
    if previous_matched_char != 0 && repeat_min > 0 {
        matched_char = TRUE;
    }

    /* Remember whether this is a variable length repeat, and default to
    single-char opcodes. */

    reqvary = if repeat_min == repeat_max {
        0
    } else {
        REQ_VARY as u32
    };

    /* Adjust first and required code units for a zero repeat. */

    if repeat_min == 0 {
        firstcu = zerofirstcu;
        firstcuflags = zerofirstcuflags;
        reqcu = zeroreqcu;
        reqcuflags = zeroreqcuflags;
    }

    /* Note the greediness and possessiveness. */

    if meta == META_MINMAX_PLUS
        || meta == META_ASTERISK_PLUS
        || meta == META_PLUS_PLUS
        || meta == META_QUERY_PLUS
    {
        repeat_type = 0; /* Force greedy */
        possessive_quantifier = TRUE;
    } else if meta == META_MINMAX_QUERY
        || meta == META_ASTERISK_QUERY
        || meta == META_PLUS_QUERY
        || meta == META_QUERY_QUERY
    {
        repeat_type = greedy_non_default;
        possessive_quantifier = FALSE;
    } else {
        repeat_type = greedy_default;
        possessive_quantifier = FALSE;
    }

    /* Save start of previous item, in case we have to move it up in order to
    insert something before it, and remember what it was. */

    /* PCRE2_ASSERT(previous != NULL); */
    tempcode = previous;
    op_previous = *previous;

    /* Now handle repetition for the different types of item. If the repeat
    minimum and the repeat maximum are both 1, we can ignore the quantifier for
    non-parenthesized items, as they have only one alternative. For anything in
    parentheses, we must not ignore if {1} is possessive. */

    /* switch (op_previous) */

    /* If previous was a character or negated character match, abolish the
    item and generate a repeat item instead. If a char item has a minimum of
    more than one, ensure that it is set in reqcu - it might not be if a
    sequence such as x{3} is the first thing in a branch because the x will
    have gone into firstcu instead.  */

    if op_previous as u32 == OP_CHAR
        || op_previous as u32 == OP_CHARI
        || op_previous as u32 == OP_NOT
        || op_previous as u32 == OP_NOTI
    {
        if repeat_max == 1 && repeat_min == 1 {
            break 'end_repeat;
        }
        op_type = *chartypeoffset
            .as_ptr()
            .add((op_previous as u32 - OP_CHAR) as usize);

        /* Deal with UTF characters that take up more than one code unit. */

        if utf != 0 && NOT_FIRSTCU!(*code.offset(-1)) {
            let mut lastchar: *mut PCRE2_UCHAR = code.offset(-1);
            BACKCHAR!(lastchar);
            mclength = code.offset_from(lastchar) as u32; /* Length of UTF character */
            memcpy(
                mcbuffer.as_mut_ptr() as *mut c_void,
                lastchar as *const c_void,
                mclength as usize,
            ); /* Save the char */
        }
        /* Handle the case of a single code unit - either with no UTF support, or
        with UTF disabled, or for a single-code-unit UTF character. In the latter
        case, for a repeated positive match, get the caseless flag for the
        required code unit from the previous character, because a class like [Aa]
        sets a caseless A but by now the req_caseopt flag has been reset. */
        else {
            mcbuffer[0] = *code.offset(-1);
            mclength = 1;
            if op_previous as u32 <= OP_CHARI && repeat_min > 1 {
                reqcu = mcbuffer[0] as u32;
                reqcuflags = (*cb).req_varyopt;
                if op_previous as u32 == OP_CHARI {
                    reqcuflags |= REQ_CASELESS as u32;
                }
            }
        }
        break 'output_single_repeat; /* Code shared with single character types */
    }
    /* If previous was a character class or a back reference, we put the
    repeat stuff after it, but just skip the item if the repeat was {0,0}. */
    else if op_previous as u32 == OP_XCLASS
        || op_previous as u32 == OP_ECLASS
        || op_previous as u32 == OP_CLASS
        || op_previous as u32 == OP_NCLASS
        || op_previous as u32 == OP_REF
        || op_previous as u32 == OP_REFI
        || op_previous as u32 == OP_DNREF
        || op_previous as u32 == OP_DNREFI
    {
        if repeat_max == 0 {
            code = previous;
            break 'end_repeat;
        }
        if repeat_max == 1 && repeat_min == 1 {
            break 'end_repeat;
        }

        if repeat_min == 0 && repeat_max == REPEAT_UNLIMITED as u32 {
            *code = (OP_CRSTAR + repeat_type) as u8;
            code = code.add(1);
        } else if repeat_min == 1 && repeat_max == REPEAT_UNLIMITED as u32 {
            *code = (OP_CRPLUS + repeat_type) as u8;
            code = code.add(1);
        } else if repeat_min == 0 && repeat_max == 1 {
            *code = (OP_CRQUERY + repeat_type) as u8;
            code = code.add(1);
        } else {
            *code = (OP_CRRANGE + repeat_type) as u8;
            code = code.add(1);
            PUT2INC!(code, 0, repeat_min);
            if repeat_max == REPEAT_UNLIMITED as u32 {
                repeat_max = 0; /* 2-byte encoding for max */
            }
            PUT2INC!(code, 0, repeat_max);
        }
        break 'after_switchb;
    }
    /* Prior to 10.30, repeated recursions were wrapped in OP_ONCE brackets
    because pcre2_match() could not handle backtracking into recursively
    called groups. Now that this backtracking is available, we no longer need
    to do this. However, we still need to replicate recursions as we do for
    groups so as to have independent backtracking points. We can replicate
    for the minimum number of repeats directly. For optional repeats we now
    wrap the recursion in OP_BRA brackets and make use of the bracket
    repetition. */
    else if op_previous as u32 == OP_RECURSE
        || op_previous as u32 == OP_ASSERT
        || op_previous as u32 == OP_ASSERT_NOT
        || op_previous as u32 == OP_ASSERT_NA
        || op_previous as u32 == OP_ASSERTBACK
        || op_previous as u32 == OP_ASSERTBACK_NOT
        || op_previous as u32 == OP_ASSERTBACK_NA
        || op_previous as u32 == OP_ASSERT_SCS
        || op_previous as u32 == OP_ONCE
        || op_previous as u32 == OP_SCRIPT_RUN
        || op_previous as u32 == OP_BRA
        || op_previous as u32 == OP_CBRA
        || op_previous as u32 == OP_COND
    {
        if op_previous as u32 == OP_RECURSE {
            /* case OP_RECURSE: */
            if repeat_max == 1 && repeat_min == 1 && possessive_quantifier == 0 {
                break 'end_repeat;
            }

            /* Generate unwrapped repeats for a non-zero minimum, except when the
            minimum is 1 and the maximum unlimited, because that can be handled with
            OP_BRA terminated by OP_KETRMAX/MIN. When the maximum is equal to the
            minimum, we just need to generate the appropriate additional copies.
            Otherwise we need to generate one more, to simulate the situation when
            the minimum is zero. */

            if repeat_min > 0 && (repeat_min != 1 || repeat_max != REPEAT_UNLIMITED as u32) {
                let mut replicate: c_int = repeat_min as c_int;

                if repeat_min == repeat_max {
                    replicate -= 1;
                }

                /* In the pre-compile phase, we don't actually do the replication. We
                just adjust the length as if we had. Do some paranoid checks for
                potential integer overflow. */

                if !lengthptr.is_null() {
                    let mut delta: PCRE2_SIZE = 0;
                    if _pcre2_ckd_smul_8(&mut delta, replicate, length_prevgroup as c_int) != 0
                        || (OFLOW_MAX as PCRE2_SIZE).wrapping_sub(*lengthptr) < delta
                    {
                        *errorcodeptr = ERR20;
                        return 0;
                    }
                    *lengthptr += delta;
                } else {
                    let mut i: c_int = 0;
                    while i < replicate {
                        memcpy(
                            code as *mut c_void,
                            previous as *const c_void,
                            length_prevgroup,
                        );
                        previous = code;
                        code = code.add(length_prevgroup);
                        i += 1;
                    }
                }

                /* If the number of repeats is fixed, we are done. Otherwise, adjust
                the counts and fall through. */

                if repeat_min == repeat_max {
                    break 'after_switchb;
                }
                if repeat_max != REPEAT_UNLIMITED as u32 {
                    repeat_max -= repeat_min;
                }
                repeat_min = 0;
            }

            /* Wrap the recursion call in OP_BRA brackets. */
            {
                let length: PCRE2_SIZE = if !lengthptr.is_null() {
                    1 + LINK_SIZE
                } else {
                    length_prevgroup
                };

                memmove(
                    previous.add(1 + LINK_SIZE) as *mut c_void,
                    previous as *const c_void,
                    length,
                );
                *previous = OP_BRA as u8;
                op_previous = OP_BRA as u8;
                PUT!(previous, 1, (1 + LINK_SIZE + length) as u32);
                *previous.add(1 + LINK_SIZE + length) = OP_KET as u8;
                PUT!(
                    previous,
                    2 + LINK_SIZE + length,
                    (1 + LINK_SIZE + length) as u32
                );
            }
            code = code.add(2 + 2 * LINK_SIZE);
            length_prevgroup += 2 + 2 * LINK_SIZE;
            group_return = -1; /* Set "may match empty string" */

            /* Now treat as a repeated OP_BRA. Fall through */
        }

        /* If previous was a bracket group, we may have to replicate it in
        certain cases. Note that at this point we can encounter only the "basic"
        bracket opcodes such as BRA and CBRA, as this is the place where they get
        converted into the more special varieties such as BRAPOS and SBRA.
        Originally, PCRE did not allow repetition of assertions, but now it does,
        for Perl compatibility. */
        {
            let mut len: c_int = code.offset_from(previous) as c_int;
            let mut bralink: *mut PCRE2_UCHAR = std::ptr::null_mut();
            let mut brazeroptr: *mut PCRE2_UCHAR = std::ptr::null_mut();

            if repeat_max == 1 && repeat_min == 1 && possessive_quantifier == 0 {
                break 'end_repeat;
            }

            /* Repeating a DEFINE group (or any group where the condition is always
            FALSE and there is only one branch) is pointless, but Perl allows the
            syntax, so we just ignore the repeat. */

            if op_previous as u32 == OP_COND
                && *previous.add(LINK_SIZE + 1) as u32 == OP_FALSE
                && *previous.add(GET!(previous, 1) as usize) as u32 != OP_ALT
            {
                break 'end_repeat;
            }

            /* Perl allows all assertions to be quantified, and when they contain
            capturing parentheses and/or are optional there are potential uses for
            this feature. PCRE2 used to force the maximum quantifier to 1 on the
            invalid grounds that further repetition was never useful. This was
            always a bit pointless, since an assertion could be wrapped with a
            repeated group to achieve the effect. General repetition is now
            permitted, but if the maximum is unlimited it is set to one more than
            the minimum. */

            if (op_previous as u32) < OP_ONCE
            /* Assertion */
            {
                if repeat_max == REPEAT_UNLIMITED as u32 {
                    repeat_max = repeat_min + 1;
                }
            }

            /* The case of a zero minimum is special because of the need to stick
            OP_BRAZERO in front of it, and because the group appears once in the
            data, whereas in other cases it appears the minimum number of times. For
            this reason, it is simplest to treat this case separately, as otherwise
            the code gets far too messy. There are several special subcases when the
            minimum is zero. */

            if repeat_min == 0 {
                /* If the maximum is also zero, we used to just omit the group from
                the output altogether, like this:

                ** if (repeat_max == 0)
                **   {
                **   code = previous;
                **   goto END_REPEAT;
                **   }

                However, that fails when a group or a subgroup within it is
                referenced as a subroutine from elsewhere in the pattern, so now we
                stick in OP_SKIPZERO in front of it so that it is skipped on
                execution. As we don't have a list of which groups are referenced, we
                cannot do this selectively.

                If the maximum is 1 or unlimited, we just have to stick in the
                BRAZERO and do no more at this point. */

                if repeat_max <= 1 || repeat_max == REPEAT_UNLIMITED as u32 {
                    memmove(
                        previous.add(1) as *mut c_void,
                        previous as *const c_void,
                        len as usize,
                    );
                    code = code.add(1);
                    if repeat_max == 0 {
                        *previous = OP_SKIPZERO as u8;
                        previous = previous.add(1);
                        break 'end_repeat;
                    }
                    brazeroptr = previous; /* Save for possessive optimizing */
                    *previous = (OP_BRAZERO + repeat_type) as u8;
                    previous = previous.add(1);
                }
                /* If the maximum is greater than 1 and limited, we have to replicate
                in a nested fashion, sticking OP_BRAZERO before each set of brackets.
                The first one has to be handled carefully because it's the original
                copy, which has to be moved up. The remainder can be handled by code
                that is common with the non-zero minimum case below. We have to
                adjust the value or repeat_max, since one less copy is required. */
                else {
                    let linkoffset: c_int;
                    memmove(
                        previous.add(2 + LINK_SIZE) as *mut c_void,
                        previous as *const c_void,
                        len as usize,
                    );
                    code = code.add(2 + LINK_SIZE);
                    *previous = (OP_BRAZERO + repeat_type) as u8;
                    previous = previous.add(1);
                    *previous = OP_BRA as u8;
                    previous = previous.add(1);

                    /* We chain together the bracket link offset fields that have to be
                    filled in later when the ends of the brackets are reached. */

                    linkoffset = if bralink.is_null() {
                        0
                    } else {
                        previous.offset_from(bralink) as c_int
                    };
                    bralink = previous;
                    PUTINC!(previous, 0, linkoffset);
                }

                if repeat_max != REPEAT_UNLIMITED as u32 {
                    repeat_max -= 1;
                }
            }
            /* If the minimum is greater than zero, replicate the group as many
            times as necessary, and adjust the maximum to the number of subsequent
            copies that we need. */
            else {
                if repeat_min > 1 {
                    /* In the pre-compile phase, we don't actually do the replication.
                    We just adjust the length as if we had. Do some paranoid checks for
                    potential integer overflow. */

                    if !lengthptr.is_null() {
                        let mut delta: PCRE2_SIZE = 0;
                        if _pcre2_ckd_smul_8(
                            &mut delta,
                            (repeat_min - 1) as c_int,
                            length_prevgroup as c_int,
                        ) != 0
                            || (OFLOW_MAX as PCRE2_SIZE).wrapping_sub(*lengthptr) < delta
                        {
                            *errorcodeptr = ERR20;
                            return 0;
                        }
                        *lengthptr += delta;
                    }
                    /* This is compiling for real. If there is a set first code unit
                    for the group, and we have not yet set a "required code unit", set
                    it. */
                    else {
                        if groupsetfirstcu != 0 && reqcuflags >= REQ_NONE as u32 {
                            reqcu = firstcu;
                            reqcuflags = firstcuflags;
                        }
                        let mut i: u32 = 1;
                        while i < repeat_min {
                            memcpy(
                                code as *mut c_void,
                                previous as *const c_void,
                                len as usize,
                            );
                            code = code.add(len as usize);
                            i += 1;
                        }
                    }
                }

                if repeat_max != REPEAT_UNLIMITED as u32 {
                    repeat_max -= repeat_min;
                }
            }

            /* This code is common to both the zero and non-zero minimum cases. If
            the maximum is limited, it replicates the group in a nested fashion,
            remembering the bracket starts on a stack. In the case of a zero
            minimum, the first one was set up above. In all cases the repeat_max
            now specifies the number of additional copies needed. Again, we must
            remember to replicate entries on the forward reference list. */

            if repeat_max != REPEAT_UNLIMITED as u32 {
                /* In the pre-compile phase, we don't actually do the replication. We
                just adjust the length as if we had. For each repetition we must add
                1 to the length for BRAZERO and for all but the last repetition we
                must add 2 + 2*LINKSIZE to allow for the nesting that occurs. Do some
                paranoid checks to avoid integer overflow. */

                if !lengthptr.is_null() && repeat_max > 0 {
                    let mut delta: PCRE2_SIZE = 0;
                    if _pcre2_ckd_smul_8(
                        &mut delta,
                        repeat_max as c_int,
                        length_prevgroup as c_int + 1 + 2 + 2 * LINK_SIZE as c_int,
                    ) != 0
                        || (OFLOW_MAX as PCRE2_SIZE + (2 + 2 * LINK_SIZE))
                            .wrapping_sub(*lengthptr)
                            < delta
                    {
                        *errorcodeptr = ERR20;
                        return 0;
                    }
                    delta = delta.wrapping_sub(2 + 2 * LINK_SIZE); /* Last one doesn't nest */
                    *lengthptr += delta;
                }
                /* This is compiling for real */
                else {
                    let mut i: u32 = repeat_max;
                    while i >= 1 {
                        *code = (OP_BRAZERO + repeat_type) as u8;
                        code = code.add(1);

                        /* All but the final copy start a new nesting, maintaining the
                        chain of brackets outstanding. */

                        if i != 1 {
                            let linkoffset: c_int;
                            *code = OP_BRA as u8;
                            code = code.add(1);
                            linkoffset = if bralink.is_null() {
                                0
                            } else {
                                code.offset_from(bralink) as c_int
                            };
                            bralink = code;
                            PUTINC!(code, 0, linkoffset);
                        }

                        memcpy(
                            code as *mut c_void,
                            previous as *const c_void,
                            len as usize,
                        );
                        code = code.add(len as usize);
                        i -= 1;
                    }
                }

                /* Now chain through the pending brackets, and fill in their length
                fields (which are holding the chain links pro tem). */

                while !bralink.is_null() {
                    let oldlinkoffset: c_int;
                    let linkoffset: c_int = (code.offset_from(bralink) + 1) as c_int;
                    let bra: *mut PCRE2_UCHAR = code.sub(linkoffset as usize);
                    oldlinkoffset = GET!(bra, 1) as c_int;
                    bralink = if oldlinkoffset == 0 {
                        std::ptr::null_mut()
                    } else {
                        bralink.sub(oldlinkoffset as usize)
                    };
                    *code = OP_KET as u8;
                    code = code.add(1);
                    PUTINC!(code, 0, linkoffset);
                    PUT!(bra, 1, linkoffset);
                }
            }
            /* If the maximum is unlimited, set a repeater in the final copy. For
            SCRIPT_RUN and ONCE brackets, that's all we need to do. However,
            possessively repeated ONCE brackets can be converted into non-capturing
            brackets, as the behaviour of (?:xx)++ is the same as (?>xx)++ and this
            saves having to deal with possessive ONCEs specially.

            Otherwise, when we are doing the actual compile phase, check to see
            whether this group is one that could match an empty string. If so,
            convert the initial operator to the S form (e.g. OP_BRA -> OP_SBRA) so
            that runtime checking can be done. [This check is also applied to ONCE
            and SCRIPT_RUN groups at runtime, but in a different way.]

            Then, if the quantifier was possessive and the bracket is not a
            conditional, we convert the BRA code to the POS form, and the KET code
            to KETRPOS. (It turns out to be convenient at runtime to detect this
            kind of subpattern at both the start and at the end.) The use of
            special opcodes makes it possible to reduce greatly the stack usage in
            pcre2_match(). If the group is preceded by OP_BRAZERO, convert this to
            OP_BRAPOSZERO.

            Then, if the minimum number of matches is 1 or 0, cancel the possessive
            flag so that the default action below, of wrapping everything inside
            atomic brackets, does not happen. When the minimum is greater than 1,
            there will be earlier copies of the group, and so we still have to wrap
            the whole thing. */
            else {
                let ketcode: *mut PCRE2_UCHAR = code.sub(1 + LINK_SIZE);
                let bracode: *mut PCRE2_UCHAR = ketcode.sub(GET!(ketcode, 1) as usize);

                /* Convert possessive ONCE brackets to non-capturing */

                if *bracode as u32 == OP_ONCE && possessive_quantifier != 0 {
                    *bracode = OP_BRA as u8;
                }

                /* For non-possessive ONCE and for SCRIPT_RUN brackets, all we need
                to do is to set the KET. */

                if *bracode as u32 == OP_ONCE || *bracode as u32 == OP_SCRIPT_RUN {
                    *ketcode = (OP_KETRMAX + repeat_type) as u8;
                }
                /* Handle non-SCRIPT_RUN and non-ONCE brackets and possessive ONCEs
                (which have been converted to non-capturing above). */
                else {
                    /* In the compile phase, adjust the opcode if the group can match
                    an empty string. For a conditional group with only one branch, the
                    value of group_return will not show "could be empty", so we must
                    check that separately. */

                    if lengthptr.is_null() {
                        if group_return < 0 {
                            *bracode = (*bracode as u32 + (OP_SBRA - OP_BRA)) as u8;
                        }
                        if *bracode as u32 == OP_COND
                            && *bracode.add(GET!(bracode, 1) as usize) as u32 != OP_ALT
                        {
                            *bracode = OP_SCOND as u8;
                        }
                    }

                    /* Handle possessive quantifiers. */

                    if possessive_quantifier != 0 {
                        /* For COND brackets, we wrap the whole thing in a possessively
                        repeated non-capturing bracket, because we have not invented POS
                        versions of the COND opcodes. */

                        if *bracode as u32 == OP_COND || *bracode as u32 == OP_SCOND {
                            let mut nlen: c_int = code.offset_from(bracode) as c_int;
                            memmove(
                                bracode.add(1 + LINK_SIZE) as *mut c_void,
                                bracode as *const c_void,
                                nlen as usize,
                            );
                            code = code.add(1 + LINK_SIZE);
                            nlen += (1 + LINK_SIZE) as c_int;
                            *bracode = if *bracode as u32 == OP_COND {
                                OP_BRAPOS as u8
                            } else {
                                OP_SBRAPOS as u8
                            };
                            *code = OP_KETRPOS as u8;
                            code = code.add(1);
                            PUTINC!(code, 0, nlen);
                            PUT!(bracode, 1, nlen);
                        }
                        /* For non-COND brackets, we modify the BRA code and use KETRPOS. */
                        else {
                            *bracode = (*bracode as u32 + 1) as u8; /* Switch to xxxPOS opcodes */
                            *ketcode = OP_KETRPOS as u8;
                        }

                        /* If the minimum is zero, mark it as possessive, then unset the
                        possessive flag when the minimum is 0 or 1. */

                        if !brazeroptr.is_null() {
                            *brazeroptr = OP_BRAPOSZERO as u8;
                        }
                        if repeat_min < 2 {
                            possessive_quantifier = FALSE;
                        }
                    }
                    /* Non-possessive quantifier */
                    else {
                        *ketcode = (OP_KETRMAX + repeat_type) as u8;
                    }
                }
            }
        }
        break 'after_switchb;
    }
    /* If previous was a character type match (\d or similar), abolish it and
    create a suitable repeat item. The code is shared with single-character
    repeats by setting op_type to add a suitable offset into repeat_type.
    Note the the Unicode property types will be present only when
    SUPPORT_UNICODE is defined, but we don't wrap the little bits of code
    here because it just makes it horribly messy. */
    else {
        /* default: */

        /* LCOV_EXCL_START */
        if op_previous as u32 >= OP_EODN || op_previous as u32 <= OP_WORD_BOUNDARY {
            /* PCRE2_DEBUG_UNREACHABLE(); */
            *errorcodeptr = ERR10; /* Not a character type - internal error */
            return 0;
        }
        /* LCOV_EXCL_STOP */

        if repeat_max == 1 && repeat_min == 1 {
            break 'end_repeat;
        }

        op_type = OP_TYPESTAR - OP_STAR; /* Use type opcodes */
        mclength = 0; /* Not a character */

        if op_previous as u32 == OP_PROP || op_previous as u32 == OP_NOTPROP {
            prop_type = *previous.add(1) as c_int;
            prop_value = *previous.add(2) as c_int;
            break 'after_prop;
        }
        /* else: come here from just above with a character in mcbuffer/mclength.
        You must also set op_type before the jump: fall into
        OUTPUT_SINGLE_REPEAT. */
    }
                                            } /* 'output_single_repeat */

    /* OUTPUT_SINGLE_REPEAT: */
    prop_type = -1;
    prop_value = -1;
                                        } /* 'after_prop */

    /* At this point, if prop_type == prop_value == -1 we either have a
    character in mcbuffer when mclength is greater than zero, or we have
    mclength zero, in which case there is a non-property character type in
    op_previous. If prop_type/value are not negative, we have a property
    character type in op_previous. */

    oldcode = code; /* Save where we were */
    code = previous; /* Usually overwrite previous item */

    /* If the maximum is zero then the minimum must also be zero; Perl allows
    this case, so we do too - by simply omitting the item altogether. */

    if repeat_max == 0 {
        break 'end_repeat;
    }

    /* Combine the op_type with the repeat_type */

    repeat_type += op_type;

    /* A minimum of zero is handled either as the special case * or ?, or as
    an UPTO, with the maximum given. */

    if repeat_min == 0 {
        if repeat_max == REPEAT_UNLIMITED as u32 {
            *code = (OP_STAR + repeat_type) as u8;
            code = code.add(1);
        } else if repeat_max == 1 {
            *code = (OP_QUERY + repeat_type) as u8;
            code = code.add(1);
        } else {
            *code = (OP_UPTO + repeat_type) as u8;
            code = code.add(1);
            PUT2INC!(code, 0, repeat_max);
        }
    }
    /* A repeat minimum of 1 is optimized into some special cases. If the
    maximum is unlimited, we use OP_PLUS. Otherwise, the original item is
    left in place and, if the maximum is greater than 1, we use OP_UPTO with
    one less than the maximum. */
    else if repeat_min == 1 {
        if repeat_max == REPEAT_UNLIMITED as u32 {
            *code = (OP_PLUS + repeat_type) as u8;
            code = code.add(1);
        } else {
            code = oldcode; /* Leave previous item in place */
            if repeat_max == 1 {
                break 'end_repeat;
            }
            *code = (OP_UPTO + repeat_type) as u8;
            code = code.add(1);
            PUT2INC!(code, 0, repeat_max - 1);
        }
    }
    /* The case {n,n} is just an EXACT, while the general case {n,m} is
    handled as an EXACT followed by an UPTO or STAR or QUERY. */
    else {
        *code = (OP_EXACT + op_type) as u8; /* NB EXACT doesn't have repeat_type */
        code = code.add(1);
        PUT2INC!(code, 0, repeat_min);

        /* Unless repeat_max equals repeat_min, fill in the data for EXACT,
        and then generate the second opcode. For a repeated Unicode property
        match, there are two extra values that define the required property,
        and mclength is set zero to indicate this. */

        if repeat_max != repeat_min {
            if mclength > 0 {
                memcpy(
                    code as *mut c_void,
                    mcbuffer.as_ptr() as *const c_void,
                    mclength as usize,
                );
                code = code.add(mclength as usize);
            } else {
                *code = op_previous;
                code = code.add(1);
                if prop_type >= 0 {
                    *code = prop_type as u8;
                    code = code.add(1);
                    *code = prop_value as u8;
                    code = code.add(1);
                }
            }

            /* Now set up the following opcode */

            if repeat_max == REPEAT_UNLIMITED as u32 {
                *code = (OP_STAR + repeat_type) as u8;
                code = code.add(1);
            } else {
                repeat_max -= repeat_min;
                if repeat_max == 1 {
                    *code = (OP_QUERY + repeat_type) as u8;
                    code = code.add(1);
                } else {
                    *code = (OP_UPTO + repeat_type) as u8;
                    code = code.add(1);
                    PUT2INC!(code, 0, repeat_max);
                }
            }
        }
    }

    /* Fill in the character or character type for the final opcode. */

    if mclength > 0 {
        memcpy(
            code as *mut c_void,
            mcbuffer.as_ptr() as *const c_void,
            mclength as usize,
        );
        code = code.add(mclength as usize);
    } else {
        *code = op_previous;
        code = code.add(1);
        if prop_type >= 0 {
            *code = prop_type as u8;
            code = code.add(1);
            *code = prop_value as u8;
            code = code.add(1);
        }
    }
                                    } /* 'after_switchb -- End of switch on different op_previous values */

    /* If the character following a repeat is '+', possessive_quantifier is
    TRUE. For some opcodes, there are special alternative opcodes for this
    case. For anything else, we wrap the entire repeated item inside OP_ONCE
    brackets. Logically, the '+' notation is just syntactic sugar, taken from
    Sun's Java package, but the special opcodes can optimize it.

    Some (but not all) possessively repeated subpatterns have already been
    completely handled in the code just above. For them, possessive_quantifier
    is always FALSE at this stage. Note that the repeated item starts at
    tempcode, not at previous, which might be the first part of a string whose
    (former) last char we repeated. */

    if possessive_quantifier != 0 {
        let mut len: c_int;

        /* Possessifying an EXACT quantifier has no effect, so we can ignore it.
        However, QUERY, STAR, or UPTO may follow (for quantifiers such as {5,6},
        {5,}, or {5,10}). We skip over an EXACT item; if the length of what
        remains is greater than zero, there's a further opcode that can be
        handled. If not, do nothing, leaving the EXACT alone. */

        let tc: u32 = *tempcode as u32;
        if tc == OP_TYPEEXACT {
            tempcode = tempcode.add(
                *_pcre2_OP_lengths_8.as_ptr().add(*tempcode as usize) as usize
                    + (if *tempcode.add(1 + IMM2_SIZE) as u32 == OP_PROP
                        || *tempcode.add(1 + IMM2_SIZE) as u32 == OP_NOTPROP
                    {
                        2
                    } else {
                        0
                    }),
            );
        }
        /* CHAR opcodes are used for exacts whose count is 1. */
        else if tc == OP_CHAR
            || tc == OP_CHARI
            || tc == OP_NOT
            || tc == OP_NOTI
            || tc == OP_EXACT
            || tc == OP_EXACTI
            || tc == OP_NOTEXACT
            || tc == OP_NOTEXACTI
        {
            tempcode = tempcode.add(*_pcre2_OP_lengths_8.as_ptr().add(*tempcode as usize) as usize);
            if utf != 0 && HAS_EXTRALEN!(*tempcode.offset(-1)) {
                tempcode = tempcode.add(GET_EXTRALEN!(*tempcode.offset(-1) as u32) as usize);
            }
        }
        /* For the class opcodes, the repeat operator appears at the end;
        adjust tempcode to point to it. */
        else if tc == OP_CLASS || tc == OP_NCLASS {
            tempcode = tempcode.add(1 + 32);
        } else if tc == OP_XCLASS || tc == OP_ECLASS {
            tempcode = tempcode.add(GET!(tempcode, 1) as usize);
        } else if tc == OP_REF || tc == OP_REFI || tc == OP_DNREF || tc == OP_DNREFI {
            tempcode = tempcode.add(*_pcre2_OP_lengths_8.as_ptr().add(*tempcode as usize) as usize);
        }

        /* If tempcode is equal to code (which points to the end of the repeated
        item), it means we have skipped an EXACT item but there is no following
        QUERY, STAR, or UPTO; the value of len will be 0, and we do nothing. In
        all other cases, tempcode will be pointing to the repeat opcode, and will
        be less than code, so the value of len will be greater than 0. */

        len = code.offset_from(tempcode) as c_int;
        if len > 0 {
            let repcode: c_uint = *tempcode as c_uint;

            /* There is a table for possessifying opcodes, all of which are less
            than OP_CALLOUT. A zero entry means there is no possessified version.
            */

            if repcode < OP_CALLOUT && *opcode_possessify.as_ptr().add(repcode as usize) > 0 {
                *tempcode = *opcode_possessify.as_ptr().add(repcode as usize);
            }
            /* For opcode without a special possessified version, wrap the item in
            ONCE brackets. */
            else {
                memmove(
                    tempcode.add(1 + LINK_SIZE) as *mut c_void,
                    tempcode as *const c_void,
                    len as usize,
                );
                code = code.add(1 + LINK_SIZE);
                len += (1 + LINK_SIZE) as c_int;
                *tempcode.add(0) = OP_ONCE as u8;
                *code = OP_KET as u8;
                code = code.add(1);
                PUTINC!(code, 0, len);
                PUT!(tempcode, 1, len);
            }
        }
    }

    /* We set the "follows varying string" flag for subsequently encountered
    reqcus if it isn't already set and we have just passed a varying length
    item. */
                                } /* 'end_repeat */

    /* END_REPEAT: */
    (*cb).req_varyopt |= reqvary;
    break 'switch_break;
                            } /* 'handle_single_reference */

    /* Come here from named backref handling when the reference is to a
    single group (that is, not to a duplicated name). The back reference
    data will have already been updated. We must disable firstcu if not
    set, to cope with cases like (?=(\w+))\1: which would otherwise set ':'
    later. */

    /* HANDLE_SINGLE_REFERENCE: */
    if firstcuflags == REQ_UNSET as u32 {
        firstcuflags = REQ_NONE as u32;
        zerofirstcuflags = REQ_NONE as u32;
    }
    *code = if (options & PCRE2_CASELESS) != 0 {
        OP_REFI as u8
    } else {
        OP_REF as u8
    };
    code = code.add(1);
    PUT2INC!(code, 0, meta_arg);
    if (options & PCRE2_CASELESS) != 0 {
        *code = ((if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0 {
            REFI_FLAG_CASELESS_RESTRICT
        } else {
            0
        }) | (if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0 {
            REFI_FLAG_TURKISH_CASING
        } else {
            0
        })) as u8;
        code = code.add(1);
    }

    /* Update the map of back references, and keep the highest one. We
    could do this in parse_regex() for numerical back references, but not
    for named back references, because we don't know the numbers to which
    named back references refer. So we do it all in this function. */

    (*cb).backref_map |= if meta_arg < 32 { 1u32 << meta_arg } else { 1 };
    if meta_arg > (*cb).top_backref {
        (*cb).top_backref = meta_arg;
    }
    break 'switch_break;
                        } /* 'handle_numerical_recursion */

    /* HANDLE_NUMERICAL_RECURSION: */
    *code = OP_RECURSE as u8;
    PUT!(code, 1, meta_arg);
    code = code.add(1 + LINK_SIZE);
    /* Repeat processing requires this information to
    determine the real length in pre-compile phase. */
    length_prevgroup = 1 + LINK_SIZE;

    if META_CODE!(*pptr.add(1)) == META_OFFSET
        || META_CODE!(*pptr.add(1)) == META_CAPTURE_NAME
        || META_CODE!(*pptr.add(1)) == META_CAPTURE_NUMBER
    {
        let args: *mut recurse_arguments;

        if !lengthptr.is_null() {
            if _pcre2_compile_parse_recurse_args8(pptr, offset, errorcodeptr, cb) == 0 {
                return 0;
            }

            args = (*cb).last_data as *mut recurse_arguments;
            length_prevgroup += (*args).size * (1 + IMM2_SIZE);
            *lengthptr += (*args).size * (1 + IMM2_SIZE);
            pptr = pptr.add((*args).skip_size);
        } else {
            let mut current: *mut u16;
            let end: *mut u16;

            args = (*cb).first_data as *mut recurse_arguments;
            /* PCRE2_ASSERT(args != NULL && args->header.type == CDATA_RECURSE_ARGS); */

            current = args.add(1) as *mut u16;
            end = current.add((*args).size);
            /* PCRE2_ASSERT(end > current); */

            loop {
                *code.add(0) = OP_CREF as u8;
                PUT2!(code, 1, *current as u32);
                code = code.add(1 + IMM2_SIZE);
                current = current.add(1);
                if !(current < end) {
                    break;
                }
            }

            length_prevgroup += (*args).size * (1 + IMM2_SIZE);
            pptr = pptr.add((*args).skip_size);
            (*cb).first_data = (*args).header.next;
            ((*(*cb).cx).memctl.free.unwrap())(
                args as *mut c_void,
                (*(*cb).cx).memctl.memory_data,
            );
        }
    }

    groupsetfirstcu = FALSE;
    (*cb).had_recurse = TRUE;
    if firstcuflags == REQ_UNSET as u32 {
        firstcuflags = REQ_NONE as u32;
    }
    zerofirstcu = firstcu;
    zerofirstcuflags = firstcuflags;
    break 'switch_break;
                    } /* 'normal_char */

    /* NORMAL_CHAR: */
    meta = *pptr; /* Get the full 32 bits */
                } /* 'normal_char_set */

    /* NORMAL_CHAR_SET: Character is already in meta */
    matched_char = TRUE;

    /* For caseless UTF or UCP mode, check whether this character has more than
    one other case. If so, generate a special OP_PROP item instead of OP_CHARI.
    When casing restrictions apply, ignore caseless sets that start with an
    ASCII character. If the character is affected by the special Turkish rules,
    hardcode the matching characters using a caseset. */

    if (utf != 0 || ucp != 0) && (options & PCRE2_CASELESS) != 0 {
        let mut caseset: u32;

        if (xoptions & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
            == PCRE2_EXTRA_TURKISH_CASING
            && UCD_ANY_I(meta)
        {
            caseset =
                _pcre2_ucd_turkish_dotted_i_caseset_8 + (if UCD_DOTTED_I(meta) { 0 } else { 3 });
        } else {
            caseset = UCD_CASESET(meta);
            if caseset != 0
                && (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                && *_pcre2_ucd_caseless_sets_8.as_ptr().add(caseset as usize) < 128
            {
                caseset = 0; /* Ignore the caseless set if it's restricted. */
            }
        }

        if caseset != 0 {
            *code = OP_PROP as u8;
            code = code.add(1);
            *code = PT_CLIST as u8;
            code = code.add(1);
            *code = caseset as u8;
            code = code.add(1);
            if firstcuflags == REQ_UNSET as u32 {
                firstcuflags = REQ_NONE as u32;
                zerofirstcuflags = REQ_NONE as u32;
            }
            break 'switch_break; /* End handling this meta item */
        }
    }

    /* Caseful matches, or caseless and not one of the multicase characters. We
    come here by goto in the case of a positive class that contains only
    case-partners of a character with just two cases; matched_char has already
    been set TRUE and options fudged if necessary. */
            } /* 'class_caseless_char */

    /* CLASS_CASELESS_CHAR: */

    /* Get the character's code units into mcbuffer, with the length in
    mclength. When not in UTF mode, the length is always 1. */

    if utf != 0 {
        mclength = _pcre2_ord2utf_8(meta, mcbuffer.as_mut_ptr());
    } else {
        mclength = 1;
        mcbuffer[0] = meta as u8;
    }

    /* Generate the appropriate code */

    *code = if (options & PCRE2_CASELESS) != 0 {
        OP_CHARI as u8
    } else {
        OP_CHAR as u8
    };
    code = code.add(1);
    memcpy(
        code as *mut c_void,
        mcbuffer.as_ptr() as *const c_void,
        mclength as usize,
    );
    code = code.add(mclength as usize);

    /* Remember if \r or \n were seen */

    if mcbuffer[0] as u32 == CHAR_CR || mcbuffer[0] as u32 == CHAR_NL {
        (*cb).external_flags |= PCRE2_HASCRORLF;
    }

    /* Set the first and required code units appropriately. If no previous
    first code unit, set it from this character, but revert to none on a zero
    repeat. Otherwise, leave the firstcu value alone, and don't change it on
    a zero repeat. */

    if firstcuflags == REQ_UNSET as u32 {
        zerofirstcuflags = REQ_NONE as u32;
        zeroreqcu = reqcu;
        zeroreqcuflags = reqcuflags;

        /* If the character is more than one code unit long, we can set a single
        firstcu only if it is not to be matched caselessly. Multiple possible
        starting code units may be picked up later in the studying code. */

        if mclength == 1 || req_caseopt == 0 {
            firstcu = mcbuffer[0] as u32;
            firstcuflags = req_caseopt;
            if mclength != 1 {
                reqcu = *code.offset(-1) as u32;
                reqcuflags = (*cb).req_varyopt;
            }
        } else {
            firstcuflags = REQ_NONE as u32;
            reqcuflags = REQ_NONE as u32;
        }
    }
    /* firstcu was previously set; we can set reqcu only if the length is
    1 or the matching is caseful. */
    else {
        zerofirstcu = firstcu;
        zerofirstcuflags = firstcuflags;
        zeroreqcu = reqcu;
        zeroreqcuflags = reqcuflags;
        if mclength == 1 || req_caseopt == 0 {
            reqcu = *code.offset(-1) as u32;
            reqcuflags = req_caseopt | (*cb).req_varyopt;
        }
    }

    /* If caselessness was temporarily instated, reset it. */

    if reset_caseful != 0 {
        options &= !PCRE2_CASELESS;
        req_caseopt = 0;
        reset_caseful = FALSE;
    }

    /* End literal character handling */
        } /* 'switch_break -- End of big switch */

        pptr = pptr.add(1);
    } /* End of big loop */

    /* LCOV_EXCL_START */
    /* PCRE2_DEBUG_UNREACHABLE(); Control should never reach here */
    /* return 0;  Avoid compiler warnings */
    /* LCOV_EXCL_STOP */
}

/*************************************************
*   Compile regex: a sequence of alternatives    *
*************************************************/

/* On entry, pptr is pointing past the bracket meta, but on return it points to
the closing bracket or META_END. The code variable is pointing at the code unit
into which the BRA operator has been stored. This function is used during the
pre-compile phase when we are trying to find out the amount of memory needed,
as well as during the real compile phase. The value of lengthptr distinguishes
the two phases.

Arguments:
  options           option bits, including any changes for this subpattern
  xoptions          extra option bits, ditto
  codeptr           -> the address of the current code pointer
  pptrptr           -> the address of the current parsed pattern pointer
  errorcodeptr      -> pointer to error code variable
  skipunits         skip this many code units at start (for brackets and OP_COND)
  firstcuptr        place to put the first required code unit
  firstcuflagsptr   place to put the first code unit flags
  reqcuptr          place to put the last required code unit
  reqcuflagsptr     place to put the last required code unit flags
  bcptr             pointer to the chain of currently open branches
  cb                points to the data block with tables pointers etc.
  lengthptr         NULL during the real compile phase
                    points to length accumulator during pre-compile phase

Returns:            0 There has been an error
                   +1 Success, this group must match at least one character
                   -1 Success, this group may match an empty string
*/
