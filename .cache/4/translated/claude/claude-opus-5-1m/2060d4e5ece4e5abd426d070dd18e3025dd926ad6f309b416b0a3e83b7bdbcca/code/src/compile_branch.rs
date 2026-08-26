// Translated from pcre2_compile.c lines 5967-8894
use crate::compile_h::*;
use crate::compile_tables::*;
use crate::compile_util::*;
use crate::internal::*;
use crate::pcre2_pub::*;
use crate::tables::*;
use crate::ucd_data::*;
use crate::ucp::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

pub(crate) unsafe fn first_significant_code(code: PCRE2_SPTR, skipassert: BOOL) -> *const PCRE2_UCHAR {
    let mut code = code;
    loop {
        let op = *code as u32;
        if op == OP_ASSERT_NOT
            || op == OP_ASSERTBACK
            || op == OP_ASSERTBACK_NOT
            || op == OP_ASSERTBACK_NA
        {
            if skipassert == 0 {
                return code;
            }
            loop {
                code = code.add(GET(code, 1) as usize);
                if *code as u32 != OP_ALT {
                    break;
                }
            }
            code = code.add(_pcre2_OP_lengths_8[*code as usize] as usize);
        } else if op == OP_WORD_BOUNDARY
            || op == OP_NOT_WORD_BOUNDARY
            || op == OP_UCP_WORD_BOUNDARY
            || op == OP_NOT_UCP_WORD_BOUNDARY
        {
            if skipassert == 0 {
                return code;
            }
            /* Fall through */
            code = code.add(_pcre2_OP_lengths_8[*code as usize] as usize);
        } else if op == OP_CALLOUT
            || op == OP_CREF
            || op == OP_DNCREF
            || op == OP_RREF
            || op == OP_DNRREF
            || op == OP_FALSE
            || op == OP_TRUE
        {
            code = code.add(_pcre2_OP_lengths_8[*code as usize] as usize);
        } else if op == OP_CALLOUT_STR {
            code = code.add(GET(code, 1 + 2 * LINK_SIZE) as usize);
        } else if op == OP_SKIPZERO {
            code = code.add(2 + GET(code, 2) as usize + LINK_SIZE);
        } else if op == OP_COND || op == OP_SCOND {
            if *code.add(1 + LINK_SIZE) as u32 != OP_FALSE
                || *code.add(GET(code, 1) as usize) as u32 != OP_KET
            {
                return code;
            }
            code = code.add(GET(code, 1) as usize + 1 + LINK_SIZE);
        } else if op == OP_MARK
            || op == OP_COMMIT_ARG
            || op == OP_PRUNE_ARG
            || op == OP_SKIP_ARG
            || op == OP_THEN_ARG
        {
            code = code.add(*code.add(1) as usize + _pcre2_OP_lengths_8[*code as usize] as usize);
        } else {
            return code;
        }
    }
}

