//! Translation of `first_significant_code`, `compile_branch` and
//! `compile_regex` from `c_src/src/pcre2_compile.c` (roughly lines 5960..8895).
//!
//! Built for the 8-bit library with `SUPPORT_UNICODE` (hence
//! `SUPPORT_WIDE_CHARS` and `MAYBE_UTF_MULTI`), `LINK_SIZE == 2`, no JIT, no
//! EBCDIC, no `PCRE2_DEBUG`.

#![allow(non_snake_case, non_upper_case_globals, unused_parens, dead_code)]

use core::ffi::c_int;

use crate::chars::*;
use crate::compile_internal::*;
use crate::internal::*;
use crate::opcodes::*;
use crate::ucp::*;

use crate::compile_tables::{
    chartypeoffset, opcode_possessify, verbops, MAX_GROUP_NUMBER, REPEAT_UNLIMITED, REQ_CASELESS,
    REQ_NONE, REQ_UNSET, REQ_VARY,
};

const INT_MAX: c_int = c_int::MAX;
/* OFLOW_MAX = INT_MAX - 20 */
const OFLOW_MAX: PCRE2_SIZE = (c_int::MAX - 20) as PCRE2_SIZE;
const WORK_SIZE_SAFETY_MARGIN: usize = 100;

/* PUTINC(a, n, d): PUT then advance the pointer by LINK_SIZE. */
#[inline]
unsafe fn putinc(a: &mut *mut PCRE2_UCHAR, n: usize, d: c_int) {
    unsafe {
        put(*a, n, d);
        *a = a.add(LINK_SIZE);
    }
}

/* PUT2INC(a, n, d): PUT2 then advance the pointer by IMM2_SIZE. */
#[inline]
unsafe fn put2inc(a: &mut *mut PCRE2_UCHAR, n: usize, d: u32) {
    unsafe {
        put2(*a, n, d);
        *a = a.add(IMM2_SIZE);
    }
}

/*************************************************
*      Find first significant op code            *
*************************************************/

/* This is called by several functions that scan a compiled expression looking
for a fixed first code unit, or an anchoring op code etc. It skips over things
that do not influence this. For some calls, it makes sense to skip negative
forward and all backward assertions, and also the \b assertion; for others it
does not.

Arguments:
  code         pointer to the start of the group
  skipassert   TRUE if certain assertions are to be skipped

Returns:       pointer to the first significant opcode
*/

pub(crate) unsafe fn first_significant_code(mut code: PCRE2_SPTR, skipassert: BOOL) -> PCRE2_SPTR {
    unsafe {
        loop {
            match *code {
                OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT | OP_ASSERTBACK_NA => {
                    if skipassert == FALSE {
                        return code;
                    }
                    loop {
                        code = code.add(get(code, 1) as usize);
                        if *code != OP_ALT {
                            break;
                        }
                    }
                    code = code.add(OP_LENGTHS[*code as usize] as usize);
                }

                OP_WORD_BOUNDARY | OP_NOT_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY
                | OP_NOT_UCP_WORD_BOUNDARY => {
                    if skipassert == FALSE {
                        return code;
                    }
                    /* Fall through */
                    code = code.add(OP_LENGTHS[*code as usize] as usize);
                }

                OP_CALLOUT | OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF | OP_FALSE | OP_TRUE => {
                    code = code.add(OP_LENGTHS[*code as usize] as usize);
                }

                OP_CALLOUT_STR => {
                    code = code.add(get(code, 1 + 2 * LINK_SIZE) as usize);
                }

                OP_SKIPZERO => {
                    code = code.add(2 + get(code, 2) as usize + LINK_SIZE);
                }

                OP_COND | OP_SCOND => {
                    if *code.add(1 + LINK_SIZE) != OP_FALSE   /* Not DEFINE */
                        || *code.add(get(code, 1) as usize) != OP_KET
                    /* More than one branch */
                    {
                        return code;
                    }
                    code = code.add(get(code, 1) as usize + 1 + LINK_SIZE);
                }

                OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                    code = code.add(*code.add(1) as usize + OP_LENGTHS[*code as usize] as usize);
                }

                _ => return code,
            }
        }
    }
}

/* Result of the big per-item switch in compile_branch(): where to go next. */
enum Action {
    /* Continue the main loop (pptr++). Corresponds to `break` from the C
    switch. */
    Continue,
    /* Return this value from compile_branch(). */
    Return(c_int),
    /* Jump to the shared group-processing block. `note_empty` corresponds to
    entering via GROUP_PROCESS_NOTE_EMPTY (TRUE) or GROUP_PROCESS (FALSE). */
    GroupProcess { note_empty: bool },
    /* Jump to the REPEAT block. */
    Repeat,
    /* Jump to HANDLE_NUMERICAL_RECURSION. */
    HandleNumericalRecursion,
    /* Jump to HANDLE_SINGLE_REFERENCE. */
    HandleSingleReference,
    /* Jump to NORMAL_CHAR_SET (character already in `meta`). */
    NormalCharSet,
    /* Jump to CLASS_CASELESS_CHAR (character already in `meta`). */
    ClassCaselessChar,
    /* Jump to CLASS_END_PROCESSING (used after compile_class_nested). */
    ClassEndProcessing,
}

/*************************************************
*           Compile one branch                   *
*************************************************/

