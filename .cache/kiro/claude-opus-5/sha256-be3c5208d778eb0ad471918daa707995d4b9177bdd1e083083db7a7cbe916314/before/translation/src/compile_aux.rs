//! Translation of PART 4 of `pcre2_compile.c` (C lines 8574–10278):
//! auxiliary compile-time helpers.
//!
//! These are `static` functions in the C source. They are exposed here as
//! `pub(crate) unsafe fn` because they are called from `compile_branch.rs`
//! and `compile.rs` — they are NOT exported symbols (no `#[no_mangle]`,
//! no `extern "C"`).
//!
//! Functions translated:
//!   * `compile_regex`
//!   * `is_anchored`
//!   * `is_startline`
//!   * `find_firstassertedcu`
//!   * `parsed_skip`
//!   * `get_grouplength`
//!   * `get_branchlength`
//!   * `set_lookbehind_lengths`
//!   * `check_lookbehinds`

use core::ffi::c_int;
use core::ptr;

use crate::compile_h::*;
use crate::compile_local::*;
use crate::consts::*;
use crate::internal::*;
// Resolve glob-import ambiguities: these names exist in both `consts` (as
// `i64`) and `internal` (as the C `BOOL`/`PCRE2_SIZE` types). We want the
// `internal` versions here.
use crate::internal::{FALSE, PCRE2_UNSET, TRUE};

// Cross-module calls (implemented in sibling modules / in-progress work).
use crate::compile_branch::{compile_branch, first_significant_code};
use crate::compile_cgroup::_pcre2_compile_find_named_group8;

/// `PRIV(OP_lengths)` table.
#[inline(always)]
unsafe fn op_length(op: u32) -> usize {
    crate::tables::_pcre2_OP_lengths_8[op as usize] as usize
}

// ===========================================================================
// Compile a regular expression (branches within a bracket)
// ===========================================================================

/// `compile_regex` — compile all the alternatives of a bracketed group.
///
/// This function is mutually recursive with `compile_branch`.
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

        // If set, call the external function that checks for stack availability.
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

        // Miscellaneous initialization
        bc.outer = bcptr;
        bc.current_branch = code;

        firstcu = 0;
        reqcu = 0;
        firstcuflags = REQ_UNSET;
        reqcuflags = REQ_UNSET;

        // Accumulate the length for use in the pre-compile phase.
        length = (2 + 2 * LINK_SIZE_U + skipunits as usize) as PCRE2_SIZE;

        // Remember if this is a lookbehind assertion, and if it is, save its
        // length and skip over the pattern offset.
        lookbehind = (*code as u32 == OP_ASSERTBACK
            || *code as u32 == OP_ASSERTBACK_NOT
            || *code as u32 == OP_ASSERTBACK_NA) as BOOL;

        if lookbehind != 0 {
            lookbehindlength = META_DATA(*pptr.offset(-1));
            lookbehindminlength = *pptr;
            pptr = pptr.add(SIZEOFFSET as usize);
        } else {
            lookbehindlength = 0;
            lookbehindminlength = 0;
        }

        // If this is a capturing subpattern, add to the chain of open capturing
        // items so that we can detect them if (*ACCEPT) is encountered.
        if *code as u32 == OP_CBRA {
            capnumber = GET2(code, 1 + LINK_SIZE_U) as c_int;
            capitem.number = capnumber as u16;
            capitem.next = open_caps;
            capitem.assert_depth = (*cb).assert_depth;
            open_caps = &mut capitem;
        }

        // Offset is set zero to mark that this bracket is still open
        PUT(code, 1, 0);
        code = code.add(1 + LINK_SIZE_U + skipunits as usize);

        // Loop for each alternative branch
        loop {
            let branch_return: c_int;
            let mut branchfirstcu: u32 = 0;
            let mut branchreqcu: u32 = 0;
            let mut branchfirstcuflags: u32 = REQ_UNSET;
            let mut branchreqcuflags: u32 = REQ_UNSET;

            // Insert OP_REVERSE or OP_VREVERSE if this is a lookbehind assertion.
            if lookbehind != 0 && lookbehindlength > 0 {
                if lookbehindminlength == LOOKBEHIND_MAX as u32
                    || lookbehindminlength == lookbehindlength
                {
                    *code = OP_REVERSE as PCRE2_UCHAR;
                    code = code.add(1);
                    PUT2INC(&mut code, 0, lookbehindlength);
                    length += (1 + IMM2_SIZE_U) as PCRE2_SIZE;
                } else {
                    *code = OP_VREVERSE as PCRE2_UCHAR;
                    code = code.add(1);
                    PUT2INC(&mut code, 0, lookbehindminlength);
                    PUT2INC(&mut code, 0, lookbehindlength);
                    length += (1 + 2 * IMM2_SIZE_U) as PCRE2_SIZE;
                }
            }

            // Now compile the branch; in the pre-compile phase its length gets
            // added into the length.
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
                    ptr::null_mut()
                } else {
                    &mut length
                },
            );
            if branch_return == 0 {
                return 0;
            }

            // If a branch can match an empty string, so can the whole group.
            if branch_return < 0 {
                okreturn = -1;
            }

            // In the real compile phase, there is some post-processing to be done.
            if lengthptr.is_null() {
                // If this is the first branch, the firstcu and reqcu values for
                // the branch become the values for the regex.
                if *last_branch as u32 != OP_ALT {
                    firstcu = branchfirstcu;
                    firstcuflags = branchfirstcuflags;
                    reqcu = branchreqcu;
                    reqcuflags = branchreqcuflags;
                }
                // If this is not the first branch, the first char and reqcu
                // have to match the values from all the previous branches.
                else {
                    // If we previously had a firstcu, but it doesn't match the
                    // new branch, we have to abandon the firstcu for the regex,
                    // but if there was previously no reqcu, it takes on the value
                    // of the old firstcu.
                    if firstcuflags != branchfirstcuflags || firstcu != branchfirstcu {
                        if firstcuflags < REQ_NONE {
                            if reqcuflags >= REQ_NONE {
                                reqcu = firstcu;
                                reqcuflags = firstcuflags;
                            }
                        }
                        firstcuflags = REQ_NONE;
                    }

                    // If we (now or from before) have no firstcu, a firstcu from
                    // the branch becomes a reqcu if there isn't a branch reqcu.
                    if firstcuflags >= REQ_NONE
                        && branchfirstcuflags < REQ_NONE
                        && branchreqcuflags >= REQ_NONE
                    {
                        branchreqcu = branchfirstcu;
                        branchreqcuflags = branchfirstcuflags;
                    }

                    // Now ensure that the reqcus match
                    if ((reqcuflags & !REQ_VARY) != (branchreqcuflags & !REQ_VARY))
                        || reqcu != branchreqcu
                    {
                        reqcuflags = REQ_NONE;
                    } else {
                        reqcu = branchreqcu;
                        reqcuflags |= branchreqcuflags; // To "or" REQ_VARY if present
                    }
                }
            }

            // Handle reaching the end of the expression, either ')' or end of
            // pattern.
            if (META_CODE(*pptr) as i64) != META_ALT {
                if lengthptr.is_null() {
                    let mut branch_length: u32 = code.offset_from(last_branch) as u32;
                    loop {
                        let prev_length: u32 = GET(last_branch, 1);
                        PUT(last_branch, 1, branch_length as i32);
                        branch_length = prev_length;
                        last_branch = last_branch.offset(-(branch_length as isize));
                        if !(branch_length > 0) {
                            break;
                        }
                    }
                }

                // Fill in the ket
                *code = OP_KET as PCRE2_UCHAR;
                PUT(code, 1, code.offset_from(start_bracket) as i32);
                code = code.add(1 + LINK_SIZE_U);

                // Set values to pass back
                *codeptr = code;
                *pptrptr = pptr;
                *firstcuptr = firstcu;
                *firstcuflagsptr = firstcuflags;
                *reqcuptr = reqcu;
                *reqcuflagsptr = reqcuflags;
                if !lengthptr.is_null() {
                    if (OFLOW_MAX as PCRE2_SIZE) - *lengthptr < length {
                        *errorcodeptr = ERR20;
                        return 0;
                    }
                    *lengthptr += length;
                }
                return okreturn;
            }

            // Another branch follows.
            if !lengthptr.is_null() {
                code = (*codeptr).add(1 + LINK_SIZE_U + skipunits as usize);
                length += (1 + LINK_SIZE_U) as PCRE2_SIZE;
            } else {
                *code = OP_ALT as PCRE2_UCHAR;
                PUT(code, 1, code.offset_from(last_branch) as i32);
                last_branch = code;
                bc.current_branch = code;
                code = code.add(1 + LINK_SIZE_U);
            }

            // Set the maximum lookbehind length for the next branch and then
            // advance past the vertical bar.
            lookbehindlength = META_DATA(*pptr);
            pptr = pptr.add(1);
        }

        // Control should never reach here.
        #[allow(unreachable_code)]
        {
            0
        }
    }
}

