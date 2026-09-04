//! Translation of the branch-compiler part of `pcre2_compile.c`:
//! `first_significant_code` and `compile_branch` (C lines 5967–8573).
//!
//! Both functions are `static` in the C source. They are called from
//! `compile_aux.rs` (which holds `compile_regex`, mutually recursive with
//! `compile_branch`) and `compile.rs`, so they are exposed here as
//! `pub(crate) unsafe fn` — deliberately *not* `#[no_mangle]`/`extern "C"`.

use core::ffi::c_int;
use core::ptr;

use crate::compile_h::*;
use crate::compile_local::*;
use crate::compile_tables::{CHARTYPEOFFSET, OPCODE_POSSESSIFY, VERBOPS};
use crate::consts::*;
use crate::internal::*;
// Resolve glob-import ambiguities: `FALSE`/`TRUE` exist in both `consts`
// (as `i64`) and `internal` (as the C `BOOL` = i32). Use the `internal` ones.
use crate::internal::{FALSE, TRUE};
use crate::tables;

// ---------------------------------------------------------------------------
// Character constants used below that are not present in `consts.rs`.
// These are the ASCII (non-EBCDIC) values, matching the C build.
// ---------------------------------------------------------------------------

const CHAR_CR: u32 = 13;
const CHAR_NL_U: u32 = 10;
const CHAR_NUL: u32 = 0;
const CHAR_0_U: u32 = 48;
const CHAR_LEFT_CURLY_BRACKET: u32 = 123;
const CHAR_RIGHT_CURLY_BRACKET: u32 = 125;

// ---------------------------------------------------------------------------
// `UCD_ANY_I` / `UCD_DOTTED_I` (private helpers, mirroring compile_class.rs).
// ---------------------------------------------------------------------------

#[inline(always)]
fn UCD_ANY_I(ch: u32) -> bool {
    (ch | 0x20u32) == 0x69u32 || (ch | 1u32) == 0x0131u32
}

#[inline(always)]
fn UCD_DOTTED_I(ch: u32) -> bool {
    ch == 0x69u32 || ch == 0x0130u32
}