pub(crate) unsafe fn compile_branch(
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
    let mut tempcode: *mut PCRE2_UCHAR = core::ptr::null_mut();
    let mut previous: *mut PCRE2_UCHAR = core::ptr::null_mut();
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

    /* Initialize no first unit, no required unit. */

    firstcu = 0;
    reqcu = 0;
    zerofirstcu = 0;
    zeroreqcu = 0;
    firstcuflags = REQ_UNSET;
    reqcuflags = REQ_UNSET;
    zerofirstcuflags = REQ_UNSET;
    zeroreqcuflags = REQ_UNSET;

    req_caseopt = if (options & PCRE2_CASELESS) != 0 { REQ_CASELESS } else { 0 };

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
        /* Hoisted out of the inner block in the "default" arm of the
        switch on op_previous, because control jumps into that block. */
        let mut prop_type: c_int = 0;
        let mut prop_value: c_int = 0;
        let mut oldcode: *mut PCRE2_UCHAR;

        /* Get next META item in the pattern and its potential argument. */

        meta = META_CODE(*pptr);
        meta_arg = META_DATA(*pptr);

        /* If we are in the pre-compile phase, accumulate the length used for the
        previous cycle of this loop, unless the next item is a quantifier. */

        if !lengthptr.is_null() {
            if code >= (*cb).start_workspace.add((*cb).workspace_size) {
                *errorcodeptr = ERR(52); /* Over-ran workspace - internal error */
                (*cb).erroroffset = 0;
                return 0;
            }

            if code
                > (*cb)
                    .start_workspace
                    .add((*cb).workspace_size - WORK_SIZE_SAFETY_MARGIN)
            {
                *errorcodeptr = ERR(86); /* Pattern too complicated */
                (*cb).erroroffset = 0;
                return 0;
            }

            /* There is at least one situation where code goes backwards. */

            if code < last_code {
                code = last_code;
            }

            if meta < META_ASTERISK || meta > META_MINMAX_QUERY {
                if (OFLOW_MAX as PCRE2_SIZE).wrapping_sub(*lengthptr)
                    < code.offset_from(orig_code) as PCRE2_SIZE
                {
                    *errorcodeptr = ERR(20); /* Integer overflow */
                    (*cb).erroroffset = 0;
                    return 0;
                }
                *lengthptr = (*lengthptr).wrapping_add(code.offset_from(orig_code) as PCRE2_SIZE);
                if *lengthptr > MAX_PATTERN_SIZE {
                    *errorcodeptr = ERR(20); /* Pattern is too large */
                    (*cb).erroroffset = 0;
                    return 0;
                }
                code = orig_code;
            }

            last_code = code;
        }

        /* Process the next parsed pattern item. */

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

        'sw: {
            'CLASS_CASELESS_CHAR: {
                'NORMAL_CHAR_SET: {
                    'NORMAL_CHAR: {
                        'HANDLE_NUMERICAL_RECURSION: {
                            'HANDLE_SINGLE_REFERENCE: {
                                'END_REPEAT: {
                                    'REPEAT: {
                                        'GROUP_PROCESS: {
                                            'GROUP_PROCESS_NOTE_EMPTY: {
                                                'VERB_ARG: {
                                                    'CLASS_END_PROCESSING: {
        match meta {
        /* The branch terminates at pattern end or | or ) */
        META_END | META_ALT | META_KET => {
            *firstcuptr = firstcu;
            *firstcuflagsptr = firstcuflags;
            *reqcuptr = reqcu;
            *reqcuflagsptr = reqcuflags;
            *codeptr = code;
            *pptrptr = pptr;
            return okreturn;
        }

        META_CIRCUMFLEX => {
            if (options & PCRE2_MULTILINE) != 0 {
                if firstcuflags == REQ_UNSET {
                    firstcuflags = REQ_NONE;
                    zerofirstcuflags = REQ_NONE;
                }
                *code = OP_CIRCM as PCRE2_UCHAR;
                code = code.add(1);
            } else {
                *code = OP_CIRC as PCRE2_UCHAR;
                code = code.add(1);
            }
            break 'sw;
        }

        META_DOLLAR => {
            *code = (if (options & PCRE2_MULTILINE) != 0 { OP_DOLLM } else { OP_DOLL })
                as PCRE2_UCHAR;
            code = code.add(1);
            break 'sw;
        }

        META_DOT => {
            matched_char = TRUE;
            if firstcuflags == REQ_UNSET {
                firstcuflags = REQ_NONE;
            }
            zerofirstcu = firstcu;
            zerofirstcuflags = firstcuflags;
            zeroreqcu = reqcu;
            zeroreqcuflags = reqcuflags;
            *code = (if (options & PCRE2_DOTALL) != 0 { OP_ALLANY } else { OP_ANY })
                as PCRE2_UCHAR;
            code = code.add(1);
            break 'sw;
        }

        META_CLASS_EMPTY | META_CLASS_EMPTY_NOT => {
            matched_char = TRUE;
            if meta == META_CLASS_EMPTY_NOT {
                *code = OP_ALLANY as PCRE2_UCHAR;
                code = code.add(1);
            } else {
                *code = OP_CLASS as PCRE2_UCHAR;
                code = code.add(1);
                memset(code as *mut c_void, 0, 32);
                code = code.add(32);
            }

            if firstcuflags == REQ_UNSET {
                firstcuflags = REQ_NONE;
            }
            zerofirstcu = firstcu;
            zerofirstcuflags = firstcuflags;
            break 'sw;
        }

        META_CLASS_NOT | META_CLASS => {
            matched_char = TRUE;

            /* Check for complex extended classes and handle them separately. */

            if (*pptr & CLASS_IS_ECLASS) != 0 {
                if crate::compile_class::_pcre2_compile_class_nested_8(
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
                break 'CLASS_END_PROCESSING;
            }

            if *pptr.add(1) < META_END && *pptr.add(2) == META_CLASS_END {
                let c: u32 = *pptr.add(1);

                pptr = pptr.add(2); /* Move on to class end */
                if meta == META_CLASS {
                    /* A positive one-char class can be */
                    /* handled as a normal literal character. */
                    meta = c; /* Set up the character */
                    break 'NORMAL_CHAR_SET;
                }

                /* Handle a negative one-character class */

                zeroreqcu = reqcu;
                zeroreqcuflags = reqcuflags;
                if firstcuflags == REQ_UNSET {
                    firstcuflags = REQ_NONE;
                }
                zerofirstcu = firstcu;
                zerofirstcuflags = firstcuflags;

                if (utf != 0 || ucp != 0) && (options & PCRE2_CASELESS) != 0 {
                    let mut caseset: u32;

                    if (xoptions
                        & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                        == PCRE2_EXTRA_TURKISH_CASING
                        && UCD_ANY_I(c)
                    {
                        caseset = _pcre2_ucd_turkish_dotted_i_caseset_8
                            + (if UCD_DOTTED_I(c) { 0 } else { 3 });
                    } else {
                        caseset = UCD_CASESET(c);
                        if caseset != 0
                            && (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                            && _pcre2_ucd_caseless_sets_8[caseset as usize] < 128
                        {
                            caseset = 0; /* Ignore the caseless set if it's restricted. */
                        }
                    }

                    if caseset != 0 {
                        *code = OP_NOTPROP as PCRE2_UCHAR;
                        code = code.add(1);
                        *code = PT_CLIST as PCRE2_UCHAR;
                        code = code.add(1);
                        *code = caseset as PCRE2_UCHAR;
                        code = code.add(1);
                        break 'sw; /* We are finished with this class */
                    }
                }

                /* Char has only one other (usable) case */

                *code = (if (options & PCRE2_CASELESS) != 0 { OP_NOTI } else { OP_NOT })
                    as PCRE2_UCHAR;
                code = code.add(1);
                code = code.add(PUTCHAR(utf != 0, c, code));
                break 'sw; /* We are finished with this class */
            } /* End of 1-char optimization */

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
                    && !((xoptions
                        & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                        == PCRE2_EXTRA_TURKISH_CASING
                        && UCD_ANY_I(c))
                {
                    let d: u32;

                    if (utf != 0 || ucp != 0) && c > 127 {
                        d = UCD_OTHERCASE(c);
                    } else {
                        d = TABLE_GET(c, (*cb).fcc, c);
                    }

                    if c != d && *pptr.add(2) == d {
                        pptr = pptr.add(3); /* Move on to class end */
                        meta = c;
                        if (options & PCRE2_CASELESS) == 0 {
                            reset_caseful = TRUE;
                            options |= PCRE2_CASELESS;
                            req_caseopt = REQ_CASELESS;
                        }
                        break 'CLASS_CASELESS_CHAR;
                    }
                }
            }

            /* Now emit the OP_CLASS/OP_NCLASS/OP_XCLASS/OP_ALLANY opcode. */

            pptr = crate::compile_class::_pcre2_compile_class_not_nested_8(
                options,
                xoptions,
                pptr.add(1),
                &mut code,
                (meta == META_CLASS_NOT) as BOOL,
                core::ptr::null_mut(),
                errorcodeptr,
                cb,
                lengthptr,
            );
            if pptr.is_null() {
                return 0;
            }

            break 'CLASS_END_PROCESSING;
        }

        /* Deal with (*VERB)s. */

        META_ACCEPT => {
            had_accept = TRUE;
            (*cb).had_accept = TRUE;
            oc = open_caps;
            while !oc.is_null() && (*oc).assert_depth >= (*cb).assert_depth {
                if !lengthptr.is_null() {
                    *lengthptr = (*lengthptr).wrapping_add(CU2BYTES(1) + IMM2_SIZE);
                } else {
                    *code = OP_CLOSE as PCRE2_UCHAR;
                    code = code.add(1);
                    PUT2(code, 0, (*oc).number as u32);
                    code = code.add(IMM2_SIZE);
                }
                oc = (*oc).next;
            }
            *code = (if (*cb).assert_depth > 0 { OP_ASSERT_ACCEPT } else { OP_ACCEPT })
                as PCRE2_UCHAR;
            code = code.add(1);
            if firstcuflags == REQ_UNSET {
                firstcuflags = REQ_NONE;
            }
            break 'sw;
        }

        META_PRUNE | META_SKIP => {
            (*cb).had_pruneorskip = TRUE;
            /* Fall through */
            *code = verbops[((meta - META_MARK) >> 16) as usize] as PCRE2_UCHAR;
            code = code.add(1);
            break 'sw;
        }

        META_COMMIT | META_FAIL => {
            *code = verbops[((meta - META_MARK) >> 16) as usize] as PCRE2_UCHAR;
            code = code.add(1);
            break 'sw;
        }

        META_THEN => {
            (*cb).external_flags |= PCRE2_HASTHEN;
            *code = OP_THEN as PCRE2_UCHAR;
            code = code.add(1);
            break 'sw;
        }

        META_THEN_ARG => {
            (*cb).external_flags |= PCRE2_HASTHEN;
            break 'VERB_ARG;
        }

        META_PRUNE_ARG | META_SKIP_ARG => {
            (*cb).had_pruneorskip = TRUE;
            /* Fall through */
            break 'VERB_ARG;
        }

        META_MARK | META_COMMIT_ARG => {
            break 'VERB_ARG;
        }

        /* Handle options change. */

        META_OPTIONS => {
            pptr = pptr.add(1);
            options = *pptr;
            *optionsptr = options;
            pptr = pptr.add(1);
            xoptions = *pptr;
            *xoptionsptr = xoptions;
            greedy_default = ((options & PCRE2_UNGREEDY) != 0) as u32;
            greedy_non_default = greedy_default ^ 1;
            req_caseopt = if (options & PCRE2_CASELESS) != 0 { REQ_CASELESS } else { 0 };
            break 'sw;
        }

        /* Handle scan substring. */

        META_OFFSET => {
            if !lengthptr.is_null() {
                pptr = crate::compile_cgroup::_pcre2_compile_parse_scan_substr_args8(
                    pptr,
                    errorcodeptr,
                    cb,
                    lengthptr,
                );
                if pptr.is_null() {
                    return 0;
                }
                break 'sw;
            }

            loop {
                let mut count: c_int;
                let mut index: c_int;
                let ng: *mut named_group;
                let mut leave_loop: BOOL = FALSE;

                match META_CODE(*pptr) {
                    META_OFFSET => {
                        pptr = pptr.add(1);
                        pptr = pptr.add(2); /* SKIPOFFSET */
                    }

                    META_CAPTURE_NAME => {
                        ng = (*cb).named_groups.add(*pptr.add(1) as usize);
                        pptr = pptr.add(2);
                        count = 0;
                        index = 0;

                        if crate::compile_cgroup::_pcre2_compile_find_dupname_details8(
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

                        *code.add(0) = OP_DNCREF as PCRE2_UCHAR;
                        PUT2(code, 1, index as u32);
                        PUT2(code, 1 + IMM2_SIZE, count as u32);
                        code = code.add(1 + 2 * IMM2_SIZE);
                    }

                    META_CAPTURE_NUMBER => {
                        pptr = pptr.add(2);
                        if *pptr.offset(-1) == 0 {
                            continue;
                        }

                        *code.add(0) = OP_CREF as PCRE2_UCHAR;
                        PUT2(code, 1, *pptr.offset(-1));
                        code = code.add(1 + IMM2_SIZE);
                    }

                    _ => {
                        leave_loop = TRUE;
                    }
                }

                if leave_loop != 0 {
                    break;
                }
            }
            pptr = pptr.offset(-1);
            break 'sw;
        }

        META_SCS => {
            bravalue = OP_ASSERT_SCS as c_int;
            (*cb).assert_depth += 1;
            break 'GROUP_PROCESS;
        }

        /* Handle conditional subpatterns. */

        META_COND_RNUMBER | META_COND_NAME | META_COND_RNAME => {
            bravalue = OP_COND as c_int;

            if !lengthptr.is_null() {
                let mut i: u32;
                let name: PCRE2_SPTR;
                let ng: *mut named_group;
                let start_pptr: *mut u32 = pptr;
                pptr = pptr.add(1);
                let length: u32 = *pptr;

                /* GETPLUSOFFSET */
                offset = ((*pptr.add(1) as PCRE2_SIZE) << 32) | (*pptr.add(2) as PCRE2_SIZE);
                pptr = pptr.add(2);
                name = (*cb).start_pattern.add(offset);

                ng = crate::compile_cgroup::_pcre2_compile_find_named_group8(name, length, cb);

                if ng.is_null() {
                    groupnumber = 0;
                    if meta == META_COND_RNUMBER {
                        i = 1;
                        while i < length {
                            groupnumber = groupnumber * 10
                                + (*name.add(i as usize) as u32).wrapping_sub(CHAR_0);
                            if groupnumber > MAX_GROUP_NUMBER {
                                *errorcodeptr = ERR(61);
                                (*cb).erroroffset = offset + i as PCRE2_SIZE;
                                return 0;
                            }
                            i += 1;
                        }
                    }

                    if meta != META_COND_RNUMBER || groupnumber > (*cb).bracount {
                        *errorcodeptr = ERR(15);
                        (*cb).erroroffset = offset;
                        return 0;
                    }

                    if groupnumber == 0 {
                        groupnumber = RREF_ANY;
                    }
                    *start_pptr.add(1) = groupnumber;
                    skipunits = (1 + IMM2_SIZE) as u32;
                    break 'GROUP_PROCESS_NOTE_EMPTY;
                }

                /* From here on, we know we have a name (not a number). */
                if meta == META_COND_RNUMBER {
                    meta = META_COND_NAME;
                }

                if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) == 0 {
                    if (*ng).number > (*cb).top_backref {
                        (*cb).top_backref = (*ng).number;
                    }

                    *start_pptr.add(0) = meta;
                    *start_pptr.add(1) = (*ng).number;

                    skipunits = (1 + IMM2_SIZE) as u32;
                    break 'GROUP_PROCESS_NOTE_EMPTY;
                }

                /* We have a duplicated name. */

                *start_pptr.add(0) = meta | 1;
                *start_pptr.add(1) = ng.offset_from((*cb).named_groups) as u32;

                skipunits = (1 + 2 * IMM2_SIZE) as u32;
            } else {
                let mut count: c_int;
                let mut index: c_int;
                let ng: *mut named_group;

                if meta == META_COND_RNUMBER {
                    *code.add(1 + LINK_SIZE) = OP_RREF as PCRE2_UCHAR;
                    PUT2(code, 2 + LINK_SIZE, *pptr.add(1));
                    skipunits = (1 + IMM2_SIZE) as u32;
                    pptr = pptr.add(1 + SIZEOFFSET);
                    break 'GROUP_PROCESS_NOTE_EMPTY;
                }

                if meta_arg == 0 {
                    *code.add(1 + LINK_SIZE) =
                        (if meta == META_COND_RNAME { OP_RREF } else { OP_CREF })
                            as PCRE2_UCHAR;
                    PUT2(code, 2 + LINK_SIZE, *pptr.add(1));
                    skipunits = (1 + IMM2_SIZE) as u32;
                    pptr = pptr.add(1 + SIZEOFFSET);
                    break 'GROUP_PROCESS_NOTE_EMPTY;
                }

                ng = (*cb).named_groups.add(*pptr.add(1) as usize);
                count = 0; /* Values for first pass (avoids compiler warning) */
                index = 0;

                if crate::compile_cgroup::_pcre2_compile_find_dupname_details8(
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

                *code.add(1 + LINK_SIZE) =
                    (if meta == META_COND_RNAME { OP_DNRREF } else { OP_DNCREF })
                        as PCRE2_UCHAR;

                PUT2(code, 2 + LINK_SIZE, index as u32);
                PUT2(code, 2 + LINK_SIZE + IMM2_SIZE, count as u32);
                skipunits = (1 + 2 * IMM2_SIZE) as u32;
                pptr = pptr.add(1 + SIZEOFFSET);
            }

            break 'GROUP_PROCESS_NOTE_EMPTY;
        }

        META_COND_DEFINE => {
            bravalue = OP_COND as c_int;
            /* GETPLUSOFFSET */
            offset = ((*pptr.add(1) as PCRE2_SIZE) << 32) | (*pptr.add(2) as PCRE2_SIZE);
            pptr = pptr.add(2);
            *code.add(1 + LINK_SIZE) = OP_DEFINE as PCRE2_UCHAR;
            skipunits = 1;
            break 'GROUP_PROCESS;
        }

        META_COND_NUMBER => {
            bravalue = OP_COND as c_int;
            /* GETPLUSOFFSET */
            offset = ((*pptr.add(1) as PCRE2_SIZE) << 32) | (*pptr.add(2) as PCRE2_SIZE);
            pptr = pptr.add(2);

            pptr = pptr.add(1);
            groupnumber = *pptr;
            if groupnumber > (*cb).bracount {
                *errorcodeptr = ERR(15);
                (*cb).erroroffset = offset;
                return 0;
            }
            if groupnumber > (*cb).top_backref {
                (*cb).top_backref = groupnumber;
            }

            /* Point at initial ( for too many branches error */
            offset -= 2;
            *code.add(1 + LINK_SIZE) = OP_CREF as PCRE2_UCHAR;
            skipunits = (1 + IMM2_SIZE) as u32;
            PUT2(code, 2 + LINK_SIZE, groupnumber);
            break 'GROUP_PROCESS_NOTE_EMPTY;
        }

        META_COND_VERSION => {
            bravalue = OP_COND as c_int;
            if *pptr.add(1) > 0 {
                *code.add(1 + LINK_SIZE) = (if (PCRE2_MAJOR > *pptr.add(2))
                    || (PCRE2_MAJOR == *pptr.add(2) && PCRE2_MINOR >= *pptr.add(3))
                {
                    OP_TRUE
                } else {
                    OP_FALSE
                }) as PCRE2_UCHAR;
            } else {
                *code.add(1 + LINK_SIZE) =
                    (if PCRE2_MAJOR == *pptr.add(2) && PCRE2_MINOR == *pptr.add(3) {
                        OP_TRUE
                    } else {
                        OP_FALSE
                    }) as PCRE2_UCHAR;
            }
            skipunits = 1;
            pptr = pptr.add(3);
            break 'GROUP_PROCESS_NOTE_EMPTY;
        }

        META_COND_ASSERT => {
            bravalue = OP_COND as c_int;
            break 'GROUP_PROCESS_NOTE_EMPTY;
        }

        /* Handle all kinds of nested bracketed groups. */

        META_LOOKAHEAD => {
            bravalue = OP_ASSERT as c_int;
            (*cb).assert_depth += 1;
            break 'GROUP_PROCESS;
        }

        META_LOOKAHEAD_NA => {
            bravalue = OP_ASSERT_NA as c_int;
            (*cb).assert_depth += 1;
            break 'GROUP_PROCESS;
        }

        META_LOOKAHEADNOT => {
            if *pptr.add(1) == META_KET
                && (*pptr.add(2) < META_ASTERISK || *pptr.add(2) > META_MINMAX_QUERY)
            {
                *code = OP_FAIL as PCRE2_UCHAR;
                code = code.add(1);
                pptr = pptr.add(1);
            } else {
                bravalue = OP_ASSERT_NOT as c_int;
                (*cb).assert_depth += 1;
                break 'GROUP_PROCESS;
            }
            break 'sw;
        }

        META_LOOKBEHIND => {
            bravalue = OP_ASSERTBACK as c_int;
            (*cb).assert_depth += 1;
            break 'GROUP_PROCESS;
        }

        META_LOOKBEHINDNOT => {
            bravalue = OP_ASSERTBACK_NOT as c_int;
            (*cb).assert_depth += 1;
            break 'GROUP_PROCESS;
        }

        META_LOOKBEHIND_NA => {
            bravalue = OP_ASSERTBACK_NA as c_int;
            (*cb).assert_depth += 1;
            break 'GROUP_PROCESS;
        }

        META_ATOMIC => {
            bravalue = OP_ONCE as c_int;
            break 'GROUP_PROCESS_NOTE_EMPTY;
        }

        META_SCRIPT_RUN => {
            bravalue = OP_SCRIPT_RUN as c_int;
            break 'GROUP_PROCESS_NOTE_EMPTY;
        }

        META_NOCAPTURE => {
            bravalue = OP_BRA as c_int;
            /* Fall through */
            break 'GROUP_PROCESS_NOTE_EMPTY;
        }

        /* Handle named backreferences and recursions. */

        META_BACKREF_BYNAME | META_RECURSE_BYNAME => {
            let mut count: c_int;
            let mut index: c_int;
            let name: PCRE2_SPTR;
            let ng: *mut named_group;
            pptr = pptr.add(1);
            let length: u32 = *pptr;

            /* GETPLUSOFFSET */
            offset = ((*pptr.add(1) as PCRE2_SIZE) << 32) | (*pptr.add(2) as PCRE2_SIZE);
            pptr = pptr.add(2);
            name = (*cb).start_pattern.add(offset);

            ng = crate::compile_cgroup::_pcre2_compile_find_named_group8(name, length, cb);

            if ng.is_null() {
                *errorcodeptr = ERR(15);
                (*cb).erroroffset = offset;
                return 0;
            }

            groupnumber = (*ng).number;

            if meta == META_RECURSE_BYNAME {
                meta_arg = groupnumber;
                break 'HANDLE_NUMERICAL_RECURSION;
            }

            (*cb).backref_map |= if groupnumber < 32 { 1u32 << groupnumber } else { 1 };
            if groupnumber > (*cb).top_backref {
                (*cb).top_backref = groupnumber;
            }

            if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) == 0 {
                meta_arg = groupnumber;
                break 'HANDLE_SINGLE_REFERENCE;
            }

            count = 0; /* Values for first pass (avoids compiler warning) */
            index = 0;
            if lengthptr.is_null()
                && crate::compile_cgroup::_pcre2_compile_find_dupname_details8(
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

            if firstcuflags == REQ_UNSET {
                firstcuflags = REQ_NONE;
            }
            *code = (if (options & PCRE2_CASELESS) != 0 { OP_DNREFI } else { OP_DNREF })
                as PCRE2_UCHAR;
            code = code.add(1);
            PUT2(code, 0, index as u32);
            code = code.add(IMM2_SIZE);
            PUT2(code, 0, count as u32);
            code = code.add(IMM2_SIZE);
            if (options & PCRE2_CASELESS) != 0 {
                *code = ((if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0 {
                    REFI_FLAG_CASELESS_RESTRICT
                } else {
                    0
                }) | (if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0 {
                    REFI_FLAG_TURKISH_CASING
                } else {
                    0
                })) as PCRE2_UCHAR;
                code = code.add(1);
            }
            break 'sw;
        }

        /* Handle a numerical callout. */

        META_CALLOUT_NUMBER => {
            *code.add(0) = OP_CALLOUT as PCRE2_UCHAR;
            PUT(code, 1, *pptr.add(1)); /* Offset to next pattern item */
            PUT(code, 1 + LINK_SIZE, *pptr.add(2)); /* Length of next pattern item */
            *code.add(1 + 2 * LINK_SIZE) = *pptr.add(3) as PCRE2_UCHAR;
            pptr = pptr.add(3);
            code = code.add(_pcre2_OP_lengths_8[OP_CALLOUT as usize] as usize);
            break 'sw;
        }

        /* Handle a callout with a string argument. */

        META_CALLOUT_STRING => {
            if !lengthptr.is_null() {
                *lengthptr = (*lengthptr).wrapping_add(
                    (*pptr.add(3)).wrapping_add((1 + 4 * LINK_SIZE) as u32) as PCRE2_SIZE,
                );
                pptr = pptr.add(3);
                pptr = pptr.add(2); /* SKIPOFFSET */
            } else {
                let mut pp: PCRE2_SPTR;
                let mut delimiter: u32;
                let mut length: u32 = *pptr.add(3);
                let mut callout_string: *mut PCRE2_UCHAR = code.add(1 + 4 * LINK_SIZE);

                *code.add(0) = OP_CALLOUT_STR as PCRE2_UCHAR;
                PUT(code, 1, *pptr.add(1)); /* Offset to next pattern item */
                PUT(code, 1 + LINK_SIZE, *pptr.add(2)); /* Length of next pattern item */

                pptr = pptr.add(3);
                /* GETPLUSOFFSET */
                offset = ((*pptr.add(1) as PCRE2_SIZE) << 32) | (*pptr.add(2) as PCRE2_SIZE);
                pptr = pptr.add(2);
                pp = (*cb).start_pattern.add(offset);
                {
                    let v = *pp;
                    pp = pp.add(1);
                    *callout_string = v;
                    callout_string = callout_string.add(1);
                    delimiter = v as u32;
                }
                if delimiter == CHAR_LEFT_CURLY_BRACKET {
                    delimiter = CHAR_RIGHT_CURLY_BRACKET;
                }
                PUT(code, 1 + 3 * LINK_SIZE, (offset + 1) as u32); /* One after delimiter */

                loop {
                    length -= 1;
                    if !(length > 1) {
                        break;
                    }
                    if *pp as u32 == delimiter && *pp.add(1) as u32 == delimiter {
                        *callout_string = delimiter as PCRE2_UCHAR;
                        callout_string = callout_string.add(1);
                        pp = pp.add(2);
                        length -= 1;
                    } else {
                        let v = *pp;
                        pp = pp.add(1);
                        *callout_string = v;
                        callout_string = callout_string.add(1);
                    }
                }
                *callout_string = CHAR_NUL as PCRE2_UCHAR;
                callout_string = callout_string.add(1);

                /* Set the length of the entire item, the advance to its end. */

                PUT(
                    code,
                    1 + 2 * LINK_SIZE,
                    callout_string.offset_from(code) as u32,
                );
                code = callout_string;
            }
            break 'sw;
        }

        /* Handle repetition. */

        META_MINMAX_PLUS | META_MINMAX_QUERY | META_MINMAX => {
            pptr = pptr.add(1);
            repeat_min = *pptr;
            pptr = pptr.add(1);
            repeat_max = *pptr;
            break 'REPEAT;
        }

        META_ASTERISK | META_ASTERISK_PLUS | META_ASTERISK_QUERY => {
            repeat_min = 0;
            repeat_max = REPEAT_UNLIMITED;
            break 'REPEAT;
        }

        META_PLUS | META_PLUS_PLUS | META_PLUS_QUERY => {
            repeat_min = 1;
            repeat_max = REPEAT_UNLIMITED;
            break 'REPEAT;
        }

        META_QUERY | META_QUERY_PLUS | META_QUERY_QUERY => {
            repeat_min = 0;
            repeat_max = 1;
            /* Falls through into REPEAT */
            break 'REPEAT;
        }

        /* Handle a 32-bit data character with a value greater than META_END. */

        META_BIGVALUE => {
            pptr = pptr.add(1);
            break 'NORMAL_CHAR;
        }

        /* Handle a back reference by number. */

        META_BACKREF => {
            if meta_arg < 10 {
                offset = (*cb).small_ref_offset[meta_arg as usize];
            } else {
                /* GETPLUSOFFSET */
                offset = ((*pptr.add(1) as PCRE2_SIZE) << 32) | (*pptr.add(2) as PCRE2_SIZE);
                pptr = pptr.add(2);
            }

            if meta_arg > (*cb).bracount {
                (*cb).erroroffset = offset;
                *errorcodeptr = ERR(15); /* Non-existent subpattern */
                return 0;
            }

            break 'HANDLE_SINGLE_REFERENCE;
        }

        /* Handle recursion. */

        META_RECURSE => {
            /* GETPLUSOFFSET */
            offset = ((*pptr.add(1) as PCRE2_SIZE) << 32) | (*pptr.add(2) as PCRE2_SIZE);
            pptr = pptr.add(2);
            if meta_arg > (*cb).bracount {
                (*cb).erroroffset = offset;
                *errorcodeptr = ERR(15); /* Non-existent subpattern */
                return 0;
            }
            break 'HANDLE_NUMERICAL_RECURSION;
        }

        /* Handle capturing parentheses. */

        META_CAPTURE => {
            bravalue = OP_CBRA as c_int;
            skipunits = IMM2_SIZE as u32;
            PUT2(code, 1 + LINK_SIZE, meta_arg);
            (*cb).lastcapture = meta_arg;
            break 'GROUP_PROCESS_NOTE_EMPTY;
        }

        /* Handle escape sequence items. */

        META_ESCAPE => {
            if meta_arg > ESC_b as u32 && meta_arg < ESC_Z as u32 {
                matched_char = TRUE;
                if firstcuflags == REQ_UNSET {
                    firstcuflags = REQ_NONE;
                }
            }

            /* Set values to reset to if this is followed by a zero repeat. */

            zerofirstcu = firstcu;
            zerofirstcuflags = firstcuflags;
            zeroreqcu = reqcu;
            zeroreqcuflags = reqcuflags;

            if meta_arg == ESC_P as u32 || meta_arg == ESC_p as u32 {
                pptr = pptr.add(1);
                let mut ptype: u32 = *pptr >> 16;
                let mut pdata: u32 = *pptr & 0xffff;

                if (options & PCRE2_CASELESS) != 0
                    && ptype == PT_PC
                    && (pdata == ucp_Lu || pdata == ucp_Ll || pdata == ucp_Lt)
                {
                    ptype = PT_LAMP;
                    pdata = 0;
                }

                if ptype == PT_ANY {
                    if meta_arg == ESC_P as u32 {
                        *code = OP_CLASS as PCRE2_UCHAR;
                        code = code.add(1);
                        memset(code as *mut c_void, 0, 32);
                        code = code.add(32);
                    } else {
                        *code = OP_ALLANY as PCRE2_UCHAR;
                        code = code.add(1);
                    }
                } else {
                    *code = (if meta_arg == ESC_p as u32 { OP_PROP } else { OP_NOTPROP })
                        as PCRE2_UCHAR;
                    code = code.add(1);
                    *code = ptype as PCRE2_UCHAR;
                    code = code.add(1);
                    *code = pdata as PCRE2_UCHAR;
                    code = code.add(1);
                }
                break 'sw; /* End META_ESCAPE */
            }

            if (*cb).assert_depth > 0
                && meta_arg == ESC_K as u32
                && (xoptions & PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK) == 0
            {
                *errorcodeptr = ERR(99);
                return 0;
            }

            /* switch(meta_arg) */
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

            *code = meta_arg as PCRE2_UCHAR;
            code = code.add(1);
            break 'sw; /* End META_ESCAPE */
        }

        /* Handle an unrecognized meta value. */

        _ => {
            if meta >= META_END {
                *errorcodeptr = ERR(89); /* Internal error - unrecognized. */
                return 0;
            }

            /* Handle a literal character. */
            break 'NORMAL_CHAR;
        }
        } /* End of big match */
                                                    } /* end 'CLASS_END_PROCESSING */

                                                    /* CLASS_END_PROCESSING: */

                                                    /* If this class is the first thing in the branch, there can be no
                                                    first char setting, whatever the repeat count. */

                                                    if firstcuflags == REQ_UNSET {
                                                        firstcuflags = REQ_NONE;
                                                    }
                                                    zerofirstcu = firstcu;
                                                    zerofirstcuflags = firstcuflags;
                                                    zeroreqcu = reqcu;
                                                    zeroreqcuflags = reqcuflags;
                                                    break 'sw; /* End of class processing */
                                                } /* end 'VERB_ARG */

                                                /* VERB_ARG: */

                                                *code = verbops
                                                    [((meta - META_MARK) >> 16) as usize]
                                                    as PCRE2_UCHAR;
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
                                                            mclength =
                                                                crate::ord2utf::_pcre2_ord2utf_8(
                                                                    meta,
                                                                    mcbuffer.as_mut_ptr(),
                                                                );
                                                        } else {
                                                            mclength = 1;
                                                            mcbuffer[0] = meta as PCRE2_UCHAR;
                                                        }
                                                        if !lengthptr.is_null() {
                                                            *lengthptr = (*lengthptr)
                                                                .wrapping_add(mclength as PCRE2_SIZE);
                                                        } else {
                                                            memcpy(
                                                                code as *mut c_void,
                                                                mcbuffer.as_ptr() as *const c_void,
                                                                CU2BYTES(mclength as usize),
                                                            );
                                                            code = code.add(mclength as usize);
                                                            verbculen += mclength;
                                                        }
                                                        i += 1;
                                                    }
                                                }

                                                *tempcode = verbculen as PCRE2_UCHAR; /* Fill in the code unit length */
                                                *code = 0; /* Terminating zero */
                                                code = code.add(1);
                                                break 'sw;
                                            } /* end 'GROUP_PROCESS_NOTE_EMPTY */

                                            /* GROUP_PROCESS_NOTE_EMPTY: */
                                            note_group_empty = TRUE;
                                            /* falls through to GROUP_PROCESS */
                                        } /* end 'GROUP_PROCESS */

                                        /* GROUP_PROCESS: */
                                        (*cb).parens_depth += 1;
                                        *code = bravalue as PCRE2_UCHAR;
                                        pptr = pptr.add(1);
                                        tempcode = code;
                                        tempreqvary = (*cb).req_varyopt; /* Save value before group */
                                        length_prevgroup = 0; /* Initialize for pre-compile phase */

                                        group_return = compile_regex(
                                            options,  /* The options state */
                                            xoptions, /* The extra options state */
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
                                                core::ptr::null_mut()
                                            } else {
                                                &mut length_prevgroup
                                            },
                                        );
                                        if group_return == 0 {
                                            return 0; /* Error */
                                        }

                                        (*cb).parens_depth -= 1;

                                        if note_group_empty != 0
                                            && bravalue != OP_COND as c_int
                                            && group_return > 0
                                        {
                                            matched_char = TRUE;
                                        }

                                        /* If we've just compiled an assertion, pop the assert depth. */

                                        if bravalue >= OP_ASSERT as c_int
                                            && bravalue <= OP_ASSERT_SCS as c_int
                                        {
                                            (*cb).assert_depth -= 1;
                                        }

                                        if bravalue == OP_COND as c_int && lengthptr.is_null() {
                                            let mut tc: *mut PCRE2_UCHAR = code;
                                            let mut condcount: c_int = 0;

                                            loop {
                                                condcount += 1;
                                                tc = tc.add(GET(tc, 1) as usize);
                                                if *tc as u32 == OP_KET {
                                                    break;
                                                }
                                            }

                                            if *code.add(LINK_SIZE + 1) as u32 == OP_DEFINE {
                                                if condcount > 1 {
                                                    (*cb).erroroffset = offset;
                                                    *errorcodeptr = ERR(54);
                                                    return 0;
                                                }
                                                *code.add(LINK_SIZE + 1) = OP_FALSE as PCRE2_UCHAR;
                                                bravalue = OP_DEFINE as c_int; /* A flag to suppress char handling below */
                                            } else {
                                                if condcount > 2 {
                                                    (*cb).erroroffset = offset;
                                                    *errorcodeptr = ERR(27);
                                                    return 0;
                                                }
                                                if condcount == 1 {
                                                    subfirstcuflags = REQ_NONE;
                                                    subreqcuflags = REQ_NONE;
                                                } else if group_return > 0 {
                                                    matched_char = TRUE;
                                                }
                                            }
                                        }

                                        if !lengthptr.is_null() {
                                            if (OFLOW_MAX as PCRE2_SIZE).wrapping_sub(*lengthptr)
                                                < length_prevgroup
                                                    .wrapping_sub(2)
                                                    .wrapping_sub(2 * LINK_SIZE)
                                            {
                                                *errorcodeptr = ERR(20);
                                                return 0;
                                            }
                                            *lengthptr = (*lengthptr).wrapping_add(
                                                length_prevgroup
                                                    .wrapping_sub(2)
                                                    .wrapping_sub(2 * LINK_SIZE),
                                            );
                                            code = code.add(1); /* This already contains bravalue */
                                            PUT(code, 0, (1 + LINK_SIZE) as u32);
                                            code = code.add(LINK_SIZE);
                                            *code = OP_KET as PCRE2_UCHAR;
                                            code = code.add(1);
                                            PUT(code, 0, (1 + LINK_SIZE) as u32);
                                            code = code.add(LINK_SIZE);
                                            break 'sw; /* No need to waste time with special character handling */
                                        }

                                        /* Otherwise update the main code pointer to the end of the group. */

                                        code = tempcode;

                                        if bravalue == OP_DEFINE as c_int {
                                            break 'sw;
                                        }

                                        zeroreqcu = reqcu;
                                        zeroreqcuflags = reqcuflags;
                                        zerofirstcu = firstcu;
                                        zerofirstcuflags = firstcuflags;
                                        groupsetfirstcu = FALSE;

                                        if bravalue >= OP_ONCE as c_int {
                                            /* Not an assertion */
                                            if firstcuflags == REQ_UNSET
                                                && subfirstcuflags != REQ_UNSET
                                            {
                                                if subfirstcuflags < REQ_NONE {
                                                    firstcu = subfirstcu;
                                                    firstcuflags = subfirstcuflags;
                                                    groupsetfirstcu = TRUE;
                                                } else {
                                                    firstcuflags = REQ_NONE;
                                                }
                                                zerofirstcuflags = REQ_NONE;
                                            } else if subfirstcuflags < REQ_NONE
                                                && subreqcuflags >= REQ_NONE
                                            {
                                                subreqcu = subfirstcu;
                                                subreqcuflags = subfirstcuflags | tempreqvary;
                                            }

                                            if subreqcuflags < REQ_NONE {
                                                reqcu = subreqcu;
                                                reqcuflags = subreqcuflags;
                                            }
                                        } else if (bravalue == OP_ASSERT as c_int
                                            || bravalue == OP_ASSERT_NA as c_int)
                                            && subreqcuflags < REQ_NONE
                                            && subfirstcuflags < REQ_NONE
                                        {
                                            reqcu = subreqcu;
                                            reqcuflags = subreqcuflags;
                                        }

                                        break 'sw; /* End of nested group handling */
                                    } /* end 'REPEAT */

                                    /* REPEAT: */
                                    if previous_matched_char != 0 && repeat_min > 0 {
                                        matched_char = TRUE;
                                    }

                                    /* Remember whether this is a variable length repeat, and default to
                                    single-char opcodes. */

                                    reqvary = if repeat_min == repeat_max { 0 } else { REQ_VARY };

                                    /* Adjust first and required code units for a zero repeat. */

                                    if repeat_min == 0 {
                                        firstcu = zerofirstcu;
                                        firstcuflags = zerofirstcuflags;
                                        reqcu = zeroreqcu;
                                        reqcuflags = zeroreqcuflags;
                                    }

                                    /* Note the greediness and possessiveness. */

                                    match meta {
                                        META_MINMAX_PLUS | META_ASTERISK_PLUS | META_PLUS_PLUS
                                        | META_QUERY_PLUS => {
                                            repeat_type = 0; /* Force greedy */
                                            possessive_quantifier = TRUE;
                                        }

                                        META_MINMAX_QUERY | META_ASTERISK_QUERY
                                        | META_PLUS_QUERY | META_QUERY_QUERY => {
                                            repeat_type = greedy_non_default;
                                            possessive_quantifier = FALSE;
                                        }

                                        _ => {
                                            repeat_type = greedy_default;
                                            possessive_quantifier = FALSE;
                                        }
                                    }

                                    /* Save start of previous item. */

                                    tempcode = previous;
                                    op_previous = *previous;

                                    'AFTER_OP_SWITCH: {
                                        'PROP_DONE: {
                                            'OUTPUT_SINGLE_REPEAT: {
                                                match op_previous as u32 {
                                                    OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI => {
                                                        if repeat_max == 1 && repeat_min == 1 {
                                                            break 'END_REPEAT;
                                                        }
                                                        op_type = chartypeoffset
                                                            [(op_previous as u32 - OP_CHAR) as usize];

                                                        /* Deal with UTF characters that take up more than one code unit. */

                                                        if utf != 0
                                                            && NOT_FIRSTCU(*code.offset(-1) as u32)
                                                        {
                                                            let mut lastchar: *mut PCRE2_UCHAR =
                                                                code.offset(-1);
                                                            while (*lastchar & 0xc0) == 0x80 {
                                                                lastchar = lastchar.offset(-1);
                                                            }
                                                            mclength = code.offset_from(lastchar)
                                                                as u32; /* Length of UTF character */
                                                            memcpy(
                                                                mcbuffer.as_mut_ptr() as *mut c_void,
                                                                lastchar as *const c_void,
                                                                CU2BYTES(mclength as usize),
                                                            ); /* Save the char */
                                                        } else {
                                                            mcbuffer[0] = *code.offset(-1);
                                                            mclength = 1;
                                                            if (op_previous as u32) <= OP_CHARI
                                                                && repeat_min > 1
                                                            {
                                                                reqcu = mcbuffer[0] as u32;
                                                                reqcuflags = (*cb).req_varyopt;
                                                                if op_previous as u32 == OP_CHARI {
                                                                    reqcuflags |= REQ_CASELESS;
                                                                }
                                                            }
                                                        }
                                                        break 'OUTPUT_SINGLE_REPEAT; /* Code shared with single character types */
                                                    }

                                                    OP_XCLASS | OP_ECLASS | OP_CLASS
                                                    | OP_NCLASS | OP_REF | OP_REFI | OP_DNREF
                                                    | OP_DNREFI => {
                                                        if repeat_max == 0 {
                                                            code = previous;
                                                            break 'END_REPEAT;
                                                        }
                                                        if repeat_max == 1 && repeat_min == 1 {
                                                            break 'END_REPEAT;
                                                        }

                                                        if repeat_min == 0
                                                            && repeat_max == REPEAT_UNLIMITED
                                                        {
                                                            *code = (OP_CRSTAR + repeat_type)
                                                                as PCRE2_UCHAR;
                                                            code = code.add(1);
                                                        } else if repeat_min == 1
                                                            && repeat_max == REPEAT_UNLIMITED
                                                        {
                                                            *code = (OP_CRPLUS + repeat_type)
                                                                as PCRE2_UCHAR;
                                                            code = code.add(1);
                                                        } else if repeat_min == 0
                                                            && repeat_max == 1
                                                        {
                                                            *code = (OP_CRQUERY + repeat_type)
                                                                as PCRE2_UCHAR;
                                                            code = code.add(1);
                                                        } else {
                                                            *code = (OP_CRRANGE + repeat_type)
                                                                as PCRE2_UCHAR;
                                                            code = code.add(1);
                                                            PUT2(code, 0, repeat_min);
                                                            code = code.add(IMM2_SIZE);
                                                            if repeat_max == REPEAT_UNLIMITED {
                                                                repeat_max = 0; /* 2-byte encoding for max */
                                                            }
                                                            PUT2(code, 0, repeat_max);
                                                            code = code.add(IMM2_SIZE);
                                                        }
                                                        break 'AFTER_OP_SWITCH;
                                                    }

                                                    OP_RECURSE | OP_ASSERT | OP_ASSERT_NOT
                                                    | OP_ASSERT_NA | OP_ASSERTBACK
                                                    | OP_ASSERTBACK_NOT | OP_ASSERTBACK_NA
                                                    | OP_ASSERT_SCS | OP_ONCE | OP_SCRIPT_RUN
                                                    | OP_BRA | OP_CBRA | OP_COND => {
                                                        if op_previous as u32 == OP_RECURSE {
                                                            if repeat_max == 1
                                                                && repeat_min == 1
                                                                && possessive_quantifier == 0
                                                            {
                                                                break 'END_REPEAT;
                                                            }

                                                            if repeat_min > 0
                                                                && (repeat_min != 1
                                                                    || repeat_max
                                                                        != REPEAT_UNLIMITED)
                                                            {
                                                                let mut replicate: c_int =
                                                                    repeat_min as c_int;

                                                                if repeat_min == repeat_max {
                                                                    replicate -= 1;
                                                                }

                                                                if !lengthptr.is_null() {
                                                                    let mut delta: PCRE2_SIZE = 0;
                                                                    if crate::chkdint::_pcre2_ckd_smul_8(
                                                                        &mut delta,
                                                                        replicate,
                                                                        length_prevgroup as c_int,
                                                                    ) != 0
                                                                        || (OFLOW_MAX as PCRE2_SIZE)
                                                                            .wrapping_sub(*lengthptr)
                                                                            < delta
                                                                    {
                                                                        *errorcodeptr = ERR(20);
                                                                        return 0;
                                                                    }
                                                                    *lengthptr = (*lengthptr)
                                                                        .wrapping_add(delta);
                                                                } else {
                                                                    let mut i: c_int = 0;
                                                                    while i < replicate {
                                                                        memcpy(
                                                                            code as *mut c_void,
                                                                            previous as *const c_void,
                                                                            CU2BYTES(length_prevgroup),
                                                                        );
                                                                        previous = code;
                                                                        code =
                                                                            code.add(length_prevgroup);
                                                                        i += 1;
                                                                    }
                                                                }

                                                                if repeat_min == repeat_max {
                                                                    break 'AFTER_OP_SWITCH;
                                                                }
                                                                if repeat_max != REPEAT_UNLIMITED {
                                                                    repeat_max -= repeat_min;
                                                                }
                                                                repeat_min = 0;
                                                            }

                                                            /* Wrap the recursion call in OP_BRA brackets. */
                                                            {
                                                                let length: PCRE2_SIZE =
                                                                    if !lengthptr.is_null() {
                                                                        1 + LINK_SIZE
                                                                    } else {
                                                                        length_prevgroup
                                                                    };

                                                                memmove(
                                                                    previous.add(1 + LINK_SIZE)
                                                                        as *mut c_void,
                                                                    previous as *const c_void,
                                                                    CU2BYTES(length),
                                                                );
                                                                *previous = OP_BRA as PCRE2_UCHAR;
                                                                op_previous = OP_BRA as PCRE2_UCHAR;
                                                                PUT(
                                                                    previous,
                                                                    1,
                                                                    (1 + LINK_SIZE + length) as u32,
                                                                );
                                                                *previous
                                                                    .add(1 + LINK_SIZE + length) =
                                                                    OP_KET as PCRE2_UCHAR;
                                                                PUT(
                                                                    previous,
                                                                    2 + LINK_SIZE + length,
                                                                    (1 + LINK_SIZE + length) as u32,
                                                                );
                                                            }
                                                            code = code.add(2 + 2 * LINK_SIZE);
                                                            length_prevgroup += 2 + 2 * LINK_SIZE;
                                                            group_return = -1; /* Set "may match empty string" */

                                                            /* Now treat as a repeated OP_BRA. Fall through */
                                                        }

                                                        {
                                                            let mut len: c_int =
                                                                code.offset_from(previous) as c_int;
                                                            let mut bralink: *mut PCRE2_UCHAR =
                                                                core::ptr::null_mut();
                                                            let mut brazeroptr: *mut PCRE2_UCHAR =
                                                                core::ptr::null_mut();

                                                            if repeat_max == 1
                                                                && repeat_min == 1
                                                                && possessive_quantifier == 0
                                                            {
                                                                break 'END_REPEAT;
                                                            }

                                                            if op_previous as u32 == OP_COND
                                                                && *previous.add(LINK_SIZE + 1)
                                                                    as u32
                                                                    == OP_FALSE
                                                                && *previous
                                                                    .add(GET(previous, 1) as usize)
                                                                    as u32
                                                                    != OP_ALT
                                                            {
                                                                break 'END_REPEAT;
                                                            }

                                                            if (op_previous as u32) < OP_ONCE {
                                                                /* Assertion */
                                                                if repeat_max == REPEAT_UNLIMITED {
                                                                    repeat_max = repeat_min + 1;
                                                                }
                                                            }

                                                            if repeat_min == 0 {
                                                                if repeat_max <= 1
                                                                    || repeat_max
                                                                        == REPEAT_UNLIMITED
                                                                {
                                                                    memmove(
                                                                        previous.add(1)
                                                                            as *mut c_void,
                                                                        previous as *const c_void,
                                                                        CU2BYTES(len as usize),
                                                                    );
                                                                    code = code.add(1);
                                                                    if repeat_max == 0 {
                                                                        *previous = OP_SKIPZERO
                                                                            as PCRE2_UCHAR;
                                                                        previous = previous.add(1);
                                                                        break 'END_REPEAT;
                                                                    }
                                                                    brazeroptr = previous; /* Save for possessive optimizing */
                                                                    *previous = (OP_BRAZERO
                                                                        + repeat_type)
                                                                        as PCRE2_UCHAR;
                                                                    previous = previous.add(1);
                                                                } else {
                                                                    let linkoffset: c_int;
                                                                    memmove(
                                                                        previous
                                                                            .add(2 + LINK_SIZE)
                                                                            as *mut c_void,
                                                                        previous as *const c_void,
                                                                        CU2BYTES(len as usize),
                                                                    );
                                                                    code = code.add(2 + LINK_SIZE);
                                                                    *previous = (OP_BRAZERO
                                                                        + repeat_type)
                                                                        as PCRE2_UCHAR;
                                                                    previous = previous.add(1);
                                                                    *previous =
                                                                        OP_BRA as PCRE2_UCHAR;
                                                                    previous = previous.add(1);

                                                                    linkoffset = if bralink
                                                                        .is_null()
                                                                    {
                                                                        0
                                                                    } else {
                                                                        previous.offset_from(bralink)
                                                                            as c_int
                                                                    };
                                                                    bralink = previous;
                                                                    PUT(
                                                                        previous,
                                                                        0,
                                                                        linkoffset as u32,
                                                                    );
                                                                    previous =
                                                                        previous.add(LINK_SIZE);
                                                                }

                                                                if repeat_max != REPEAT_UNLIMITED {
                                                                    repeat_max -= 1;
                                                                }
                                                            } else {
                                                                if repeat_min > 1 {
                                                                    if !lengthptr.is_null() {
                                                                        let mut delta: PCRE2_SIZE =
                                                                            0;
                                                                        if crate::chkdint::_pcre2_ckd_smul_8(
                                                                            &mut delta,
                                                                            (repeat_min - 1) as c_int,
                                                                            length_prevgroup as c_int,
                                                                        ) != 0
                                                                            || (OFLOW_MAX
                                                                                as PCRE2_SIZE)
                                                                                .wrapping_sub(
                                                                                    *lengthptr,
                                                                                )
                                                                                < delta
                                                                        {
                                                                            *errorcodeptr = ERR(20);
                                                                            return 0;
                                                                        }
                                                                        *lengthptr = (*lengthptr)
                                                                            .wrapping_add(delta);
                                                                    } else {
                                                                        if groupsetfirstcu != 0
                                                                            && reqcuflags
                                                                                >= REQ_NONE
                                                                        {
                                                                            reqcu = firstcu;
                                                                            reqcuflags =
                                                                                firstcuflags;
                                                                        }
                                                                        let mut i: u32 = 1;
                                                                        while i < repeat_min {
                                                                            memcpy(
                                                                                code as *mut c_void,
                                                                                previous
                                                                                    as *const c_void,
                                                                                CU2BYTES(
                                                                                    len as usize,
                                                                                ),
                                                                            );
                                                                            code = code
                                                                                .add(len as usize);
                                                                            i += 1;
                                                                        }
                                                                    }
                                                                }

                                                                if repeat_max != REPEAT_UNLIMITED {
                                                                    repeat_max -= repeat_min;
                                                                }
                                                            }

                                                            if repeat_max != REPEAT_UNLIMITED {
                                                                if !lengthptr.is_null()
                                                                    && repeat_max > 0
                                                                {
                                                                    let mut delta: PCRE2_SIZE = 0;
                                                                    if crate::chkdint::_pcre2_ckd_smul_8(
                                                                        &mut delta,
                                                                        repeat_max as c_int,
                                                                        length_prevgroup as c_int
                                                                            + 1
                                                                            + 2
                                                                            + 2 * LINK_SIZE as c_int,
                                                                    ) != 0
                                                                        || ((OFLOW_MAX
                                                                            + (2 + 2 * LINK_SIZE as c_int))
                                                                            as PCRE2_SIZE)
                                                                            .wrapping_sub(*lengthptr)
                                                                            < delta
                                                                    {
                                                                        *errorcodeptr = ERR(20);
                                                                        return 0;
                                                                    }
                                                                    delta = delta.wrapping_sub(
                                                                        2 + 2 * LINK_SIZE,
                                                                    ); /* Last one doesn't nest */
                                                                    *lengthptr = (*lengthptr)
                                                                        .wrapping_add(delta);
                                                                } else {
                                                                    let mut i: u32 = repeat_max;
                                                                    while i >= 1 {
                                                                        *code = (OP_BRAZERO
                                                                            + repeat_type)
                                                                            as PCRE2_UCHAR;
                                                                        code = code.add(1);

                                                                        if i != 1 {
                                                                            let linkoffset: c_int;
                                                                            *code = OP_BRA
                                                                                as PCRE2_UCHAR;
                                                                            code = code.add(1);
                                                                            linkoffset =
                                                                                if bralink.is_null()
                                                                                {
                                                                                    0
                                                                                } else {
                                                                                    code.offset_from(
                                                                                        bralink,
                                                                                    ) as c_int
                                                                                };
                                                                            bralink = code;
                                                                            PUT(
                                                                                code,
                                                                                0,
                                                                                linkoffset as u32,
                                                                            );
                                                                            code = code
                                                                                .add(LINK_SIZE);
                                                                        }

                                                                        memcpy(
                                                                            code as *mut c_void,
                                                                            previous as *const c_void,
                                                                            CU2BYTES(len as usize),
                                                                        );
                                                                        code =
                                                                            code.add(len as usize);
                                                                        i -= 1;
                                                                    }
                                                                }

                                                                /* Now chain through the pending brackets. */

                                                                while !bralink.is_null() {
                                                                    let oldlinkoffset: c_int;
                                                                    let linkoffset: c_int = (code
                                                                        .offset_from(bralink)
                                                                        + 1)
                                                                        as c_int;
                                                                    let bra: *mut PCRE2_UCHAR =
                                                                        code.offset(
                                                                            -(linkoffset as isize),
                                                                        );
                                                                    oldlinkoffset =
                                                                        GET(bra, 1) as c_int;
                                                                    bralink = if oldlinkoffset == 0 {
                                                                        core::ptr::null_mut()
                                                                    } else {
                                                                        bralink.offset(
                                                                            -(oldlinkoffset as isize),
                                                                        )
                                                                    };
                                                                    *code = OP_KET as PCRE2_UCHAR;
                                                                    code = code.add(1);
                                                                    PUT(code, 0, linkoffset as u32);
                                                                    code = code.add(LINK_SIZE);
                                                                    PUT(bra, 1, linkoffset as u32);
                                                                }
                                                            } else {
                                                                let ketcode: *mut PCRE2_UCHAR =
                                                                    code.offset(
                                                                        -1 - LINK_SIZE as isize,
                                                                    );
                                                                let bracode: *mut PCRE2_UCHAR =
                                                                    ketcode.offset(
                                                                        -(GET(ketcode, 1) as isize),
                                                                    );

                                                                /* Convert possessive ONCE brackets to non-capturing */

                                                                if *bracode as u32 == OP_ONCE
                                                                    && possessive_quantifier != 0
                                                                {
                                                                    *bracode =
                                                                        OP_BRA as PCRE2_UCHAR;
                                                                }

                                                                if *bracode as u32 == OP_ONCE
                                                                    || *bracode as u32
                                                                        == OP_SCRIPT_RUN
                                                                {
                                                                    *ketcode = (OP_KETRMAX
                                                                        + repeat_type)
                                                                        as PCRE2_UCHAR;
                                                                } else {
                                                                    if lengthptr.is_null() {
                                                                        if group_return < 0 {
                                                                            *bracode = (*bracode
                                                                                as u32
                                                                                + (OP_SBRA
                                                                                    - OP_BRA))
                                                                                as PCRE2_UCHAR;
                                                                        }
                                                                        if *bracode as u32
                                                                            == OP_COND
                                                                            && *bracode.add(
                                                                                GET(bracode, 1)
                                                                                    as usize,
                                                                            ) as u32
                                                                                != OP_ALT
                                                                        {
                                                                            *bracode = OP_SCOND
                                                                                as PCRE2_UCHAR;
                                                                        }
                                                                    }

                                                                    /* Handle possessive quantifiers. */

                                                                    if possessive_quantifier != 0 {
                                                                        if *bracode as u32
                                                                            == OP_COND
                                                                            || *bracode as u32
                                                                                == OP_SCOND
                                                                        {
                                                                            let mut nlen: c_int =
                                                                                code.offset_from(
                                                                                    bracode,
                                                                                ) as c_int;
                                                                            memmove(
                                                                                bracode.add(
                                                                                    1 + LINK_SIZE,
                                                                                )
                                                                                    as *mut c_void,
                                                                                bracode
                                                                                    as *const c_void,
                                                                                CU2BYTES(
                                                                                    nlen as usize,
                                                                                ),
                                                                            );
                                                                            code = code
                                                                                .add(1 + LINK_SIZE);
                                                                            nlen += (1 + LINK_SIZE)
                                                                                as c_int;
                                                                            *bracode = (if *bracode
                                                                                as u32
                                                                                == OP_COND
                                                                            {
                                                                                OP_BRAPOS
                                                                            } else {
                                                                                OP_SBRAPOS
                                                                            })
                                                                                as PCRE2_UCHAR;
                                                                            *code = OP_KETRPOS
                                                                                as PCRE2_UCHAR;
                                                                            code = code.add(1);
                                                                            PUT(
                                                                                code,
                                                                                0,
                                                                                nlen as u32,
                                                                            );
                                                                            code = code
                                                                                .add(LINK_SIZE);
                                                                            PUT(
                                                                                bracode,
                                                                                1,
                                                                                nlen as u32,
                                                                            );
                                                                        } else {
                                                                            *bracode = (*bracode
                                                                                as u32
                                                                                + 1)
                                                                                as PCRE2_UCHAR; /* Switch to xxxPOS opcodes */
                                                                            *ketcode = OP_KETRPOS
                                                                                as PCRE2_UCHAR;
                                                                        }

                                                                        if !brazeroptr.is_null() {
                                                                            *brazeroptr =
                                                                                OP_BRAPOSZERO
                                                                                    as PCRE2_UCHAR;
                                                                        }
                                                                        if repeat_min < 2 {
                                                                            possessive_quantifier =
                                                                                FALSE;
                                                                        }
                                                                    } else {
                                                                        *ketcode = (OP_KETRMAX
                                                                            + repeat_type)
                                                                            as PCRE2_UCHAR;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        break 'AFTER_OP_SWITCH;
                                                    }

                                                    _ => {
                                                        if (op_previous as u32) >= OP_EODN
                                                            || (op_previous as u32)
                                                                <= OP_WORD_BOUNDARY
                                                        {
                                                            *errorcodeptr = ERR(10); /* Not a character type - internal error */
                                                            return 0;
                                                        }

                                                        if repeat_max == 1 && repeat_min == 1 {
                                                            break 'END_REPEAT;
                                                        }

                                                        op_type = OP_TYPESTAR - OP_STAR; /* Use type opcodes */
                                                        mclength = 0; /* Not a character */

                                                        if op_previous as u32 == OP_PROP
                                                            || op_previous as u32 == OP_NOTPROP
                                                        {
                                                            prop_type = *previous.add(1) as c_int;
                                                            prop_value = *previous.add(2) as c_int;
                                                            break 'PROP_DONE;
                                                        }
                                                        /* Come here from just above with a character in
                                                        mcbuffer/mclength. */
                                                        break 'OUTPUT_SINGLE_REPEAT;
                                                    }
                                                } /* End of switch on different op_previous values */
                                            } /* end 'OUTPUT_SINGLE_REPEAT */

                                            /* OUTPUT_SINGLE_REPEAT: */
                                            prop_type = -1;
                                            prop_value = -1;
                                        } /* end 'PROP_DONE */

                                        oldcode = code; /* Save where we were */
                                        code = previous; /* Usually overwrite previous item */

                                        if repeat_max == 0 {
                                            break 'END_REPEAT;
                                        }

                                        /* Combine the op_type with the repeat_type */

                                        repeat_type += op_type;

                                        if repeat_min == 0 {
                                            if repeat_max == REPEAT_UNLIMITED {
                                                *code = (OP_STAR + repeat_type) as PCRE2_UCHAR;
                                                code = code.add(1);
                                            } else if repeat_max == 1 {
                                                *code = (OP_QUERY + repeat_type) as PCRE2_UCHAR;
                                                code = code.add(1);
                                            } else {
                                                *code = (OP_UPTO + repeat_type) as PCRE2_UCHAR;
                                                code = code.add(1);
                                                PUT2(code, 0, repeat_max);
                                                code = code.add(IMM2_SIZE);
                                            }
                                        } else if repeat_min == 1 {
                                            if repeat_max == REPEAT_UNLIMITED {
                                                *code = (OP_PLUS + repeat_type) as PCRE2_UCHAR;
                                                code = code.add(1);
                                            } else {
                                                code = oldcode; /* Leave previous item in place */
                                                if repeat_max == 1 {
                                                    break 'END_REPEAT;
                                                }
                                                *code = (OP_UPTO + repeat_type) as PCRE2_UCHAR;
                                                code = code.add(1);
                                                PUT2(code, 0, repeat_max - 1);
                                                code = code.add(IMM2_SIZE);
                                            }
                                        } else {
                                            *code = (OP_EXACT + op_type) as PCRE2_UCHAR; /* NB EXACT doesn't have repeat_type */
                                            code = code.add(1);
                                            PUT2(code, 0, repeat_min);
                                            code = code.add(IMM2_SIZE);

                                            if repeat_max != repeat_min {
                                                if mclength > 0 {
                                                    memcpy(
                                                        code as *mut c_void,
                                                        mcbuffer.as_ptr() as *const c_void,
                                                        CU2BYTES(mclength as usize),
                                                    );
                                                    code = code.add(mclength as usize);
                                                } else {
                                                    *code = op_previous;
                                                    code = code.add(1);
                                                    if prop_type >= 0 {
                                                        *code = prop_type as PCRE2_UCHAR;
                                                        code = code.add(1);
                                                        *code = prop_value as PCRE2_UCHAR;
                                                        code = code.add(1);
                                                    }
                                                }

                                                /* Now set up the following opcode */

                                                if repeat_max == REPEAT_UNLIMITED {
                                                    *code = (OP_STAR + repeat_type) as PCRE2_UCHAR;
                                                    code = code.add(1);
                                                } else {
                                                    repeat_max -= repeat_min;
                                                    if repeat_max == 1 {
                                                        *code =
                                                            (OP_QUERY + repeat_type) as PCRE2_UCHAR;
                                                        code = code.add(1);
                                                    } else {
                                                        *code =
                                                            (OP_UPTO + repeat_type) as PCRE2_UCHAR;
                                                        code = code.add(1);
                                                        PUT2(code, 0, repeat_max);
                                                        code = code.add(IMM2_SIZE);
                                                    }
                                                }
                                            }
                                        }

                                        /* Fill in the character or character type for the final opcode. */

                                        if mclength > 0 {
                                            memcpy(
                                                code as *mut c_void,
                                                mcbuffer.as_ptr() as *const c_void,
                                                CU2BYTES(mclength as usize),
                                            );
                                            code = code.add(mclength as usize);
                                        } else {
                                            *code = op_previous;
                                            code = code.add(1);
                                            if prop_type >= 0 {
                                                *code = prop_type as PCRE2_UCHAR;
                                                code = code.add(1);
                                                *code = prop_value as PCRE2_UCHAR;
                                                code = code.add(1);
                                            }
                                        }
                                    } /* end 'AFTER_OP_SWITCH */

                                    /* If the character following a repeat is '+', possessive_quantifier is
                                    TRUE. */

                                    if possessive_quantifier != 0 {
                                        let len: c_int;

                                        match *tempcode as u32 {
                                            OP_TYPEEXACT => {
                                                tempcode = tempcode.add(
                                                    _pcre2_OP_lengths_8[*tempcode as usize]
                                                        as usize
                                                        + (if *tempcode.add(1 + IMM2_SIZE) as u32
                                                            == OP_PROP
                                                            || *tempcode.add(1 + IMM2_SIZE) as u32
                                                                == OP_NOTPROP
                                                        {
                                                            2
                                                        } else {
                                                            0
                                                        }),
                                                );
                                            }

                                            /* CHAR opcodes are used for exacts whose count is 1. */
                                            OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_EXACT
                                            | OP_EXACTI | OP_NOTEXACT | OP_NOTEXACTI => {
                                                tempcode = tempcode.add(
                                                    _pcre2_OP_lengths_8[*tempcode as usize] as usize,
                                                );
                                                if utf != 0
                                                    && HAS_EXTRALEN(*tempcode.offset(-1) as u32)
                                                {
                                                    tempcode = tempcode.add(GET_EXTRALEN(
                                                        *tempcode.offset(-1) as u32,
                                                    ) as usize);
                                                }
                                            }

                                            OP_CLASS | OP_NCLASS => {
                                                tempcode = tempcode.add(1 + 32);
                                            }

                                            OP_XCLASS | OP_ECLASS => {
                                                tempcode = tempcode.add(GET(tempcode, 1) as usize);
                                            }

                                            OP_REF | OP_REFI | OP_DNREF | OP_DNREFI => {
                                                tempcode = tempcode.add(
                                                    _pcre2_OP_lengths_8[*tempcode as usize] as usize,
                                                );
                                            }

                                            _ => {}
                                        }

                                        len = code.offset_from(tempcode) as c_int;
                                        if len > 0 {
                                            let repcode: c_uint = *tempcode as c_uint;

                                            if (repcode as u32) < OP_CALLOUT
                                                && opcode_possessify[repcode as usize] > 0
                                            {
                                                *tempcode = opcode_possessify[repcode as usize];
                                            } else {
                                                let mut len2: c_int = len;
                                                memmove(
                                                    tempcode.add(1 + LINK_SIZE) as *mut c_void,
                                                    tempcode as *const c_void,
                                                    CU2BYTES(len2 as usize),
                                                );
                                                code = code.add(1 + LINK_SIZE);
                                                len2 += (1 + LINK_SIZE) as c_int;
                                                *tempcode.add(0) = OP_ONCE as PCRE2_UCHAR;
                                                *code = OP_KET as PCRE2_UCHAR;
                                                code = code.add(1);
                                                PUT(code, 0, len2 as u32);
                                                code = code.add(LINK_SIZE);
                                                PUT(tempcode, 1, len2 as u32);
                                            }
                                        }
                                    }

                                    /* Falls through into END_REPEAT */
                                } /* end 'END_REPEAT */

                                /* END_REPEAT: */
                                (*cb).req_varyopt |= reqvary;
                                break 'sw;
                            } /* end 'HANDLE_SINGLE_REFERENCE */

                            /* HANDLE_SINGLE_REFERENCE: */
                            if firstcuflags == REQ_UNSET {
                                firstcuflags = REQ_NONE;
                                zerofirstcuflags = REQ_NONE;
                            }
                            *code = (if (options & PCRE2_CASELESS) != 0 { OP_REFI } else { OP_REF })
                                as PCRE2_UCHAR;
                            code = code.add(1);
                            PUT2(code, 0, meta_arg);
                            code = code.add(IMM2_SIZE);
                            if (options & PCRE2_CASELESS) != 0 {
                                *code = ((if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0 {
                                    REFI_FLAG_CASELESS_RESTRICT
                                } else {
                                    0
                                }) | (if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0 {
                                    REFI_FLAG_TURKISH_CASING
                                } else {
                                    0
                                })) as PCRE2_UCHAR;
                                code = code.add(1);
                            }

                            /* Update the map of back references, and keep the highest one. */

                            (*cb).backref_map |= if meta_arg < 32 { 1u32 << meta_arg } else { 1 };
                            if meta_arg > (*cb).top_backref {
                                (*cb).top_backref = meta_arg;
                            }
                            break 'sw;
                        } /* end 'HANDLE_NUMERICAL_RECURSION */

                        /* HANDLE_NUMERICAL_RECURSION: */
                        *code = OP_RECURSE as PCRE2_UCHAR;
                        PUT(code, 1, meta_arg);
                        code = code.add(1 + LINK_SIZE);
                        /* Repeat processing requires this information to
                        determine the real length in pre-compile phase. */
                        length_prevgroup = 1 + LINK_SIZE;

                        if META_CODE(*pptr.add(1)) == META_OFFSET
                            || META_CODE(*pptr.add(1)) == META_CAPTURE_NAME
                            || META_CODE(*pptr.add(1)) == META_CAPTURE_NUMBER
                        {
                            let args: *mut recurse_arguments;

                            if !lengthptr.is_null() {
                                if crate::compile_cgroup::_pcre2_compile_parse_recurse_args8(
                                    pptr,
                                    offset,
                                    errorcodeptr,
                                    cb,
                                ) == 0
                                {
                                    return 0;
                                }

                                args = (*cb).last_data as *mut recurse_arguments;
                                length_prevgroup += (*args).size * (1 + IMM2_SIZE);
                                *lengthptr = (*lengthptr)
                                    .wrapping_add((*args).size * (1 + IMM2_SIZE));
                                pptr = pptr.add((*args).skip_size);
                            } else {
                                let mut current: *mut u16;
                                let end: *mut u16;

                                args = (*cb).first_data as *mut recurse_arguments;

                                current = args.add(1) as *mut u16;
                                end = current.add((*args).size);

                                loop {
                                    *code.add(0) = OP_CREF as PCRE2_UCHAR;
                                    PUT2(code, 1, *current as u32);
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
                        if firstcuflags == REQ_UNSET {
                            firstcuflags = REQ_NONE;
                        }
                        zerofirstcu = firstcu;
                        zerofirstcuflags = firstcuflags;
                        break 'sw;
                    } /* end 'NORMAL_CHAR */

                    /* NORMAL_CHAR: */
                    meta = *pptr; /* Get the full 32 bits */
                    /* falls through to NORMAL_CHAR_SET */
                } /* end 'NORMAL_CHAR_SET */

                /* NORMAL_CHAR_SET: Character is already in meta */
                matched_char = TRUE;

                if (utf != 0 || ucp != 0) && (options & PCRE2_CASELESS) != 0 {
                    let mut caseset: u32;

                    if (xoptions & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                        == PCRE2_EXTRA_TURKISH_CASING
                        && UCD_ANY_I(meta)
                    {
                        caseset = _pcre2_ucd_turkish_dotted_i_caseset_8
                            + (if UCD_DOTTED_I(meta) { 0 } else { 3 });
                    } else {
                        caseset = UCD_CASESET(meta);
                        if caseset != 0
                            && (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                            && _pcre2_ucd_caseless_sets_8[caseset as usize] < 128
                        {
                            caseset = 0; /* Ignore the caseless set if it's restricted. */
                        }
                    }

                    if caseset != 0 {
                        *code = OP_PROP as PCRE2_UCHAR;
                        code = code.add(1);
                        *code = PT_CLIST as PCRE2_UCHAR;
                        code = code.add(1);
                        *code = caseset as PCRE2_UCHAR;
                        code = code.add(1);
                        if firstcuflags == REQ_UNSET {
                            firstcuflags = REQ_NONE;
                            zerofirstcuflags = REQ_NONE;
                        }
                        break 'sw; /* End handling this meta item */
                    }
                }
                /* Falls through into CLASS_CASELESS_CHAR */
            } /* end 'CLASS_CASELESS_CHAR */

            /* CLASS_CASELESS_CHAR: */

            /* Get the character's code units into mcbuffer, with the length in
            mclength. When not in UTF mode, the length is always 1. */

            if utf != 0 {
                mclength = crate::ord2utf::_pcre2_ord2utf_8(meta, mcbuffer.as_mut_ptr());
            } else {
                mclength = 1;
                mcbuffer[0] = meta as PCRE2_UCHAR;
            }

            /* Generate the appropriate code */

            *code = (if (options & PCRE2_CASELESS) != 0 { OP_CHARI } else { OP_CHAR })
                as PCRE2_UCHAR;
            code = code.add(1);
            memcpy(
                code as *mut c_void,
                mcbuffer.as_ptr() as *const c_void,
                CU2BYTES(mclength as usize),
            );
            code = code.add(mclength as usize);

            /* Remember if \r or \n were seen */

            if mcbuffer[0] as u32 == CHAR_CR || mcbuffer[0] as u32 == CHAR_NL {
                (*cb).external_flags |= PCRE2_HASCRORLF;
            }

            if firstcuflags == REQ_UNSET {
                zerofirstcuflags = REQ_NONE;
                zeroreqcu = reqcu;
                zeroreqcuflags = reqcuflags;

                if mclength == 1 || req_caseopt == 0 {
                    firstcu = mcbuffer[0] as u32;
                    firstcuflags = req_caseopt;
                    if mclength != 1 {
                        reqcu = *code.offset(-1) as u32;
                        reqcuflags = (*cb).req_varyopt;
                    }
                } else {
                    firstcuflags = REQ_NONE;
                    reqcuflags = REQ_NONE;
                }
            } else {
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

            break 'sw; /* End literal character handling */
        } /* end 'sw */

        pptr = pptr.add(1);
    }
}