// ===========================================================================
// Check for anchored pattern
// ===========================================================================

/// `is_anchored` — try to find out if this is an anchored regular expression.
pub(crate) unsafe fn is_anchored(
    mut code: PCRE2_SPTR,
    bracket_map: u32,
    cb: *mut compile_block,
    atomcount: c_int,
    inassert: BOOL,
    dotstar_anchor: BOOL,
) -> BOOL {
    unsafe {
        loop {
            let scode: PCRE2_SPTR =
                first_significant_code(code.add(op_length(*code as u32)), FALSE);
            let op = *scode as u32;

            // Non-capturing brackets
            if op == OP_BRA || op == OP_BRAPOS || op == OP_SBRA || op == OP_SBRAPOS {
                if is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == 0 {
                    return FALSE;
                }
            }
            // Capturing brackets
            else if op == OP_CBRA || op == OP_CBRAPOS || op == OP_SCBRA || op == OP_SCBRAPOS {
                let n = GET2(scode, 1 + LINK_SIZE_U);
                let new_map = bracket_map | (if n < 32 { 1u32 << n } else { 1 });
                if is_anchored(scode, new_map, cb, atomcount, inassert, dotstar_anchor) == 0 {
                    return FALSE;
                }
            }
            // Positive forward assertion
            else if op == OP_ASSERT || op == OP_ASSERT_NA {
                if is_anchored(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor) == 0 {
                    return FALSE;
                }
            }
            // Condition. If there is no second branch, it can't be anchored.
            else if op == OP_COND || op == OP_SCOND {
                if *scode.add(GET(scode, 1) as usize) as u32 != OP_ALT {
                    return FALSE;
                }
                if is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == 0 {
                    return FALSE;
                }
            }
            // Atomic groups
            else if op == OP_ONCE {
                if is_anchored(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor)
                    == 0
                {
                    return FALSE;
                }
            }
            // .* is not anchored unless DOTALL is set (which generates
            // OP_ALLANY) and it isn't in brackets that are or may be referenced
            // or inside an atomic group or an assertion.
            else if op == OP_TYPESTAR || op == OP_TYPEMINSTAR || op == OP_TYPEPOSSTAR {
                if *scode.add(1) as u32 != OP_ALLANY
                    || (bracket_map & (*cb).backref_map) != 0
                    || atomcount > 0
                    || (*cb).had_pruneorskip != 0
                    || inassert != 0
                    || dotstar_anchor == 0
                {
                    return FALSE;
                }
            }
            // Check for explicit anchoring
            else if op != OP_SOD && op != OP_SOM && op != OP_CIRC {
                return FALSE;
            }

            code = code.add(GET(code, 1) as usize);
            if !(*code as u32 == OP_ALT) {
                break;
            }
        }
        TRUE
    }
}