/// `first_significant_code(code, skipassert)` — return a pointer to the first
/// significant opcode in a compiled group, skipping over assertions and other
/// zero-width items (and, when `skipassert` is set, word-boundary assertions).
pub(crate) unsafe fn first_significant_code(
    mut code: PCRE2_SPTR,
    skipassert: BOOL,
) -> *const PCRE2_UCHAR {
    unsafe {
        loop {
            match *code as u32 {
                OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT | OP_ASSERTBACK_NA => {
                    if skipassert == FALSE {
                        return code;
                    }
                    loop {
                        code = code.add(GET(code, 1) as usize);
                        if *code as u32 != OP_ALT {
                            break;
                        }
                    }
                    code = code.add(tables::_pcre2_OP_lengths_8[*code as usize] as usize);
                }

                OP_WORD_BOUNDARY | OP_NOT_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY
                | OP_NOT_UCP_WORD_BOUNDARY => {
                    if skipassert == FALSE {
                        return code;
                    }
                    // Fall through to the zero-width group.
                    code = code.add(tables::_pcre2_OP_lengths_8[*code as usize] as usize);
                }

                OP_CALLOUT | OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF | OP_FALSE | OP_TRUE => {
                    code = code.add(tables::_pcre2_OP_lengths_8[*code as usize] as usize);
                }

                OP_CALLOUT_STR => {
                    code = code.add(GET(code, 1 + 2 * LINK_SIZE_U) as usize);
                }

                OP_SKIPZERO => {
                    code = code.add(2 + GET(code, 2) as usize + LINK_SIZE_U);
                }

                OP_COND | OP_SCOND => {
                    if *code.add(1 + LINK_SIZE_U) as u32 != OP_FALSE   /* Not DEFINE */
                        || *code.add(GET(code, 1) as usize) as u32 != OP_KET
                    /* More than one branch */
                    {
                        return code;
                    }
                    code = code.add(GET(code, 1) as usize + 1 + LINK_SIZE_U);
                }

                OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                    code = code.add(
                        *code.add(1) as usize
                            + tables::_pcre2_OP_lengths_8[*code as usize] as usize,
                    );
                }

                _ => return code,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// compile_branch
// ---------------------------------------------------------------------------

/// Goto targets within `compile_branch`'s per-item body. The C source uses
/// `goto` to jump between shared code blocks; we model that with a dispatch
/// enum and an inner loop.
#[derive(Clone, Copy, PartialEq)]
enum Dispatch {
    MainSwitch,
    ClassEndProcessing,
    VerbArg,
    GroupProcessNoteEmpty,
    GroupProcess,
    Repeat,
    OutputSingleRepeat,
    PossessiveHandling,
    EndRepeat,
    HandleSingleReference,
    HandleNumericalRecursion,
    NormalChar,
    NormalCharSet,
    ClassCaselessChar,
}

/// `compile_branch(...)` — compile one branch of a pattern.
///
/// Returns `0` on error (with `*errorcodeptr` set), `+1` if the branch must
/// match at least one character, or `-1` if it may match an empty string.
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
        let mut repeat_max: u32 = 0; // To please picky compilers
        let mut greedy_default: u32;
        let mut greedy_non_default: u32;
        let mut repeat_type: u32 = 0;
        let mut op_type: u32 = 0;
        let mut options: u32 = *optionsptr; // May change dynamically
        let mut xoptions: u32 = *xoptionsptr; // May change dynamically
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
        // Some opcodes depend on the previous value of offset.
        let mut offset: PCRE2_SIZE = 0;
        let mut length_prevgroup: PCRE2_SIZE = 0;
        let mut code: *mut PCRE2_UCHAR = *codeptr;
        let mut last_code: *mut PCRE2_UCHAR = code;
        let orig_code: *mut PCRE2_UCHAR = code;
        let mut tempcode: *mut PCRE2_UCHAR = ptr::null_mut();
        let mut previous: *mut PCRE2_UCHAR = ptr::null_mut();
        let mut op_previous: PCRE2_UCHAR = 0;
        let mut groupsetfirstcu: BOOL = FALSE;
        let mut had_accept: BOOL = FALSE;
        let mut matched_char: BOOL = FALSE;
        let mut previous_matched_char: BOOL;
        let mut reset_caseful: BOOL = FALSE;

        // Fish out the UTF/UCP settings once.
        let utf: BOOL = ((options & PCRE2_UTF as u32) != 0) as BOOL;
        let ucp: BOOL = ((options & PCRE2_UCP as u32) != 0) as BOOL;
        let utf_b: bool = utf != FALSE;

        // Set up default/non-default greediness.
        greedy_default = ((options & PCRE2_UNGREEDY as u32) != 0) as u32;
        greedy_non_default = greedy_default ^ 1;

        // Initialize no first unit, no required unit.
        firstcu = 0;
        reqcu = 0;
        zerofirstcu = 0;
        zeroreqcu = 0;
        firstcuflags = REQ_UNSET;
        reqcuflags = REQ_UNSET;
        zerofirstcuflags = REQ_UNSET;
        zeroreqcuflags = REQ_UNSET;

        req_caseopt = if (options & PCRE2_CASELESS as u32) != 0 {
            REQ_CASELESS
        } else {
            0
        };

        // Switch on next META item until the end of the branch.
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
            // Extra locals shared between Repeat and OutputSingleRepeat.
            let mut prop_type: c_int = 0;
            let mut prop_value: c_int = 0;
            let mut oldcode: *mut PCRE2_UCHAR = ptr::null_mut();

            // Get next META item and its potential argument.
            meta = META_CODE(*pptr);
            meta_arg = META_DATA(*pptr);

            // Pre-compile phase length accumulation.
            if !lengthptr.is_null() {
                if code >= (*cb).start_workspace.add((*cb).workspace_size) {
                    *errorcodeptr = ERR52;
                    (*cb).erroroffset = 0;
                    return 0;
                }

                if code
                    > (*cb)
                        .start_workspace
                        .add((*cb).workspace_size - WORK_SIZE_SAFETY_MARGIN)
                {
                    *errorcodeptr = ERR86;
                    (*cb).erroroffset = 0;
                    return 0;
                }

                if code < last_code {
                    code = last_code;
                }

                if meta < META_ASTERISK as u32 || meta > META_MINMAX_QUERY as u32 {
                    if (OFLOW_MAX as i64) - (*lengthptr as i64)
                        < (code.offset_from(orig_code) as i64)
                    {
                        *errorcodeptr = ERR20;
                        (*cb).erroroffset = 0;
                        return 0;
                    }
                    *lengthptr += code.offset_from(orig_code) as PCRE2_SIZE;
                    if *lengthptr > MAX_PATTERN_SIZE_U {
                        *errorcodeptr = ERR20;
                        (*cb).erroroffset = 0;
                        return 0;
                    }
                    code = orig_code;
                }

                last_code = code;
            }

            // If not a quantifier, remember where this item starts.
            if meta < META_ASTERISK as u32 || meta > META_MINMAX_QUERY as u32 {
                previous = code;
                if matched_char != FALSE && had_accept == FALSE {
                    okreturn = 1;
                }
            }

            previous_matched_char = matched_char;
            matched_char = FALSE;
            note_group_empty = FALSE;
            skipunits = 0;

            let mut gto = Dispatch::MainSwitch;
            'item: loop {
                match gto {
                    Dispatch::MainSwitch => {
                        match meta as i64 {
                            // The branch terminates at pattern end or | or )
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
                                if (options & PCRE2_MULTILINE as u32) != 0 {
                                    if firstcuflags == REQ_UNSET {
                                        zerofirstcuflags = REQ_NONE;
                                        firstcuflags = REQ_NONE;
                                    }
                                    *code = OP_CIRCM as u8;
                                    code = code.add(1);
                                } else {
                                    *code = OP_CIRC as u8;
                                    code = code.add(1);
                                }
                                break 'item;
                            }

                            META_DOLLAR => {
                                *code = if (options & PCRE2_MULTILINE as u32) != 0 {
                                    OP_DOLLM as u8
                                } else {
                                    OP_DOLL as u8
                                };
                                code = code.add(1);
                                break 'item;
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
                                *code = if (options & PCRE2_DOTALL as u32) != 0 {
                                    OP_ALLANY as u8
                                } else {
                                    OP_ANY as u8
                                };
                                code = code.add(1);
                                break 'item;
                            }

                            META_CLASS_EMPTY | META_CLASS_EMPTY_NOT => {
                                matched_char = TRUE;
                                if meta as i64 == META_CLASS_EMPTY_NOT {
                                    *code = OP_ALLANY as u8;
                                    code = code.add(1);
                                } else {
                                    *code = OP_CLASS as u8;
                                    code = code.add(1);
                                    ptr::write_bytes(code, 0, 32);
                                    code = code.add(32);
                                }

                                if firstcuflags == REQ_UNSET {
                                    firstcuflags = REQ_NONE;
                                }
                                zerofirstcu = firstcu;
                                zerofirstcuflags = firstcuflags;
                                break 'item;
                            }

                            META_CLASS_NOT | META_CLASS => {
                                matched_char = TRUE;

                                if (*pptr & CLASS_IS_ECLASS as u32) != 0 {
                                    if crate::compile_class::_pcre2_compile_class_nested_8(
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
                                    gto = Dispatch::ClassEndProcessing;
                                    continue 'item;
                                }

                                if *pptr.add(1) < META_END as u32
                                    && *pptr.add(2) as i64 == META_CLASS_END
                                {
                                    let c = *pptr.add(1);

                                    pptr = pptr.add(2);
                                    if meta as i64 == META_CLASS {
                                        meta = c;
                                        gto = Dispatch::NormalCharSet;
                                        continue 'item;
                                    }

                                    zeroreqcu = reqcu;
                                    zeroreqcuflags = reqcuflags;
                                    if firstcuflags == REQ_UNSET {
                                        firstcuflags = REQ_NONE;
                                    }
                                    zerofirstcu = firstcu;
                                    zerofirstcuflags = firstcuflags;

                                    if (utf_b || ucp != FALSE)
                                        && (options & PCRE2_CASELESS as u32) != 0
                                    {
                                        let mut caseset: u32;

                                        if (xoptions
                                            & (PCRE2_EXTRA_TURKISH_CASING
                                                | PCRE2_EXTRA_CASELESS_RESTRICT)
                                                as u32)
                                            == PCRE2_EXTRA_TURKISH_CASING as u32
                                            && UCD_ANY_I(c)
                                        {
                                            caseset = tables::_pcre2_ucd_turkish_dotted_i_caseset_8
                                                + (if UCD_DOTTED_I(c) { 0 } else { 3 });
                                        } else {
                                            caseset = UCD_CASESET(c);
                                            if caseset != 0
                                                && (xoptions
                                                    & PCRE2_EXTRA_CASELESS_RESTRICT as u32)
                                                    != 0
                                                && tables::_pcre2_ucd_caseless_sets_8
                                                    [caseset as usize]
                                                    < 128
                                            {
                                                caseset = 0;
                                            }
                                        }

                                        if caseset != 0 {
                                            *code = OP_NOTPROP as u8;
                                            code = code.add(1);
                                            *code = PT_CLIST as u8;
                                            code = code.add(1);
                                            *code = caseset as u8;
                                            code = code.add(1);
                                            break 'item;
                                        }
                                    }

                                    *code = if (options & PCRE2_CASELESS as u32) != 0 {
                                        OP_NOTI as u8
                                    } else {
                                        OP_NOT as u8
                                    };
                                    code = code.add(1);
                                    code = code.add(PUTCHAR(c, code, utf_b) as usize);
                                    break 'item;
                                }

                                if meta as i64 == META_CLASS
                                    && *pptr.add(1) < META_END as u32
                                    && *pptr.add(2) < META_END as u32
                                    && *pptr.add(3) as i64 == META_CLASS_END
                                {
                                    let c = *pptr.add(1);

                                    if (UCD_CASESET(c) == 0
                                        || ((xoptions & PCRE2_EXTRA_CASELESS_RESTRICT as u32) != 0
                                            && c < 128
                                            && *pptr.add(2) < 128))
                                        && !((xoptions
                                            & (PCRE2_EXTRA_TURKISH_CASING
                                                | PCRE2_EXTRA_CASELESS_RESTRICT)
                                                as u32)
                                            == PCRE2_EXTRA_TURKISH_CASING as u32
                                            && UCD_ANY_I(c))
                                    {
                                        let d: u32;

                                        if (utf_b || ucp != FALSE) && c > 127 {
                                            d = UCD_OTHERCASE(c);
                                        } else {
                                            d = TABLE_GET(c, (*cb).fcc, c);
                                        }

                                        if c != d && *pptr.add(2) == d {
                                            pptr = pptr.add(3);
                                            meta = c;
                                            if (options & PCRE2_CASELESS as u32) == 0 {
                                                reset_caseful = TRUE;
                                                options |= PCRE2_CASELESS as u32;
                                                req_caseopt = REQ_CASELESS;
                                            }
                                            gto = Dispatch::ClassCaselessChar;
                                            continue 'item;
                                        }
                                    }
                                }

                                pptr = crate::compile_class::_pcre2_compile_class_not_nested_8(
                                    options,
                                    xoptions,
                                    pptr.add(1),
                                    &mut code,
                                    (meta as i64 == META_CLASS_NOT) as BOOL,
                                    ptr::null_mut(),
                                    errorcodeptr,
                                    cb,
                                    lengthptr,
                                );
                                if pptr.is_null() {
                                    return 0;
                                }

                                gto = Dispatch::ClassEndProcessing;
                                continue 'item;
                            }

                            META_ACCEPT => {
                                (*cb).had_accept = TRUE;
                                had_accept = TRUE;
                                oc = open_caps;
                                while !oc.is_null()
                                    && (*oc).assert_depth >= (*cb).assert_depth
                                {
                                    if !lengthptr.is_null() {
                                        *lengthptr += CU2BYTES(1) + IMM2_SIZE_U;
                                    } else {
                                        *code = OP_CLOSE as u8;
                                        code = code.add(1);
                                        PUT2INC(&mut code, 0, (*oc).number as u32);
                                    }
                                    oc = (*oc).next;
                                }
                                *code = if (*cb).assert_depth > 0 {
                                    OP_ASSERT_ACCEPT as u8
                                } else {
                                    OP_ACCEPT as u8
                                };
                                code = code.add(1);
                                if firstcuflags == REQ_UNSET {
                                    firstcuflags = REQ_NONE;
                                }
                                break 'item;
                            }

                            META_PRUNE | META_SKIP => {
                                (*cb).had_pruneorskip = TRUE;
                                *code = VERBOPS[((meta as i64 - META_MARK) >> 16) as usize] as u8;
                                code = code.add(1);
                                break 'item;
                            }

                            META_COMMIT | META_FAIL => {
                                *code = VERBOPS[((meta as i64 - META_MARK) >> 16) as usize] as u8;
                                code = code.add(1);
                                break 'item;
                            }

                            META_THEN => {
                                (*cb).external_flags |= PCRE2_HASTHEN as u32;
                                *code = OP_THEN as u8;
                                code = code.add(1);
                                break 'item;
                            }

                            META_THEN_ARG => {
                                (*cb).external_flags |= PCRE2_HASTHEN as u32;
                                gto = Dispatch::VerbArg;
                                continue 'item;
                            }

                            META_PRUNE_ARG | META_SKIP_ARG => {
                                (*cb).had_pruneorskip = TRUE;
                                gto = Dispatch::VerbArg;
                                continue 'item;
                            }

                            META_MARK | META_COMMIT_ARG => {
                                gto = Dispatch::VerbArg;
                                continue 'item;
                            }

                            META_OPTIONS => {
                                pptr = pptr.add(1);
                                options = *pptr;
                                *optionsptr = options;
                                pptr = pptr.add(1);
                                xoptions = *pptr;
                                *xoptionsptr = xoptions;
                                greedy_default = ((options & PCRE2_UNGREEDY as u32) != 0) as u32;
                                greedy_non_default = greedy_default ^ 1;
                                req_caseopt = if (options & PCRE2_CASELESS as u32) != 0 {
                                    REQ_CASELESS
                                } else {
                                    0
                                };
                                break 'item;
                            }

                            META_OFFSET => {
                                if !lengthptr.is_null() {
                                    pptr =
                                        crate::compile_cgroup::_pcre2_compile_parse_scan_substr_args8(
                                            pptr,
                                            errorcodeptr,
                                            cb,
                                            lengthptr,
                                        );
                                    if pptr.is_null() {
                                        return 0;
                                    }
                                    break 'item;
                                }

                                loop {
                                    match META_CODE(*pptr) as i64 {
                                        META_OFFSET => {
                                            pptr = pptr.add(1);
                                            SKIPOFFSET(&mut pptr);
                                            continue;
                                        }

                                        META_CAPTURE_NAME => {
                                            let ng = (*cb).named_groups.add(*pptr.add(1) as usize);
                                            pptr = pptr.add(2);
                                            let mut count: c_int = 0;
                                            let mut index: c_int = 0;

                                            if crate::compile_cgroup::_pcre2_compile_find_dupname_details8(
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

                                            *code.add(0) = OP_DNCREF as u8;
                                            PUT2(code, 1, index as u32);
                                            PUT2(code, 1 + IMM2_SIZE_U, count as u32);
                                            code = code.add(1 + 2 * IMM2_SIZE_U);
                                            continue;
                                        }

                                        META_CAPTURE_NUMBER => {
                                            pptr = pptr.add(2);
                                            if *pptr.offset(-1) == 0 {
                                                continue;
                                            }

                                            *code.add(0) = OP_CREF as u8;
                                            PUT2(code, 1, *pptr.offset(-1));
                                            code = code.add(1 + IMM2_SIZE_U);
                                            continue;
                                        }

                                        _ => {}
                                    }

                                    break;
                                }
                                pptr = pptr.offset(-1);
                                break 'item;
                            }

                            META_SCS => {
                                bravalue = OP_ASSERT_SCS as c_int;
                                (*cb).assert_depth += 1;
                                gto = Dispatch::GroupProcess;
                                continue 'item;
                            }

                            META_COND_RNUMBER | META_COND_NAME | META_COND_RNAME => {
                                bravalue = OP_COND as c_int;

                                if !lengthptr.is_null() {
                                    let start_pptr: *mut u32 = pptr;
                                    pptr = pptr.add(1);
                                    let length: u32 = *pptr;

                                    offset = GETPLUSOFFSET(&mut pptr);
                                    let name = (*cb).start_pattern.add(offset);

                                    let ng =
                                        crate::compile_cgroup::_pcre2_compile_find_named_group8(
                                            name, length, cb,
                                        );

                                    if ng.is_null() {
                                        groupnumber = 0;
                                        if meta as i64 == META_COND_RNUMBER {
                                            let mut i: u32 = 1;
                                            while i < length {
                                                groupnumber = groupnumber * 10
                                                    + (*name.add(i as usize) as u32 - CHAR_0_U);
                                                if groupnumber > MAX_GROUP_NUMBER {
                                                    *errorcodeptr = ERR61;
                                                    (*cb).erroroffset = offset + i as PCRE2_SIZE;
                                                    return 0;
                                                }
                                                i += 1;
                                            }
                                        }

                                        if meta as i64 != META_COND_RNUMBER
                                            || groupnumber > (*cb).bracount
                                        {
                                            *errorcodeptr = ERR15;
                                            (*cb).erroroffset = offset;
                                            return 0;
                                        }

                                        if groupnumber == 0 {
                                            groupnumber = RREF_ANY as u32;
                                        }
                                        *start_pptr.add(1) = groupnumber;
                                        skipunits = 1 + IMM2_SIZE_U as u32;
                                        gto = Dispatch::GroupProcessNoteEmpty;
                                        continue 'item;
                                    }

                                    if meta as i64 == META_COND_RNUMBER {
                                        meta = META_COND_NAME as u32;
                                    }

                                    if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME_U) == 0 {
                                        if (*ng).number > (*cb).top_backref {
                                            (*cb).top_backref = (*ng).number;
                                        }

                                        *start_pptr.add(0) = meta;
                                        *start_pptr.add(1) = (*ng).number;

                                        skipunits = 1 + IMM2_SIZE_U as u32;
                                        gto = Dispatch::GroupProcessNoteEmpty;
                                        continue 'item;
                                    }

                                    *start_pptr.add(0) = meta | 1;
                                    *start_pptr.add(1) =
                                        ng.offset_from((*cb).named_groups) as u32;

                                    skipunits = 1 + 2 * IMM2_SIZE_U as u32;
                                } else {
                                    if meta as i64 == META_COND_RNUMBER {
                                        *code.add(1 + LINK_SIZE_U) = OP_RREF as u8;
                                        PUT2(code, 2 + LINK_SIZE_U, *pptr.add(1));
                                        skipunits = 1 + IMM2_SIZE_U as u32;
                                        pptr = pptr.add(1 + SIZEOFFSET_U);
                                        gto = Dispatch::GroupProcessNoteEmpty;
                                        continue 'item;
                                    }

                                    if meta_arg == 0 {
                                        *code.add(1 + LINK_SIZE_U) =
                                            if meta as i64 == META_COND_RNAME {
                                                OP_RREF as u8
                                            } else {
                                                OP_CREF as u8
                                            };
                                        PUT2(code, 2 + LINK_SIZE_U, *pptr.add(1));
                                        skipunits = 1 + IMM2_SIZE_U as u32;
                                        pptr = pptr.add(1 + SIZEOFFSET_U);
                                        gto = Dispatch::GroupProcessNoteEmpty;
                                        continue 'item;
                                    }

                                    let ng = (*cb).named_groups.add(*pptr.add(1) as usize);
                                    let mut count: c_int = 0;
                                    let mut index: c_int = 0;

                                    if crate::compile_cgroup::_pcre2_compile_find_dupname_details8(
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

                                    *code.add(1 + LINK_SIZE_U) =
                                        if meta as i64 == META_COND_RNAME {
                                            OP_DNRREF as u8
                                        } else {
                                            OP_DNCREF as u8
                                        };

                                    PUT2(code, 2 + LINK_SIZE_U, index as u32);
                                    PUT2(code, 2 + LINK_SIZE_U + IMM2_SIZE_U, count as u32);
                                    skipunits = 1 + 2 * IMM2_SIZE_U as u32;
                                    pptr = pptr.add(1 + SIZEOFFSET_U);
                                }

                                gto = Dispatch::GroupProcessNoteEmpty;
                                continue 'item;
                            }

                            META_COND_DEFINE => {
                                bravalue = OP_COND as c_int;
                                offset = GETPLUSOFFSET(&mut pptr);
                                *code.add(1 + LINK_SIZE_U) = OP_DEFINE as u8;
                                skipunits = 1;
                                gto = Dispatch::GroupProcess;
                                continue 'item;
                            }

                            META_COND_NUMBER => {
                                bravalue = OP_COND as c_int;
                                offset = GETPLUSOFFSET(&mut pptr);

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

                                offset -= 2;
                                *code.add(1 + LINK_SIZE_U) = OP_CREF as u8;
                                skipunits = 1 + IMM2_SIZE_U as u32;
                                PUT2(code, 2 + LINK_SIZE_U, groupnumber);
                                gto = Dispatch::GroupProcessNoteEmpty;
                                continue 'item;
                            }

                            META_COND_VERSION => {
                                bravalue = OP_COND as c_int;
                                if *pptr.add(1) > 0 {
                                    *code.add(1 + LINK_SIZE_U) = if (PCRE2_MAJOR
                                        > *pptr.add(2) as i64)
                                        || (PCRE2_MAJOR == *pptr.add(2) as i64
                                            && PCRE2_MINOR >= *pptr.add(3) as i64)
                                    {
                                        OP_TRUE as u8
                                    } else {
                                        OP_FALSE as u8
                                    };
                                } else {
                                    *code.add(1 + LINK_SIZE_U) = if PCRE2_MAJOR
                                        == *pptr.add(2) as i64
                                        && PCRE2_MINOR == *pptr.add(3) as i64
                                    {
                                        OP_TRUE as u8
                                    } else {
                                        OP_FALSE as u8
                                    };
                                }
                                skipunits = 1;
                                pptr = pptr.add(3);
                                gto = Dispatch::GroupProcessNoteEmpty;
                                continue 'item;
                            }

                            META_COND_ASSERT => {
                                bravalue = OP_COND as c_int;
                                gto = Dispatch::GroupProcessNoteEmpty;
                                continue 'item;
                            }

                            META_LOOKAHEAD => {
                                bravalue = OP_ASSERT as c_int;
                                (*cb).assert_depth += 1;
                                gto = Dispatch::GroupProcess;
                                continue 'item;
                            }

                            META_LOOKAHEAD_NA => {
                                bravalue = OP_ASSERT_NA as c_int;
                                (*cb).assert_depth += 1;
                                gto = Dispatch::GroupProcess;
                                continue 'item;
                            }

                            META_LOOKAHEADNOT => {
                                if *pptr.add(1) as i64 == META_KET
                                    && (*pptr.add(2) < META_ASTERISK as u32
                                        || *pptr.add(2) > META_MINMAX_QUERY as u32)
                                {
                                    *code = OP_FAIL as u8;
                                    code = code.add(1);
                                    pptr = pptr.add(1);
                                    break 'item;
                                } else {
                                    bravalue = OP_ASSERT_NOT as c_int;
                                    (*cb).assert_depth += 1;
                                    gto = Dispatch::GroupProcess;
                                    continue 'item;
                                }
                            }

                            META_LOOKBEHIND => {
                                bravalue = OP_ASSERTBACK as c_int;
                                (*cb).assert_depth += 1;
                                gto = Dispatch::GroupProcess;
                                continue 'item;
                            }

                            META_LOOKBEHINDNOT => {
                                bravalue = OP_ASSERTBACK_NOT as c_int;
                                (*cb).assert_depth += 1;
                                gto = Dispatch::GroupProcess;
                                continue 'item;
                            }

                            META_LOOKBEHIND_NA => {
                                bravalue = OP_ASSERTBACK_NA as c_int;
                                (*cb).assert_depth += 1;
                                gto = Dispatch::GroupProcess;
                                continue 'item;
                            }

                            META_ATOMIC => {
                                bravalue = OP_ONCE as c_int;
                                gto = Dispatch::GroupProcessNoteEmpty;
                                continue 'item;
                            }

                            META_SCRIPT_RUN => {
                                bravalue = OP_SCRIPT_RUN as c_int;
                                gto = Dispatch::GroupProcessNoteEmpty;
                                continue 'item;
                            }

                            META_NOCAPTURE => {
                                bravalue = OP_BRA as c_int;
                                gto = Dispatch::GroupProcessNoteEmpty;
                                continue 'item;
                            }

                            META_BACKREF_BYNAME | META_RECURSE_BYNAME => {
                                pptr = pptr.add(1);
                                let length: u32 = *pptr;

                                offset = GETPLUSOFFSET(&mut pptr);
                                let name = (*cb).start_pattern.add(offset);

                                let ng =
                                    crate::compile_cgroup::_pcre2_compile_find_named_group8(
                                        name, length, cb,
                                    );

                                if ng.is_null() {
                                    *errorcodeptr = ERR15;
                                    (*cb).erroroffset = offset;
                                    return 0;
                                }

                                groupnumber = (*ng).number;

                                if meta as i64 == META_RECURSE_BYNAME {
                                    meta_arg = groupnumber;
                                    gto = Dispatch::HandleNumericalRecursion;
                                    continue 'item;
                                }

                                (*cb).backref_map |= if groupnumber < 32 {
                                    1u32 << groupnumber
                                } else {
                                    1
                                };
                                if groupnumber > (*cb).top_backref {
                                    (*cb).top_backref = groupnumber;
                                }

                                if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME_U) == 0 {
                                    meta_arg = groupnumber;
                                    gto = Dispatch::HandleSingleReference;
                                    continue 'item;
                                }

                                let mut count: c_int = 0;
                                let mut index: c_int = 0;
                                if lengthptr.is_null()
                                    && crate::compile_cgroup::_pcre2_compile_find_dupname_details8(
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
                                *code = if (options & PCRE2_CASELESS as u32) != 0 {
                                    OP_DNREFI as u8
                                } else {
                                    OP_DNREF as u8
                                };
                                code = code.add(1);
                                PUT2INC(&mut code, 0, index as u32);
                                PUT2INC(&mut code, 0, count as u32);
                                if (options & PCRE2_CASELESS as u32) != 0 {
                                    *code = (if (xoptions
                                        & PCRE2_EXTRA_CASELESS_RESTRICT as u32)
                                        != 0
                                    {
                                        REFI_FLAG_CASELESS_RESTRICT
                                    } else {
                                        0
                                    } | if (xoptions & PCRE2_EXTRA_TURKISH_CASING as u32) != 0 {
                                        REFI_FLAG_TURKISH_CASING
                                    } else {
                                        0
                                    }) as u8;
                                    code = code.add(1);
                                }
                                break 'item;
                            }

                            META_CALLOUT_NUMBER => {
                                *code.add(0) = OP_CALLOUT as u8;
                                PUT(code, 1, *pptr.add(1) as i32);
                                PUT(code, 1 + LINK_SIZE_U, *pptr.add(2) as i32);
                                *code.add(1 + 2 * LINK_SIZE_U) = *pptr.add(3) as u8;
                                pptr = pptr.add(3);
                                code = code.add(
                                    tables::_pcre2_OP_lengths_8[OP_CALLOUT as usize] as usize,
                                );
                                break 'item;
                            }

                            META_CALLOUT_STRING => {
                                if !lengthptr.is_null() {
                                    *lengthptr +=
                                        *pptr.add(3) as PCRE2_SIZE + (1 + 4 * LINK_SIZE_U);
                                    pptr = pptr.add(3);
                                    SKIPOFFSET(&mut pptr);
                                } else {
                                    let mut pp: PCRE2_SPTR;
                                    let mut delimiter: u32;
                                    let mut length: u32 = *pptr.add(3);
                                    let mut callout_string: *mut PCRE2_UCHAR =
                                        code.add(1 + 4 * LINK_SIZE_U);

                                    *code.add(0) = OP_CALLOUT_STR as u8;
                                    PUT(code, 1, *pptr.add(1) as i32);
                                    PUT(code, 1 + LINK_SIZE_U, *pptr.add(2) as i32);

                                    pptr = pptr.add(3);
                                    offset = GETPLUSOFFSET(&mut pptr);
                                    pp = (*cb).start_pattern.add(offset);
                                    delimiter = *pp as u32;
                                    *callout_string = *pp;
                                    callout_string = callout_string.add(1);
                                    pp = pp.add(1);
                                    if delimiter == CHAR_LEFT_CURLY_BRACKET {
                                        delimiter = CHAR_RIGHT_CURLY_BRACKET;
                                    }
                                    PUT(code, 1 + 3 * LINK_SIZE_U, (offset + 1) as i32);

                                    length -= 1;
                                    while length > 1 {
                                        if *pp as u32 == delimiter
                                            && *pp.add(1) as u32 == delimiter
                                        {
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

                                    PUT(
                                        code,
                                        1 + 2 * LINK_SIZE_U,
                                        callout_string.offset_from(code) as i32,
                                    );
                                    code = callout_string;
                                }
                                break 'item;
                            }

                            META_MINMAX_PLUS | META_MINMAX_QUERY | META_MINMAX => {
                                pptr = pptr.add(1);
                                repeat_min = *pptr;
                                pptr = pptr.add(1);
                                repeat_max = *pptr;
                                gto = Dispatch::Repeat;
                                continue 'item;
                            }

                            META_ASTERISK | META_ASTERISK_PLUS | META_ASTERISK_QUERY => {
                                repeat_min = 0;
                                repeat_max = REPEAT_UNLIMITED;
                                gto = Dispatch::Repeat;
                                continue 'item;
                            }

                            META_PLUS | META_PLUS_PLUS | META_PLUS_QUERY => {
                                repeat_min = 1;
                                repeat_max = REPEAT_UNLIMITED;
                                gto = Dispatch::Repeat;
                                continue 'item;
                            }

                            META_QUERY | META_QUERY_PLUS | META_QUERY_QUERY => {
                                repeat_min = 0;
                                repeat_max = 1;
                                gto = Dispatch::Repeat;
                                continue 'item;
                            }

                            META_BIGVALUE => {
                                pptr = pptr.add(1);
                                gto = Dispatch::NormalChar;
                                continue 'item;
                            }

                            META_BACKREF => {
                                if meta_arg < 10 {
                                    offset = (*cb).small_ref_offset[meta_arg as usize];
                                } else {
                                    offset = GETPLUSOFFSET(&mut pptr);
                                }

                                if meta_arg > (*cb).bracount {
                                    (*cb).erroroffset = offset;
                                    *errorcodeptr = ERR15;
                                    return 0;
                                }

                                gto = Dispatch::HandleSingleReference;
                                continue 'item;
                            }

                            META_RECURSE => {
                                offset = GETPLUSOFFSET(&mut pptr);
                                if meta_arg > (*cb).bracount {
                                    (*cb).erroroffset = offset;
                                    *errorcodeptr = ERR15;
                                    return 0;
                                }
                                gto = Dispatch::HandleNumericalRecursion;
                                continue 'item;
                            }

                            META_CAPTURE => {
                                bravalue = OP_CBRA as c_int;
                                skipunits = IMM2_SIZE_U as u32;
                                PUT2(code, 1 + LINK_SIZE_U, meta_arg);
                                (*cb).lastcapture = meta_arg;
                                gto = Dispatch::GroupProcessNoteEmpty;
                                continue 'item;
                            }

                            META_ESCAPE => {
                                if meta_arg > ESC_b && meta_arg < ESC_Z {
                                    matched_char = TRUE;
                                    if firstcuflags == REQ_UNSET {
                                        firstcuflags = REQ_NONE;
                                    }
                                }

                                zerofirstcu = firstcu;
                                zerofirstcuflags = firstcuflags;
                                zeroreqcu = reqcu;
                                zeroreqcuflags = reqcuflags;

                                if meta_arg == ESC_P || meta_arg == ESC_p {
                                    pptr = pptr.add(1);
                                    let mut ptype: u32 = *pptr >> 16;
                                    let mut pdata: u32 = *pptr & 0xffff;

                                    if (options & PCRE2_CASELESS as u32) != 0
                                        && ptype == PT_PC as u32
                                        && (pdata == ucp_Lu
                                            || pdata == ucp_Ll
                                            || pdata == ucp_Lt)
                                    {
                                        ptype = PT_LAMP as u32;
                                        pdata = 0;
                                    }

                                    if ptype == PT_ANY as u32 {
                                        if meta_arg == ESC_P {
                                            *code = OP_CLASS as u8;
                                            code = code.add(1);
                                            ptr::write_bytes(code, 0, 32);
                                            code = code.add(32);
                                        } else {
                                            *code = OP_ALLANY as u8;
                                            code = code.add(1);
                                        }
                                    } else {
                                        *code = if meta_arg == ESC_p {
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
                                    break 'item;
                                }

                                if (*cb).assert_depth > 0
                                    && meta_arg == ESC_K
                                    && (xoptions & PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK as u32) == 0
                                {
                                    *errorcodeptr = ERR99;
                                    return 0;
                                }

                                match meta_arg {
                                    ESC_C => {
                                        (*cb).external_flags |= PCRE2_HASBKC as u32;
                                        if utf == FALSE {
                                            meta_arg = OP_ALLANY;
                                        }
                                    }

                                    ESC_B | ESC_b => {
                                        if (options & PCRE2_UCP as u32) != 0
                                            && (xoptions & PCRE2_EXTRA_ASCII_BSW as u32) == 0
                                        {
                                            meta_arg = if meta_arg == ESC_B {
                                                OP_NOT_UCP_WORD_BOUNDARY
                                            } else {
                                                OP_UCP_WORD_BOUNDARY
                                            };
                                        }
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
                                        (*cb).external_flags |= PCRE2_HASBSK as u32;
                                    }

                                    _ => {}
                                }

                                *code = meta_arg as u8;
                                code = code.add(1);
                                break 'item;
                            }

                            _ => {
                                if meta >= META_END as u32 {
                                    *errorcodeptr = ERR89;
                                    return 0;
                                }

                                gto = Dispatch::NormalChar;
                                continue 'item;
                            }
                        }
                    }

                    Dispatch::ClassEndProcessing => {
                        if firstcuflags == REQ_UNSET {
                            firstcuflags = REQ_NONE;
                        }
                        zerofirstcu = firstcu;
                        zerofirstcuflags = firstcuflags;
                        zeroreqcu = reqcu;
                        zeroreqcuflags = reqcuflags;
                        break 'item;
                    }

                    Dispatch::VerbArg => {
                        *code = VERBOPS[((meta as i64 - META_MARK) >> 16) as usize] as u8;
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
                            if utf != FALSE {
                                mclength = crate::ord2utf::_pcre2_ord2utf_8(
                                    meta,
                                    mcbuffer.as_mut_ptr(),
                                );
                            } else {
                                mclength = 1;
                                mcbuffer[0] = meta as u8;
                            }
                            if !lengthptr.is_null() {
                                *lengthptr += mclength as PCRE2_SIZE;
                            } else {
                                ptr::copy_nonoverlapping(
                                    mcbuffer.as_ptr(),
                                    code,
                                    CU2BYTES(mclength as usize),
                                );
                                code = code.add(mclength as usize);
                                verbculen += mclength;
                            }
                            i += 1;
                        }

                        *tempcode = verbculen as u8;
                        *code = 0;
                        code = code.add(1);
                        break 'item;
                    }

                    Dispatch::GroupProcessNoteEmpty => {
                        note_group_empty = TRUE;
                        gto = Dispatch::GroupProcess;
                        continue 'item;
                    }

                    Dispatch::GroupProcess => {
                        (*cb).parens_depth += 1;
                        *code = bravalue as u8;
                        pptr = pptr.add(1);
                        tempcode = code;
                        tempreqvary = (*cb).req_varyopt;
                        length_prevgroup = 0;

                        group_return = crate::compile_aux::compile_regex(
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
                                ptr::null_mut()
                            } else {
                                &mut length_prevgroup
                            },
                        );
                        if group_return == 0 {
                            return 0;
                        }

                        (*cb).parens_depth -= 1;

                        if note_group_empty != FALSE
                            && bravalue != OP_COND as c_int
                            && group_return > 0
                        {
                            matched_char = TRUE;
                        }

                        if bravalue >= OP_ASSERT as c_int && bravalue <= OP_ASSERT_SCS as c_int {
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

                            if *code.add(LINK_SIZE_U + 1) as u32 == OP_DEFINE {
                                if condcount > 1 {
                                    (*cb).erroroffset = offset;
                                    *errorcodeptr = ERR54;
                                    return 0;
                                }
                                *code.add(LINK_SIZE_U + 1) = OP_FALSE as u8;
                                bravalue = OP_DEFINE as c_int;
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

                        if !lengthptr.is_null() {
                            if (OFLOW_MAX as i64) - (*lengthptr as i64)
                                < (length_prevgroup as i64) - 2 - 2 * (LINK_SIZE_U as i64)
                            {
                                *errorcodeptr = ERR20;
                                return 0;
                            }
                            *lengthptr += length_prevgroup - 2 - 2 * LINK_SIZE_U;
                            code = code.add(1);
                            PUTINC(&mut code, 0, (1 + LINK_SIZE_U) as i32);
                            *code = OP_KET as u8;
                            code = code.add(1);
                            PUTINC(&mut code, 0, (1 + LINK_SIZE_U) as i32);
                            break 'item;
                        }

                        code = tempcode;

                        if bravalue == OP_DEFINE as c_int {
                            break 'item;
                        }

                        zeroreqcu = reqcu;
                        zeroreqcuflags = reqcuflags;
                        zerofirstcu = firstcu;
                        zerofirstcuflags = firstcuflags;
                        groupsetfirstcu = FALSE;

                        if bravalue >= OP_ONCE as c_int {
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
                        } else if (bravalue == OP_ASSERT as c_int
                            || bravalue == OP_ASSERT_NA as c_int)
                            && subreqcuflags < REQ_NONE
                            && subfirstcuflags < REQ_NONE
                        {
                            reqcu = subreqcu;
                            reqcuflags = subreqcuflags;
                        }

                        break 'item;
                    }

                    Dispatch::Repeat => {
                        if previous_matched_char != FALSE && repeat_min > 0 {
                            matched_char = TRUE;
                        }

                        reqvary = if repeat_min == repeat_max { 0 } else { REQ_VARY };

                        if repeat_min == 0 {
                            firstcu = zerofirstcu;
                            firstcuflags = zerofirstcuflags;
                            reqcu = zeroreqcu;
                            reqcuflags = zeroreqcuflags;
                        }

                        match meta as i64 {
                            META_MINMAX_PLUS | META_ASTERISK_PLUS | META_PLUS_PLUS
                            | META_QUERY_PLUS => {
                                repeat_type = 0; // Force greedy
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

                        // PCRE2_ASSERT(previous != NULL);
                        tempcode = previous;
                        op_previous = *previous;

                        // op_previous switch.
                        match op_previous as u32 {
                            OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI => {
                                if repeat_max == 1 && repeat_min == 1 {
                                    gto = Dispatch::EndRepeat;
                                    continue 'item;
                                }
                                op_type = CHARTYPEOFFSET[(op_previous as u32 - OP_CHAR) as usize];

                                // MAYBE_UTF_MULTI
                                if utf != FALSE && NOT_FIRSTCU(*code.offset(-1) as u32) {
                                    let mut lastchar: PCRE2_SPTR = code.offset(-1);
                                    BACKCHAR(&mut lastchar);
                                    mclength = code.offset_from(lastchar) as u32;
                                    ptr::copy_nonoverlapping(
                                        lastchar,
                                        mcbuffer.as_mut_ptr(),
                                        CU2BYTES(mclength as usize),
                                    );
                                } else {
                                    mcbuffer[0] = *code.offset(-1);
                                    mclength = 1;
                                    if (op_previous as u32) <= OP_CHARI && repeat_min > 1 {
                                        reqcu = mcbuffer[0] as u32;
                                        reqcuflags = (*cb).req_varyopt;
                                        if op_previous as u32 == OP_CHARI {
                                            reqcuflags |= REQ_CASELESS;
                                        }
                                    }
                                }
                                prop_type = -1;
                                prop_value = -1;
                                gto = Dispatch::OutputSingleRepeat;
                                continue 'item;
                            }

                            OP_XCLASS | OP_ECLASS | OP_CLASS | OP_NCLASS | OP_REF | OP_REFI
                            | OP_DNREF | OP_DNREFI => {
                                if repeat_max == 0 {
                                    code = previous;
                                    gto = Dispatch::EndRepeat;
                                    continue 'item;
                                }
                                if repeat_max == 1 && repeat_min == 1 {
                                    gto = Dispatch::EndRepeat;
                                    continue 'item;
                                }

                                if repeat_min == 0 && repeat_max == REPEAT_UNLIMITED {
                                    *code = (OP_CRSTAR + repeat_type) as u8;
                                    code = code.add(1);
                                } else if repeat_min == 1 && repeat_max == REPEAT_UNLIMITED {
                                    *code = (OP_CRPLUS + repeat_type) as u8;
                                    code = code.add(1);
                                } else if repeat_min == 0 && repeat_max == 1 {
                                    *code = (OP_CRQUERY + repeat_type) as u8;
                                    code = code.add(1);
                                } else {
                                    *code = (OP_CRRANGE + repeat_type) as u8;
                                    code = code.add(1);
                                    PUT2INC(&mut code, 0, repeat_min);
                                    if repeat_max == REPEAT_UNLIMITED {
                                        repeat_max = 0;
                                    }
                                    PUT2INC(&mut code, 0, repeat_max);
                                }
                                gto = Dispatch::PossessiveHandling;
                                continue 'item;
                            }

                            _ => {
                                // OP_RECURSE, brackets, or a character-type match.
                                if op_previous as u32 == OP_RECURSE {
                                    if repeat_max == 1
                                        && repeat_min == 1
                                        && possessive_quantifier == FALSE
                                    {
                                        gto = Dispatch::EndRepeat;
                                        continue 'item;
                                    }

                                    if repeat_min > 0
                                        && (repeat_min != 1 || repeat_max != REPEAT_UNLIMITED)
                                    {
                                        let mut replicate: c_int = repeat_min as c_int;

                                        if repeat_min == repeat_max {
                                            replicate -= 1;
                                        }

                                        if !lengthptr.is_null() {
                                            let mut delta: PCRE2_SIZE = 0;
                                            if crate::chkdint::_pcre2_ckd_smul_8(
                                                &mut delta,
                                                replicate,
                                                length_prevgroup as c_int,
                                            ) != FALSE
                                                || (OFLOW_MAX as i64) - (*lengthptr as i64)
                                                    < delta as i64
                                            {
                                                *errorcodeptr = ERR20;
                                                return 0;
                                            }
                                            *lengthptr += delta;
                                        } else {
                                            let mut i: c_int = 0;
                                            while i < replicate {
                                                ptr::copy_nonoverlapping(
                                                    previous,
                                                    code,
                                                    CU2BYTES(length_prevgroup),
                                                );
                                                previous = code;
                                                code = code.add(length_prevgroup);
                                                i += 1;
                                            }
                                        }

                                        if repeat_min == repeat_max {
                                            break 'item;
                                        }
                                        if repeat_max != REPEAT_UNLIMITED {
                                            repeat_max -= repeat_min;
                                        }
                                        repeat_min = 0;
                                    }

                                    // Wrap the recursion in OP_BRA brackets.
                                    {
                                        let length: PCRE2_SIZE = if !lengthptr.is_null() {
                                            1 + LINK_SIZE_U
                                        } else {
                                            length_prevgroup
                                        };

                                        ptr::copy(
                                            previous,
                                            previous.add(1 + LINK_SIZE_U),
                                            CU2BYTES(length),
                                        );
                                        *previous = OP_BRA as u8;
                                        op_previous = OP_BRA as u8;
                                        PUT(previous, 1, (1 + LINK_SIZE_U + length) as i32);
                                        *previous.add(1 + LINK_SIZE_U + length) = OP_KET as u8;
                                        PUT(
                                            previous,
                                            2 + LINK_SIZE_U + length,
                                            (1 + LINK_SIZE_U + length) as i32,
                                        );
                                    }
                                    code = code.add(2 + 2 * LINK_SIZE_U);
                                    length_prevgroup += 2 + 2 * LINK_SIZE_U;
                                    group_return = -1; // Set "may match empty string"
                                    // Fall through to the repeated OP_BRA handling.
                                }

                                // Now handle the bracket-group opcodes (including
                                // the OP_RECURSE-wrapped OP_BRA above).
                                match op_previous as u32 {
                                    OP_ASSERT | OP_ASSERT_NOT | OP_ASSERT_NA | OP_ASSERTBACK
                                    | OP_ASSERTBACK_NOT | OP_ASSERTBACK_NA | OP_ASSERT_SCS
                                    | OP_ONCE | OP_SCRIPT_RUN | OP_BRA | OP_CBRA | OP_COND => {
                                        let mut len: c_int = code.offset_from(previous) as c_int;
                                        let mut bralink: *mut PCRE2_UCHAR = ptr::null_mut();
                                        let mut brazeroptr: *mut PCRE2_UCHAR = ptr::null_mut();

                                        if repeat_max == 1
                                            && repeat_min == 1
                                            && possessive_quantifier == FALSE
                                        {
                                            gto = Dispatch::EndRepeat;
                                            continue 'item;
                                        }

                                        if op_previous as u32 == OP_COND
                                            && *previous.add(LINK_SIZE_U + 1) as u32 == OP_FALSE
                                            && *previous.add(GET(previous, 1) as usize) as u32
                                                != OP_ALT
                                        {
                                            gto = Dispatch::EndRepeat;
                                            continue 'item;
                                        }

                                        if (op_previous as u32) < OP_ONCE {
                                            // Assertion
                                            if repeat_max == REPEAT_UNLIMITED {
                                                repeat_max = repeat_min + 1;
                                            }
                                        }

                                        if repeat_min == 0 {
                                            if repeat_max <= 1 || repeat_max == REPEAT_UNLIMITED {
                                                ptr::copy(
                                                    previous,
                                                    previous.add(1),
                                                    CU2BYTES(len as usize),
                                                );
                                                code = code.add(1);
                                                if repeat_max == 0 {
                                                    *previous = OP_SKIPZERO as u8;
                                                    previous = previous.add(1);
                                                    gto = Dispatch::EndRepeat;
                                                    continue 'item;
                                                }
                                                brazeroptr = previous;
                                                *previous = (OP_BRAZERO + repeat_type) as u8;
                                                previous = previous.add(1);
                                            } else {
                                                ptr::copy(
                                                    previous,
                                                    previous.add(2 + LINK_SIZE_U),
                                                    CU2BYTES(len as usize),
                                                );
                                                code = code.add(2 + LINK_SIZE_U);
                                                *previous = (OP_BRAZERO + repeat_type) as u8;
                                                previous = previous.add(1);
                                                *previous = OP_BRA as u8;
                                                previous = previous.add(1);

                                                let linkoffset: c_int = if bralink.is_null() {
                                                    0
                                                } else {
                                                    previous.offset_from(bralink) as c_int
                                                };
                                                bralink = previous;
                                                PUTINC(&mut previous, 0, linkoffset);
                                            }

                                            if repeat_max != REPEAT_UNLIMITED {
                                                repeat_max -= 1;
                                            }
                                        } else {
                                            if repeat_min > 1 {
                                                if !lengthptr.is_null() {
                                                    let mut delta: PCRE2_SIZE = 0;
                                                    if crate::chkdint::_pcre2_ckd_smul_8(
                                                        &mut delta,
                                                        (repeat_min - 1) as c_int,
                                                        length_prevgroup as c_int,
                                                    ) != FALSE
                                                        || (OFLOW_MAX as i64)
                                                            - (*lengthptr as i64)
                                                            < delta as i64
                                                    {
                                                        *errorcodeptr = ERR20;
                                                        return 0;
                                                    }
                                                    *lengthptr += delta;
                                                } else {
                                                    if groupsetfirstcu != FALSE
                                                        && reqcuflags >= REQ_NONE
                                                    {
                                                        reqcu = firstcu;
                                                        reqcuflags = firstcuflags;
                                                    }
                                                    let mut i: u32 = 1;
                                                    while i < repeat_min {
                                                        ptr::copy_nonoverlapping(
                                                            previous,
                                                            code,
                                                            CU2BYTES(len as usize),
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

                                        // Common code for both zero and non-zero minimum.
                                        if repeat_max != REPEAT_UNLIMITED {
                                            if !lengthptr.is_null() && repeat_max > 0 {
                                                let mut delta: PCRE2_SIZE = 0;
                                                if crate::chkdint::_pcre2_ckd_smul_8(
                                                    &mut delta,
                                                    repeat_max as c_int,
                                                    length_prevgroup as c_int
                                                        + 1
                                                        + 2
                                                        + 2 * LINK_SIZE_U as c_int,
                                                ) != FALSE
                                                    || (OFLOW_MAX as i64)
                                                        + (2 + 2 * LINK_SIZE_U as i64)
                                                        - (*lengthptr as i64)
                                                        < delta as i64
                                                {
                                                    *errorcodeptr = ERR20;
                                                    return 0;
                                                }
                                                delta -= 2 + 2 * LINK_SIZE_U;
                                                *lengthptr += delta;
                                            } else {
                                                let mut i: u32 = repeat_max;
                                                while i >= 1 {
                                                    *code = (OP_BRAZERO + repeat_type) as u8;
                                                    code = code.add(1);

                                                    if i != 1 {
                                                        *code = OP_BRA as u8;
                                                        code = code.add(1);
                                                        let linkoffset: c_int =
                                                            if bralink.is_null() {
                                                                0
                                                            } else {
                                                                code.offset_from(bralink) as c_int
                                                            };
                                                        bralink = code;
                                                        PUTINC(&mut code, 0, linkoffset);
                                                    }

                                                    ptr::copy_nonoverlapping(
                                                        previous,
                                                        code,
                                                        CU2BYTES(len as usize),
                                                    );
                                                    code = code.add(len as usize);
                                                    i -= 1;
                                                }
                                            }

                                            while !bralink.is_null() {
                                                let linkoffset: c_int =
                                                    (code.offset_from(bralink) + 1) as c_int;
                                                let bra: *mut PCRE2_UCHAR =
                                                    code.offset(-(linkoffset as isize));
                                                let oldlinkoffset: c_int =
                                                    GET(bra, 1) as c_int;
                                                bralink = if oldlinkoffset == 0 {
                                                    ptr::null_mut()
                                                } else {
                                                    bralink.offset(-(oldlinkoffset as isize))
                                                };
                                                *code = OP_KET as u8;
                                                code = code.add(1);
                                                PUTINC(&mut code, 0, linkoffset);
                                                PUT(bra, 1, linkoffset);
                                            }
                                        } else {
                                            let ketcode: *mut PCRE2_UCHAR =
                                                code.offset(-1 - LINK_SIZE_U as isize);
                                            let bracode: *mut PCRE2_UCHAR =
                                                ketcode.offset(-(GET(ketcode, 1) as isize));

                                            if *bracode as u32 == OP_ONCE
                                                && possessive_quantifier != FALSE
                                            {
                                                *bracode = OP_BRA as u8;
                                            }

                                            if *bracode as u32 == OP_ONCE
                                                || *bracode as u32 == OP_SCRIPT_RUN
                                            {
                                                *ketcode = (OP_KETRMAX + repeat_type) as u8;
                                            } else {
                                                if lengthptr.is_null() {
                                                    if group_return < 0 {
                                                        *bracode = (*bracode as u32
                                                            + (OP_SBRA - OP_BRA))
                                                            as u8;
                                                    }
                                                    if *bracode as u32 == OP_COND
                                                        && *bracode
                                                            .add(GET(bracode, 1) as usize)
                                                            as u32
                                                            != OP_ALT
                                                    {
                                                        *bracode = OP_SCOND as u8;
                                                    }
                                                }

                                                if possessive_quantifier != FALSE {
                                                    if *bracode as u32 == OP_COND
                                                        || *bracode as u32 == OP_SCOND
                                                    {
                                                        let mut nlen: c_int =
                                                            code.offset_from(bracode) as c_int;
                                                        ptr::copy(
                                                            bracode,
                                                            bracode.add(1 + LINK_SIZE_U),
                                                            CU2BYTES(nlen as usize),
                                                        );
                                                        code = code.add(1 + LINK_SIZE_U);
                                                        nlen += (1 + LINK_SIZE_U) as c_int;
                                                        *bracode = if *bracode as u32 == OP_COND
                                                        {
                                                            OP_BRAPOS as u8
                                                        } else {
                                                            OP_SBRAPOS as u8
                                                        };
                                                        *code = OP_KETRPOS as u8;
                                                        code = code.add(1);
                                                        PUTINC(&mut code, 0, nlen);
                                                        PUT(bracode, 1, nlen);
                                                    } else {
                                                        *bracode = (*bracode as u32 + 1) as u8;
                                                        *ketcode = OP_KETRPOS as u8;
                                                    }

                                                    if !brazeroptr.is_null() {
                                                        *brazeroptr = OP_BRAPOSZERO as u8;
                                                    }
                                                    if repeat_min < 2 {
                                                        possessive_quantifier = FALSE;
                                                    }
                                                } else {
                                                    *ketcode = (OP_KETRMAX + repeat_type) as u8;
                                                }
                                            }
                                        }

                                        let _ = len;
                                        gto = Dispatch::PossessiveHandling;
                                        continue 'item;
                                    }

                                    _ => {
                                        // Character-type match (\d etc).
                                        if op_previous as u32 >= OP_EODN
                                            || op_previous as u32 <= OP_WORD_BOUNDARY
                                        {
                                            *errorcodeptr = ERR10;
                                            return 0;
                                        }

                                        if repeat_max == 1 && repeat_min == 1 {
                                            gto = Dispatch::EndRepeat;
                                            continue 'item;
                                        }

                                        op_type = OP_TYPESTAR - OP_STAR;
                                        mclength = 0;

                                        if op_previous as u32 == OP_PROP
                                            || op_previous as u32 == OP_NOTPROP
                                        {
                                            prop_type = *previous.add(1) as c_int;
                                            prop_value = *previous.add(2) as c_int;
                                        } else {
                                            prop_type = -1;
                                            prop_value = -1;
                                        }

                                        gto = Dispatch::OutputSingleRepeat;
                                        continue 'item;
                                    }
                                }
                            }
                        }
                    }

                    Dispatch::OutputSingleRepeat => {
                        oldcode = code; // Save where we were
                        code = previous; // Usually overwrite previous item

                        if repeat_max == 0 {
                            gto = Dispatch::EndRepeat;
                            continue 'item;
                        }

                        // Combine op_type with repeat_type.
                        repeat_type += op_type;

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
                                PUT2INC(&mut code, 0, repeat_max);
                            }
                        } else if repeat_min == 1 {
                            if repeat_max == REPEAT_UNLIMITED {
                                *code = (OP_PLUS + repeat_type) as u8;
                                code = code.add(1);
                            } else {
                                code = oldcode; // Leave previous item in place
                                if repeat_max == 1 {
                                    gto = Dispatch::EndRepeat;
                                    continue 'item;
                                }
                                *code = (OP_UPTO + repeat_type) as u8;
                                code = code.add(1);
                                PUT2INC(&mut code, 0, repeat_max - 1);
                            }
                        } else {
                            *code = (OP_EXACT + op_type) as u8; // NB EXACT has no repeat_type
                            code = code.add(1);
                            PUT2INC(&mut code, 0, repeat_min);

                            if repeat_max != repeat_min {
                                if mclength > 0 {
                                    ptr::copy_nonoverlapping(
                                        mcbuffer.as_ptr(),
                                        code,
                                        CU2BYTES(mclength as usize),
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

                                if repeat_max == REPEAT_UNLIMITED {
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
                                        PUT2INC(&mut code, 0, repeat_max);
                                    }
                                }
                            }
                        }

                        // Fill in the character or character type for the final opcode.
                        if mclength > 0 {
                            ptr::copy_nonoverlapping(
                                mcbuffer.as_ptr(),
                                code,
                                CU2BYTES(mclength as usize),
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

                        gto = Dispatch::PossessiveHandling;
                        continue 'item;
                    }

                    Dispatch::PossessiveHandling => {
                        if possessive_quantifier != FALSE {
                            let mut len: c_int;

                            match *tempcode as u32 {
                                OP_TYPEEXACT => {
                                    tempcode = tempcode.add(
                                        tables::_pcre2_OP_lengths_8[*tempcode as usize] as usize
                                            + if *tempcode.add(1 + IMM2_SIZE_U) as u32 == OP_PROP
                                                || *tempcode.add(1 + IMM2_SIZE_U) as u32
                                                    == OP_NOTPROP
                                            {
                                                2
                                            } else {
                                                0
                                            },
                                    );
                                }

                                OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_EXACT | OP_EXACTI
                                | OP_NOTEXACT | OP_NOTEXACTI => {
                                    tempcode = tempcode.add(
                                        tables::_pcre2_OP_lengths_8[*tempcode as usize] as usize,
                                    );
                                    if utf != FALSE && HAS_EXTRALEN(*tempcode.offset(-1) as u32) {
                                        tempcode = tempcode
                                            .add(GET_EXTRALEN(*tempcode.offset(-1) as u32)
                                                as usize);
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
                                        tables::_pcre2_OP_lengths_8[*tempcode as usize] as usize,
                                    );
                                }

                                _ => {}
                            }

                            len = code.offset_from(tempcode) as c_int;
                            if len > 0 {
                                let repcode: u32 = *tempcode as u32;

                                if repcode < OP_CALLOUT
                                    && OPCODE_POSSESSIFY[repcode as usize] > 0
                                {
                                    *tempcode = OPCODE_POSSESSIFY[repcode as usize];
                                } else {
                                    ptr::copy(
                                        tempcode,
                                        tempcode.add(1 + LINK_SIZE_U),
                                        CU2BYTES(len as usize),
                                    );
                                    code = code.add(1 + LINK_SIZE_U);
                                    len += (1 + LINK_SIZE_U) as c_int;
                                    *tempcode.add(0) = OP_ONCE as u8;
                                    *code = OP_KET as u8;
                                    code = code.add(1);
                                    PUTINC(&mut code, 0, len);
                                    PUT(tempcode, 1, len);
                                }
                            }
                        }

                        gto = Dispatch::EndRepeat;
                        continue 'item;
                    }

                    Dispatch::EndRepeat => {
                        (*cb).req_varyopt |= reqvary;
                        break 'item;
                    }

                    Dispatch::HandleSingleReference => {
                        if firstcuflags == REQ_UNSET {
                            zerofirstcuflags = REQ_NONE;
                            firstcuflags = REQ_NONE;
                        }
                        *code = if (options & PCRE2_CASELESS as u32) != 0 {
                            OP_REFI as u8
                        } else {
                            OP_REF as u8
                        };
                        code = code.add(1);
                        PUT2INC(&mut code, 0, meta_arg);
                        if (options & PCRE2_CASELESS as u32) != 0 {
                            *code = (if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT as u32) != 0 {
                                REFI_FLAG_CASELESS_RESTRICT
                            } else {
                                0
                            } | if (xoptions & PCRE2_EXTRA_TURKISH_CASING as u32) != 0 {
                                REFI_FLAG_TURKISH_CASING
                            } else {
                                0
                            }) as u8;
                            code = code.add(1);
                        }

                        (*cb).backref_map |= if meta_arg < 32 {
                            1u32 << meta_arg
                        } else {
                            1
                        };
                        if meta_arg > (*cb).top_backref {
                            (*cb).top_backref = meta_arg;
                        }
                        break 'item;
                    }

                    Dispatch::HandleNumericalRecursion => {
                        *code = OP_RECURSE as u8;
                        PUT(code, 1, meta_arg as i32);
                        code = code.add(1 + LINK_SIZE_U);
                        length_prevgroup = 1 + LINK_SIZE_U;

                        if META_CODE(*pptr.add(1)) as i64 == META_OFFSET
                            || META_CODE(*pptr.add(1)) as i64 == META_CAPTURE_NAME
                            || META_CODE(*pptr.add(1)) as i64 == META_CAPTURE_NUMBER
                        {
                            if !lengthptr.is_null() {
                                if crate::compile_cgroup::_pcre2_compile_parse_recurse_args8(
                                    pptr,
                                    offset,
                                    errorcodeptr,
                                    cb,
                                ) == FALSE
                                {
                                    return 0;
                                }

                                let args = (*cb).last_data as *mut recurse_arguments;
                                length_prevgroup += (*args).size * (1 + IMM2_SIZE_U);
                                *lengthptr += (*args).size * (1 + IMM2_SIZE_U);
                                pptr = pptr.add((*args).skip_size);
                            } else {
                                let args = (*cb).first_data as *mut recurse_arguments;
                                // PCRE2_ASSERT(args != NULL && ...)

                                let mut current = args.add(1) as *mut u16;
                                let end = current.add((*args).size);
                                // PCRE2_ASSERT(end > current);

                                loop {
                                    *code.add(0) = OP_CREF as u8;
                                    PUT2(code, 1, *current as u32);
                                    code = code.add(1 + IMM2_SIZE_U);
                                    current = current.add(1);
                                    if !(current < end) {
                                        break;
                                    }
                                }

                                length_prevgroup += (*args).size * (1 + IMM2_SIZE_U);
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
                        break 'item;
                    }

                    Dispatch::NormalChar => {
                        meta = *pptr; // Get the full 32 bits
                        gto = Dispatch::NormalCharSet;
                        continue 'item;
                    }

                    Dispatch::NormalCharSet => {
                        matched_char = TRUE;

                        if (utf_b || ucp != FALSE) && (options & PCRE2_CASELESS as u32) != 0 {
                            let mut caseset: u32;

                            if (xoptions
                                & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT)
                                    as u32)
                                == PCRE2_EXTRA_TURKISH_CASING as u32
                                && UCD_ANY_I(meta)
                            {
                                caseset = tables::_pcre2_ucd_turkish_dotted_i_caseset_8
                                    + (if UCD_DOTTED_I(meta) { 0 } else { 3 });
                            } else {
                                caseset = UCD_CASESET(meta);
                                if caseset != 0
                                    && (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT as u32) != 0
                                    && tables::_pcre2_ucd_caseless_sets_8[caseset as usize] < 128
                                {
                                    caseset = 0;
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
                                break 'item;
                            }
                        }

                        gto = Dispatch::ClassCaselessChar;
                        continue 'item;
                    }

                    Dispatch::ClassCaselessChar => {
                        if utf != FALSE {
                            mclength = crate::ord2utf::_pcre2_ord2utf_8(meta, mcbuffer.as_mut_ptr());
                        } else {
                            mclength = 1;
                            mcbuffer[0] = meta as u8;
                        }

                        *code = if (options & PCRE2_CASELESS as u32) != 0 {
                            OP_CHARI as u8
                        } else {
                            OP_CHAR as u8
                        };
                        code = code.add(1);
                        ptr::copy_nonoverlapping(
                            mcbuffer.as_ptr(),
                            code,
                            CU2BYTES(mclength as usize),
                        );
                        code = code.add(mclength as usize);

                        if mcbuffer[0] as u32 == CHAR_CR || mcbuffer[0] as u32 == CHAR_NL_U {
                            (*cb).external_flags |= PCRE2_HASCRORLF as u32;
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

                        if reset_caseful != FALSE {
                            options &= !(PCRE2_CASELESS as u32);
                            req_caseopt = 0;
                            reset_caseful = FALSE;
                        }

                        break 'item;
                    }
                } // End match gto
            } // End 'item loop

            pptr = pptr.add(1);
        } // End of the main per-item loop
    }
}