pub(crate) unsafe fn compile_regex(
    options: u32,
    xoptions: u32,
    codeptr: *mut *mut PCRE2_UCHAR,
    pptrptr: *mut *mut u32,
    errorcodeptr: *mut c_int,
    skipunits: u32,
    firstcuptr: *mut u32,
    firstcuflagsptr: *mut u32,
    reqcuptr: *mut u32,
    reqcuflagsptr: *mut u32,
    bcptr: *mut branch_chain,
    open_caps: *mut open_capitem,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    let mut options = options;
    let mut xoptions = xoptions;
    let mut open_caps = open_caps;
    let mut code: *mut PCRE2_UCHAR = *codeptr;
    let mut last_branch: *mut PCRE2_UCHAR = code;
    let start_bracket: *mut PCRE2_UCHAR = code;
    let lookbehind: BOOL;
    let mut capitem: open_capitem = core::mem::zeroed();
    let mut capnumber: c_int = 0;
    let mut okreturn: c_int = 1;
    let mut pptr: *mut u32 = *pptrptr;
    let mut firstcu: u32;
    let mut reqcu: u32;
    let mut lookbehindlength: u32;
    let mut lookbehindminlength: u32;
    let mut firstcuflags: u32;
    let mut reqcuflags: u32;
    let mut length: PCRE2_SIZE;
    let mut bc: branch_chain = core::mem::zeroed();

    /* If set, call the external function that checks for stack availability. */

    if (*(*cb).cx).stack_guard.is_some()
        && ((*(*cb).cx).stack_guard.unwrap())(
            (*cb).parens_depth as u32,
            (*(*cb).cx).stack_guard_data,
        ) != 0
    {
        *errorcodeptr = ERR(33);
        (*cb).erroroffset = 0;
        return 0;
    }

    /* Miscellaneous initialization */

    bc.outer = bcptr;
    bc.current_branch = code;

    firstcu = 0;
    reqcu = 0;
    firstcuflags = REQ_UNSET;
    reqcuflags = REQ_UNSET;

    length = 2 + 2 * LINK_SIZE + skipunits as PCRE2_SIZE;

    /* Remember if this is a lookbehind assertion. */

    lookbehind = (*code as u32 == OP_ASSERTBACK
        || *code as u32 == OP_ASSERTBACK_NOT
        || *code as u32 == OP_ASSERTBACK_NA) as BOOL;

    if lookbehind != 0 {
        lookbehindlength = META_DATA(*pptr.offset(-1));
        lookbehindminlength = *pptr;
        pptr = pptr.add(SIZEOFFSET);
    } else {
        lookbehindlength = 0;
        lookbehindminlength = 0;
    }

    /* If this is a capturing subpattern, add to the chain of open capturing items. */

    if *code as u32 == OP_CBRA {
        capnumber = GET2(code, 1 + LINK_SIZE) as c_int;
        capitem.number = capnumber as u16;
        capitem.next = open_caps;
        capitem.assert_depth = (*cb).assert_depth;
        open_caps = &mut capitem;
    }

    /* Offset is set zero to mark that this bracket is still open */

    PUT(code, 1, 0);
    code = code.add(1 + LINK_SIZE + skipunits as usize);

    /* Loop for each alternative branch */

    loop {
        let branch_return: c_int;
        let mut branchfirstcu: u32 = 0;
        let mut branchreqcu: u32 = 0;
        let mut branchfirstcuflags: u32 = REQ_UNSET;
        let mut branchreqcuflags: u32 = REQ_UNSET;

        /* Insert OP_REVERSE or OP_VREVERSE if this is a lookbehind assertion. */

        if lookbehind != 0 && lookbehindlength > 0 {
            if lookbehindminlength == LOOKBEHIND_MAX as u32
                || lookbehindminlength == lookbehindlength
            {
                *code = OP_REVERSE as PCRE2_UCHAR;
                code = code.add(1);
                PUT2(code, 0, lookbehindlength);
                code = code.add(IMM2_SIZE);
                length += 1 + IMM2_SIZE;
            } else {
                *code = OP_VREVERSE as PCRE2_UCHAR;
                code = code.add(1);
                PUT2(code, 0, lookbehindminlength);
                code = code.add(IMM2_SIZE);
                PUT2(code, 0, lookbehindlength);
                code = code.add(IMM2_SIZE);
                length += 1 + 2 * IMM2_SIZE;
            }
        }

        /* Now compile the branch. */

        branch_return = compile_branch(
            &mut options,
            &mut xoptions,
            &mut code,
            &mut pptr,
            errorcodeptr,
            &mut branchfirstcu,
            &mut branchfirstcuflags,
            &mut branchreqcu,
            &mut branchreqcuflags,
            &mut bc,
            open_caps,
            cb,
            if lengthptr.is_null() {
                core::ptr::null_mut()
            } else {
                &mut length
            },
        );
        if branch_return == 0 {
            return 0;
        }

        /* If a branch can match an empty string, so can the whole group. */

        if branch_return < 0 {
            okreturn = -1;
        }

        /* In the real compile phase, there is some post-processing to be done. */

        if lengthptr.is_null() {
            if *last_branch as u32 != OP_ALT {
                firstcu = branchfirstcu;
                firstcuflags = branchfirstcuflags;
                reqcu = branchreqcu;
                reqcuflags = branchreqcuflags;
            } else {
                if firstcuflags != branchfirstcuflags || firstcu != branchfirstcu {
                    if firstcuflags < REQ_NONE {
                        if reqcuflags >= REQ_NONE {
                            reqcu = firstcu;
                            reqcuflags = firstcuflags;
                        }
                    }
                    firstcuflags = REQ_NONE;
                }

                if firstcuflags >= REQ_NONE
                    && branchfirstcuflags < REQ_NONE
                    && branchreqcuflags >= REQ_NONE
                {
                    branchreqcu = branchfirstcu;
                    branchreqcuflags = branchfirstcuflags;
                }

                /* Now ensure that the reqcus match */

                if ((reqcuflags & !REQ_VARY) != (branchreqcuflags & !REQ_VARY))
                    || reqcu != branchreqcu
                {
                    reqcuflags = REQ_NONE;
                } else {
                    reqcu = branchreqcu;
                    reqcuflags |= branchreqcuflags; /* To "or" REQ_VARY if present */
                }
            }
        }

        /* Handle reaching the end of the expression, either ')' or end of pattern. */

        if META_CODE(*pptr) != META_ALT {
            if lengthptr.is_null() {
                let mut branch_length: u32 = code.offset_from(last_branch) as u32;
                loop {
                    let prev_length: u32 = GET(last_branch, 1);
                    PUT(last_branch, 1, branch_length);
                    branch_length = prev_length;
                    last_branch = last_branch.offset(-(branch_length as isize));
                    if !(branch_length > 0) {
                        break;
                    }
                }
            }

            /* Fill in the ket */

            *code = OP_KET as PCRE2_UCHAR;
            PUT(code, 1, code.offset_from(start_bracket) as u32);
            code = code.add(1 + LINK_SIZE);

            /* Set values to pass back */

            *codeptr = code;
            *pptrptr = pptr;
            *firstcuptr = firstcu;
            *firstcuflagsptr = firstcuflags;
            *reqcuptr = reqcu;
            *reqcuflagsptr = reqcuflags;
            if !lengthptr.is_null() {
                if (OFLOW_MAX as PCRE2_SIZE).wrapping_sub(*lengthptr) < length {
                    *errorcodeptr = ERR(20);
                    return 0;
                }
                *lengthptr = (*lengthptr).wrapping_add(length);
            }
            return okreturn;
        }

        /* Another branch follows. */

        if !lengthptr.is_null() {
            code = (*codeptr).add(1 + LINK_SIZE + skipunits as usize);
            length += 1 + LINK_SIZE;
        } else {
            *code = OP_ALT as PCRE2_UCHAR;
            PUT(code, 1, code.offset_from(last_branch) as u32);
            last_branch = code;
            bc.current_branch = code;
            code = code.add(1 + LINK_SIZE);
        }

        /* Set the maximum lookbehind length for the next branch. */

        lookbehindlength = META_DATA(*pptr);
        pptr = pptr.add(1);
    }
}