// ===========================================================================
// Check for starting with ^ or .*
// ===========================================================================

/// `is_startline` — find out if every branch starts with ^ or .*.
pub(crate) unsafe fn is_startline(
    mut code: PCRE2_SPTR,
    bracket_map: u32,
    cb: *mut compile_block,
    atomcount: c_int,
    inassert: BOOL,
    dotstar_anchor: BOOL,
) -> BOOL {
    unsafe {
        loop {
            let mut scode: PCRE2_SPTR =
                first_significant_code(code.add(op_length(*code as u32)), FALSE);
            let mut op = *scode as u32;

            // If we are at the start of a conditional assertion group, *both*
            // the conditional assertion *and* what follows the condition must
            // satisfy the test for start of line.
            if op == OP_COND {
                scode = scode.add(1 + LINK_SIZE_U);

                if *scode as u32 == OP_CALLOUT {
                    scode = scode.add(op_length(OP_CALLOUT));
                } else if *scode as u32 == OP_CALLOUT_STR {
                    scode = scode.add(GET(scode, 1 + 2 * LINK_SIZE_U) as usize);
                }

                match *scode as u32 {
                    OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF | OP_FAIL | OP_FALSE | OP_TRUE => {
                        return FALSE;
                    }
                    _ => {
                        // Assertion
                        if is_startline(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor)
                            == 0
                        {
                            return FALSE;
                        }
                        loop {
                            scode = scode.add(GET(scode, 1) as usize);
                            if !(*scode as u32 == OP_ALT) {
                                break;
                            }
                        }
                        scode = scode.add(1 + LINK_SIZE_U);
                    }
                }
                scode = first_significant_code(scode, FALSE);
                op = *scode as u32;
            }

            // Non-capturing brackets
            if op == OP_BRA || op == OP_BRAPOS || op == OP_SBRA || op == OP_SBRAPOS {
                if is_startline(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == 0 {
                    return FALSE;
                }
            }
            // Capturing brackets
            else if op == OP_CBRA || op == OP_CBRAPOS || op == OP_SCBRA || op == OP_SCBRAPOS {
                let n = GET2(scode, 1 + LINK_SIZE_U);
                let new_map = bracket_map | (if n < 32 { 1u32 << n } else { 1 });
                if is_startline(scode, new_map, cb, atomcount, inassert, dotstar_anchor) == 0 {
                    return FALSE;
                }
            }
            // Positive forward assertions
            else if op == OP_ASSERT || op == OP_ASSERT_NA {
                if is_startline(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor) == 0 {
                    return FALSE;
                }
            }
            // Atomic brackets
            else if op == OP_ONCE {
                if is_startline(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor)
                    == 0
                {
                    return FALSE;
                }
            }
            // .* means "start at start or after \n" if it isn't in atomic
            // brackets or brackets that may be referenced or an assertion.
            else if op == OP_TYPESTAR || op == OP_TYPEMINSTAR || op == OP_TYPEPOSSTAR {
                if *scode.add(1) as u32 != OP_ANY
                    || (bracket_map & (*cb).backref_map) != 0
                    || atomcount > 0
                    || (*cb).had_pruneorskip != 0
                    || inassert != 0
                    || dotstar_anchor == 0
                {
                    return FALSE;
                }
            }
            // Check for explicit circumflex; anything else gives a FALSE result.
            else if op != OP_CIRC && op != OP_CIRCM {
                return FALSE;
            }

            // Move on to the next alternative
            code = code.add(GET(code, 1) as usize);
            if !(*code as u32 == OP_ALT) {
                break;
            }
        }
        TRUE
    }
}

// ===========================================================================
// Check for asserted fixed first code unit
// ===========================================================================

/// `find_firstassertedcu` — scan the regex for an initial asserted first code
/// unit that is common to all branches.
pub(crate) unsafe fn find_firstassertedcu(
    mut code: PCRE2_SPTR,
    flags: *mut u32,
    inassert: u32,
) -> u32 {
    unsafe {
        let mut c: u32 = 0;
        let mut cflags: u32 = REQ_NONE;

        *flags = REQ_NONE;
        loop {
            let d: u32;
            let mut dflags: u32 = 0;
            let xl = if *code as u32 == OP_CBRA
                || *code as u32 == OP_SCBRA
                || *code as u32 == OP_CBRAPOS
                || *code as u32 == OP_SCBRAPOS
            {
                IMM2_SIZE_U
            } else {
                0
            };
            let mut scode: PCRE2_SPTR =
                first_significant_code(code.add(1 + LINK_SIZE_U + xl), TRUE);
            let op = *scode as u32;

            match op {
                OP_BRA | OP_BRAPOS | OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS | OP_ASSERT
                | OP_ASSERT_NA | OP_ONCE | OP_SCRIPT_RUN => {
                    d = find_firstassertedcu(
                        scode,
                        &mut dflags,
                        inassert + (if op == OP_ASSERT || op == OP_ASSERT_NA { 1 } else { 0 }),
                    );
                    if dflags >= REQ_NONE {
                        return 0;
                    }
                    if cflags >= REQ_NONE {
                        c = d;
                        cflags = dflags;
                    } else if c != d || cflags != dflags {
                        return 0;
                    }
                }

                OP_EXACT => {
                    scode = scode.add(IMM2_SIZE_U);
                    // Fall through to OP_CHAR handling.
                    if inassert == 0 {
                        return 0;
                    }
                    if cflags >= REQ_NONE {
                        c = *scode.add(1) as u32;
                        cflags = 0;
                    } else if c != *scode.add(1) as u32 {
                        return 0;
                    }
                }

                OP_CHAR | OP_PLUS | OP_MINPLUS | OP_POSPLUS => {
                    if inassert == 0 {
                        return 0;
                    }
                    if cflags >= REQ_NONE {
                        c = *scode.add(1) as u32;
                        cflags = 0;
                    } else if c != *scode.add(1) as u32 {
                        return 0;
                    }
                }

                OP_EXACTI => {
                    scode = scode.add(IMM2_SIZE_U);
                    // Fall through to OP_CHARI handling.
                    if inassert == 0 {
                        return 0;
                    }
                    // If the character is more than one code unit long, we
                    // cannot set its first code unit when matching caselessly.
                    if *scode.add(1) as u32 >= 0x80 {
                        return 0;
                    }
                    if cflags >= REQ_NONE {
                        c = *scode.add(1) as u32;
                        cflags = REQ_CASELESS;
                    } else if c != *scode.add(1) as u32 {
                        return 0;
                    }
                }

                OP_CHARI | OP_PLUSI | OP_MINPLUSI | OP_POSPLUSI => {
                    if inassert == 0 {
                        return 0;
                    }
                    if *scode.add(1) as u32 >= 0x80 {
                        return 0;
                    }
                    if cflags >= REQ_NONE {
                        c = *scode.add(1) as u32;
                        cflags = REQ_CASELESS;
                    } else if c != *scode.add(1) as u32 {
                        return 0;
                    }
                }

                _ => {
                    return 0;
                }
            }

            code = code.add(GET(code, 1) as usize);
            if !(*code as u32 == OP_ALT) {
                break;
            }
        }

        *flags = cflags;
        c
    }
}

// ===========================================================================
// Skip in parsed pattern
// ===========================================================================

/// `parsed_skip` — skip parts of the parsed pattern when finding the length of
/// a lookbehind branch. Returns NULL if the parsed regex is malformed.
pub(crate) unsafe fn parsed_skip(mut pptr: *mut u32, skiptype: u32) -> *mut u32 {
    unsafe {
        let mut nestlevel: u32 = 0;

        loop {
            let mut meta = META_CODE(*pptr);
            // `true` means the C `default: continue;` path (a literal): advance
            // one element and restart the loop, skipping the extra-length table.
            let mut is_literal_continue = false;

            match meta as i64 {
                // The parsed regex is malformed; we reached the end and did
                // not find the end of the construct being skipped.
                META_END => {
                    return ptr::null_mut();
                }

                // The data for these items is variable in length.
                META_BACKREF => {
                    // Offset is present only if group >= 10
                    if META_DATA(*pptr) >= 10 {
                        pptr = pptr.add(SIZEOFFSET as usize);
                    }
                }

                META_ESCAPE => {
                    if *pptr - (META_ESCAPE as u32) == ESC_P
                        || *pptr - (META_ESCAPE as u32) == ESC_p
                    {
                        pptr = pptr.add(1); // Skip prop data
                    }
                }

                META_MARK | META_COMMIT_ARG | META_PRUNE_ARG | META_SKIP_ARG
                | META_THEN_ARG => {
                    // Add the length of the name.
                    pptr = pptr.add(*pptr.add(1) as usize);
                }

                // These are the "active" items in this loop.
                META_CLASS_END => {
                    if skiptype == PSKIP_CLASS {
                        return pptr;
                    }
                }

                META_ATOMIC | META_CAPTURE | META_COND_ASSERT | META_COND_DEFINE
                | META_COND_NAME | META_COND_NUMBER | META_COND_RNAME | META_COND_RNUMBER
                | META_COND_VERSION | META_SCS | META_LOOKAHEAD | META_LOOKAHEADNOT
                | META_LOOKAHEAD_NA | META_LOOKBEHIND | META_LOOKBEHINDNOT
                | META_LOOKBEHIND_NA | META_NOCAPTURE | META_SCRIPT_RUN => {
                    nestlevel += 1;
                }

                META_ALT => {
                    if nestlevel == 0 && skiptype == PSKIP_ALT {
                        return pptr;
                    }
                }

                META_KET => {
                    if nestlevel == 0 {
                        return pptr;
                    }
                    nestlevel -= 1;
                }

                _ => {
                    // Just skip over most items
                    if meta < META_END as u32 {
                        // Literal: C `continue`s the loop here.
                        is_literal_continue = true;
                    }
                    // otherwise fall out of the match to the extra-length
                    // handling below (C `break`).
                }
            }

            if is_literal_continue {
                pptr = pptr.add(1);
                continue;
            }

            // The extra data item length for each meta is in a table.
            meta = (meta >> 16) & 0x7fff;
            if meta as usize >= META_EXTRA_LENGTHS.len() {
                return ptr::null_mut();
            }
            pptr = pptr.add(META_EXTRA_LENGTHS[meta as usize] as usize);

            pptr = pptr.add(1);
        }
    }
}

// ===========================================================================
// Find length of a parsed group
// ===========================================================================

/// `get_grouplength` — find the maximum (and minimum) length of a nested group
/// within a lookbehind branch, using caching for capturing groups.
pub(crate) unsafe fn get_grouplength(
    pptrptr: *mut *mut u32,
    minptr: *mut c_int,
    isinline: BOOL,
    errcodeptr: *mut c_int,
    lcptr: *mut c_int,
    group: c_int,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> c_int {
    unsafe {
        let gi: *mut u32 = (*cb).groupinfo.add(2 * group as usize);
        let mut branchlength: c_int;
        let mut branchminlength: c_int = 0;
        let mut grouplength: c_int = -1;
        let mut groupminlength: c_int = c_int::MAX;

        // The cache can be used only if there is no possibility of there being
        // two groups with the same number.
        if group > 0 && ((*cb).external_flags & PCRE2_DUPCAPUSED as u32) == 0 {
            let groupinfo = *gi.add(0);
            if (groupinfo & GI_NOT_FIXED_LENGTH) != 0 {
                return -1;
            }
            if (groupinfo & GI_SET_FIXED_LENGTH) != 0 {
                if isinline != 0 {
                    *pptrptr = parsed_skip(*pptrptr, PSKIP_KET);
                }
                *minptr = *gi.add(1) as c_int;
                return (groupinfo & GI_FIXED_LENGTH_MASK) as c_int;
            }
        }

        // Scan the group. In this case we find the end pointer of necessity.
        let is_not_fixed;
        loop {
            branchlength = get_branchlength(
                pptrptr,
                &mut branchminlength,
                errcodeptr,
                lcptr,
                recurses,
                cb,
            );
            if branchlength < 0 {
                is_not_fixed = true;
                break;
            }
            if branchlength > grouplength {
                grouplength = branchlength;
            }
            if branchminlength < groupminlength {
                groupminlength = branchminlength;
            }
            if (**pptrptr as i64) == META_KET {
                is_not_fixed = false;
                break;
            }
            *pptrptr = (*pptrptr).add(1); // Skip META_ALT
        }

        if is_not_fixed {
            // ISNOTFIXED:
            if group > 0 {
                *gi.add(0) |= GI_NOT_FIXED_LENGTH;
            }
            return -1;
        }

        if group > 0 {
            *gi.add(0) |= GI_SET_FIXED_LENGTH | grouplength as u32;
            *gi.add(1) = groupminlength as u32;
        }

        *minptr = groupminlength;
        grouplength
    }
}

// ===========================================================================
// Find length of a parsed branch
// ===========================================================================

/// `get_branchlength` — return fixed maximum and minimum lengths for a branch
/// in a lookbehind, giving an error if the length is not limited.
pub(crate) unsafe fn get_branchlength(
    pptrptr: *mut *mut u32,
    minptr: *mut c_int,
    errcodeptr: *mut c_int,
    lcptr: *mut c_int,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> c_int {
    unsafe {
        let mut branchlength: c_int = 0;
        let mut branchminlength: c_int = 0;
        let mut grouplength: c_int;
        let mut groupminlength: c_int = 0;
        let mut lastitemlength: u32 = 0;
        let mut lastitemminlength: u32 = 0;
        let mut pptr: *mut u32 = *pptrptr;
        let mut offset: PCRE2_SIZE = 0;
        let mut this_recurse: parsed_recurse_check = core::mem::zeroed();

        // A large and/or complex regex can take too long to process.
        {
            let lc = *lcptr;
            *lcptr = lc + 1;
            if lc > 2000 {
                *errcodeptr = ERR35; // Lookbehind is too complicated
                return -1;
            }
        }

        // Scan the branch, accumulating the length.
        loop {
            let mut itemlength: u32 = 0;
            let mut itemminlength: u32 = 0;
            let escape: u32;
            let mut group: u32 = 0;

            // Label targets emulated via flags/blocks.
            let mut goto_isnotfixed = false;
            let mut goto_recurse_or_backref = false;
            let mut goto_check_group = false;
            let mut goto_exit = false;
            // For the REPETITION shared code: Some((min, max)) if a repetition
            // must be processed after the match.
            let mut repetition_state: Option<(u32, u32)> = None;

            if (*pptr) < META_END as u32 {
                itemlength = 1;
                itemminlength = 1;
            } else {
                match META_CODE(*pptr) as i64 {
                    META_KET | META_ALT => {
                        goto_exit = true;
                    }

                    // (*ACCEPT) and (*FAIL) terminate the branch, but we must
                    // skip to the actual termination.
                    META_ACCEPT | META_FAIL => {
                        pptr = parsed_skip(pptr, PSKIP_ALT);
                        if pptr.is_null() {
                            // PARSED_SKIP_FAILED
                            *errcodeptr = ERR90;
                            return -1;
                        }
                        goto_exit = true;
                    }

                    META_MARK | META_COMMIT_ARG | META_PRUNE_ARG | META_SKIP_ARG
                    | META_THEN_ARG => {
                        pptr = pptr.add(*pptr.add(1) as usize + 1);
                    }

                    META_CIRCUMFLEX | META_COMMIT | META_DOLLAR | META_PRUNE | META_SKIP
                    | META_THEN => {}

                    META_OPTIONS => {
                        pptr = pptr.add(2);
                    }

                    META_BIGVALUE => {
                        itemlength = 1;
                        itemminlength = 1;
                        pptr = pptr.add(1);
                    }

                    META_CLASS | META_CLASS_NOT => {
                        itemlength = 1;
                        itemminlength = 1;
                        pptr = parsed_skip(pptr, PSKIP_CLASS);
                        if pptr.is_null() {
                            *errcodeptr = ERR90;
                            return -1;
                        }
                    }

                    META_CLASS_EMPTY_NOT | META_DOT => {
                        itemlength = 1;
                        itemminlength = 1;
                    }

                    META_CALLOUT_NUMBER => {
                        pptr = pptr.add(3);
                    }

                    META_CALLOUT_STRING => {
                        pptr = pptr.add(3 + SIZEOFFSET as usize);
                    }

                    // Only some escapes consume a character.
                    META_ESCAPE => {
                        escape = META_DATA(*pptr);
                        if escape == ESC_X {
                            return -1;
                        }
                        if escape == ESC_R {
                            itemminlength = 1;
                            itemlength = 2;
                        } else if escape > ESC_b && escape < ESC_Z {
                            if ((*cb).external_options & PCRE2_UTF as u32) != 0 && escape == ESC_C {
                                *errcodeptr = ERR36;
                                return -1;
                            }
                            itemlength = 1;
                            itemminlength = 1;
                            if escape == ESC_p || escape == ESC_P {
                                pptr = pptr.add(1); // Skip prop data
                            }
                        }
                    }

                    // Lookaheads do not contribute to the length of this branch,
                    // but they may contain lookbehinds within them whose lengths
                    // need to be set.
                    META_LOOKAHEAD | META_LOOKAHEADNOT | META_LOOKAHEAD_NA | META_SCS => {
                        *errcodeptr =
                            check_lookbehinds(pptr.add(1), &mut pptr, recurses, cb, lcptr);
                        if *errcodeptr != 0 {
                            return -1;
                        }

                        // Ignore any qualifiers that follow a lookahead assertion.
                        match *pptr.add(1) as i64 {
                            META_ASTERISK | META_ASTERISK_PLUS | META_ASTERISK_QUERY
                            | META_PLUS | META_PLUS_PLUS | META_PLUS_QUERY | META_QUERY
                            | META_QUERY_PLUS | META_QUERY_QUERY => {
                                pptr = pptr.add(1);
                            }

                            META_MINMAX | META_MINMAX_PLUS | META_MINMAX_QUERY => {
                                pptr = pptr.add(3);
                            }

                            _ => {}
                        }
                    }

                    // A nested lookbehind does not contribute any length to this
                    // lookbehind, but must itself be checked and have its lengths
                    // set.
                    META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA => {
                        if set_lookbehind_lengths(&mut pptr, errcodeptr, lcptr, recurses, cb) == 0 {
                            return -1;
                        }
                    }

                    // Back references and recursions.
                    META_BACKREF_BYNAME => {
                        if ((*cb).external_options & PCRE2_MATCH_UNSET_BACKREF as u32) != 0 {
                            goto_isnotfixed = true;
                        } else {
                            // Fall through to META_RECURSE_BYNAME logic.
                            let (gi_res, isnotfixed) =
                                byname_backref(&mut pptr, &mut offset, errcodeptr, cb);
                            if let Some(g) = gi_res {
                                group = g;
                                goto_recurse_or_backref = true;
                            } else if isnotfixed {
                                goto_isnotfixed = true;
                            } else {
                                return -1;
                            }
                        }
                    }

                    META_RECURSE_BYNAME => {
                        let (gi_res, isnotfixed) =
                            byname_backref(&mut pptr, &mut offset, errcodeptr, cb);
                        if let Some(g) = gi_res {
                            group = g;
                            goto_recurse_or_backref = true;
                        } else if isnotfixed {
                            goto_isnotfixed = true;
                        } else {
                            return -1;
                        }
                    }

                    // The offset values for back references < 10 are in a
                    // separate vector.
                    META_BACKREF => {
                        if ((*cb).external_options & PCRE2_MATCH_UNSET_BACKREF as u32) != 0
                            || ((*cb).external_flags & PCRE2_DUPCAPUSED as u32) != 0
                        {
                            goto_isnotfixed = true;
                        } else {
                            group = META_DATA(*pptr);
                            if group < 10 {
                                offset = (*cb).small_ref_offset[group as usize];
                                goto_recurse_or_backref = true;
                            } else {
                                // Fall through to META_RECURSE (groups >= 10).
                                group = META_DATA(*pptr);
                                offset = GETPLUSOFFSET(&mut pptr);
                                goto_recurse_or_backref = true;
                            }
                        }
                    }

                    // A true recursion implies not fixed length, but a
                    // subroutine call may be OK.
                    META_RECURSE => {
                        group = META_DATA(*pptr);
                        offset = GETPLUSOFFSET(&mut pptr);
                        goto_recurse_or_backref = true;
                    }

                    // A (DEFINE) group is never obeyed inline.
                    META_COND_DEFINE => {
                        pptr = parsed_skip(pptr.add(1), PSKIP_KET);
                    }

                    // Check other nested groups.
                    META_COND_NAME | META_COND_NUMBER | META_COND_RNAME | META_COND_RNUMBER => {
                        pptr = pptr.add(2 + SIZEOFFSET as usize);
                        goto_check_group = true;
                    }

                    META_COND_ASSERT => {
                        pptr = pptr.add(1);
                        goto_check_group = true;
                    }

                    META_COND_VERSION => {
                        pptr = pptr.add(4);
                        goto_check_group = true;
                    }

                    META_CAPTURE => {
                        group = META_DATA(*pptr);
                        // Fall through to META_ATOMIC handling.
                        pptr = pptr.add(1);
                        goto_check_group = true;
                    }

                    META_ATOMIC | META_NOCAPTURE | META_SCRIPT_RUN => {
                        pptr = pptr.add(1);
                        goto_check_group = true;
                    }

                    META_QUERY | META_QUERY_PLUS | META_QUERY_QUERY => {
                        // min = 0, max = 1
                        repetition_state = Some((0, 1));
                    }

                    META_MINMAX | META_MINMAX_PLUS | META_MINMAX_QUERY => {
                        let min = *pptr.add(1);
                        let max = *pptr.add(2);
                        pptr = pptr.add(2);
                        repetition_state = Some((min, max));
                    }

                    _ => {
                        // Any other item means this branch does not have a fixed
                        // length.
                        goto_isnotfixed = true;
                    }
                }
            }

            // --- Emulate the C gotos in order ----------------------------------

            // CHECK_GROUP:
            if goto_check_group {
                grouplength = get_grouplength(
                    &mut pptr,
                    &mut groupminlength,
                    TRUE,
                    errcodeptr,
                    lcptr,
                    group as c_int,
                    recurses,
                    cb,
                );
                if grouplength < 0 {
                    return -1;
                }
                itemlength = grouplength as u32;
                itemminlength = groupminlength as u32;
            }

            // RECURSE_OR_BACKREF_LENGTH:
            if goto_recurse_or_backref {
                if group > (*cb).bracount {
                    (*cb).erroroffset = offset;
                    *errcodeptr = ERR15; // Non-existent subpattern
                    return -1;
                }
                if group == 0 {
                    goto_isnotfixed = true; // Local recursion
                } else {
                    let mut gptr: *mut u32 = (*cb).parsed_pattern;
                    while *gptr != META_END as u32 {
                        if (META_CODE(*gptr) as i64) == META_BIGVALUE {
                            gptr = gptr.add(1);
                        } else if *gptr == (META_CAPTURE as u32) | group {
                            break;
                        }
                        gptr = gptr.add(1);
                    }

                    // We must start the search for the end of the group at the
                    // first meta code inside the group.
                    let gptrend = parsed_skip(gptr.add(1), PSKIP_KET);
                    if gptrend.is_null() {
                        *errcodeptr = ERR90;
                        return -1;
                    }
                    if pptr > gptr && pptr < gptrend {
                        goto_isnotfixed = true; // Local recursion
                    } else {
                        let mut r: *mut parsed_recurse_check = recurses;
                        while !r.is_null() {
                            if (*r).groupptr == gptr {
                                break;
                            }
                            r = (*r).prev;
                        }
                        if !r.is_null() {
                            goto_isnotfixed = true; // Mutual recursion
                        } else {
                            this_recurse.prev = recurses;
                            this_recurse.groupptr = gptr;

                            let mut gptr2 = gptr.add(1);
                            grouplength = get_grouplength(
                                &mut gptr2,
                                &mut groupminlength,
                                FALSE,
                                errcodeptr,
                                lcptr,
                                group as c_int,
                                &mut this_recurse,
                                cb,
                            );
                            if grouplength < 0 {
                                if *errcodeptr == 0 {
                                    goto_isnotfixed = true;
                                } else {
                                    return -1; // Error already set
                                }
                            } else {
                                itemlength = grouplength as u32;
                                itemminlength = groupminlength as u32;
                            }
                        }
                    }
                }
            }

            // REPETITION shared code.
            if let Some((min, max)) = repetition_state {
                if max != REPEAT_UNLIMITED {
                    if lastitemlength != 0
                        && max != 0
                        && (c_int::MAX - branchlength) as u32 / lastitemlength < max - 1
                    {
                        *errcodeptr = ERR87; // Integer overflow; lookbehind too big
                        return -1;
                    }
                    if min == 0 {
                        branchminlength -= lastitemminlength as c_int;
                    } else {
                        itemminlength = (min - 1) * lastitemminlength;
                    }
                    if max == 0 {
                        branchlength -= lastitemlength as c_int;
                    } else {
                        itemlength = (max - 1) * lastitemlength;
                    }
                    // Fall to length accumulation below.
                } else {
                    // Unlimited repetition -> not fixed length.
                    goto_isnotfixed = true;
                }
            }

            // ISNOTFIXED:
            if goto_isnotfixed {
                *errcodeptr = ERR25; // Not fixed length
                return -1;
            }

            // EXIT: (from META_KET / META_ALT / ACCEPT / FAIL)
            if goto_exit {
                *pptrptr = pptr;
                *minptr = branchminlength;
                return branchlength;
            }

            // Add the item length to the branchlength, checking for integer
            // overflow and for the branch length exceeding the overall limit.
            if (c_int::MAX - branchlength) < itemlength as c_int || {
                branchlength += itemlength as c_int;
                branchlength > LOOKBEHIND_MAX
            } {
                *errcodeptr = ERR87;
                return -1;
            }

            branchminlength += itemminlength as c_int;

            // Save this item length for use if the next item is a quantifier.
            lastitemlength = itemlength;
            lastitemminlength = itemminlength;

            pptr = pptr.add(1);
        }
    }
}

// Helper that emulates the shared META_BACKREF_BYNAME / META_RECURSE_BYNAME
// code path in get_branchlength. Returns:
//   (Some(group), _)      -> jump to RECURSE_OR_BACKREF_LENGTH with `group`
//   (None, true)          -> jump to ISNOTFIXED
//   (None, false)         -> a hard error was set; caller must return -1
unsafe fn byname_backref(
    pptr: &mut *mut u32,
    offset: &mut PCRE2_SIZE,
    errcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> (Option<u32>, bool) {
    unsafe {
        let meta_code = META_CODE(**pptr);
        *pptr = pptr.add(1);
        let length = **pptr;

        *offset = GETPLUSOFFSET(pptr);
        let name: PCRE2_SPTR = (*cb).start_pattern.add(*offset);
        let ng = _pcre2_compile_find_named_group8(name, length, cb);

        if ng.is_null() {
            *errcodeptr = ERR15; // Non-existent subpattern
            (*cb).erroroffset = *offset;
            return (None, false);
        }

        let group = (*ng).number;
        let is_dupname = ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME as u16) != 0;

        // A numerical back reference can be fixed length if duplicate capturing
        // groups are not being used. A non-duplicate named back reference can
        // also be handled.
        if meta_code as i64 == META_RECURSE_BYNAME
            || (!is_dupname && ((*cb).external_flags & PCRE2_DUPCAPUSED as u32) == 0)
        {
            return (Some(group), false); // Handle as a numbered version.
        }

        (None, true) // Duplicate name or number -> ISNOTFIXED
    }
}

// ===========================================================================
// Set lengths in a lookbehind
// ===========================================================================

/// `set_lookbehind_lengths` — set the lengths in the branches of a lookbehind.
pub(crate) unsafe fn set_lookbehind_lengths(
    pptrptr: *mut *mut u32,
    errcodeptr: *mut c_int,
    lcptr: *mut c_int,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> BOOL {
    unsafe {
        let offset: PCRE2_SIZE;
        let mut bptr: *mut u32 = *pptrptr;
        let gbptr: *mut u32 = bptr;
        let mut maxlength: c_int = 0;
        let mut minlength: c_int = c_int::MAX;
        let mut variable: BOOL = FALSE;

        offset = READPLUSOFFSET(bptr); // Offset for error messages
        *pptrptr = (*pptrptr).add(SIZEOFFSET as usize);

        // Each branch can have a different maximum length, but we can keep only
        // a single minimum for the whole group.
        loop {
            let mut branchminlength: c_int = 0;

            *pptrptr = (*pptrptr).add(1);
            let branchlength = get_branchlength(
                pptrptr,
                &mut branchminlength,
                errcodeptr,
                lcptr,
                recurses,
                cb,
            );

            if branchlength < 0 {
                // The errorcode and offset may already be set from a nested
                // lookbehind.
                if *errcodeptr == 0 {
                    *errcodeptr = ERR25;
                }
                if (*cb).erroroffset == PCRE2_UNSET {
                    (*cb).erroroffset = offset;
                }
                return FALSE;
            }

            if branchlength != branchminlength {
                variable = TRUE;
            }
            if branchminlength < minlength {
                minlength = branchminlength;
            }
            if branchlength > maxlength {
                maxlength = branchlength;
            }
            if branchlength > (*cb).max_lookbehind {
                (*cb).max_lookbehind = branchlength;
            }
            *bptr |= branchlength as u32; // branchlength never more than 65535
            bptr = *pptrptr;

            if !((META_CODE(*bptr) as i64) == META_ALT) {
                break;
            }
        }

        // If any branch is of variable length, the whole lookbehind is of
        // variable length.
        if variable != 0 {
            *gbptr.add(1) = minlength as u32;
            if (maxlength as PCRE2_SIZE) > (*cb).max_varlookbehind as PCRE2_SIZE {
                *errcodeptr = ERR100;
                (*cb).erroroffset = offset;
                return FALSE;
            }
        } else {
            *gbptr.add(1) = LOOKBEHIND_MAX as u32;
        }

        TRUE
    }
}

// ===========================================================================
// Check parsed pattern lookbehinds
// ===========================================================================

/// `check_lookbehinds` — scan the parsed pattern for lookbehinds, calling
/// `set_lookbehind_lengths` for each one. Returns 0 on success or an errorcode.
pub(crate) unsafe fn check_lookbehinds(
    mut pptr: *mut u32,
    retptr: *mut *mut u32,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
    lcptr: *mut c_int,
) -> c_int {
    unsafe {
        let mut errorcode: c_int = 0;
        let mut nestlevel: c_int = 0;

        (*cb).erroroffset = PCRE2_UNSET;

        while *pptr != META_END as u32 {
            if *pptr < META_END as u32 {
                // Literal
                pptr = pptr.add(1);
                continue;
            }

            match META_CODE(*pptr) as i64 {
                META_ESCAPE => {
                    if *pptr - (META_ESCAPE as u32) == ESC_P
                        || *pptr - (META_ESCAPE as u32) == ESC_p
                    {
                        pptr = pptr.add(1); // Skip prop data
                    }
                }

                META_KET => {
                    nestlevel -= 1;
                    if nestlevel < 0 {
                        if !retptr.is_null() {
                            *retptr = pptr;
                        }
                        return 0;
                    }
                }

                META_ATOMIC | META_CAPTURE | META_COND_ASSERT | META_SCS | META_LOOKAHEAD
                | META_LOOKAHEADNOT | META_LOOKAHEAD_NA | META_NOCAPTURE | META_SCRIPT_RUN => {
                    nestlevel += 1;
                }

                META_ACCEPT | META_ALT | META_ASTERISK | META_ASTERISK_PLUS
                | META_ASTERISK_QUERY | META_BACKREF | META_CIRCUMFLEX | META_CLASS
                | META_CLASS_EMPTY | META_CLASS_EMPTY_NOT | META_CLASS_END | META_CLASS_NOT
                | META_COMMIT | META_DOLLAR | META_DOT | META_FAIL | META_PLUS
                | META_PLUS_PLUS | META_PLUS_QUERY | META_PRUNE | META_QUERY | META_QUERY_PLUS
                | META_QUERY_QUERY | META_RANGE_ESCAPED | META_RANGE_LITERAL | META_SKIP
                | META_THEN => {}

                META_OFFSET | META_RECURSE => {
                    pptr = pptr.add(SIZEOFFSET as usize);
                }

                META_BACKREF_BYNAME | META_RECURSE_BYNAME => {
                    pptr = pptr.add(1 + SIZEOFFSET as usize);
                }

                META_COND_DEFINE => {
                    pptr = pptr.add(SIZEOFFSET as usize);
                    nestlevel += 1;
                }

                META_COND_NAME | META_COND_NUMBER | META_COND_RNAME | META_COND_RNUMBER => {
                    pptr = pptr.add(1 + SIZEOFFSET as usize);
                    nestlevel += 1;
                }

                META_COND_VERSION => {
                    pptr = pptr.add(3);
                    nestlevel += 1;
                }

                META_CALLOUT_STRING => {
                    pptr = pptr.add(3 + SIZEOFFSET as usize);
                }

                META_BIGVALUE | META_POSIX | META_POSIX_NEG | META_CAPTURE_NAME
                | META_CAPTURE_NUMBER => {
                    pptr = pptr.add(1);
                }

                META_MINMAX | META_MINMAX_QUERY | META_MINMAX_PLUS | META_OPTIONS => {
                    pptr = pptr.add(2);
                }

                META_CALLOUT_NUMBER => {
                    pptr = pptr.add(3);
                }

                META_MARK | META_COMMIT_ARG | META_PRUNE_ARG | META_SKIP_ARG | META_THEN_ARG => {
                    pptr = pptr.add(1 + *pptr.add(1) as usize);
                }

                // Note that set_lookbehind_lengths() updates pptr, leaving it
                // pointing to the final ket of the group.
                META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA => {
                    if set_lookbehind_lengths(&mut pptr, &mut errorcode, lcptr, recurses, cb) == 0 {
                        return errorcode;
                    }
                }

                _ => {
                    // Unrecognized meta code (should be unreachable).
                    (*cb).erroroffset = 0;
                    return ERR70;
                }
            }

            pptr = pptr.add(1);
        }

        0
    }
}
