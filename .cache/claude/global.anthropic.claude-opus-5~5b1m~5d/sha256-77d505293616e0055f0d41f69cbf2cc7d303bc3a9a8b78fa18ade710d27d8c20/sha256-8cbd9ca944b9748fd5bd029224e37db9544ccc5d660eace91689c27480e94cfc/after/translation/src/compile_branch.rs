//! Translated from pcre2_compile.c, lines 5968-8574 (first_significant_code, compile_branch).
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use crate::compile_tables::*;
use crate::compile::*;
use crate::compile_parse::*;
use crate::compile_aux::*;
use crate::compile_class::{_pcre2_compile_class_not_nested_8, _pcre2_compile_class_nested_8,
                          _pcre2_update_classbits_8};
use crate::compile_cgroup::*;
use crate::auto_possess::_pcre2_auto_possessify_8;
use crate::find_bracket::_pcre2_find_bracket_8;
use crate::string_utils::*;
use crate::tables::*;
use crate::ucd::*;
use crate::ord2utf::_pcre2_ord2utf_8;
use crate::chkdint::_pcre2_ckd_smul_8;
use core::ffi::{c_char, c_void};

/* From pcre2_compile.c line 117: #define MAX_GROUP_NUMBER 65535u */
const MAX_GROUP_NUMBER: u32 = 65535u32;

/* PRIV(OP_lengths)[x] without a bounds check. */
macro_rules! OPLEN {
    ($c:expr) => {
        *crate::tables::_pcre2_OP_lengths_8.as_ptr().add($c as usize) as u32
    };
}

/*************************************************
*       Find first significant opcode            *
*************************************************/

/* This is called by several functions that scan a compiled expression looking
for a fixed first character, or an anchoring opcode etc. It skips over things
that do not influence this. For some calls, it makes sense to skip negative
forward and all backward assertions, and also the \b assertion; for others it
does not.

Arguments:
  code         pointer to the start of the group
  skipassert   TRUE if certain assertions are to be skipped

Returns:       pointer to the first significant opcode
*/

pub(crate) unsafe fn first_significant_code(
    code: PCRE2_SPTR,
    skipassert: BOOL,
) -> *const PCRE2_UCHAR {
    let mut code: PCRE2_SPTR = code;

    loop {
        match *code as i32 as u32 {
            OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT | OP_ASSERTBACK_NA => {
                if skipassert == 0 {
                    return code;
                }
                /* do code += GET(code, 1); while (*code == OP_ALT); */
                loop {
                    code = code.add(GET!(code, 1) as usize);
                    if *code as u32 != OP_ALT {
                        break;
                    }
                }
                code = code.add(OPLEN!(*code) as usize);
            }

            OP_WORD_BOUNDARY | OP_NOT_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY
            | OP_NOT_UCP_WORD_BOUNDARY => {
                if skipassert == 0 {
                    return code;
                }
                /* Fall through */
                code = code.add(OPLEN!(*code) as usize);
            }

            OP_CALLOUT | OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF | OP_FALSE | OP_TRUE => {
                code = code.add(OPLEN!(*code) as usize);
            }

            OP_CALLOUT_STR => {
                code = code.add(GET!(code, 1 + 2 * LINK_SIZE) as usize);
            }

            OP_SKIPZERO => {
                code = code.add(2 + GET!(code, 2) as usize + LINK_SIZE);
            }

            OP_COND | OP_SCOND => {
                if *code.add(1 + LINK_SIZE) as u32 != OP_FALSE ||  /* Not DEFINE */
                   *code.add(GET!(code, 1) as usize) as u32 != OP_KET
                /* More than one branch */
                {
                    return code;
                }
                code = code.add(GET!(code, 1) as usize + 1 + LINK_SIZE);
            }

            OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                let delta = *code.add(1) as usize + OPLEN!(*code) as usize;
                code = code.add(delta);
            }

            _ => {
                return code;
            }
        }
    }

    /* PCRE2_DEBUG_UNREACHABLE(); Control should never reach here */
}

/*************************************************
*           Compile one branch                   *
*************************************************/

/* Scan the parsed pattern, compiling it into the a vector of PCRE2_UCHAR. If
the options are changed during the branch, the pointer is used to change the
external options bits. This function is used during the pre-compile phase when
we are trying to find out the amount of memory needed, as well as during the
real compile phase. The value of lengthptr distinguishes the two phases.

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

Returns:            0 There's been an error, *errorcodeptr is non-zero
                   +1 Success, this branch must match at least one character
                   -1 Success, this branch may match an empty string
*/

/* States used to emulate the labels inside the big for(;;) loop. */
const S_SWITCH: u32 = 0;
const S_CLASS_END_PROCESSING: u32 = 1;
const S_VERB_ARG: u32 = 2;
const S_GROUP_PROCESS_NOTE_EMPTY: u32 = 3;
const S_GROUP_PROCESS: u32 = 4;
const S_REPEAT: u32 = 5;
const S_HANDLE_SINGLE_REFERENCE: u32 = 6;
const S_HANDLE_NUMERICAL_RECURSION: u32 = 7;
const S_NORMAL_CHAR: u32 = 8;
const S_NORMAL_CHAR_SET: u32 = 9;
const S_CLASS_CASELESS_CHAR: u32 = 10;

/* States used for the inner switch(op_previous) of the repeat handling. */
const R_SWITCH: u32 = 0;
const R_BRACKET: u32 = 1;
const R_DEFAULT: u32 = 2;
const R_OUTPUT_SINGLE_REPEAT: u32 = 3;
const R_TAIL: u32 = 4;