/* Scan the parsed pattern, compiling it into a vector of PCRE2_UCHAR. See the
C source for the full description.

Returns:            0 There's been an error, *errorcodeptr is non-zero
                   +1 Success, this branch must match at least one character
                   -1 Success, this branch may match an empty string
*/

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
    unsafe {
        let mut bravalue: c_int = 0;
        let mut okreturn: c_int = -1;
        let mut group_return: c_int = 0;
        let mut repeat_min: u32 = 0;
        let mut repeat_max: u32 = 0; /* To please picky compilers */
        let mut greedy_default: u32;
        let mut greedy_non_default: u32;
        let mut repeat_type: u32;
        let mut op_type: u32;
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
        let mut reqvary: u32;
        let mut tempreqvary: u32;
        /* Some opcodes, such as META_CAPTURE_NUMBER or META_CAPTURE_NAME,
        depends on the previous value of offset. */
        let mut offset: PCRE2_SIZE = 0;
        let mut length_prevgroup: PCRE2_SIZE = 0;
        let mut code: *mut PCRE2_UCHAR = *codeptr;
        let mut last_code: *mut PCRE2_UCHAR = code;
        let orig_code: *mut PCRE2_UCHAR = code;
        let mut tempcode: *mut PCRE2_UCHAR;
        let mut previous: *mut PCRE2_UCHAR = core::ptr::null_mut();
        let mut op_previous: PCRE2_UCHAR;
        let mut groupsetfirstcu: BOOL = FALSE;
        let mut had_accept: BOOL = FALSE;
        let mut matched_char: BOOL = FALSE;
        let mut previous_matched_char: BOOL;
        let mut reset_caseful: BOOL = FALSE;

        /* We can fish out the UTF setting once and for all into a BOOL. */

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
            let mut possessive_quantifier: BOOL;
            let mut note_group_empty: BOOL;
            let mut mclength: u32;
            let mut skipunits: u32;
            let mut subreqcu: u32 = 0;
            let mut subfirstcu: u32 = 0;
            let mut groupnumber: u32;
            let verbarglen: u32;
            let mut verbculen: u32;
            let mut subreqcuflags: u32 = REQ_UNSET;
            let mut subfirstcuflags: u32 = REQ_UNSET;
            let mut oc: *mut open_capitem;
            let mut mcbuffer = [0u8; 8];

            /* Get next META item in the pattern and its potential argument. */

            meta = meta_code(*pptr);
            meta_arg = meta_data(*pptr);

            /* If we are in the pre-compile phase, accumulate the length used for
            the previous cycle of this loop, unless the next item is a
            quantifier. */

            if !lengthptr.is_null() {
                if code >= (*cb).start_workspace.add((*cb).workspace_size) {
                    *errorcodeptr = ERR52; /* Over-ran workspace - internal error */
                    (*cb).erroroffset = 0;
                    return 0;
                }

                if code > (*cb).start_workspace.add((*cb).workspace_size - WORK_SIZE_SAFETY_MARGIN) {
                    *errorcodeptr = ERR86; /* Pattern too complicated */
                    (*cb).erroroffset = 0;
                    return 0;
                }

                /* There is at least one situation where code goes backwards:
                this is the case of a zero quantifier after a class (e.g.
                [ab]{0}). Don't ever reduce the length at this point. */

                if code < last_code {
                    code = last_code;
                }

                /* If the next thing is not a quantifier, we add the length of the
                previous item into the total, and reset the code pointer to the
                start of the workspace. */

                if meta < META_ASTERISK || meta > META_MINMAX_QUERY {
                    if OFLOW_MAX - *lengthptr < code.offset_from(orig_code) as PCRE2_SIZE {
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

                /* Remember where this code item starts so we can catch the
                "backwards" case above next time round. */

                last_code = code;
            }

            /* Process the next parsed pattern item. If it is not a quantifier,
            remember where it starts so that it can be quantified when a
            quantifier follows. */

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

            /* Compute the action for this meta item. The big switch is
            translated into a helper closure-like block that yields an Action. */

            let action: Action = 'dispatch: {
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

                    /* Handle single-character metacharacters. */
                    META_CIRCUMFLEX => {
                        if (options & PCRE2_MULTILINE) != 0 {
                            if firstcuflags == REQ_UNSET {
                                zerofirstcuflags = REQ_NONE;
                                firstcuflags = REQ_NONE;
                            }
                            *code = OP_CIRCM;
                            code = code.add(1);
                        } else {
                            *code = OP_CIRC;
                            code = code.add(1);
                        }
                        Action::Continue
                    }

                    META_DOLLAR => {
                        *code = if (options & PCRE2_MULTILINE) != 0 { OP_DOLLM } else { OP_DOLL };
                        code = code.add(1);
                        Action::Continue
                    }

                    /* There can never be a first char if '.' is first. */
                    META_DOT => {
                        matched_char = TRUE;
                        if firstcuflags == REQ_UNSET {
                            firstcuflags = REQ_NONE;
                        }
                        zerofirstcu = firstcu;
                        zerofirstcuflags = firstcuflags;
                        zeroreqcu = reqcu;
                        zeroreqcuflags = reqcuflags;
                        *code = if (options & PCRE2_DOTALL) != 0 { OP_ALLANY } else { OP_ANY };
                        code = code.add(1);
                        Action::Continue
                    }

                    /* Empty character classes. */
                    META_CLASS_EMPTY | META_CLASS_EMPTY_NOT => {
                        matched_char = TRUE;
                        if meta == META_CLASS_EMPTY_NOT {
                            *code = OP_ALLANY;
                            code = code.add(1);
                        } else {
                            *code = OP_CLASS;
                            code = code.add(1);
                            memset(code, 0, 32);
                            code = code.add(32);
                        }

                        if firstcuflags == REQ_UNSET {
                            firstcuflags = REQ_NONE;
                        }
                        zerofirstcu = firstcu;
                        zerofirstcuflags = firstcuflags;
                        Action::Continue
                    }

                    /* Non-empty character class. */
                    META_CLASS_NOT | META_CLASS => {
                        matched_char = TRUE;

                        /* Check for complex extended classes. */
                        if (*pptr & CLASS_IS_ECLASS) != 0 {
                            if crate::compile_class::compile_class_nested(
                                options,
                                xoptions,
                                &mut pptr,
                                &mut code,
                                errorcodeptr,
                                cb,
                                lengthptr,
                            ) == FALSE
                            {
                                return 0;
                            }
                            break 'dispatch Action::ClassEndProcessing;
                        }

                        /* Optimize a single character in a class. */
                        if *pptr.add(1) < META_END && *pptr.add(2) == META_CLASS_END {
                            let c: u32 = *pptr.add(1);

                            pptr = pptr.add(2); /* Move on to class end */
                            if meta == META_CLASS {
                                /* A positive one-char class handled as a
                                normal literal character. */
                                meta = c;
                                break 'dispatch Action::NormalCharSet;
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
                                    && ucd_any_i(c)
                                {
                                    caseset = UCD_TURKISH_DOTTED_I_CASESET
                                        + (if ucd_dotted_i(c) { 0 } else { 3 });
                                } else {
                                    caseset = ucd_caseset(c);
                                    if caseset != 0
                                        && (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                                        && UCD_CASELESS_SETS[caseset as usize] < 128
                                    {
                                        caseset = 0; /* Ignore restricted caseless set. */
                                    }
                                }

                                if caseset != 0 {
                                    *code = OP_NOTPROP;
                                    code = code.add(1);
                                    *code = PT_CLIST as u8;
                                    code = code.add(1);
                                    *code = caseset as u8;
                                    code = code.add(1);
                                    break 'dispatch Action::Continue; /* Finished with this class */
                                }
                            }

                            /* Char has only one other (usable) case. */

                            *code = if (options & PCRE2_CASELESS) != 0 { OP_NOTI } else { OP_NOT };
                            code = code.add(1);
                            code = code.add(putchar_(c, code, utf != 0) as usize);
                            break 'dispatch Action::Continue; /* Finished with this class */
                        } /* End of 1-char optimization */

                        /* Two characters that are case partners. */
                        if meta == META_CLASS
                            && *pptr.add(1) < META_END
                            && *pptr.add(2) < META_END
                            && *pptr.add(3) == META_CLASS_END
                        {
                            let c: u32 = *pptr.add(1);

                            if (ucd_caseset(c) == 0
                                || ((xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                                    && c < 128
                                    && *pptr.add(2) < 128))
                                && !((xoptions
                                    & (PCRE2_EXTRA_TURKISH_CASING
                                        | PCRE2_EXTRA_CASELESS_RESTRICT))
                                    == PCRE2_EXTRA_TURKISH_CASING
                                    && ucd_any_i(c))
                            {
                                let d: u32;

                                if (utf != 0 || ucp != 0) && c > 127 {
                                    d = ucd_othercase(c);
                                } else {
                                    d = table_get(c, (*cb).fcc, c);
                                }

                                if c != d && *pptr.add(2) == d {
                                    pptr = pptr.add(3); /* Move on to class end */
                                    meta = c;
                                    if (options & PCRE2_CASELESS) == 0 {
                                        reset_caseful = TRUE;
                                        options |= PCRE2_CASELESS;
                                        req_caseopt = REQ_CASELESS;
                                    }
                                    break 'dispatch Action::ClassCaselessChar;
                                }
                            }
                        }

                        /* Now emit the OP_CLASS/OP_NCLASS/OP_XCLASS/OP_ALLANY. */

                        pptr = crate::compile_class::compile_class_not_nested(
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
                        debug_assert!(*pptr == META_CLASS_END);

                        Action::ClassEndProcessing
                    }

                    /* Deal with (*VERB)s. */
                    META_ACCEPT => {
                        (*cb).had_accept = TRUE;
                        had_accept = TRUE;
                        oc = open_caps;
                        while !oc.is_null() && (*oc).assert_depth >= (*cb).assert_depth {
                            if !lengthptr.is_null() {
                                *lengthptr += 1 + IMM2_SIZE;
                            } else {
                                *code = OP_CLOSE;
                                code = code.add(1);
                                put2inc(&mut code, 0, (*oc).number as u32);
                            }
                            oc = (*oc).next;
                        }
                        *code = if (*cb).assert_depth > 0 { OP_ASSERT_ACCEPT } else { OP_ACCEPT };
                        code = code.add(1);
                        if firstcuflags == REQ_UNSET {
                            firstcuflags = REQ_NONE;
                        }
                        Action::Continue
                    }

                    META_PRUNE | META_SKIP => {
                        (*cb).had_pruneorskip = TRUE;
                        /* Fall through */
                        *code = verbops[((meta - META_MARK) >> 16) as usize] as u8;
                        code = code.add(1);
                        Action::Continue
                    }
                    META_COMMIT | META_FAIL => {
                        *code = verbops[((meta - META_MARK) >> 16) as usize] as u8;
                        code = code.add(1);
                        Action::Continue
                    }

                    META_THEN => {
                        (*cb).external_flags |= PCRE2_HASTHEN;
                        *code = OP_THEN;
                        code = code.add(1);
                        Action::Continue
                    }

                    /* Handle verbs with arguments. */
                    META_THEN_ARG => {
                        (*cb).external_flags |= PCRE2_HASTHEN;
                        /* goto VERB_ARG */
                        *code = verbops[((meta - META_MARK) >> 16) as usize] as u8;
                        code = code.add(1);
                        pptr = pptr.add(1);
                        verbarglen = *pptr;
                        verbculen = 0;
                        tempcode = code;
                        code = code.add(1);
                        let mut i: c_int = 0;
                        while i < verbarglen as c_int {
                            pptr = pptr.add(1);
                            meta = *pptr;
                            if utf != 0 {
                                mclength = crate::ord2utf::ord2utf(meta, mcbuffer.as_mut_ptr());
                            } else {
                                mclength = 1;
                                mcbuffer[0] = meta as u8;
                            }
                            if !lengthptr.is_null() {
                                *lengthptr += mclength as PCRE2_SIZE;
                            } else {
                                memcpy(code, mcbuffer.as_ptr(), mclength as usize);
                                code = code.add(mclength as usize);
                                verbculen += mclength;
                            }
                            i += 1;
                        }
                        *tempcode = verbculen as u8; /* Fill in the code unit length */
                        *code = 0; /* Terminating zero */
                        code = code.add(1);
                        Action::Continue
                    }

                    META_PRUNE_ARG | META_SKIP_ARG => {
                        (*cb).had_pruneorskip = TRUE;
                        /* Fall through */
                        /* VERB_ARG */
                        *code = verbops[((meta - META_MARK) >> 16) as usize] as u8;
                        code = code.add(1);
                        pptr = pptr.add(1);
                        verbarglen = *pptr;
                        verbculen = 0;
                        tempcode = code;
                        code = code.add(1);
                        let mut i: c_int = 0;
                        while i < verbarglen as c_int {
                            pptr = pptr.add(1);
                            meta = *pptr;
                            if utf != 0 {
                                mclength = crate::ord2utf::ord2utf(meta, mcbuffer.as_mut_ptr());
                            } else {
                                mclength = 1;
                                mcbuffer[0] = meta as u8;
                            }
                            if !lengthptr.is_null() {
                                *lengthptr += mclength as PCRE2_SIZE;
                            } else {
                                memcpy(code, mcbuffer.as_ptr(), mclength as usize);
                                code = code.add(mclength as usize);
                                verbculen += mclength;
                            }
                            i += 1;
                        }
                        *tempcode = verbculen as u8;
                        *code = 0;
                        code = code.add(1);
                        Action::Continue
                    }

                    META_MARK | META_COMMIT_ARG => {
                        /* VERB_ARG */
                        *code = verbops[((meta - META_MARK) >> 16) as usize] as u8;
                        code = code.add(1);
                        pptr = pptr.add(1);
                        verbarglen = *pptr;
                        verbculen = 0;
                        tempcode = code;
                        code = code.add(1);
                        let mut i: c_int = 0;
                        while i < verbarglen as c_int {
                            pptr = pptr.add(1);
                            meta = *pptr;
                            if utf != 0 {
                                mclength = crate::ord2utf::ord2utf(meta, mcbuffer.as_mut_ptr());
                            } else {
                                mclength = 1;
                                mcbuffer[0] = meta as u8;
                            }
                            if !lengthptr.is_null() {
                                *lengthptr += mclength as PCRE2_SIZE;
                            } else {
                                memcpy(code, mcbuffer.as_ptr(), mclength as usize);
                                code = code.add(mclength as usize);
                                verbculen += mclength;
                            }
                            i += 1;
                        }
                        *tempcode = verbculen as u8;
                        *code = 0;
                        code = code.add(1);
                        Action::Continue
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
                        req_caseopt =
                            if (options & PCRE2_CASELESS) != 0 { REQ_CASELESS } else { 0 };
                        Action::Continue
                    }

                    /* Handle scan substring. */
                    META_OFFSET => {
                        if !lengthptr.is_null() {
                            pptr = crate::compile_cgroup::parse_scan_substr_args(
                                pptr,
                                errorcodeptr,
                                cb,
                                lengthptr,
                            );
                            if pptr.is_null() {
                                return 0;
                            }
                            break 'dispatch Action::Continue;
                        }

                        loop {
                            let mut count: c_int;
                            let mut index: c_int;

                            match meta_code(*pptr) {
                                META_OFFSET => {
                                    pptr = pptr.add(1);
                                    skipoffset(&mut pptr);
                                    continue;
                                }

                                META_CAPTURE_NAME => {
                                    let ng = (*cb).named_groups.add(*pptr.add(1) as usize);
                                    pptr = pptr.add(2);
                                    count = 0;
                                    index = 0;

                                    if crate::compile_cgroup::find_dupname_details(
                                        (*ng).name,
                                        (*ng).length as u32,
                                        &mut index,
                                        &mut count,
                                        errorcodeptr,
                                        cb,
                                    ) == FALSE
                                    {
                                        return 0;
                                    }

                                    *code.add(0) = OP_DNCREF;
                                    put2(code, 1, index as u32);
                                    put2(code, 1 + IMM2_SIZE, count as u32);
                                    code = code.add(1 + 2 * IMM2_SIZE);
                                    continue;
                                }

                                META_CAPTURE_NUMBER => {
                                    pptr = pptr.add(2);
                                    if *pptr.sub(1) == 0 {
                                        continue;
                                    }

                                    *code.add(0) = OP_CREF;
                                    put2(code, 1, *pptr.sub(1));
                                    code = code.add(1 + IMM2_SIZE);
                                    continue;
                                }

                                _ => {}
                            }

                            break;
                        }
                        pptr = pptr.sub(1);
                        Action::Continue
                    }

                    META_SCS => {
                        bravalue = OP_ASSERT_SCS as c_int;
                        (*cb).assert_depth += 1;
                        Action::GroupProcess { note_empty: false }
                    }

                    /* Handle conditional subpatterns. */
                    META_COND_RNUMBER | META_COND_NAME | META_COND_RNAME => {
                        bravalue = OP_COND as c_int;

                        if !lengthptr.is_null() {
                            let start_pptr: *mut u32 = pptr;
                            pptr = pptr.add(1);
                            let length: u32 = *pptr;

                            offset = getplusoffset(&mut pptr);
                            let name: PCRE2_SPTR = (*cb).start_pattern.add(offset);

                            let ng = crate::compile_cgroup::find_named_group(name, length, cb);

                            if ng.is_null() {
                                groupnumber = 0;
                                if meta == META_COND_RNUMBER {
                                    let mut i: u32 = 1;
                                    while i < length {
                                        groupnumber = groupnumber * 10
                                            + (*name.add(i as usize) as u32 - CHAR_0);
                                        if groupnumber > MAX_GROUP_NUMBER {
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

                                if groupnumber == 0 {
                                    groupnumber = RREF_ANY;
                                }
                                debug_assert!(*start_pptr == META_COND_RNUMBER);
                                *start_pptr.add(1) = groupnumber;
                                skipunits = 1 + IMM2_SIZE as u32;
                                break 'dispatch Action::GroupProcess { note_empty: true };
                            }

                            /* From here on, we know we have a name. */
                            if meta == META_COND_RNUMBER {
                                meta = META_COND_NAME;
                            }

                            if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) == 0 {
                                if (*ng).number > (*cb).top_backref {
                                    (*cb).top_backref = (*ng).number;
                                }

                                *start_pptr.add(0) = meta;
                                *start_pptr.add(1) = (*ng).number;

                                skipunits = 1 + IMM2_SIZE as u32;
                                break 'dispatch Action::GroupProcess { note_empty: true };
                            }

                            /* Duplicated name. */
                            *start_pptr.add(0) = meta | 1;
                            *start_pptr.add(1) = ng.offset_from((*cb).named_groups) as u32;

                            skipunits = 1 + 2 * IMM2_SIZE as u32;
                        } else {
                            /* Second phase of compilation. */
                            let mut count: c_int;
                            let mut index: c_int;

                            if meta == META_COND_RNUMBER {
                                *code.add(1 + LINK_SIZE) = OP_RREF;
                                put2(code, 2 + LINK_SIZE, *pptr.add(1));
                                skipunits = 1 + IMM2_SIZE as u32;
                                pptr = pptr.add(1 + SIZEOFFSET);
                                break 'dispatch Action::GroupProcess { note_empty: true };
                            }

                            if meta_arg == 0 {
                                *code.add(1 + LINK_SIZE) =
                                    if meta == META_COND_RNAME { OP_RREF } else { OP_CREF };
                                put2(code, 2 + LINK_SIZE, *pptr.add(1));
                                skipunits = 1 + IMM2_SIZE as u32;
                                pptr = pptr.add(1 + SIZEOFFSET);
                                break 'dispatch Action::GroupProcess { note_empty: true };
                            }

                            let ng = (*cb).named_groups.add(*pptr.add(1) as usize);
                            count = 0;
                            index = 0;

                            if crate::compile_cgroup::find_dupname_details(
                                (*ng).name,
                                (*ng).length as u32,
                                &mut index,
                                &mut count,
                                errorcodeptr,
                                cb,
                            ) == FALSE
                            {
                                return 0;
                            }

                            *code.add(1 + LINK_SIZE) =
                                if meta == META_COND_RNAME { OP_DNRREF } else { OP_DNCREF };

                            put2(code, 2 + LINK_SIZE, index as u32);
                            put2(code, 2 + LINK_SIZE + IMM2_SIZE, count as u32);
                            skipunits = 1 + 2 * IMM2_SIZE as u32;
                            pptr = pptr.add(1 + SIZEOFFSET);
                        }

                        debug_assert!(meta != META_CAPTURE_NAME);
                        Action::GroupProcess { note_empty: true }
                    }

                    /* The DEFINE condition is always false. */
                    META_COND_DEFINE => {
                        bravalue = OP_COND as c_int;
                        offset = getplusoffset(&mut pptr);
                        *code.add(1 + LINK_SIZE) = OP_DEFINE;
                        skipunits = 1;
                        Action::GroupProcess { note_empty: false }
                    }

                    /* Conditional test of a group's being set. */
                    META_COND_NUMBER => {
                        bravalue = OP_COND as c_int;
                        offset = getplusoffset(&mut pptr);

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
                        offset = offset.wrapping_sub(2);
                        *code.add(1 + LINK_SIZE) = OP_CREF;
                        skipunits = 1 + IMM2_SIZE as u32;
                        put2(code, 2 + LINK_SIZE, groupnumber);
                        Action::GroupProcess { note_empty: true }
                    }

                    /* Test for the PCRE2 version. */
                    META_COND_VERSION => {
                        bravalue = OP_COND as c_int;
                        if *pptr.add(1) > 0 {
                            *code.add(1 + LINK_SIZE) = if (PCRE2_MAJOR > *pptr.add(2))
                                || (PCRE2_MAJOR == *pptr.add(2) && PCRE2_MINOR >= *pptr.add(3))
                            {
                                OP_TRUE
                            } else {
                                OP_FALSE
                            };
                        } else {
                            *code.add(1 + LINK_SIZE) =
                                if PCRE2_MAJOR == *pptr.add(2) && PCRE2_MINOR == *pptr.add(3) {
                                    OP_TRUE
                                } else {
                                    OP_FALSE
                                };
                        }
                        skipunits = 1;
                        pptr = pptr.add(3);
                        Action::GroupProcess { note_empty: true }
                    }

                    /* The condition is an assertion. */
                    META_COND_ASSERT => {
                        bravalue = OP_COND as c_int;
                        Action::GroupProcess { note_empty: true }
                    }

                    /* Handle nested bracketed groups. */
                    META_LOOKAHEAD => {
                        bravalue = OP_ASSERT as c_int;
                        (*cb).assert_depth += 1;
                        Action::GroupProcess { note_empty: false }
                    }

                    META_LOOKAHEAD_NA => {
                        bravalue = OP_ASSERT_NA as c_int;
                        (*cb).assert_depth += 1;
                        Action::GroupProcess { note_empty: false }
                    }

                    META_LOOKAHEADNOT => {
                        if *pptr.add(1) == META_KET
                            && (*pptr.add(2) < META_ASTERISK || *pptr.add(2) > META_MINMAX_QUERY)
                        {
                            *code = OP_FAIL;
                            code = code.add(1);
                            pptr = pptr.add(1);
                            Action::Continue
                        } else {
                            bravalue = OP_ASSERT_NOT as c_int;
                            (*cb).assert_depth += 1;
                            Action::GroupProcess { note_empty: false }
                        }
                    }

                    META_LOOKBEHIND => {
                        bravalue = OP_ASSERTBACK as c_int;
                        (*cb).assert_depth += 1;
                        Action::GroupProcess { note_empty: false }
                    }

                    META_LOOKBEHINDNOT => {
                        bravalue = OP_ASSERTBACK_NOT as c_int;
                        (*cb).assert_depth += 1;
                        Action::GroupProcess { note_empty: false }
                    }

                    META_LOOKBEHIND_NA => {
                        bravalue = OP_ASSERTBACK_NA as c_int;
                        (*cb).assert_depth += 1;
                        Action::GroupProcess { note_empty: false }
                    }

                    META_ATOMIC => {
                        bravalue = OP_ONCE as c_int;
                        Action::GroupProcess { note_empty: true }
                    }

                    META_SCRIPT_RUN => {
                        bravalue = OP_SCRIPT_RUN as c_int;
                        Action::GroupProcess { note_empty: true }
                    }

                    META_NOCAPTURE => {
                        bravalue = OP_BRA as c_int;
                        /* Fall through to GROUP_PROCESS_NOTE_EMPTY */
                        Action::GroupProcess { note_empty: true }
                    }

                    /* Handle named backreferences and recursions. */
                    META_BACKREF_BYNAME | META_RECURSE_BYNAME => {
                        let mut count: c_int;
                        let mut index: c_int;
                        pptr = pptr.add(1);
                        let length: u32 = *pptr;

                        offset = getplusoffset(&mut pptr);
                        let name: PCRE2_SPTR = (*cb).start_pattern.add(offset);

                        let ng = crate::compile_cgroup::find_named_group(name, length, cb);

                        if ng.is_null() {
                            *errorcodeptr = ERR15;
                            (*cb).erroroffset = offset;
                            return 0;
                        }

                        groupnumber = (*ng).number;

                        if meta == META_RECURSE_BYNAME {
                            meta_arg = groupnumber;
                            break 'dispatch Action::HandleNumericalRecursion;
                        }

                        (*cb).backref_map |=
                            if groupnumber < 32 { 1u32 << groupnumber } else { 1 };
                        if groupnumber > (*cb).top_backref {
                            (*cb).top_backref = groupnumber;
                        }

                        if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) == 0 {
                            meta_arg = groupnumber;
                            break 'dispatch Action::HandleSingleReference;
                        }

                        count = 0;
                        index = 0;
                        if lengthptr.is_null()
                            && crate::compile_cgroup::find_dupname_details(
                                name,
                                length,
                                &mut index,
                                &mut count,
                                errorcodeptr,
                                cb,
                            ) == FALSE
                        {
                            return 0;
                        }

                        if firstcuflags == REQ_UNSET {
                            firstcuflags = REQ_NONE;
                        }
                        *code = if (options & PCRE2_CASELESS) != 0 { OP_DNREFI } else { OP_DNREF };
                        code = code.add(1);
                        put2inc(&mut code, 0, index as u32);
                        put2inc(&mut code, 0, count as u32);
                        if (options & PCRE2_CASELESS) != 0 {
                            *code = (if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0 {
                                REFI_FLAG_CASELESS_RESTRICT
                            } else {
                                0
                            } | if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0 {
                                REFI_FLAG_TURKISH_CASING
                            } else {
                                0
                            }) as u8;
                            code = code.add(1);
                        }
                        Action::Continue
                    }

                    /* Handle a numerical callout. */
                    META_CALLOUT_NUMBER => {
                        *code.add(0) = OP_CALLOUT;
                        put(code, 1, *pptr.add(1) as c_int); /* Offset to next pattern item */
                        put(code, 1 + LINK_SIZE, *pptr.add(2) as c_int); /* Length of next item */
                        *code.add(1 + 2 * LINK_SIZE) = *pptr.add(3) as u8;
                        pptr = pptr.add(3);
                        code = code.add(OP_LENGTHS[OP_CALLOUT as usize] as usize);
                        Action::Continue
                    }

                    /* Handle a callout with a string argument. */
                    META_CALLOUT_STRING => {
                        if !lengthptr.is_null() {
                            *lengthptr += *pptr.add(3) as PCRE2_SIZE + (1 + 4 * LINK_SIZE);
                            pptr = pptr.add(3);
                            skipoffset(&mut pptr);
                        } else {
                            let mut pp: PCRE2_SPTR;
                            let mut delimiter: u32;
                            let mut length: u32 = *pptr.add(3);
                            let mut callout_string: *mut PCRE2_UCHAR =
                                code.add(1 + 4 * LINK_SIZE);

                            *code.add(0) = OP_CALLOUT_STR;
                            put(code, 1, *pptr.add(1) as c_int); /* Offset to next pattern item */
                            put(code, 1 + LINK_SIZE, *pptr.add(2) as c_int); /* Length of next */

                            pptr = pptr.add(3);
                            offset = getplusoffset(&mut pptr); /* Offset to string in pattern */
                            pp = (*cb).start_pattern.add(offset);
                            delimiter = *pp as u32;
                            *callout_string = *pp;
                            callout_string = callout_string.add(1);
                            pp = pp.add(1);
                            if delimiter == CHAR_LEFT_CURLY_BRACKET {
                                delimiter = CHAR_RIGHT_CURLY_BRACKET;
                            }
                            put(code, 1 + 3 * LINK_SIZE, (offset + 1) as c_int); /* After delim */

                            length -= 1;
                            while length > 1 {
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
                                length -= 1;
                            }
                            *callout_string = CHAR_NUL as u8;
                            callout_string = callout_string.add(1);

                            /* Set the length of the entire item, advance to end. */
                            put(code, 1 + 2 * LINK_SIZE, callout_string.offset_from(code) as c_int);
                            code = callout_string;
                        }
                        Action::Continue
                    }

                    /* Handle repetition. */
                    META_MINMAX_PLUS | META_MINMAX_QUERY | META_MINMAX => {
                        pptr = pptr.add(1);
                        repeat_min = *pptr;
                        pptr = pptr.add(1);
                        repeat_max = *pptr;
                        Action::Repeat
                    }

                    META_ASTERISK | META_ASTERISK_PLUS | META_ASTERISK_QUERY => {
                        repeat_min = 0;
                        repeat_max = REPEAT_UNLIMITED;
                        Action::Repeat
                    }

                    META_PLUS | META_PLUS_PLUS | META_PLUS_QUERY => {
                        repeat_min = 1;
                        repeat_max = REPEAT_UNLIMITED;
                        Action::Repeat
                    }

                    META_QUERY | META_QUERY_PLUS | META_QUERY_QUERY => {
                        repeat_min = 0;
                        repeat_max = 1;
                        Action::Repeat
                    }

                    /* Handle a 32-bit data character with a value greater than
                    META_END. In 8-bit mode BIGVALUE holds the character. */
                    META_BIGVALUE => {
                        pptr = pptr.add(1);
                        /* goto NORMAL_CHAR */
                        meta = *pptr; /* Get the full 32 bits */
                        break 'dispatch Action::NormalCharSet;
                    }

                    /* Handle a back reference by number. */
                    META_BACKREF => {
                        if meta_arg < 10 {
                            offset = (*cb).small_ref_offset[meta_arg as usize];
                        } else {
                            offset = getplusoffset(&mut pptr);
                        }

                        if meta_arg > (*cb).bracount {
                            (*cb).erroroffset = offset;
                            *errorcodeptr = ERR15; /* Non-existent subpattern */
                            return 0;
                        }

                        Action::HandleSingleReference
                    }

                    /* Handle recursion. */
                    META_RECURSE => {
                        offset = getplusoffset(&mut pptr);
                        if meta_arg > (*cb).bracount {
                            (*cb).erroroffset = offset;
                            *errorcodeptr = ERR15; /* Non-existent subpattern */
                            return 0;
                        }
                        Action::HandleNumericalRecursion
                    }

                    /* Handle capturing parentheses. */
                    META_CAPTURE => {
                        bravalue = OP_CBRA as c_int;
                        skipunits = IMM2_SIZE as u32;
                        put2(code, 1 + LINK_SIZE, meta_arg);
                        (*cb).lastcapture = meta_arg;
                        Action::GroupProcess { note_empty: true }
                    }

                    /* Handle escape sequence items. */
                    META_ESCAPE => {
                        /* We can test for escape sequences that consume a
                        character because their values lie between ESC_b and
                        ESC_Z. */
                        if meta_arg > ESC_b as u32 && meta_arg < ESC_Z as u32 {
                            matched_char = TRUE;
                            if firstcuflags == REQ_UNSET {
                                firstcuflags = REQ_NONE;
                            }
                        }

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
                                    *code = OP_CLASS;
                                    code = code.add(1);
                                    memset(code, 0, 32);
                                    code = code.add(32);
                                } else {
                                    *code = OP_ALLANY;
                                    code = code.add(1);
                                }
                            } else {
                                *code =
                                    if meta_arg == ESC_p as u32 { OP_PROP } else { OP_NOTPROP };
                                code = code.add(1);
                                *code = ptype as u8;
                                code = code.add(1);
                                *code = pdata as u8;
                                code = code.add(1);
                            }
                            break 'dispatch Action::Continue; /* End META_ESCAPE */
                        }

                        /* \K is forbidden in lookarounds since 10.38. */
                        if (*cb).assert_depth > 0
                            && meta_arg == ESC_K as u32
                            && (xoptions & PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK) == 0
                        {
                            *errorcodeptr = ERR99;
                            return 0;
                        }

                        if meta_arg == ESC_C as u32 {
                            (*cb).external_flags |= PCRE2_HASBKC; /* Record */
                            if utf == 0 {
                                meta_arg = OP_ALLANY as u32;
                            }
                        } else if meta_arg == ESC_B as u32 || meta_arg == ESC_b as u32 {
                            if (options & PCRE2_UCP) != 0 && (xoptions & PCRE2_EXTRA_ASCII_BSW) == 0
                            {
                                meta_arg = if meta_arg == ESC_B as u32 {
                                    OP_NOT_UCP_WORD_BOUNDARY as u32
                                } else {
                                    OP_UCP_WORD_BOUNDARY as u32
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
                        Action::Continue /* End META_ESCAPE */
                    }

                    /* Handle an unrecognized meta value. */
                    _ => {
                        if meta >= META_END {
                            *errorcodeptr = ERR89; /* Internal error - unrecognized. */
                            return 0;
                        }

                        /* Handle a literal character (NORMAL_CHAR). */
                        meta = *pptr; /* Get the full 32 bits */
                        Action::NormalCharSet
                    }
                }
            };

            /* Now act on the computed action, emulating the forward gotos of the
            C source. */
            let enter_caseless_char = matches!(action, Action::ClassCaselessChar);
            match action {
                Action::Continue => {
                    pptr = pptr.add(1);
                    continue;
                }
                Action::Return(v) => return v,
                Action::ClassEndProcessing => {
                    /* CLASS_END_PROCESSING */
                    if firstcuflags == REQ_UNSET {
                        firstcuflags = REQ_NONE;
                    }
                    zerofirstcu = firstcu;
                    zerofirstcuflags = firstcuflags;
                    zeroreqcu = reqcu;
                    zeroreqcuflags = reqcuflags;
                    pptr = pptr.add(1);
                    continue;
                }
                Action::GroupProcess { note_empty } => {
                    note_group_empty = note_empty as BOOL;

                    /* GROUP_PROCESS */
                    (*cb).parens_depth += 1;
                    *code = bravalue as u8;
                    pptr = pptr.add(1);
                    tempcode = code;
                    tempreqvary = (*cb).req_varyopt; /* Save value before group */
                    length_prevgroup = 0; /* Initialize for pre-compile phase */

                    group_return = compile_regex(
                        options,
                        xoptions,
                        &mut tempcode,
                        &mut pptr,
                        errorcodeptr,
                        skipunits,
                        &mut subfirstcu,
                        &mut subfirstcuflags,
                        &mut subreqcu,
                        &mut subreqcuflags,
                        bcptr,
                        open_caps,
                        cb,
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

                    if note_group_empty != 0 && bravalue != OP_COND as c_int && group_return > 0 {
                        matched_char = TRUE;
                    }

                    if bravalue >= OP_ASSERT as c_int && bravalue <= OP_ASSERT_SCS as c_int {
                        (*cb).assert_depth -= 1;
                    }

                    /* Conditional bracket branch checks. */
                    if bravalue == OP_COND as c_int && lengthptr.is_null() {
                        let mut tc: *mut PCRE2_UCHAR = code;
                        let mut condcount: c_int = 0;

                        loop {
                            condcount += 1;
                            tc = tc.add(get(tc, 1) as usize);
                            if *tc == OP_KET {
                                break;
                            }
                        }

                        if *code.add(LINK_SIZE + 1) == OP_DEFINE {
                            if condcount > 1 {
                                (*cb).erroroffset = offset;
                                *errorcodeptr = ERR54;
                                return 0;
                            }
                            *code.add(LINK_SIZE + 1) = OP_FALSE;
                            bravalue = OP_DEFINE as c_int; /* Suppress char handling below */
                        } else {
                            if condcount > 2 {
                                (*cb).erroroffset = offset;
                                *errorcodeptr = ERR27;
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

                    /* In pre-compile phase, update length and simplify code. */
                    if !lengthptr.is_null() {
                        if OFLOW_MAX - *lengthptr
                            < length_prevgroup.wrapping_sub(2).wrapping_sub(2 * LINK_SIZE)
                        {
                            *errorcodeptr = ERR20;
                            return 0;
                        }
                        *lengthptr = (*lengthptr)
                            .wrapping_add(length_prevgroup)
                            .wrapping_sub(2)
                            .wrapping_sub(2 * LINK_SIZE);
                        code = code.add(1); /* This already contains bravalue */
                        putinc(&mut code, 0, (1 + LINK_SIZE) as c_int);
                        *code = OP_KET;
                        code = code.add(1);
                        putinc(&mut code, 0, (1 + LINK_SIZE) as c_int);
                        pptr = pptr.add(1);
                        continue; /* No need for special character handling */
                    }

                    /* Otherwise update the main code pointer to end of group. */
                    code = tempcode;

                    if bravalue == OP_DEFINE as c_int {
                        pptr = pptr.add(1);
                        continue;
                    }

                    /* Handle updating of first/required code units. */
                    zeroreqcu = reqcu;
                    zeroreqcuflags = reqcuflags;
                    zerofirstcu = firstcu;
                    zerofirstcuflags = firstcuflags;
                    groupsetfirstcu = FALSE;

                    if bravalue >= OP_ONCE as c_int {
                        /* Not an assertion */
                        if firstcuflags == REQ_UNSET && subfirstcuflags != REQ_UNSET {
                            if subfirstcuflags < REQ_NONE {
                                firstcu = subfirstcu;
                                firstcuflags = subfirstcuflags;
                                groupsetfirstcu = TRUE;
                            } else {
                                firstcuflags = REQ_NONE;
                            }
                            zerofirstcuflags = REQ_NONE;
                        } else if subfirstcuflags < REQ_NONE && subreqcuflags >= REQ_NONE {
                            subreqcu = subfirstcu;
                            subreqcuflags = subfirstcuflags | tempreqvary;
                        }

                        if subreqcuflags < REQ_NONE {
                            reqcu = subreqcu;
                            reqcuflags = subreqcuflags;
                        }
                    } else if (bravalue == OP_ASSERT as c_int || bravalue == OP_ASSERT_NA as c_int)
                        && subreqcuflags < REQ_NONE
                        && subfirstcuflags < REQ_NONE
                    {
                        reqcu = subreqcu;
                        reqcuflags = subreqcuflags;
                    }

                    pptr = pptr.add(1);
                    continue; /* End of nested group handling */
                }

                Action::HandleSingleReference => {
                    /* HANDLE_SINGLE_REFERENCE */
                    if firstcuflags == REQ_UNSET {
                        zerofirstcuflags = REQ_NONE;
                        firstcuflags = REQ_NONE;
                    }
                    *code = if (options & PCRE2_CASELESS) != 0 { OP_REFI } else { OP_REF };
                    code = code.add(1);
                    put2inc(&mut code, 0, meta_arg);
                    if (options & PCRE2_CASELESS) != 0 {
                        *code = (if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0 {
                            REFI_FLAG_CASELESS_RESTRICT
                        } else {
                            0
                        } | if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0 {
                            REFI_FLAG_TURKISH_CASING
                        } else {
                            0
                        }) as u8;
                        code = code.add(1);
                    }

                    (*cb).backref_map |= if meta_arg < 32 { 1u32 << meta_arg } else { 1 };
                    if meta_arg > (*cb).top_backref {
                        (*cb).top_backref = meta_arg;
                    }
                    pptr = pptr.add(1);
                    continue;
                }

                Action::HandleNumericalRecursion => {
                    /* HANDLE_NUMERICAL_RECURSION */
                    *code = OP_RECURSE;
                    put(code, 1, meta_arg as c_int);
                    code = code.add(1 + LINK_SIZE);
                    length_prevgroup = 1 + LINK_SIZE;

                    if meta_code(*pptr.add(1)) == META_OFFSET
                        || meta_code(*pptr.add(1)) == META_CAPTURE_NAME
                        || meta_code(*pptr.add(1)) == META_CAPTURE_NUMBER
                    {
                        let args: *mut recurse_arguments;

                        if !lengthptr.is_null() {
                            if crate::compile_cgroup::parse_recurse_args(
                                pptr,
                                offset,
                                errorcodeptr,
                                cb,
                            ) == FALSE
                            {
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
                            debug_assert!(!args.is_null());

                            current = args.add(1) as *mut u16;
                            end = current.add((*args).size);
                            debug_assert!(end > current);

                            loop {
                                *code.add(0) = OP_CREF;
                                put2(code, 1, *current as u32);
                                code = code.add(1 + IMM2_SIZE);
                                current = current.add(1);
                                if current >= end {
                                    break;
                                }
                            }

                            length_prevgroup += (*args).size * (1 + IMM2_SIZE);
                            pptr = pptr.add((*args).skip_size);
                            (*cb).first_data = (*args).header.next;
                            ((*(*cb).cx).memctl.free.unwrap())(
                                args as *mut core::ffi::c_void,
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
                    pptr = pptr.add(1);
                    continue;
                }

                Action::NormalCharSet | Action::ClassCaselessChar => {
                    /* Shared literal-character emission.
                    Action::NormalCharSet enters at NORMAL_CHAR_SET; the caseless
                    handling below is skipped when entering at
                    CLASS_CASELESS_CHAR. */

                    if !enter_caseless_char {
                        /* NORMAL_CHAR_SET */
                        matched_char = TRUE;

                        if (utf != 0 || ucp != 0) && (options & PCRE2_CASELESS) != 0 {
                            let mut caseset: u32;

                            if (xoptions
                                & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                                == PCRE2_EXTRA_TURKISH_CASING
                                && ucd_any_i(meta)
                            {
                                caseset = UCD_TURKISH_DOTTED_I_CASESET
                                    + (if ucd_dotted_i(meta) { 0 } else { 3 });
                            } else {
                                caseset = ucd_caseset(meta);
                                if caseset != 0
                                    && (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                                    && UCD_CASELESS_SETS[caseset as usize] < 128
                                {
                                    caseset = 0; /* Ignore restricted caseless set. */
                                }
                            }

                            if caseset != 0 {
                                *code = OP_PROP;
                                code = code.add(1);
                                *code = PT_CLIST as u8;
                                code = code.add(1);
                                *code = caseset as u8;
                                code = code.add(1);
                                if firstcuflags == REQ_UNSET {
                                    firstcuflags = REQ_NONE;
                                    zerofirstcuflags = REQ_NONE;
                                }
                                pptr = pptr.add(1);
                                continue; /* End handling this meta item */
                            }
                        }
                    }

                    /* CLASS_CASELESS_CHAR */

                    if utf != 0 {
                        mclength = crate::ord2utf::ord2utf(meta, mcbuffer.as_mut_ptr());
                    } else {
                        mclength = 1;
                        mcbuffer[0] = meta as u8;
                    }

                    *code = if (options & PCRE2_CASELESS) != 0 { OP_CHARI } else { OP_CHAR };
                    code = code.add(1);
                    memcpy(code, mcbuffer.as_ptr(), mclength as usize);
                    code = code.add(mclength as usize);

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
                                reqcu = *code.sub(1) as u32;
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
                            reqcu = *code.sub(1) as u32;
                            reqcuflags = req_caseopt | (*cb).req_varyopt;
                        }
                    }

                    if reset_caseful != 0 {
                        options &= !PCRE2_CASELESS;
                        req_caseopt = 0;
                        reset_caseful = FALSE;
                    }

                    pptr = pptr.add(1);
                    continue; /* End literal character handling */
                }

                Action::Repeat => {
                    /* REPEAT */
                    if previous_matched_char != 0 && repeat_min > 0 {
                        matched_char = TRUE;
                    }

                    reqvary = if repeat_min == repeat_max { 0 } else { REQ_VARY };

                    if repeat_min == 0 {
                        firstcu = zerofirstcu;
                        firstcuflags = zerofirstcuflags;
                        reqcu = zeroreqcu;
                        reqcuflags = zeroreqcuflags;
                    }

                    /* Note the greediness and possessiveness. */
                    match meta {
                        META_MINMAX_PLUS | META_ASTERISK_PLUS | META_PLUS_PLUS | META_QUERY_PLUS => {
                            repeat_type = 0; /* Force greedy */
                            possessive_quantifier = TRUE;
                        }
                        META_MINMAX_QUERY | META_ASTERISK_QUERY | META_PLUS_QUERY
                        | META_QUERY_QUERY => {
                            repeat_type = greedy_non_default;
                            possessive_quantifier = FALSE;
                        }
                        _ => {
                            repeat_type = greedy_default;
                            possessive_quantifier = FALSE;
                        }
                    }

                    debug_assert!(!previous.is_null());
                    tempcode = previous;
                    op_previous = *previous;

                    /* END_REPEAT target flag. */
                    let mut goto_end_repeat = false;

                    /* op_type default for single char/type repeats. */
                    op_type = 0;
                    mclength = 0;

                    /* prop_type/prop_value for OUTPUT_SINGLE_REPEAT. */
                    let mut prop_type: c_int = -1;
                    let mut prop_value: c_int = -1;
                    /* True when we must run the shared OUTPUT_SINGLE_REPEAT tail. */
                    let mut do_output_single_repeat = false;
                    /* True when we must run the shared bracket-group repeat body. */
                    let mut do_bracket_group = false;

                    'repeat_switch: {
                        match op_previous {
                            OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI => {
                                if repeat_max == 1 && repeat_min == 1 {
                                    goto_end_repeat = true;
                                    break 'repeat_switch;
                                }
                                op_type = chartypeoffset[(op_previous - OP_CHAR) as usize];

                                /* MAYBE_UTF_MULTI is defined in 8-bit mode. */
                                if utf != 0 && not_firstcu(*code.sub(1) as u32) {
                                    let mut lastchar: PCRE2_SPTR = code.sub(1);
                                    backchar(&mut lastchar);
                                    mclength = code.offset_from(lastchar) as u32;
                                    memcpy(mcbuffer.as_mut_ptr(), lastchar, mclength as usize);
                                } else {
                                    mcbuffer[0] = *code.sub(1);
                                    mclength = 1;
                                    if op_previous <= OP_CHARI && repeat_min > 1 {
                                        reqcu = mcbuffer[0] as u32;
                                        reqcuflags = (*cb).req_varyopt;
                                        if op_previous == OP_CHARI {
                                            reqcuflags |= REQ_CASELESS;
                                        }
                                    }
                                }
                                /* goto OUTPUT_SINGLE_REPEAT */
                                do_output_single_repeat = true;
                                break 'repeat_switch;
                            }

                            OP_XCLASS | OP_ECLASS | OP_CLASS | OP_NCLASS | OP_REF | OP_REFI
                            | OP_DNREF | OP_DNREFI => {
                                if repeat_max == 0 {
                                    code = previous;
                                    goto_end_repeat = true;
                                    break 'repeat_switch;
                                }
                                if repeat_max == 1 && repeat_min == 1 {
                                    goto_end_repeat = true;
                                    break 'repeat_switch;
                                }

                                if repeat_min == 0 && repeat_max == REPEAT_UNLIMITED {
                                    *code = OP_CRSTAR + repeat_type as u8;
                                    code = code.add(1);
                                } else if repeat_min == 1 && repeat_max == REPEAT_UNLIMITED {
                                    *code = OP_CRPLUS + repeat_type as u8;
                                    code = code.add(1);
                                } else if repeat_min == 0 && repeat_max == 1 {
                                    *code = OP_CRQUERY + repeat_type as u8;
                                    code = code.add(1);
                                } else {
                                    *code = OP_CRRANGE + repeat_type as u8;
                                    code = code.add(1);
                                    put2inc(&mut code, 0, repeat_min);
                                    if repeat_max == REPEAT_UNLIMITED {
                                        repeat_max = 0; /* 2-byte encoding for max */
                                    }
                                    put2inc(&mut code, 0, repeat_max);
                                }
                            }

                            OP_RECURSE => {
                                if repeat_max == 1 && repeat_min == 1 && possessive_quantifier == 0 {
                                    goto_end_repeat = true;
                                    break 'repeat_switch;
                                }

                                /* Generate unwrapped repeats for a non-zero minimum. */
                                if repeat_min > 0
                                    && (repeat_min != 1 || repeat_max != REPEAT_UNLIMITED)
                                {
                                    let mut replicate: c_int = repeat_min as c_int;

                                    if repeat_min == repeat_max {
                                        replicate -= 1;
                                    }

                                    if !lengthptr.is_null() {
                                        let mut delta: PCRE2_SIZE = 0;
                                        if crate::chkdint::ckd_smul(
                                            &mut delta,
                                            replicate,
                                            length_prevgroup as c_int,
                                        ) != FALSE
                                            || OFLOW_MAX - *lengthptr < delta
                                        {
                                            *errorcodeptr = ERR20;
                                            return 0;
                                        }
                                        *lengthptr += delta;
                                    } else {
                                        let mut i: c_int = 0;
                                        while i < replicate {
                                            memcpy(code, previous, length_prevgroup);
                                            previous = code;
                                            code = code.add(length_prevgroup);
                                            i += 1;
                                        }
                                    }

                                    if repeat_min == repeat_max {
                                        break 'repeat_switch;
                                    }
                                    if repeat_max != REPEAT_UNLIMITED {
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

                                    memmove(previous.add(1 + LINK_SIZE), previous, length);
                                    *previous = OP_BRA;
                                    op_previous = OP_BRA;
                                    put(previous, 1, (1 + LINK_SIZE + length) as c_int);
                                    *previous.add(1 + LINK_SIZE + length) = OP_KET;
                                    put(
                                        previous,
                                        2 + LINK_SIZE + length,
                                        (1 + LINK_SIZE + length) as c_int,
                                    );
                                }
                                code = code.add(2 + 2 * LINK_SIZE);
                                length_prevgroup += 2 + 2 * LINK_SIZE;
                                group_return = -1; /* Set "may match empty string" */

                                /* Fall through into the bracket-group handling. */
                                do_bracket_group = true;
                            }

                            OP_ASSERT | OP_ASSERT_NOT | OP_ASSERT_NA | OP_ASSERTBACK
                            | OP_ASSERTBACK_NOT | OP_ASSERTBACK_NA | OP_ASSERT_SCS | OP_ONCE
                            | OP_SCRIPT_RUN | OP_BRA | OP_CBRA | OP_COND => {
                                do_bracket_group = true;
                            }

                            /* Character type match (\d etc.) -> repeat item. */
                            _ => {
                                if op_previous >= OP_EODN || op_previous <= OP_WORD_BOUNDARY {
                                    *errorcodeptr = ERR10; /* Not a char type - internal error */
                                    return 0;
                                }

                                if repeat_max == 1 && repeat_min == 1 {
                                    goto_end_repeat = true;
                                    break 'repeat_switch;
                                }

                                op_type = (OP_TYPESTAR - OP_STAR) as u32; /* Use type opcodes */
                                mclength = 0; /* Not a character */

                                if op_previous == OP_PROP || op_previous == OP_NOTPROP {
                                    prop_type = *previous.add(1) as c_int;
                                    prop_value = *previous.add(2) as c_int;
                                } else {
                                    prop_type = -1;
                                    prop_value = -1;
                                }

                                do_output_single_repeat = true;
                            }
                        } /* End of switch on op_previous */
                    } /* 'repeat_switch */

                    /* Shared bracket-group repeat body (C: the OP_ASSERT..OP_COND
                    case, into which OP_RECURSE falls through). */
                    if do_bracket_group && !goto_end_repeat {
                        let len: c_int = code.offset_from(previous) as c_int;
                        let mut bralink: *mut PCRE2_UCHAR = core::ptr::null_mut();
                        let mut brazeroptr: *mut PCRE2_UCHAR = core::ptr::null_mut();

                        'bracket_body: {
                            if repeat_max == 1 && repeat_min == 1 && possessive_quantifier == 0 {
                                goto_end_repeat = true;
                                break 'bracket_body;
                            }

                            /* Repeating a DEFINE group is pointless; ignore. */
                            if op_previous == OP_COND
                                && *previous.add(LINK_SIZE + 1) == OP_FALSE
                                && *previous.add(get(previous, 1) as usize) != OP_ALT
                            {
                                goto_end_repeat = true;
                                break 'bracket_body;
                            }

                            if op_previous < OP_ONCE {
                                /* Assertion */
                                if repeat_max == REPEAT_UNLIMITED {
                                    repeat_max = repeat_min + 1;
                                }
                            }

                            /* The case of a zero minimum. */
                            if repeat_min == 0 {
                                if repeat_max <= 1 || repeat_max == REPEAT_UNLIMITED {
                                    memmove(previous.add(1), previous, len as usize);
                                    code = code.add(1);
                                    if repeat_max == 0 {
                                        *previous = OP_SKIPZERO;
                                        previous = previous.add(1);
                                        goto_end_repeat = true;
                                        break 'bracket_body;
                                    }
                                    brazeroptr = previous; /* Save for possessive optimizing */
                                    *previous = OP_BRAZERO + repeat_type as u8;
                                    previous = previous.add(1);
                                } else {
                                    let linkoffset: c_int;
                                    memmove(previous.add(2 + LINK_SIZE), previous, len as usize);
                                    code = code.add(2 + LINK_SIZE);
                                    *previous = OP_BRAZERO + repeat_type as u8;
                                    previous = previous.add(1);
                                    *previous = OP_BRA;
                                    previous = previous.add(1);

                                    linkoffset = if bralink.is_null() {
                                        0
                                    } else {
                                        previous.offset_from(bralink) as c_int
                                    };
                                    bralink = previous;
                                    putinc(&mut previous, 0, linkoffset);
                                }

                                if repeat_max != REPEAT_UNLIMITED {
                                    repeat_max -= 1;
                                }
                            }
                            /* Minimum greater than zero. */
                            else {
                                if repeat_min > 1 {
                                    if !lengthptr.is_null() {
                                        let mut delta: PCRE2_SIZE = 0;
                                        if crate::chkdint::ckd_smul(
                                            &mut delta,
                                            (repeat_min - 1) as c_int,
                                            length_prevgroup as c_int,
                                        ) != FALSE
                                            || OFLOW_MAX - *lengthptr < delta
                                        {
                                            *errorcodeptr = ERR20;
                                            return 0;
                                        }
                                        *lengthptr += delta;
                                    } else {
                                        if groupsetfirstcu != 0 && reqcuflags >= REQ_NONE {
                                            reqcu = firstcu;
                                            reqcuflags = firstcuflags;
                                        }
                                        let mut i: u32 = 1;
                                        while i < repeat_min {
                                            memcpy(code, previous, len as usize);
                                            code = code.add(len as usize);
                                            i += 1;
                                        }
                                    }
                                }

                                if repeat_max != REPEAT_UNLIMITED {
                                    repeat_max -= repeat_min;
                                }
                            }

                            /* Common to both zero and non-zero minimum: limited max. */
                            if repeat_max != REPEAT_UNLIMITED {
                                if !lengthptr.is_null() && repeat_max > 0 {
                                    let mut delta: PCRE2_SIZE = 0;
                                    if crate::chkdint::ckd_smul(
                                        &mut delta,
                                        repeat_max as c_int,
                                        length_prevgroup as c_int + 1 + 2 + 2 * LINK_SIZE as c_int,
                                    ) != FALSE
                                        || OFLOW_MAX + (2 + 2 * LINK_SIZE) - *lengthptr < delta
                                    {
                                        *errorcodeptr = ERR20;
                                        return 0;
                                    }
                                    delta -= 2 + 2 * LINK_SIZE; /* Last one doesn't nest */
                                    *lengthptr += delta;
                                } else {
                                    /* This is compiling for real */
                                    let mut i: u32 = repeat_max;
                                    while i >= 1 {
                                        *code = OP_BRAZERO + repeat_type as u8;
                                        code = code.add(1);

                                        if i != 1 {
                                            let linkoffset: c_int;
                                            *code = OP_BRA;
                                            code = code.add(1);
                                            linkoffset = if bralink.is_null() {
                                                0
                                            } else {
                                                code.offset_from(bralink) as c_int
                                            };
                                            bralink = code;
                                            putinc(&mut code, 0, linkoffset);
                                        }

                                        memcpy(code, previous, len as usize);
                                        code = code.add(len as usize);
                                        i -= 1;
                                    }
                                }

                                /* Now chain through the pending brackets, and fill in
                                their length fields (which are holding the chain links
                                pro tem). This runs in both the pre-compile (length)
                                and the real compile phases: in the zero-minimum case
                                the first bracket set up above leaves a pending link
                                even during the length pass, and its closing KET must
                                be accounted for. */
                                while !bralink.is_null() {
                                    let oldlinkoffset: c_int;
                                    let linkoffset: c_int =
                                        (code.offset_from(bralink) + 1) as c_int;
                                    let bra: *mut PCRE2_UCHAR = code.sub(linkoffset as usize);
                                    oldlinkoffset = get(bra, 1);
                                    bralink = if oldlinkoffset == 0 {
                                        core::ptr::null_mut()
                                    } else {
                                        bralink.sub(oldlinkoffset as usize)
                                    };
                                    *code = OP_KET;
                                    code = code.add(1);
                                    putinc(&mut code, 0, linkoffset);
                                    put(bra, 1, linkoffset);
                                }
                            }
                            /* Maximum is unlimited: set a repeater in the final copy. */
                            else {
                                let ketcode: *mut PCRE2_UCHAR = code.sub(1 + LINK_SIZE);
                                let bracode: *mut PCRE2_UCHAR =
                                    ketcode.sub(get(ketcode, 1) as usize);

                                /* Convert possessive ONCE brackets to non-capturing. */
                                if *bracode == OP_ONCE && possessive_quantifier != 0 {
                                    *bracode = OP_BRA;
                                }

                                if *bracode == OP_ONCE || *bracode == OP_SCRIPT_RUN {
                                    *ketcode = OP_KETRMAX + repeat_type as u8;
                                } else {
                                    if lengthptr.is_null() {
                                        if group_return < 0 {
                                            *bracode += OP_SBRA - OP_BRA;
                                        }
                                        if *bracode == OP_COND
                                            && *bracode.add(get(bracode, 1) as usize) != OP_ALT
                                        {
                                            *bracode = OP_SCOND;
                                        }
                                    }

                                    if possessive_quantifier != 0 {
                                        if *bracode == OP_COND || *bracode == OP_SCOND {
                                            let mut nlen: c_int =
                                                code.offset_from(bracode) as c_int;
                                            memmove(
                                                bracode.add(1 + LINK_SIZE),
                                                bracode,
                                                nlen as usize,
                                            );
                                            code = code.add(1 + LINK_SIZE);
                                            nlen += (1 + LINK_SIZE) as c_int;
                                            *bracode = if *bracode == OP_COND {
                                                OP_BRAPOS
                                            } else {
                                                OP_SBRAPOS
                                            };
                                            *code = OP_KETRPOS;
                                            code = code.add(1);
                                            putinc(&mut code, 0, nlen);
                                            put(bracode, 1, nlen);
                                        } else {
                                            *bracode += 1; /* Switch to xxxPOS opcodes */
                                            *ketcode = OP_KETRPOS;
                                        }

                                        if !brazeroptr.is_null() {
                                            *brazeroptr = OP_BRAPOSZERO;
                                        }
                                        if repeat_min < 2 {
                                            possessive_quantifier = FALSE;
                                        }
                                    } else {
                                        *ketcode = OP_KETRMAX + repeat_type as u8;
                                    }
                                }
                            }
                        } /* 'bracket_body */

                        let _ = len;
                    }

                    /* OUTPUT_SINGLE_REPEAT shared tail. */
                    if do_output_single_repeat && !goto_end_repeat {
                        let oldcode: *mut PCRE2_UCHAR = code; /* Save where we were */
                        code = previous; /* Usually overwrite previous item */

                        if repeat_max == 0 {
                            goto_end_repeat = true;
                        } else {
                            repeat_type += op_type;

                            if repeat_min == 0 {
                                if repeat_max == REPEAT_UNLIMITED {
                                    *code = OP_STAR + repeat_type as u8;
                                    code = code.add(1);
                                } else if repeat_max == 1 {
                                    *code = OP_QUERY + repeat_type as u8;
                                    code = code.add(1);
                                } else {
                                    *code = OP_UPTO + repeat_type as u8;
                                    code = code.add(1);
                                    put2inc(&mut code, 0, repeat_max);
                                }
                            } else if repeat_min == 1 {
                                if repeat_max == REPEAT_UNLIMITED {
                                    *code = OP_PLUS + repeat_type as u8;
                                    code = code.add(1);
                                } else {
                                    code = oldcode; /* Leave previous item in place */
                                    if repeat_max == 1 {
                                        goto_end_repeat = true;
                                    } else {
                                        *code = OP_UPTO + repeat_type as u8;
                                        code = code.add(1);
                                        put2inc(&mut code, 0, repeat_max - 1);
                                    }
                                }
                            } else {
                                *code = OP_EXACT + op_type as u8; /* EXACT has no repeat_type */
                                code = code.add(1);
                                put2inc(&mut code, 0, repeat_min);

                                if repeat_max != repeat_min {
                                    if mclength > 0 {
                                        memcpy(code, mcbuffer.as_ptr(), mclength as usize);
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

                                    if repeat_max == REPEAT_UNLIMITED {
                                        *code = OP_STAR + repeat_type as u8;
                                        code = code.add(1);
                                    } else {
                                        repeat_max -= repeat_min;
                                        if repeat_max == 1 {
                                            *code = OP_QUERY + repeat_type as u8;
                                            code = code.add(1);
                                        } else {
                                            *code = OP_UPTO + repeat_type as u8;
                                            code = code.add(1);
                                            put2inc(&mut code, 0, repeat_max);
                                        }
                                    }
                                }
                            }

                            /* Fill in the character or character type for the
                            final opcode. */
                            if !goto_end_repeat {
                                if mclength > 0 {
                                    memcpy(code, mcbuffer.as_ptr(), mclength as usize);
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
                            }
                        }
                    }

                    /* Possessive quantifier tail. */
                    if !goto_end_repeat && possessive_quantifier != 0 {
                        let mut len: c_int;

                        match *tempcode {
                            OP_TYPEEXACT => {
                                tempcode = tempcode.add(
                                    OP_LENGTHS[*tempcode as usize] as usize
                                        + if *tempcode.add(1 + IMM2_SIZE) == OP_PROP
                                            || *tempcode.add(1 + IMM2_SIZE) == OP_NOTPROP
                                        {
                                            2
                                        } else {
                                            0
                                        },
                                );
                            }

                            OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_EXACT | OP_EXACTI
                            | OP_NOTEXACT | OP_NOTEXACTI => {
                                tempcode = tempcode.add(OP_LENGTHS[*tempcode as usize] as usize);
                                if utf != 0 && has_extralen(*tempcode.sub(1) as u32) {
                                    tempcode = tempcode.add(get_extralen(*tempcode.sub(1) as u32) as usize);
                                }
                            }

                            OP_CLASS | OP_NCLASS => {
                                tempcode = tempcode.add(1 + 32);
                            }

                            OP_XCLASS | OP_ECLASS => {
                                tempcode = tempcode.add(get(tempcode, 1) as usize);
                            }

                            OP_REF | OP_REFI | OP_DNREF | OP_DNREFI => {
                                tempcode = tempcode.add(OP_LENGTHS[*tempcode as usize] as usize);
                            }

                            _ => {}
                        }

                        len = code.offset_from(tempcode) as c_int;
                        if len > 0 {
                            let repcode: u32 = *tempcode as u32;

                            if repcode < OP_CALLOUT as u32
                                && opcode_possessify[repcode as usize] > 0
                            {
                                *tempcode = opcode_possessify[repcode as usize];
                            } else {
                                memmove(
                                    tempcode.add(1 + LINK_SIZE),
                                    tempcode,
                                    len as usize,
                                );
                                code = code.add(1 + LINK_SIZE);
                                len += (1 + LINK_SIZE) as c_int;
                                *tempcode.add(0) = OP_ONCE;
                                *code = OP_KET;
                                code = code.add(1);
                                putinc(&mut code, 0, len);
                                put(tempcode, 1, len);
                            }
                        }
                    }

                    /* END_REPEAT */
                    (*cb).req_varyopt |= reqvary;
                    pptr = pptr.add(1);
                    continue;
                }
            }
        } /* End of big loop */
    }
}


/*************************************************
*   Compile regex: a sequence of alternatives    *
*************************************************/

/* On entry, pptr is pointing past the bracket meta, but on return it points to
the closing bracket or META_END. The code variable is pointing at the code unit
into which the BRA operator has been stored.

Returns:            0 There has been an error
                   +1 Success, this group must match at least one character
                   -1 Success, this group may match an empty string
*/

pub(crate) unsafe fn compile_regex(
    mut options: u32,
    mut xoptions: u32,
    codeptr: *mut *mut PCRE2_UCHAR,
    pptrptr: *mut *mut u32,
    errorcodeptr: *mut c_int,
    skipunits: u32,
    firstcuptr: *mut u32,
    firstcuflagsptr: *mut u32,
    reqcuptr: *mut u32,
    reqcuflagsptr: *mut u32,
    bcptr: *mut branch_chain,
    mut open_caps: *mut open_capitem,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
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
            *errorcodeptr = ERR33;
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

        /* Accumulate the length for use in the pre-compile phase. */

        length = 2 + 2 * LINK_SIZE + skipunits as PCRE2_SIZE;

        /* Remember if this is a lookbehind assertion. */

        lookbehind = (*code == OP_ASSERTBACK
            || *code == OP_ASSERTBACK_NOT
            || *code == OP_ASSERTBACK_NA) as BOOL;

        if lookbehind != 0 {
            lookbehindlength = meta_data(*pptr.sub(1));
            lookbehindminlength = *pptr;
            pptr = pptr.add(SIZEOFFSET);
        } else {
            lookbehindlength = 0;
            lookbehindminlength = 0;
        }

        /* If this is a capturing subpattern, add to the chain of open items. */

        if *code == OP_CBRA {
            capnumber = get2(code, 1 + LINK_SIZE) as c_int;
            capitem.number = capnumber as u16;
            capitem.next = open_caps;
            capitem.assert_depth = (*cb).assert_depth;
            open_caps = &mut capitem;
        }

        /* Offset is set zero to mark that this bracket is still open */

        put(code, 1, 0);
        code = code.add(1 + LINK_SIZE + skipunits as usize);

        /* Loop for each alternative branch */

        loop {
            let branch_return: c_int;
            let mut branchfirstcu: u32 = 0;
            let mut branchreqcu: u32 = 0;
            let mut branchfirstcuflags: u32 = REQ_UNSET;
            let mut branchreqcuflags: u32 = REQ_UNSET;

            /* Insert OP_REVERSE or OP_VREVERSE if this is a lookbehind. */

            if lookbehind != 0 && lookbehindlength > 0 {
                if lookbehindminlength == LOOKBEHIND_MAX as u32
                    || lookbehindminlength == lookbehindlength
                {
                    *code = OP_REVERSE;
                    code = code.add(1);
                    put2inc(&mut code, 0, lookbehindlength);
                    length += 1 + IMM2_SIZE;
                } else {
                    *code = OP_VREVERSE;
                    code = code.add(1);
                    put2inc(&mut code, 0, lookbehindminlength);
                    put2inc(&mut code, 0, lookbehindlength);
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
                if lengthptr.is_null() { core::ptr::null_mut() } else { &mut length },
            );
            if branch_return == 0 {
                return 0;
            }

            /* If a branch can match an empty string, so can the whole group. */

            if branch_return < 0 {
                okreturn = -1;
            }

            /* In the real compile phase, there is some post-processing. */

            if lengthptr.is_null() {
                if *last_branch != OP_ALT {
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

            /* Handle reaching the end of the expression. */

            if meta_code(*pptr) != META_ALT {
                if lengthptr.is_null() {
                    let mut branch_length: u32 = code.offset_from(last_branch) as u32;
                    loop {
                        let prev_length: u32 = get(last_branch, 1) as u32;
                        put(last_branch, 1, branch_length as c_int);
                        branch_length = prev_length;
                        last_branch = last_branch.sub(branch_length as usize);
                        if branch_length == 0 {
                            break;
                        }
                    }
                }

                /* Fill in the ket */

                *code = OP_KET;
                put(code, 1, code.offset_from(start_bracket) as c_int);
                code = code.add(1 + LINK_SIZE);

                /* Set values to pass back */

                *codeptr = code;
                *pptrptr = pptr;
                *firstcuptr = firstcu;
                *firstcuflagsptr = firstcuflags;
                *reqcuptr = reqcu;
                *reqcuflagsptr = reqcuflags;
                if !lengthptr.is_null() {
                    if OFLOW_MAX - *lengthptr < length {
                        *errorcodeptr = ERR20;
                        return 0;
                    }
                    *lengthptr += length;
                }
                return okreturn;
            }

            /* Another branch follows. */

            if !lengthptr.is_null() {
                code = (*codeptr).add(1 + LINK_SIZE + skipunits as usize);
                length += 1 + LINK_SIZE;
            } else {
                *code = OP_ALT;
                put(code, 1, code.offset_from(last_branch) as c_int);
                last_branch = code;
                bc.current_branch = last_branch;
                code = code.add(1 + LINK_SIZE);
            }

            /* Set the maximum lookbehind length for the next branch and then
            advance past the vertical bar. */

            lookbehindlength = meta_data(*pptr);
            pptr = pptr.add(1);
        }
    }
}