pub(crate) unsafe fn compile_branch(
    optionsptr: *mut u32,
    xoptionsptr: *mut u32,
    codeptr: *mut *mut PCRE2_UCHAR,
    pptrptr: *mut *mut u32,
    errorcodeptr: *mut i32,
    firstcuptr: *mut u32,
    firstcuflagsptr: *mut u32,
    reqcuptr: *mut u32,
    reqcuflagsptr: *mut u32,
    bcptr: *mut branch_chain,
    open_caps: *mut open_capitem,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> i32 {
    let mut bravalue: i32 = 0;
    let mut okreturn: i32 = -1;
    let mut group_return: i32 = 0;
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

    /* The variable req_caseopt contains either the REQ_CASELESS bit or zero. */

    req_caseopt = if (options & PCRE2_CASELESS) != 0 {
        REQ_CASELESS
    } else {
        0
    };

    /* Switch on next META item until the end of the branch */

    /* for (;; pptr++) */
    'outer: loop {
        let mut possessive_quantifier: BOOL = FALSE;
        let mut note_group_empty: BOOL = FALSE;
        let mut mclength: u32 = 0;
        let mut skipunits: u32 = 0;
        let mut subreqcu: u32 = 0;
        let mut subfirstcu: u32 = 0;
        let mut groupnumber: u32 = 0;
        let mut verbarglen: u32 = 0;
        let mut verbculen: u32 = 0;
        let mut subreqcuflags: u32 = 0;
        let mut subfirstcuflags: u32 = 0;
        let mut oc: *mut open_capitem;
        let mut mcbuffer: [PCRE2_UCHAR; 8] = [0; 8];

        /* Get next META item in the pattern and its potential argument. */

        meta = META_CODE!(*pptr);
        meta_arg = META_DATA!(*pptr);

        /* If we are in the pre-compile phase, accumulate the length used for the
        previous cycle of this loop, unless the next item is a quantifier. */

        if !lengthptr.is_null() {
            if code >= (*cb).start_workspace.add((*cb).workspace_size) {
                *errorcodeptr = ERR52; /* Over-ran workspace - internal error */
                (*cb).erroroffset = 0;
                return 0;
            }

            if code
                > (*cb)
                    .start_workspace
                    .add((*cb).workspace_size - WORK_SIZE_SAFETY_MARGIN)
            /* Check for overrun */
            {
                *errorcodeptr = ERR86; /* Pattern too complicated */
                (*cb).erroroffset = 0;
                return 0;
            }

            /* There is at least one situation where code goes backwards. */

            if code < last_code {
                code = last_code;
            }

            /* If the next thing is not a quantifier, we add the length of the
            previous item into the total, and reset the code pointer to the start
            of the workspace. */

            if meta < META_ASTERISK || meta > META_MINMAX_QUERY {
                if (OFLOW_MAX as usize).wrapping_sub(*lengthptr)
                    < code.offset_from(orig_code) as PCRE2_SIZE
                {
                    *errorcodeptr = ERR20; /* Integer overflow */
                    (*cb).erroroffset = 0;
                    return 0;
                }
                *lengthptr = (*lengthptr).wrapping_add(code.offset_from(orig_code) as PCRE2_SIZE);
                if *lengthptr > MAX_PATTERN_SIZE {
                    *errorcodeptr = ERR20; /* Pattern is too large */
                    (*cb).erroroffset = 0;
                    return 0;
                }
                code = orig_code;
            }

            /* Remember where this code item starts. */

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

        /* switch(meta) -- 'body is the end of the switch (C "break") */
        'body: {
            let mut state: u32 = S_SWITCH;
            'sm: loop {
                match state {
                    /* ============================================================ */
                    S_SWITCH => {
                        match meta {
                            /* ======================================================*/
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

                            /* ======================================================*/
                            /* Handle single-character metacharacters. */
                            META_CIRCUMFLEX => {
                                if (options & PCRE2_MULTILINE) != 0 {
                                    if firstcuflags == REQ_UNSET {
                                        firstcuflags = REQ_NONE;
                                        zerofirstcuflags = REQ_NONE;
                                    }
                                    *code = OP_CIRCM as u8;
                                    code = code.add(1);
                                } else {
                                    *code = OP_CIRC as u8;
                                    code = code.add(1);
                                }
                                break 'body;
                            }

                            META_DOLLAR => {
                                *code = (if (options & PCRE2_MULTILINE) != 0 {
                                    OP_DOLLM
                                } else {
                                    OP_DOLL
                                }) as u8;
                                code = code.add(1);
                                break 'body;
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
                                *code = (if (options & PCRE2_DOTALL) != 0 {
                                    OP_ALLANY
                                } else {
                                    OP_ANY
                                }) as u8;
                                code = code.add(1);
                                break 'body;
                            }

                            /* ======================================================*/
                            /* Empty character classes. */
                            META_CLASS_EMPTY | META_CLASS_EMPTY_NOT => {
                                matched_char = TRUE;
                                if meta == META_CLASS_EMPTY_NOT {
                                    *code = OP_ALLANY as u8;
                                    code = code.add(1);
                                } else {
                                    *code = OP_CLASS as u8;
                                    code = code.add(1);
                                    core::ptr::write_bytes(code, 0u8, 32);
                                    code = code.add(32);
                                }

                                if firstcuflags == REQ_UNSET {
                                    firstcuflags = REQ_NONE;
                                }
                                zerofirstcu = firstcu;
                                zerofirstcuflags = firstcuflags;
                                break 'body;
                            }

                            /* ======================================================*/
                            /* Non-empty character class. */
                            META_CLASS_NOT | META_CLASS => {
                                matched_char = TRUE;

                                /* Check for complex extended classes. */

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
                                    /* goto CLASS_END_PROCESSING */
                                    state = S_CLASS_END_PROCESSING;
                                    continue 'sm;
                                }

                                /* Single character in a class optimization. */

                                if *pptr.add(1) < META_END && *pptr.add(2) == META_CLASS_END {
                                    let c: u32 = *pptr.add(1);

                                    pptr = pptr.add(2); /* Move on to class end */
                                    if meta == META_CLASS {
                                        meta = c; /* Set up the character */
                                        /* goto NORMAL_CHAR_SET */
                                        state = S_NORMAL_CHAR_SET;
                                        continue 'sm;
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
                                            & (PCRE2_EXTRA_TURKISH_CASING
                                                | PCRE2_EXTRA_CASELESS_RESTRICT))
                                            == PCRE2_EXTRA_TURKISH_CASING
                                            && UCD_ANY_I!(c)
                                        {
                                            caseset = _pcre2_ucd_turkish_dotted_i_caseset_8
                                                + (if UCD_DOTTED_I!(c) { 0 } else { 3 });
                                        } else {
                                            caseset = UCD_CASESET!(c);
                                            if caseset != 0
                                                && (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                                                && *_pcre2_ucd_caseless_sets_8
                                                    .as_ptr()
                                                    .add(caseset as usize)
                                                    < 128
                                            {
                                                caseset = 0; /* Ignore restricted set. */
                                            }
                                        }

                                        if caseset != 0 {
                                            *code = OP_NOTPROP as u8;
                                            code = code.add(1);
                                            *code = PT_CLIST as u8;
                                            code = code.add(1);
                                            *code = caseset as u8;
                                            code = code.add(1);
                                            break 'body; /* Finished with this class */
                                        }
                                    }

                                    /* Char has only one other (usable) case */

                                    *code = (if (options & PCRE2_CASELESS) != 0 {
                                        OP_NOTI
                                    } else {
                                        OP_NOT
                                    }) as u8;
                                    code = code.add(1);
                                    code = code.add(PUTCHAR!(c, code, utf) as usize);
                                    break 'body; /* Finished with this class */
                                } /* End of 1-char optimization */

                                /* Exactly two characters that are case partners? */

                                if meta == META_CLASS
                                    && *pptr.add(1) < META_END
                                    && *pptr.add(2) < META_END
                                    && *pptr.add(3) == META_CLASS_END
                                {
                                    let c: u32 = *pptr.add(1);

                                    if (UCD_CASESET!(c) == 0
                                        || ((xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                                            && c < 128
                                            && *pptr.add(2) < 128))
                                        && !((xoptions
                                            & (PCRE2_EXTRA_TURKISH_CASING
                                                | PCRE2_EXTRA_CASELESS_RESTRICT))
                                            == PCRE2_EXTRA_TURKISH_CASING
                                            && UCD_ANY_I!(c))
                                    {
                                        let d: u32;

                                        if (utf != 0 || ucp != 0) && c > 127 {
                                            d = UCD_OTHERCASE!(c);
                                        } else {
                                            d = TABLE_GET!(c, (*cb).fcc, c) as u32;
                                        }

                                        if c != d && *pptr.add(2) == d {
                                            pptr = pptr.add(3); /* Move on to class end */
                                            meta = c;
                                            if (options & PCRE2_CASELESS) == 0 {
                                                reset_caseful = TRUE;
                                                options |= PCRE2_CASELESS;
                                                req_caseopt = REQ_CASELESS;
                                            }
                                            /* goto CLASS_CASELESS_CHAR */
                                            state = S_CLASS_CASELESS_CHAR;
                                            continue 'sm;
                                        }
                                    }
                                }

                                /* Now emit the OP_CLASS/OP_NCLASS/OP_XCLASS/OP_ALLANY. */

                                pptr = _pcre2_compile_class_not_nested_8(
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
                                /* PCRE2_ASSERT(*pptr == META_CLASS_END); */

                                /* Fall through to CLASS_END_PROCESSING */
                                state = S_CLASS_END_PROCESSING;
                                continue 'sm;
                            }

                            /* ======================================================*/
                            /* Deal with (*VERB)s. */
                            META_ACCEPT => {
                                had_accept = TRUE;
                                (*cb).had_accept = had_accept;
                                oc = open_caps;
                                while !oc.is_null()
                                    && (*oc).assert_depth >= (*cb).assert_depth
                                {
                                    if !lengthptr.is_null() {
                                        *lengthptr =
                                            (*lengthptr).wrapping_add(1 + IMM2_SIZE);
                                    } else {
                                        *code = OP_CLOSE as u8;
                                        code = code.add(1);
                                        PUT2INC!(code, 0, (*oc).number);
                                    }
                                    oc = (*oc).next;
                                }
                                *code = (if (*cb).assert_depth > 0 {
                                    OP_ASSERT_ACCEPT
                                } else {
                                    OP_ACCEPT
                                }) as u8;
                                code = code.add(1);
                                if firstcuflags == REQ_UNSET {
                                    firstcuflags = REQ_NONE;
                                }
                                break 'body;
                            }

                            META_PRUNE | META_SKIP => {
                                (*cb).had_pruneorskip = TRUE;
                                /* Fall through */
                                *code = *verbops
                                    .as_ptr()
                                    .add(((meta - META_MARK) >> 16) as usize)
                                    as u8;
                                code = code.add(1);
                                break 'body;
                            }

                            META_COMMIT | META_FAIL => {
                                *code = *verbops
                                    .as_ptr()
                                    .add(((meta - META_MARK) >> 16) as usize)
                                    as u8;
                                code = code.add(1);
                                break 'body;
                            }

                            META_THEN => {
                                (*cb).external_flags |= PCRE2_HASTHEN;
                                *code = OP_THEN as u8;
                                code = code.add(1);
                                break 'body;
                            }

                            /* Handle verbs with arguments. */
                            META_THEN_ARG => {
                                (*cb).external_flags |= PCRE2_HASTHEN;
                                /* goto VERB_ARG */
                                state = S_VERB_ARG;
                                continue 'sm;
                            }

                            META_PRUNE_ARG | META_SKIP_ARG => {
                                (*cb).had_pruneorskip = TRUE;
                                /* Fall through to VERB_ARG */
                                state = S_VERB_ARG;
                                continue 'sm;
                            }

                            META_MARK | META_COMMIT_ARG => {
                                /* Fall through to VERB_ARG */
                                state = S_VERB_ARG;
                                continue 'sm;
                            }

                            /* ======================================================*/
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
                                req_caseopt = if (options & PCRE2_CASELESS) != 0 {
                                    REQ_CASELESS
                                } else {
                                    0
                                };
                                break 'body;
                            }

                            /* ======================================================*/
                            /* Handle scan substring. */
                            META_OFFSET => {
                                if !lengthptr.is_null() {
                                    pptr = _pcre2_compile_parse_scan_substr_args8(
                                        pptr,
                                        errorcodeptr,
                                        cb,
                                        lengthptr,
                                    );
                                    if pptr.is_null() {
                                        return 0;
                                    }
                                    break 'body;
                                }

                                /* while (TRUE) */
                                'substr: loop {
                                    let mut count: i32;
                                    let mut index: i32;
                                    let ng: *mut named_group;

                                    match META_CODE!(*pptr) {
                                        META_OFFSET => {
                                            pptr = pptr.add(1);
                                            SKIPOFFSET!(pptr);
                                            continue 'substr;
                                        }

                                        META_CAPTURE_NAME => {
                                            ng = (*cb)
                                                .named_groups
                                                .add(*pptr.add(1) as usize);
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
                                            continue 'substr;
                                        }

                                        META_CAPTURE_NUMBER => {
                                            pptr = pptr.add(2);
                                            if *pptr.offset(-1) == 0 {
                                                continue 'substr;
                                            }

                                            *code.add(0) = OP_CREF as u8;
                                            PUT2!(code, 1, *pptr.offset(-1));
                                            code = code.add(1 + IMM2_SIZE);
                                            continue 'substr;
                                        }

                                        _ => {}
                                    }

                                    break 'substr;
                                }
                                pptr = pptr.offset(-1);
                                break 'body;
                            }

                            META_SCS => {
                                bravalue = OP_ASSERT_SCS as i32;
                                (*cb).assert_depth += 1;
                                /* goto GROUP_PROCESS */
                                state = S_GROUP_PROCESS;
                                continue 'sm;
                            }

                            /* ======================================================*/
                            /* Handle conditional subpatterns. */
                            META_COND_RNUMBER | META_COND_NAME | META_COND_RNAME => {
                                bravalue = OP_COND as i32;

                                if !lengthptr.is_null() {
                                    let mut i: u32;
                                    let name: PCRE2_SPTR;
                                    let ng: *mut named_group;
                                    let start_pptr: *mut u32 = pptr;
                                    pptr = pptr.add(1);
                                    let length: u32 = *pptr;

                                    GETPLUSOFFSET!(offset, pptr);
                                    name = (*cb).start_pattern.add(offset);

                                    ng = _pcre2_compile_find_named_group8(name, length, cb);

                                    if ng.is_null() {
                                        /* Bad reference, unless R<digits>. */

                                        groupnumber = 0;
                                        if meta == META_COND_RNUMBER {
                                            i = 1;
                                            while i < length {
                                                groupnumber = groupnumber * 10
                                                    + (*name.add(i as usize) as u32
                                                        - b'0' as u32);
                                                if groupnumber > MAX_GROUP_NUMBER {
                                                    *errorcodeptr = ERR61;
                                                    (*cb).erroroffset =
                                                        offset + i as PCRE2_SIZE;
                                                    return 0;
                                                }
                                                i += 1;
                                            }
                                        }

                                        if meta != META_COND_RNUMBER
                                            || groupnumber > (*cb).bracount
                                        {
                                            *errorcodeptr = ERR15;
                                            (*cb).erroroffset = offset;
                                            return 0;
                                        }

                                        if groupnumber == 0 {
                                            groupnumber = RREF_ANY;
                                        }
                                        *start_pptr.add(1) = groupnumber;
                                        skipunits = (1 + IMM2_SIZE) as u32;
                                        /* goto GROUP_PROCESS_NOTE_EMPTY */
                                        state = S_GROUP_PROCESS_NOTE_EMPTY;
                                        continue 'sm;
                                    }

                                    /* From here on we know we have a name. */
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
                                        /* goto GROUP_PROCESS_NOTE_EMPTY */
                                        state = S_GROUP_PROCESS_NOTE_EMPTY;
                                        continue 'sm;
                                    }

                                    /* We have a duplicated name. */

                                    *start_pptr.add(0) = meta | 1;
                                    *start_pptr.add(1) =
                                        ng.offset_from((*cb).named_groups) as u32;

                                    skipunits = (1 + 2 * IMM2_SIZE) as u32;
                                } else {
                                    /* Second phase of compilation. */
                                    let mut count: i32;
                                    let mut index: i32;
                                    let ng: *mut named_group;

                                    if meta == META_COND_RNUMBER {
                                        *code.add(1 + LINK_SIZE) = OP_RREF as u8;
                                        PUT2!(code, 2 + LINK_SIZE, *pptr.add(1));
                                        skipunits = (1 + IMM2_SIZE) as u32;
                                        pptr = pptr.add(1 + SIZEOFFSET);
                                        /* goto GROUP_PROCESS_NOTE_EMPTY */
                                        state = S_GROUP_PROCESS_NOTE_EMPTY;
                                        continue 'sm;
                                    }

                                    if meta_arg == 0 {
                                        *code.add(1 + LINK_SIZE) =
                                            (if meta == META_COND_RNAME {
                                                OP_RREF
                                            } else {
                                                OP_CREF
                                            }) as u8;
                                        PUT2!(code, 2 + LINK_SIZE, *pptr.add(1));
                                        skipunits = (1 + IMM2_SIZE) as u32;
                                        pptr = pptr.add(1 + SIZEOFFSET);
                                        /* goto GROUP_PROCESS_NOTE_EMPTY */
                                        state = S_GROUP_PROCESS_NOTE_EMPTY;
                                        continue 'sm;
                                    }

                                    ng = (*cb).named_groups.add(*pptr.add(1) as usize);
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

                                    *code.add(1 + LINK_SIZE) =
                                        (if meta == META_COND_RNAME {
                                            OP_DNRREF
                                        } else {
                                            OP_DNCREF
                                        }) as u8;

                                    PUT2!(code, 2 + LINK_SIZE, index);
                                    PUT2!(code, 2 + LINK_SIZE + IMM2_SIZE, count);
                                    skipunits = (1 + 2 * IMM2_SIZE) as u32;
                                    pptr = pptr.add(1 + SIZEOFFSET);
                                }

                                /* goto GROUP_PROCESS_NOTE_EMPTY */
                                state = S_GROUP_PROCESS_NOTE_EMPTY;
                                continue 'sm;
                            }

                            /* The DEFINE condition is always false. */
                            META_COND_DEFINE => {
                                bravalue = OP_COND as i32;
                                GETPLUSOFFSET!(offset, pptr);
                                *code.add(1 + LINK_SIZE) = OP_DEFINE as u8;
                                skipunits = 1;
                                /* goto GROUP_PROCESS */
                                state = S_GROUP_PROCESS;
                                continue 'sm;
                            }

                            /* Conditional test of a group's being set. */
                            META_COND_NUMBER => {
                                bravalue = OP_COND as i32;
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
                                offset = offset.wrapping_sub(2);
                                *code.add(1 + LINK_SIZE) = OP_CREF as u8;
                                skipunits = (1 + IMM2_SIZE) as u32;
                                PUT2!(code, 2 + LINK_SIZE, groupnumber);
                                /* goto GROUP_PROCESS_NOTE_EMPTY */
                                state = S_GROUP_PROCESS_NOTE_EMPTY;
                                continue 'sm;
                            }

                            /* Test for the PCRE2 version. */
                            META_COND_VERSION => {
                                bravalue = OP_COND as i32;
                                if *pptr.add(1) > 0 {
                                    *code.add(1 + LINK_SIZE) = (if (PCRE2_MAJOR
                                        > *pptr.add(2))
                                        || (PCRE2_MAJOR == *pptr.add(2)
                                            && PCRE2_MINOR >= *pptr.add(3))
                                    {
                                        OP_TRUE
                                    } else {
                                        OP_FALSE
                                    }) as u8;
                                } else {
                                    *code.add(1 + LINK_SIZE) = (if PCRE2_MAJOR
                                        == *pptr.add(2)
                                        && PCRE2_MINOR == *pptr.add(3)
                                    {
                                        OP_TRUE
                                    } else {
                                        OP_FALSE
                                    }) as u8;
                                }
                                skipunits = 1;
                                pptr = pptr.add(3);
                                /* goto GROUP_PROCESS_NOTE_EMPTY */
                                state = S_GROUP_PROCESS_NOTE_EMPTY;
                                continue 'sm;
                            }

                            /* The condition is an assertion. */
                            META_COND_ASSERT => {
                                bravalue = OP_COND as i32;
                                /* goto GROUP_PROCESS_NOTE_EMPTY */
                                state = S_GROUP_PROCESS_NOTE_EMPTY;
                                continue 'sm;
                            }

                            /* ======================================================*/
                            /* Handle all kinds of nested bracketed groups. */
                            META_LOOKAHEAD => {
                                bravalue = OP_ASSERT as i32;
                                (*cb).assert_depth += 1;
                                /* goto GROUP_PROCESS */
                                state = S_GROUP_PROCESS;
                                continue 'sm;
                            }

                            META_LOOKAHEAD_NA => {
                                bravalue = OP_ASSERT_NA as i32;
                                (*cb).assert_depth += 1;
                                /* goto GROUP_PROCESS */
                                state = S_GROUP_PROCESS;
                                continue 'sm;
                            }

                            /* Optimize (?!) to (*FAIL) unless it is quantified. */
                            META_LOOKAHEADNOT => {
                                if *pptr.add(1) == META_KET
                                    && (*pptr.add(2) < META_ASTERISK
                                        || *pptr.add(2) > META_MINMAX_QUERY)
                                {
                                    *code = OP_FAIL as u8;
                                    code = code.add(1);
                                    pptr = pptr.add(1);
                                } else {
                                    bravalue = OP_ASSERT_NOT as i32;
                                    (*cb).assert_depth += 1;
                                    /* goto GROUP_PROCESS */
                                    state = S_GROUP_PROCESS;
                                    continue 'sm;
                                }
                                break 'body;
                            }

                            META_LOOKBEHIND => {
                                bravalue = OP_ASSERTBACK as i32;
                                (*cb).assert_depth += 1;
                                /* goto GROUP_PROCESS */
                                state = S_GROUP_PROCESS;
                                continue 'sm;
                            }

                            META_LOOKBEHINDNOT => {
                                bravalue = OP_ASSERTBACK_NOT as i32;
                                (*cb).assert_depth += 1;
                                /* goto GROUP_PROCESS */
                                state = S_GROUP_PROCESS;
                                continue 'sm;
                            }

                            META_LOOKBEHIND_NA => {
                                bravalue = OP_ASSERTBACK_NA as i32;
                                (*cb).assert_depth += 1;
                                /* goto GROUP_PROCESS */
                                state = S_GROUP_PROCESS;
                                continue 'sm;
                            }

                            META_ATOMIC => {
                                bravalue = OP_ONCE as i32;
                                /* goto GROUP_PROCESS_NOTE_EMPTY */
                                state = S_GROUP_PROCESS_NOTE_EMPTY;
                                continue 'sm;
                            }

                            META_SCRIPT_RUN => {
                                bravalue = OP_SCRIPT_RUN as i32;
                                /* goto GROUP_PROCESS_NOTE_EMPTY */
                                state = S_GROUP_PROCESS_NOTE_EMPTY;
                                continue 'sm;
                            }

                            META_NOCAPTURE => {
                                bravalue = OP_BRA as i32;
                                /* Fall through to GROUP_PROCESS_NOTE_EMPTY */
                                state = S_GROUP_PROCESS_NOTE_EMPTY;
                                continue 'sm;
                            }

                            /* ======================================================*/
                            /* Handle named backreferences and recursions. */
                            META_BACKREF_BYNAME | META_RECURSE_BYNAME => {
                                let mut count: i32;
                                let mut index: i32;
                                let name: PCRE2_SPTR;
                                let ng: *mut named_group;
                                pptr = pptr.add(1);
                                let length: u32 = *pptr;

                                GETPLUSOFFSET!(offset, pptr);
                                name = (*cb).start_pattern.add(offset);

                                ng = _pcre2_compile_find_named_group8(name, length, cb);

                                if ng.is_null() {
                                    /* Bad reference. */
                                    *errorcodeptr = ERR15;
                                    (*cb).erroroffset = offset;
                                    return 0;
                                }

                                groupnumber = (*ng).number;

                                if meta == META_RECURSE_BYNAME {
                                    meta_arg = groupnumber;
                                    /* goto HANDLE_NUMERICAL_RECURSION */
                                    state = S_HANDLE_NUMERICAL_RECURSION;
                                    continue 'sm;
                                }

                                (*cb).backref_map |= if groupnumber < 32 {
                                    1u32 << groupnumber
                                } else {
                                    1
                                };
                                if groupnumber > (*cb).top_backref {
                                    (*cb).top_backref = groupnumber;
                                }

                                if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) == 0 {
                                    meta_arg = groupnumber;
                                    /* goto HANDLE_SINGLE_REFERENCE */
                                    state = S_HANDLE_SINGLE_REFERENCE;
                                    continue 'sm;
                                }

                                count = 0;
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

                                if firstcuflags == REQ_UNSET {
                                    firstcuflags = REQ_NONE;
                                }
                                *code = (if (options & PCRE2_CASELESS) != 0 {
                                    OP_DNREFI
                                } else {
                                    OP_DNREF
                                }) as u8;
                                code = code.add(1);
                                PUT2INC!(code, 0, index);
                                PUT2INC!(code, 0, count);
                                if (options & PCRE2_CASELESS) != 0 {
                                    *code = ((if (xoptions
                                        & PCRE2_EXTRA_CASELESS_RESTRICT)
                                        != 0
                                    {
                                        REFI_FLAG_CASELESS_RESTRICT
                                    } else {
                                        0
                                    }) | (if (xoptions & PCRE2_EXTRA_TURKISH_CASING)
                                        != 0
                                    {
                                        REFI_FLAG_TURKISH_CASING
                                    } else {
                                        0
                                    })) as u8;
                                    code = code.add(1);
                                }
                                break 'body;
                            }

                            /* ======================================================*/
                            /* Handle a numerical callout. */
                            META_CALLOUT_NUMBER => {
                                *code.add(0) = OP_CALLOUT as u8;
                                PUT!(code, 1, *pptr.add(1)); /* Offset to next item */
                                PUT!(code, 1 + LINK_SIZE, *pptr.add(2)); /* Length */
                                *code.add(1 + 2 * LINK_SIZE) = *pptr.add(3) as u8;
                                pptr = pptr.add(3);
                                code = code.add(OPLEN!(OP_CALLOUT) as usize);
                                break 'body;
                            }

                            /* ======================================================*/
                            /* Handle a callout with a string argument. */
                            META_CALLOUT_STRING => {
                                if !lengthptr.is_null() {
                                    /* pptr[3] + (1 + 4*LINK_SIZE) is computed in
                                    uint32_t in C, then widened. */
                                    *lengthptr = (*lengthptr).wrapping_add(
                                        (*pptr.add(3))
                                            .wrapping_add((1 + 4 * LINK_SIZE) as u32)
                                            as PCRE2_SIZE,
                                    );
                                    pptr = pptr.add(3);
                                    SKIPOFFSET!(pptr);
                                } else {
                                    let mut pp: PCRE2_SPTR;
                                    let mut delimiter: u32;
                                    let mut length: u32 = *pptr.add(3);
                                    let mut callout_string: *mut PCRE2_UCHAR =
                                        code.add(1 + 4 * LINK_SIZE);

                                    *code.add(0) = OP_CALLOUT_STR as u8;
                                    PUT!(code, 1, *pptr.add(1)); /* Offset to next */
                                    PUT!(code, 1 + LINK_SIZE, *pptr.add(2)); /* Length */

                                    pptr = pptr.add(3);
                                    GETPLUSOFFSET!(offset, pptr);
                                    pp = (*cb).start_pattern.add(offset);
                                    delimiter = {
                                        let t = *pp;
                                        pp = pp.add(1);
                                        *callout_string = t;
                                        callout_string = callout_string.add(1);
                                        t as u32
                                    };
                                    if delimiter == b'{' as u32 {
                                        delimiter = b'}' as u32;
                                    }
                                    /* One after delimiter */
                                    PUT!(code, 1 + 3 * LINK_SIZE, (offset + 1) as i32);

                                    /* while (--length > 1) */
                                    loop {
                                        length = length.wrapping_sub(1);
                                        if !(length > 1) {
                                            break;
                                        }
                                        if *pp as u32 == delimiter
                                            && *pp.add(1) as u32 == delimiter
                                        {
                                            *callout_string = delimiter as u8;
                                            callout_string = callout_string.add(1);
                                            pp = pp.add(2);
                                            length = length.wrapping_sub(1);
                                        } else {
                                            *callout_string = *pp;
                                            callout_string = callout_string.add(1);
                                            pp = pp.add(1);
                                        }
                                    }
                                    *callout_string = 0; /* CHAR_NUL */
                                    callout_string = callout_string.add(1);

                                    /* Set the length of the entire item. */

                                    PUT!(
                                        code,
                                        1 + 2 * LINK_SIZE,
                                        callout_string.offset_from(code) as i32
                                    );
                                    code = callout_string;
                                }
                                break 'body;
                            }

                            /* ======================================================*/
                            /* Handle repetition. */
                            META_MINMAX_PLUS | META_MINMAX_QUERY | META_MINMAX => {
                                pptr = pptr.add(1);
                                repeat_min = *pptr;
                                pptr = pptr.add(1);
                                repeat_max = *pptr;
                                /* goto REPEAT */
                                state = S_REPEAT;
                                continue 'sm;
                            }

                            META_ASTERISK | META_ASTERISK_PLUS | META_ASTERISK_QUERY => {
                                repeat_min = 0;
                                repeat_max = REPEAT_UNLIMITED;
                                /* goto REPEAT */
                                state = S_REPEAT;
                                continue 'sm;
                            }

                            META_PLUS | META_PLUS_PLUS | META_PLUS_QUERY => {
                                repeat_min = 1;
                                repeat_max = REPEAT_UNLIMITED;
                                /* goto REPEAT */
                                state = S_REPEAT;
                                continue 'sm;
                            }

                            META_QUERY | META_QUERY_PLUS | META_QUERY_QUERY => {
                                repeat_min = 0;
                                repeat_max = 1;
                                /* Falls into REPEAT */
                                state = S_REPEAT;
                                continue 'sm;
                            }

                            /* ======================================================*/
                            /* Handle a 32-bit data character > META_END. */
                            META_BIGVALUE => {
                                pptr = pptr.add(1);
                                /* goto NORMAL_CHAR */
                                state = S_NORMAL_CHAR;
                                continue 'sm;
                            }

                            /* ======================================================*/
                            /* Handle a back reference by number. */
                            META_BACKREF => {
                                if meta_arg < 10 {
                                    offset = *(*cb)
                                        .small_ref_offset
                                        .as_ptr()
                                        .add(meta_arg as usize);
                                } else {
                                    GETPLUSOFFSET!(offset, pptr);
                                }

                                if meta_arg > (*cb).bracount {
                                    (*cb).erroroffset = offset;
                                    *errorcodeptr = ERR15; /* Non-existent subpattern */
                                    return 0;
                                }

                                /* Falls into HANDLE_SINGLE_REFERENCE */
                                state = S_HANDLE_SINGLE_REFERENCE;
                                continue 'sm;
                            }

                            /* ======================================================*/
                            /* Handle recursion by number. */
                            META_RECURSE => {
                                GETPLUSOFFSET!(offset, pptr);
                                if meta_arg > (*cb).bracount {
                                    (*cb).erroroffset = offset;
                                    *errorcodeptr = ERR15; /* Non-existent subpattern */
                                    return 0;
                                }
                                /* Falls into HANDLE_NUMERICAL_RECURSION */
                                state = S_HANDLE_NUMERICAL_RECURSION;
                                continue 'sm;
                            }

                            /* ======================================================*/
                            /* Handle capturing parentheses. */
                            META_CAPTURE => {
                                bravalue = OP_CBRA as i32;
                                skipunits = IMM2_SIZE as u32;
                                PUT2!(code, 1 + LINK_SIZE, meta_arg);
                                (*cb).lastcapture = meta_arg;
                                /* goto GROUP_PROCESS_NOTE_EMPTY */
                                state = S_GROUP_PROCESS_NOTE_EMPTY;
                                continue 'sm;
                            }

                            /* ======================================================*/
                            /* Handle escape sequence items. */
                            META_ESCAPE => {
                                if meta_arg > ESC_b && meta_arg < ESC_Z {
                                    matched_char = TRUE;
                                    if firstcuflags == REQ_UNSET {
                                        firstcuflags = REQ_NONE;
                                    }
                                }

                                /* Set values to reset to for a zero repeat. */

                                zerofirstcu = firstcu;
                                zerofirstcuflags = firstcuflags;
                                zeroreqcu = reqcu;
                                zeroreqcuflags = reqcuflags;

                                if meta_arg == ESC_P || meta_arg == ESC_p {
                                    pptr = pptr.add(1);
                                    let mut ptype: u32 = *pptr >> 16;
                                    let mut pdata: u32 = *pptr & 0xffff;

                                    if (options & PCRE2_CASELESS) != 0
                                        && ptype == PT_PC
                                        && (pdata == ucp_Lu
                                            || pdata == ucp_Ll
                                            || pdata == ucp_Lt)
                                    {
                                        ptype = PT_LAMP;
                                        pdata = 0;
                                    }

                                    if ptype == PT_ANY {
                                        if meta_arg == ESC_P {
                                            *code = OP_CLASS as u8;
                                            code = code.add(1);
                                            core::ptr::write_bytes(code, 0u8, 32);
                                            code = code.add(32);
                                        } else {
                                            *code = OP_ALLANY as u8;
                                            code = code.add(1);
                                        }
                                    } else {
                                        *code = (if meta_arg == ESC_p {
                                            OP_PROP
                                        } else {
                                            OP_NOTPROP
                                        }) as u8;
                                        code = code.add(1);
                                        *code = ptype as u8;
                                        code = code.add(1);
                                        *code = pdata as u8;
                                        code = code.add(1);
                                    }
                                    break 'body; /* End META_ESCAPE */
                                }

                                /* \K is forbidden in lookarounds since 10.38. */

                                if (*cb).assert_depth > 0
                                    && meta_arg == ESC_K
                                    && (xoptions & PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK) == 0
                                {
                                    *errorcodeptr = ERR99;
                                    return 0;
                                }

                                match meta_arg {
                                    ESC_C => {
                                        (*cb).external_flags |= PCRE2_HASBKC; /* Record */
                                        if utf == 0 {
                                            meta_arg = OP_ALLANY;
                                        }
                                    }

                                    ESC_B | ESC_b => {
                                        if (options & PCRE2_UCP) != 0
                                            && (xoptions & PCRE2_EXTRA_ASCII_BSW) == 0
                                        {
                                            meta_arg = if meta_arg == ESC_B {
                                                OP_NOT_UCP_WORD_BOUNDARY
                                            } else {
                                                OP_UCP_WORD_BOUNDARY
                                            };
                                        }
                                        /* Fall through */
                                        if (*cb).max_lookbehind == 0 {
                                            (*cb).max_lookbehind = 1;
                                        }
                                    }

                                    ESC_A => {
                                        if (*cb).max_lookbehind == 0 {
                                            (*cb).max_lookbehind = 1;
                                        }
                                    }

                                    ESC_K => {
                                        (*cb).external_flags |= PCRE2_HASBSK; /* Record */
                                    }

                                    _ => {}
                                }

                                *code = meta_arg as u8;
                                code = code.add(1);
                                break 'body; /* End META_ESCAPE */
                            }

                            /* ======================================================*/
                            /* Handle an unrecognized meta value / literal. */
                            _ => {
                                if meta >= META_END {
                                    *errorcodeptr = ERR89; /* Internal error */
                                    return 0;
                                }

                                /* Falls into NORMAL_CHAR */
                                state = S_NORMAL_CHAR;
                                continue 'sm;
                            }
                        }
                    }

                    /* ============================================================ */
                    /* CLASS_END_PROCESSING: */
                    S_CLASS_END_PROCESSING => {
                        /* If this class is the first thing in the branch, there can be
                        no first char setting, whatever the repeat count. */

                        if firstcuflags == REQ_UNSET {
                            firstcuflags = REQ_NONE;
                        }
                        zerofirstcu = firstcu;
                        zerofirstcuflags = firstcuflags;
                        zeroreqcu = reqcu;
                        zeroreqcuflags = reqcuflags;
                        break 'body; /* End of class processing */
                    }

                    /* ============================================================ */
                    /* VERB_ARG: */
                    S_VERB_ARG => {
                        *code =
                            *verbops.as_ptr().add(((meta - META_MARK) >> 16) as usize) as u8;
                        code = code.add(1);
                        /* The length is in characters. */
                        pptr = pptr.add(1);
                        verbarglen = *pptr;
                        verbculen = 0;
                        tempcode = code;
                        code = code.add(1);
                        let mut i: i32 = 0;
                        while i < verbarglen as i32 {
                            pptr = pptr.add(1);
                            meta = *pptr;
                            if utf != 0 {
                                mclength = _pcre2_ord2utf_8(meta, mcbuffer.as_mut_ptr());
                            } else {
                                mclength = 1;
                                mcbuffer[0] = meta as u8;
                            }
                            if !lengthptr.is_null() {
                                *lengthptr =
                                    (*lengthptr).wrapping_add(mclength as PCRE2_SIZE);
                            } else {
                                core::ptr::copy_nonoverlapping(
                                    mcbuffer.as_ptr(),
                                    code,
                                    mclength as usize,
                                );
                                code = code.add(mclength as usize);
                                verbculen += mclength;
                            }
                            i += 1;
                        }

                        *tempcode = verbculen as u8; /* Fill in code unit length */
                        *code = 0; /* Terminating zero */
                        code = code.add(1);
                        break 'body;
                    }

                    /* ============================================================ */
                    /* GROUP_PROCESS_NOTE_EMPTY: */
                    S_GROUP_PROCESS_NOTE_EMPTY => {
                        note_group_empty = TRUE;
                        /* Falls into GROUP_PROCESS */
                        state = S_GROUP_PROCESS;
                        continue 'sm;
                    }

                    /* ============================================================ */
                    /* GROUP_PROCESS: */
                    S_GROUP_PROCESS => {
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
                                core::ptr::null_mut()
                            } else {
                                &mut length_prevgroup
                            },
                        );
                        if group_return == 0 {
                            return 0; /* Error */
                        }

                        (*cb).parens_depth -= 1;

                        /* If that was a non-conditional significant group that matches at
                        least one character, then the current item matches a character. */

                        if note_group_empty != 0
                            && bravalue != OP_COND as i32
                            && group_return > 0
                        {
                            matched_char = TRUE;
                        }

                        /* If we've just compiled an assertion, pop the assert depth. */

                        if bravalue >= OP_ASSERT as i32 && bravalue <= OP_ASSERT_SCS as i32 {
                            (*cb).assert_depth -= 1;
                        }

                        /* Check conditional bracket branch counts. */

                        if bravalue == OP_COND as i32 && lengthptr.is_null() {
                            let mut tc: *mut PCRE2_UCHAR = code;
                            let mut condcount: i32 = 0;

                            loop {
                                condcount += 1;
                                tc = tc.add(GET!(tc, 1) as usize);
                                if !(*tc as u32 != OP_KET) {
                                    break;
                                }
                            }

                            /* A DEFINE group must have only one branch. */

                            if *code.add(LINK_SIZE + 1) as u32 == OP_DEFINE {
                                if condcount > 1 {
                                    (*cb).erroroffset = offset;
                                    *errorcodeptr = ERR54;
                                    return 0;
                                }
                                *code.add(LINK_SIZE + 1) = OP_FALSE as u8;
                                bravalue = OP_DEFINE as i32; /* Suppress char handling */
                            }
                            /* A "normal" conditional group. */
                            else {
                                if condcount > 2 {
                                    (*cb).erroroffset = offset;
                                    *errorcodeptr = ERR27;
                                    return 0;
                                }
                                if condcount == 1 {
                                    subreqcuflags = REQ_NONE;
                                    subfirstcuflags = REQ_NONE;
                                } else if group_return > 0 {
                                    matched_char = TRUE;
                                }
                            }
                        }

                        /* In the pre-compile phase, update the length. */

                        if !lengthptr.is_null() {
                            if (OFLOW_MAX as usize).wrapping_sub(*lengthptr)
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
                            PUTINC!(code, 0, 1 + LINK_SIZE);
                            *code = OP_KET as u8;
                            code = code.add(1);
                            PUTINC!(code, 0, 1 + LINK_SIZE);
                            break 'body; /* No special character handling */
                        }

                        /* Otherwise update the main code pointer to the group end. */

                        code = tempcode;

                        /* For a DEFINE group, char settings are not relevant. */

                        if bravalue == OP_DEFINE as i32 {
                            break 'body;
                        }

                        /* Handle updating of the required and first code units. */

                        zeroreqcu = reqcu;
                        zeroreqcuflags = reqcuflags;
                        zerofirstcu = firstcu;
                        zerofirstcuflags = firstcuflags;
                        groupsetfirstcu = FALSE;

                        if bravalue >= OP_ONCE as i32
                        /* Not an assertion */
                        {
                            if firstcuflags == REQ_UNSET && subfirstcuflags != REQ_UNSET {
                                if subfirstcuflags < REQ_NONE {
                                    firstcu = subfirstcu;
                                    firstcuflags = subfirstcuflags;
                                    groupsetfirstcu = TRUE;
                                } else {
                                    firstcuflags = REQ_NONE;
                                }
                                zerofirstcuflags = REQ_NONE;
                            }
                            /* Convert the subpattern's firstcu into reqcu. */
                            else if subfirstcuflags < REQ_NONE && subreqcuflags >= REQ_NONE
                            {
                                subreqcu = subfirstcu;
                                subreqcuflags = subfirstcuflags | tempreqvary;
                            }

                            if subreqcuflags < REQ_NONE {
                                reqcu = subreqcu;
                                reqcuflags = subreqcuflags;
                            }
                        }
                        /* For a forward assertion, we take the reqcu, if set. */
                        else if (bravalue == OP_ASSERT as i32
                            || bravalue == OP_ASSERT_NA as i32)
                            && subreqcuflags < REQ_NONE
                            && subfirstcuflags < REQ_NONE
                        {
                            reqcu = subreqcu;
                            reqcuflags = subreqcuflags;
                        }

                        break 'body; /* End of nested group handling */
                    }

                    /* ============================================================ */
                    /* REPEAT: ... END_REPEAT: */
                    S_REPEAT => {
                        /* 'end_repeat is the END_REPEAT label (just past this block) */
                        'end_repeat: {
                            if previous_matched_char != 0 && repeat_min > 0 {
                                matched_char = TRUE;
                            }

                            /* Remember whether this is a variable length repeat. */

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

                            /* Save start of previous item. */

                            tempcode = previous;
                            op_previous = *previous;

                            /* Now handle repetition for the different types of item. */

                            let mut prop_type: i32 = 0;
                            let mut prop_value: i32 = 0;
                            let mut oldcode: *mut PCRE2_UCHAR = core::ptr::null_mut();
                            let mut len: i32 = 0;
                            let mut bralink: *mut PCRE2_UCHAR = core::ptr::null_mut();
                            let mut brazeroptr: *mut PCRE2_UCHAR = core::ptr::null_mut();
                            let mut rstate: u32 = R_SWITCH;

                            /* switch (op_previous); 'rsw exit == C break out of switch */
                            'rsw: loop {
                                match rstate {
                                    R_SWITCH => {
                                        match op_previous as u32 {
                                            /* Character or negated character match. */
                                            OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI => {
                                                if repeat_max == 1 && repeat_min == 1 {
                                                    /* goto END_REPEAT */
                                                    break 'end_repeat;
                                                }
                                                op_type = *chartypeoffset
                                                    .as_ptr()
                                                    .add((op_previous as u32 - OP_CHAR)
                                                        as usize);

                                                /* UTF characters > one code unit. */

                                                if utf != 0 && NOT_FIRSTCU!(*code.offset(-1))
                                                {
                                                    let mut lastchar: *mut PCRE2_UCHAR =
                                                        code.offset(-1);
                                                    BACKCHAR!(lastchar);
                                                    mclength =
                                                        code.offset_from(lastchar) as u32;
                                                    core::ptr::copy_nonoverlapping(
                                                        lastchar as *const u8,
                                                        mcbuffer.as_mut_ptr(),
                                                        mclength as usize,
                                                    );
                                                } else {
                                                    mcbuffer[0] = *code.offset(-1);
                                                    mclength = 1;
                                                    if op_previous as u32 <= OP_CHARI
                                                        && repeat_min > 1
                                                    {
                                                        reqcu = mcbuffer[0] as u32;
                                                        reqcuflags = (*cb).req_varyopt;
                                                        if op_previous as u32 == OP_CHARI {
                                                            reqcuflags |= REQ_CASELESS;
                                                        }
                                                    }
                                                }
                                                /* goto OUTPUT_SINGLE_REPEAT */
                                                rstate = R_OUTPUT_SINGLE_REPEAT;
                                                continue 'rsw;
                                            }

                                            /* Character class or back reference. */
                                            OP_XCLASS | OP_ECLASS | OP_CLASS | OP_NCLASS
                                            | OP_REF | OP_REFI | OP_DNREF | OP_DNREFI => {
                                                if repeat_max == 0 {
                                                    code = previous;
                                                    /* goto END_REPEAT */
                                                    break 'end_repeat;
                                                }
                                                if repeat_max == 1 && repeat_min == 1 {
                                                    /* goto END_REPEAT */
                                                    break 'end_repeat;
                                                }

                                                if repeat_min == 0
                                                    && repeat_max == REPEAT_UNLIMITED
                                                {
                                                    *code = (OP_CRSTAR + repeat_type) as u8;
                                                    code = code.add(1);
                                                } else if repeat_min == 1
                                                    && repeat_max == REPEAT_UNLIMITED
                                                {
                                                    *code = (OP_CRPLUS + repeat_type) as u8;
                                                    code = code.add(1);
                                                } else if repeat_min == 0 && repeat_max == 1
                                                {
                                                    *code =
                                                        (OP_CRQUERY + repeat_type) as u8;
                                                    code = code.add(1);
                                                } else {
                                                    *code =
                                                        (OP_CRRANGE + repeat_type) as u8;
                                                    code = code.add(1);
                                                    PUT2INC!(code, 0, repeat_min);
                                                    if repeat_max == REPEAT_UNLIMITED {
                                                        repeat_max = 0; /* 2-byte max */
                                                    }
                                                    PUT2INC!(code, 0, repeat_max);
                                                }
                                                break 'rsw; /* C break */
                                            }

                                            /* Repeated recursion. */
                                            OP_RECURSE => {
                                                if repeat_max == 1
                                                    && repeat_min == 1
                                                    && possessive_quantifier == 0
                                                {
                                                    /* goto END_REPEAT */
                                                    break 'end_repeat;
                                                }

                                                if repeat_min > 0
                                                    && (repeat_min != 1
                                                        || repeat_max != REPEAT_UNLIMITED)
                                                {
                                                    let mut replicate: i32 =
                                                        repeat_min as i32;

                                                    if repeat_min == repeat_max {
                                                        replicate -= 1;
                                                    }

                                                    if !lengthptr.is_null() {
                                                        let mut delta: PCRE2_SIZE = 0;
                                                        if _pcre2_ckd_smul_8(
                                                            &mut delta,
                                                            replicate,
                                                            length_prevgroup as i32,
                                                        ) != 0
                                                            || (OFLOW_MAX as usize)
                                                                .wrapping_sub(*lengthptr)
                                                                < delta
                                                        {
                                                            *errorcodeptr = ERR20;
                                                            return 0;
                                                        }
                                                        *lengthptr = (*lengthptr)
                                                            .wrapping_add(delta);
                                                    } else {
                                                        let mut i: i32 = 0;
                                                        while i < replicate {
                                                            core::ptr::copy_nonoverlapping(
                                                                previous as *const u8,
                                                                code,
                                                                length_prevgroup,
                                                            );
                                                            previous = code;
                                                            code = code.add(length_prevgroup);
                                                            i += 1;
                                                        }
                                                    }

                                                    if repeat_min == repeat_max {
                                                        break 'rsw; /* C break */
                                                    }
                                                    if repeat_max != REPEAT_UNLIMITED {
                                                        repeat_max -= repeat_min;
                                                    }
                                                    repeat_min = 0;
                                                }

                                                /* Wrap the recursion in OP_BRA brackets. */
                                                {
                                                    let length: PCRE2_SIZE =
                                                        if !lengthptr.is_null() {
                                                            1 + LINK_SIZE
                                                        } else {
                                                            length_prevgroup
                                                        };

                                                    core::ptr::copy(
                                                        previous as *const u8,
                                                        previous.add(1 + LINK_SIZE),
                                                        length,
                                                    );
                                                    *previous = OP_BRA as u8;
                                                    op_previous = OP_BRA as u8;
                                                    PUT!(
                                                        previous,
                                                        1,
                                                        1 + LINK_SIZE + length
                                                    );
                                                    *previous.add(1 + LINK_SIZE + length) =
                                                        OP_KET as u8;
                                                    PUT!(
                                                        previous,
                                                        2 + LINK_SIZE + length,
                                                        1 + LINK_SIZE + length
                                                    );
                                                }
                                                code = code.add(2 + 2 * LINK_SIZE);
                                                length_prevgroup = length_prevgroup
                                                    .wrapping_add(2 + 2 * LINK_SIZE);
                                                group_return = -1; /* May match empty */

                                                /* Now treat as a repeated OP_BRA:
                                                fall through */
                                                rstate = R_BRACKET;
                                                continue 'rsw;
                                            }

                                            /* Bracket group. */
                                            OP_ASSERT | OP_ASSERT_NOT | OP_ASSERT_NA
                                            | OP_ASSERTBACK | OP_ASSERTBACK_NOT
                                            | OP_ASSERTBACK_NA | OP_ASSERT_SCS | OP_ONCE
                                            | OP_SCRIPT_RUN | OP_BRA | OP_CBRA | OP_COND => {
                                                rstate = R_BRACKET;
                                                continue 'rsw;
                                            }

                                            /* Character type match (\d or similar). */
                                            _ => {
                                                rstate = R_DEFAULT;
                                                continue 'rsw;
                                            }
                                        }
                                    }

                                    /* ------------------------------------------------ */
                                    R_BRACKET => {
                                        len = code.offset_from(previous) as i32;
                                        bralink = core::ptr::null_mut();
                                        brazeroptr = core::ptr::null_mut();

                                        if repeat_max == 1
                                            && repeat_min == 1
                                            && possessive_quantifier == 0
                                        {
                                            /* goto END_REPEAT */
                                            break 'end_repeat;
                                        }

                                        /* Repeating a DEFINE group is pointless. */

                                        if op_previous as u32 == OP_COND
                                            && *previous.add(LINK_SIZE + 1) as u32
                                                == OP_FALSE
                                            && *previous.add(GET!(previous, 1) as usize)
                                                as u32
                                                != OP_ALT
                                        {
                                            /* goto END_REPEAT */
                                            break 'end_repeat;
                                        }

                                        if (op_previous as u32) < OP_ONCE
                                        /* Assertion */
                                        {
                                            if repeat_max == REPEAT_UNLIMITED {
                                                repeat_max = repeat_min + 1;
                                            }
                                        }

                                        /* The case of a zero minimum is special. */

                                        if repeat_min == 0 {
                                            if repeat_max <= 1
                                                || repeat_max == REPEAT_UNLIMITED
                                            {
                                                core::ptr::copy(
                                                    previous as *const u8,
                                                    previous.add(1),
                                                    len as usize,
                                                );
                                                code = code.add(1);
                                                if repeat_max == 0 {
                                                    *previous = OP_SKIPZERO as u8;
                                                    previous = previous.add(1);
                                                    /* goto END_REPEAT */
                                                    break 'end_repeat;
                                                }
                                                brazeroptr = previous; /* For possessive */
                                                *previous =
                                                    (OP_BRAZERO + repeat_type) as u8;
                                                previous = previous.add(1);
                                            }
                                            /* Maximum greater than 1 and limited. */
                                            else {
                                                let linkoffset: i32;
                                                core::ptr::copy(
                                                    previous as *const u8,
                                                    previous.add(2 + LINK_SIZE),
                                                    len as usize,
                                                );
                                                code = code.add(2 + LINK_SIZE);
                                                *previous =
                                                    (OP_BRAZERO + repeat_type) as u8;
                                                previous = previous.add(1);
                                                *previous = OP_BRA as u8;
                                                previous = previous.add(1);

                                                linkoffset = if bralink.is_null() {
                                                    0
                                                } else {
                                                    previous.offset_from(bralink) as i32
                                                };
                                                bralink = previous;
                                                PUTINC!(previous, 0, linkoffset);
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
                                                    if _pcre2_ckd_smul_8(
                                                        &mut delta,
                                                        (repeat_min - 1) as i32,
                                                        length_prevgroup as i32,
                                                    ) != 0
                                                        || (OFLOW_MAX as usize)
                                                            .wrapping_sub(*lengthptr)
                                                            < delta
                                                    {
                                                        *errorcodeptr = ERR20;
                                                        return 0;
                                                    }
                                                    *lengthptr =
                                                        (*lengthptr).wrapping_add(delta);
                                                }
                                                /* Compiling for real. */
                                                else {
                                                    if groupsetfirstcu != 0
                                                        && reqcuflags >= REQ_NONE
                                                    {
                                                        reqcu = firstcu;
                                                        reqcuflags = firstcuflags;
                                                    }
                                                    let mut i: u32 = 1;
                                                    while i < repeat_min {
                                                        core::ptr::copy_nonoverlapping(
                                                            previous as *const u8,
                                                            code,
                                                            len as usize,
                                                        );
                                                        code = code.add(len as usize);
                                                        i += 1;
                                                    }
                                                }
                                            }

                                            if repeat_max != REPEAT_UNLIMITED {
                                                repeat_max -= repeat_min;
                                            }
                                        }

                                        /* Code common to both zero and non-zero minimum. */

                                        if repeat_max != REPEAT_UNLIMITED {
                                            if !lengthptr.is_null() && repeat_max > 0 {
                                                let mut delta: PCRE2_SIZE = 0;
                                                if _pcre2_ckd_smul_8(
                                                    &mut delta,
                                                    repeat_max as i32,
                                                    (length_prevgroup as i32)
                                                        + 1
                                                        + 2
                                                        + 2 * LINK_SIZE as i32,
                                                ) != 0
                                                    || ((OFLOW_MAX as usize)
                                                        + (2 + 2 * LINK_SIZE))
                                                        .wrapping_sub(*lengthptr)
                                                        < delta
                                                {
                                                    *errorcodeptr = ERR20;
                                                    return 0;
                                                }
                                                /* Last one doesn't nest */
                                                delta =
                                                    delta.wrapping_sub(2 + 2 * LINK_SIZE);
                                                *lengthptr =
                                                    (*lengthptr).wrapping_add(delta);
                                            }
                                            /* This is compiling for real */
                                            else {
                                                let mut i: u32 = repeat_max;
                                                while i >= 1 {
                                                    *code =
                                                        (OP_BRAZERO + repeat_type) as u8;
                                                    code = code.add(1);

                                                    /* All but the final copy start a new
                                                    nesting. */

                                                    if i != 1 {
                                                        let linkoffset: i32;
                                                        *code = OP_BRA as u8;
                                                        code = code.add(1);
                                                        linkoffset = if bralink.is_null() {
                                                            0
                                                        } else {
                                                            code.offset_from(bralink) as i32
                                                        };
                                                        bralink = code;
                                                        PUTINC!(code, 0, linkoffset);
                                                    }

                                                    core::ptr::copy_nonoverlapping(
                                                        previous as *const u8,
                                                        code,
                                                        len as usize,
                                                    );
                                                    code = code.add(len as usize);
                                                    i -= 1;
                                                }
                                            }

                                            /* Now chain through the pending brackets. */

                                            while !bralink.is_null() {
                                                let oldlinkoffset: i32;
                                                let linkoffset: i32 =
                                                    (code.offset_from(bralink) + 1) as i32;
                                                let bra: *mut PCRE2_UCHAR =
                                                    code.offset(-(linkoffset as isize));
                                                oldlinkoffset = GET!(bra, 1) as i32;
                                                bralink = if oldlinkoffset == 0 {
                                                    core::ptr::null_mut()
                                                } else {
                                                    bralink
                                                        .offset(-(oldlinkoffset as isize))
                                                };
                                                *code = OP_KET as u8;
                                                code = code.add(1);
                                                PUTINC!(code, 0, linkoffset);
                                                PUT!(bra, 1, linkoffset);
                                            }
                                        }
                                        /* Maximum is unlimited: set a repeater in the
                                        final copy. */
                                        else {
                                            let ketcode: *mut PCRE2_UCHAR =
                                                code.offset(-1).offset(-(LINK_SIZE as isize));
                                            let bracode: *mut PCRE2_UCHAR = ketcode
                                                .offset(-(GET!(ketcode, 1) as isize));

                                            /* Convert possessive ONCE to non-capturing */

                                            if *bracode as u32 == OP_ONCE
                                                && possessive_quantifier != 0
                                            {
                                                *bracode = OP_BRA as u8;
                                            }

                                            /* For ONCE and SCRIPT_RUN, just set the KET. */

                                            if *bracode as u32 == OP_ONCE
                                                || *bracode as u32 == OP_SCRIPT_RUN
                                            {
                                                *ketcode =
                                                    (OP_KETRMAX + repeat_type) as u8;
                                            }
                                            /* Other brackets. */
                                            else {
                                                if lengthptr.is_null() {
                                                    if group_return < 0 {
                                                        *bracode = (*bracode as u32
                                                            + (OP_SBRA - OP_BRA))
                                                            as u8;
                                                    }
                                                    if *bracode as u32 == OP_COND
                                                        && *bracode.add(
                                                            GET!(bracode, 1) as usize,
                                                        ) as u32
                                                            != OP_ALT
                                                    {
                                                        *bracode = OP_SCOND as u8;
                                                    }
                                                }

                                                /* Handle possessive quantifiers. */

                                                if possessive_quantifier != 0 {
                                                    /* For COND brackets, wrap the whole
                                                    thing. */

                                                    if *bracode as u32 == OP_COND
                                                        || *bracode as u32 == OP_SCOND
                                                    {
                                                        let mut nlen: i32 = code
                                                            .offset_from(bracode)
                                                            as i32;
                                                        core::ptr::copy(
                                                            bracode as *const u8,
                                                            bracode.add(1 + LINK_SIZE),
                                                            nlen as usize,
                                                        );
                                                        code = code.add(1 + LINK_SIZE);
                                                        nlen += (1 + LINK_SIZE) as i32;
                                                        *bracode = (if *bracode as u32
                                                            == OP_COND
                                                        {
                                                            OP_BRAPOS
                                                        } else {
                                                            OP_SBRAPOS
                                                        }) as u8;
                                                        *code = OP_KETRPOS as u8;
                                                        code = code.add(1);
                                                        PUTINC!(code, 0, nlen);
                                                        PUT!(bracode, 1, nlen);
                                                    }
                                                    /* For non-COND brackets. */
                                                    else {
                                                        /* Switch to xxxPOS opcodes */
                                                        *bracode =
                                                            (*bracode as u32 + 1) as u8;
                                                        *ketcode = OP_KETRPOS as u8;
                                                    }

                                                    if !brazeroptr.is_null() {
                                                        *brazeroptr = OP_BRAPOSZERO as u8;
                                                    }
                                                    if repeat_min < 2 {
                                                        possessive_quantifier = FALSE;
                                                    }
                                                }
                                                /* Non-possessive quantifier */
                                                else {
                                                    *ketcode =
                                                        (OP_KETRMAX + repeat_type) as u8;
                                                }
                                            }
                                        }
                                        break 'rsw; /* C break */
                                    }

                                    /* ------------------------------------------------ */
                                    /* default: character type match */
                                    R_DEFAULT => {
                                        if op_previous as u32 >= OP_EODN
                                            || op_previous as u32 <= OP_WORD_BOUNDARY
                                        {
                                            /* Not a character type - internal error */
                                            *errorcodeptr = ERR10;
                                            return 0;
                                        }

                                        if repeat_max == 1 && repeat_min == 1 {
                                            /* goto END_REPEAT */
                                            break 'end_repeat;
                                        }

                                        op_type = OP_TYPESTAR - OP_STAR; /* Type opcodes */
                                        mclength = 0; /* Not a character */

                                        if op_previous as u32 == OP_PROP
                                            || op_previous as u32 == OP_NOTPROP
                                        {
                                            prop_type = *previous.add(1) as i32;
                                            prop_value = *previous.add(2) as i32;
                                            rstate = R_TAIL;
                                            continue 'rsw;
                                        } else {
                                            /* Falls into OUTPUT_SINGLE_REPEAT */
                                            rstate = R_OUTPUT_SINGLE_REPEAT;
                                            continue 'rsw;
                                        }
                                    }

                                    /* ------------------------------------------------ */
                                    /* OUTPUT_SINGLE_REPEAT: */
                                    R_OUTPUT_SINGLE_REPEAT => {
                                        prop_type = -1;
                                        prop_value = -1;
                                        rstate = R_TAIL;
                                        continue 'rsw;
                                    }

                                    /* ------------------------------------------------ */
                                    R_TAIL => {
                                        oldcode = code; /* Save where we were */
                                        code = previous; /* Overwrite previous item */

                                        /* If the maximum is zero, omit the item. */

                                        if repeat_max == 0 {
                                            /* goto END_REPEAT */
                                            break 'end_repeat;
                                        }

                                        /* Combine the op_type with the repeat_type */

                                        repeat_type += op_type;

                                        /* A minimum of zero. */

                                        if repeat_min == 0 {
                                            if repeat_max == REPEAT_UNLIMITED {
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
                                        /* A repeat minimum of 1. */
                                        else if repeat_min == 1 {
                                            if repeat_max == REPEAT_UNLIMITED {
                                                *code = (OP_PLUS + repeat_type) as u8;
                                                code = code.add(1);
                                            } else {
                                                code = oldcode; /* Leave previous item */
                                                if repeat_max == 1 {
                                                    /* goto END_REPEAT */
                                                    break 'end_repeat;
                                                }
                                                *code = (OP_UPTO + repeat_type) as u8;
                                                code = code.add(1);
                                                PUT2INC!(code, 0, repeat_max - 1);
                                            }
                                        }
                                        /* The general case. */
                                        else {
                                            /* NB EXACT doesn't have repeat_type */
                                            *code = (OP_EXACT + op_type) as u8;
                                            code = code.add(1);
                                            PUT2INC!(code, 0, repeat_min);

                                            if repeat_max != repeat_min {
                                                if mclength > 0 {
                                                    core::ptr::copy_nonoverlapping(
                                                        mcbuffer.as_ptr(),
                                                        code,
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

                                                if repeat_max == REPEAT_UNLIMITED {
                                                    *code = (OP_STAR + repeat_type) as u8;
                                                    code = code.add(1);
                                                } else {
                                                    repeat_max -= repeat_min;
                                                    if repeat_max == 1 {
                                                        *code =
                                                            (OP_QUERY + repeat_type) as u8;
                                                        code = code.add(1);
                                                    } else {
                                                        *code =
                                                            (OP_UPTO + repeat_type) as u8;
                                                        code = code.add(1);
                                                        PUT2INC!(code, 0, repeat_max);
                                                    }
                                                }
                                            }
                                        }

                                        /* Fill in the character or character type. */

                                        if mclength > 0 {
                                            core::ptr::copy_nonoverlapping(
                                                mcbuffer.as_ptr(),
                                                code,
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
                                        break 'rsw; /* C break */
                                    }

                                    _ => {
                                        break 'rsw;
                                    }
                                }
                            } /* End of switch on different op_previous values */

                            /* If the character following a repeat is '+',
                            possessive_quantifier is TRUE. */

                            if possessive_quantifier != 0 {
                                let mut plen: i32;

                                match *tempcode as u32 {
                                    OP_TYPEEXACT => {
                                        tempcode = tempcode.add(
                                            OPLEN!(*tempcode) as usize
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

                                    /* CHAR opcodes are used for exacts of count 1. */
                                    OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_EXACT
                                    | OP_EXACTI | OP_NOTEXACT | OP_NOTEXACTI => {
                                        tempcode =
                                            tempcode.add(OPLEN!(*tempcode) as usize);
                                        if utf != 0
                                            && HAS_EXTRALEN!(*tempcode.offset(-1))
                                        {
                                            tempcode = tempcode.add(GET_EXTRALEN!(
                                                *tempcode.offset(-1)
                                            ) as usize);
                                        }
                                    }

                                    /* For class opcodes, the repeat is at the end. */
                                    OP_CLASS | OP_NCLASS => {
                                        tempcode = tempcode.add(1 + 32);
                                    }

                                    OP_XCLASS | OP_ECLASS => {
                                        tempcode =
                                            tempcode.add(GET!(tempcode, 1) as usize);
                                    }

                                    OP_REF | OP_REFI | OP_DNREF | OP_DNREFI => {
                                        tempcode =
                                            tempcode.add(OPLEN!(*tempcode) as usize);
                                    }

                                    _ => {}
                                }

                                plen = code.offset_from(tempcode) as i32;
                                if plen > 0 {
                                    let repcode: u32 = *tempcode as u32;

                                    /* Table for possessifying opcodes. */

                                    if repcode < OP_CALLOUT
                                        && *opcode_possessify
                                            .as_ptr()
                                            .add(repcode as usize)
                                            > 0
                                    {
                                        *tempcode = *opcode_possessify
                                            .as_ptr()
                                            .add(repcode as usize);
                                    }
                                    /* Wrap the item in ONCE brackets. */
                                    else {
                                        core::ptr::copy(
                                            tempcode as *const u8,
                                            tempcode.add(1 + LINK_SIZE),
                                            plen as usize,
                                        );
                                        code = code.add(1 + LINK_SIZE);
                                        plen += (1 + LINK_SIZE) as i32;
                                        *tempcode.add(0) = OP_ONCE as u8;
                                        *code = OP_KET as u8;
                                        code = code.add(1);
                                        PUTINC!(code, 0, plen);
                                        PUT!(tempcode, 1, plen);
                                    }
                                }
                            }
                        } /* END_REPEAT: */

                        /* We set the "follows varying string" flag for subsequently
                        encountered reqcus. */

                        (*cb).req_varyopt |= reqvary;
                        break 'body;
                    }


                    /* ============================================================ */
                    /* HANDLE_SINGLE_REFERENCE: */
                    S_HANDLE_SINGLE_REFERENCE => {
                        if firstcuflags == REQ_UNSET {
                            firstcuflags = REQ_NONE;
                            zerofirstcuflags = REQ_NONE;
                        }
                        *code = (if (options & PCRE2_CASELESS) != 0 {
                            OP_REFI
                        } else {
                            OP_REF
                        }) as u8;
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

                        /* Update the map of back references. */

                        (*cb).backref_map |= if meta_arg < 32 {
                            1u32 << meta_arg
                        } else {
                            1
                        };
                        if meta_arg > (*cb).top_backref {
                            (*cb).top_backref = meta_arg;
                        }
                        break 'body;
                    }

                    /* ============================================================ */
                    /* HANDLE_NUMERICAL_RECURSION: */
                    S_HANDLE_NUMERICAL_RECURSION => {
                        *code = OP_RECURSE as u8;
                        PUT!(code, 1, meta_arg);
                        code = code.add(1 + LINK_SIZE);
                        /* Repeat processing requires this information. */
                        length_prevgroup = 1 + LINK_SIZE;

                        if META_CODE!(*pptr.add(1)) == META_OFFSET
                            || META_CODE!(*pptr.add(1)) == META_CAPTURE_NAME
                            || META_CODE!(*pptr.add(1)) == META_CAPTURE_NUMBER
                        {
                            let mut args: *mut recurse_arguments;

                            if !lengthptr.is_null() {
                                if _pcre2_compile_parse_recurse_args8(
                                    pptr,
                                    offset,
                                    errorcodeptr,
                                    cb,
                                ) == 0
                                {
                                    return 0;
                                }

                                args = (*cb).last_data as *mut recurse_arguments;
                                length_prevgroup = length_prevgroup
                                    .wrapping_add((*args).size * (1 + IMM2_SIZE));
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
                                    *code.add(0) = OP_CREF as u8;
                                    PUT2!(code, 1, *current);
                                    code = code.add(1 + IMM2_SIZE);
                                    current = current.add(1);
                                    if !(current < end) {
                                        break;
                                    }
                                }

                                length_prevgroup = length_prevgroup
                                    .wrapping_add((*args).size * (1 + IMM2_SIZE));
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
                        break 'body;
                    }

                    /* ============================================================ */
                    /* NORMAL_CHAR: */
                    S_NORMAL_CHAR => {
                        meta = *pptr; /* Get the full 32 bits */
                        /* Falls into NORMAL_CHAR_SET */
                        state = S_NORMAL_CHAR_SET;
                        continue 'sm;
                    }

                    /* ============================================================ */
                    /* NORMAL_CHAR_SET: Character is already in meta */
                    S_NORMAL_CHAR_SET => {
                        matched_char = TRUE;

                        /* For caseless UTF or UCP mode, check for multiple other cases. */

                        if (utf != 0 || ucp != 0) && (options & PCRE2_CASELESS) != 0 {
                            let mut caseset: u32;

                            if (xoptions
                                & (PCRE2_EXTRA_TURKISH_CASING
                                    | PCRE2_EXTRA_CASELESS_RESTRICT))
                                == PCRE2_EXTRA_TURKISH_CASING
                                && UCD_ANY_I!(meta)
                            {
                                caseset = _pcre2_ucd_turkish_dotted_i_caseset_8
                                    + (if UCD_DOTTED_I!(meta) { 0 } else { 3 });
                            } else {
                                caseset = UCD_CASESET!(meta);
                                if caseset != 0
                                    && (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                                    && *_pcre2_ucd_caseless_sets_8
                                        .as_ptr()
                                        .add(caseset as usize)
                                        < 128
                                {
                                    caseset = 0; /* Ignore the restricted set. */
                                }
                            }

                            if caseset != 0 {
                                *code = OP_PROP as u8;
                                code = code.add(1);
                                *code = PT_CLIST as u8;
                                code = code.add(1);
                                *code = caseset as u8;
                                code = code.add(1);
                                if firstcuflags == REQ_UNSET {
                                    firstcuflags = REQ_NONE;
                                    zerofirstcuflags = REQ_NONE;
                                }
                                break 'body; /* End handling this meta item */
                            }
                        }

                        /* Falls into CLASS_CASELESS_CHAR */
                        state = S_CLASS_CASELESS_CHAR;
                        continue 'sm;
                    }

                    /* ============================================================ */
                    /* CLASS_CASELESS_CHAR: */
                    S_CLASS_CASELESS_CHAR => {
                        /* Get the character's code units into mcbuffer. */

                        if utf != 0 {
                            mclength = _pcre2_ord2utf_8(meta, mcbuffer.as_mut_ptr());
                        } else {
                            mclength = 1;
                            mcbuffer[0] = meta as u8;
                        }

                        /* Generate the appropriate code */

                        *code = (if (options & PCRE2_CASELESS) != 0 {
                            OP_CHARI
                        } else {
                            OP_CHAR
                        }) as u8;
                        code = code.add(1);
                        core::ptr::copy_nonoverlapping(
                            mcbuffer.as_ptr(),
                            code,
                            mclength as usize,
                        );
                        code = code.add(mclength as usize);

                        /* Remember if \r or \n were seen */

                        if mcbuffer[0] == 0x0d /* CHAR_CR */ || mcbuffer[0] == 0x0a
                        /* CHAR_NL */
                        {
                            (*cb).external_flags |= PCRE2_HASCRORLF;
                        }

                        /* Set the first and required code units appropriately. */

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
                        }
                        /* firstcu was previously set. */
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

                        break 'body; /* End literal character handling */
                    }

                    _ => {
                        break 'body;
                    }
                }
            }
        }

        pptr = pptr.add(1); /* for (;; pptr++) */
    }
}
