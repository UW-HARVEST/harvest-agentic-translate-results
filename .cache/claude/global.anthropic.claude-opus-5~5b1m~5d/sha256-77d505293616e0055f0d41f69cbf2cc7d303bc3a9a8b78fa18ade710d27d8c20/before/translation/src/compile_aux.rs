//! Translated from pcre2_compile.c, lines 8575-10279 (compile_regex, is_anchored, is_startline, find_recurse, find_firstassertedcu, parsed_skip, get_grouplength, get_branchlength, set_lookbehind_lengths, check_lookbehinds).
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use crate::compile_tables::*;
use crate::compile::*;
use crate::compile_parse::*;
use crate::compile_branch::*;
use crate::compile_cgroup::*;
use crate::tables::_pcre2_OP_lengths_8;
use core::ffi::{c_char, c_void};

/* INT_MAX from <limits.h> */
const INT_MAX: i32 = 2147483647;

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

pub(crate) unsafe fn compile_regex(
    options: u32,
    xoptions: u32,
    codeptr: *mut *mut PCRE2_UCHAR,
    pptrptr: *mut *mut u32,
    errorcodeptr: *mut i32,
    skipunits: u32,
    firstcuptr: *mut u32,
    firstcuflagsptr: *mut u32,
    reqcuptr: *mut u32,
    reqcuflagsptr: *mut u32,
    bcptr: *mut branch_chain,
    open_caps: *mut open_capitem,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> i32 {
    let mut options: u32 = options;
    let mut xoptions: u32 = xoptions;
    let mut open_caps: *mut open_capitem = open_caps;

    let mut code: *mut PCRE2_UCHAR = *codeptr;
    let mut last_branch: *mut PCRE2_UCHAR = code;
    let start_bracket: *mut PCRE2_UCHAR = code;
    let lookbehind: BOOL;
    let mut capitem: open_capitem = open_capitem {
        next: core::ptr::null_mut(),
        number: 0,
        assert_depth: 0,
    };
    let mut capnumber: i32 = 0;
    let mut okreturn: i32 = 1;
    let mut pptr: *mut u32 = *pptrptr;
    let mut firstcu: u32;
    let mut reqcu: u32;
    let mut lookbehindlength: u32;
    let mut lookbehindminlength: u32;
    let mut firstcuflags: u32;
    let mut reqcuflags: u32;
    let mut length: PCRE2_SIZE;
    let mut bc: branch_chain = branch_chain {
        outer: core::ptr::null_mut(),
        current_branch: core::ptr::null_mut(),
    };

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

    reqcu = 0;
    firstcu = reqcu;
    reqcuflags = REQ_UNSET;
    firstcuflags = reqcuflags;

    /* Accumulate the length for use in the pre-compile phase. Start with the
    length of the BRA and KET and any extra code units that are required at the
    beginning. We accumulate in a local variable to save frequent testing of
    lengthptr for NULL. We cannot do this by looking at the value of 'code' at the
    start and end of each alternative, because compiled items are discarded during
    the pre-compile phase so that the workspace is not exceeded. */

    length = (2 + 2 * LINK_SIZE) as PCRE2_SIZE + skipunits as PCRE2_SIZE;

    /* Remember if this is a lookbehind assertion, and if it is, save its length
    and skip over the pattern offset. */

    lookbehind = (*code as u32 == OP_ASSERTBACK
        || *code as u32 == OP_ASSERTBACK_NOT
        || *code as u32 == OP_ASSERTBACK_NA) as BOOL;

    if lookbehind != 0 {
        lookbehindlength = META_DATA!(*pptr.offset(-1));
        lookbehindminlength = *pptr;
        pptr = pptr.add(SIZEOFFSET);
    } else {
        lookbehindminlength = 0;
        lookbehindlength = lookbehindminlength;
    }

    /* If this is a capturing subpattern, add to the chain of open capturing items
    so that we can detect them if (*ACCEPT) is encountered. Note that only OP_CBRA
    need be tested here; changing this opcode to one of its variants, e.g.
    OP_SCBRAPOS, happens later, after the group has been compiled. */

    if *code as u32 == OP_CBRA {
        capnumber = GET2!(code, 1 + LINK_SIZE) as i32;
        capitem.number = capnumber as u16;
        capitem.next = open_caps;
        capitem.assert_depth = (*cb).assert_depth;
        open_caps = &mut capitem as *mut open_capitem;
    }

    /* Offset is set zero to mark that this bracket is still open */

    PUT!(code, 1, 0);
    code = code.add(1 + LINK_SIZE + skipunits as usize);

    /* Loop for each alternative branch */

    loop {
        let branch_return: i32;
        let mut branchfirstcu: u32 = 0;
        let mut branchreqcu: u32 = 0;
        let mut branchfirstcuflags: u32 = REQ_UNSET;
        let mut branchreqcuflags: u32 = REQ_UNSET;

        /* Insert OP_REVERSE or OP_VREVERSE if this is a lookbehind assertion. There
        is only a single minimum length for the whole assertion. When the minimum
        length is LOOKBEHIND_MAX it means that all branches are of fixed length,
        though not necessarily the same length. In this case, the original OP_REVERSE
        can be used. It can also be used if a branch in a variable length lookbehind
        has the same maximum and minimum. Otherwise, use OP_VREVERSE, which has both
        maximum and minimum values. */

        if lookbehind != 0 && lookbehindlength > 0 {
            if lookbehindminlength == LOOKBEHIND_MAX as u32
                || lookbehindminlength == lookbehindlength
            {
                *code = OP_REVERSE as u8;
                code = code.add(1);
                PUT2INC!(code, 0, lookbehindlength);
                length += (1 + IMM2_SIZE) as PCRE2_SIZE;
            } else {
                *code = OP_VREVERSE as u8;
                code = code.add(1);
                PUT2INC!(code, 0, lookbehindminlength);
                PUT2INC!(code, 0, lookbehindlength);
                length += (1 + 2 * IMM2_SIZE) as PCRE2_SIZE;
            }
        }

        /* Now compile the branch; in the pre-compile phase its length gets added
        into the length. */

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
            /* If this is the first branch, the firstcu and reqcu values for the
            branch become the values for the regex. */

            if *last_branch as u32 != OP_ALT {
                firstcu = branchfirstcu;
                firstcuflags = branchfirstcuflags;
                reqcu = branchreqcu;
                reqcuflags = branchreqcuflags;
            }
            /* If this is not the first branch, the first char and reqcu have to
            match the values from all the previous branches, except that if the
            previous value for reqcu didn't have REQ_VARY set, it can still match,
            and we set REQ_VARY for the group from this branch's value. */
            else {
                /* If we previously had a firstcu, but it doesn't match the new branch,
                we have to abandon the firstcu for the regex, but if there was
                previously no reqcu, it takes on the value of the old firstcu. */

                if firstcuflags != branchfirstcuflags || firstcu != branchfirstcu {
                    if firstcuflags < REQ_NONE {
                        if reqcuflags >= REQ_NONE {
                            reqcu = firstcu;
                            reqcuflags = firstcuflags;
                        }
                    }
                    firstcuflags = REQ_NONE;
                }

                /* If we (now or from before) have no firstcu, a firstcu from the
                branch becomes a reqcu if there isn't a branch reqcu. */

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

        /* Handle reaching the end of the expression, either ')' or end of pattern.
        In the real compile phase, go back through the alternative branches and
        reverse the chain of offsets, with the field in the BRA item now becoming an
        offset to the first alternative. If there are no alternatives, it points to
        the end of the group. The length in the terminating ket is always the length
        of the whole bracketed item. Return leaving the pointer at the terminating
        char. */

        if META_CODE!(*pptr) != META_ALT {
            if lengthptr.is_null() {
                let mut branch_length: u32 = (code as usize - last_branch as usize) as u32;
                loop {
                    let prev_length: u32 = GET!(last_branch, 1);
                    PUT!(last_branch, 1, branch_length);
                    branch_length = prev_length;
                    last_branch = last_branch.wrapping_sub(branch_length as usize);
                    if !(branch_length > 0) {
                        break;
                    }
                }
            }

            /* Fill in the ket */

            *code = OP_KET as u8;
            PUT!(code, 1, (code as usize - start_bracket as usize) as u32);
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
                    *errorcodeptr = ERR20;
                    return 0;
                }
                *lengthptr += length;
            }
            return okreturn;
        }

        /* Another branch follows. In the pre-compile phase, we can move the code
        pointer back to where it was for the start of the first branch. (That is,
        pretend that each branch is the only one.)

        In the real compile phase, insert an ALT node. Its length field points back
        to the previous branch while the bracket remains open. At the end the chain
        is reversed. It's done like this so that the start of the bracket has a
        zero offset until it is closed, making it possible to detect recursion. */

        if !lengthptr.is_null() {
            code = (*codeptr).add(1 + LINK_SIZE + skipunits as usize);
            length += (1 + LINK_SIZE) as PCRE2_SIZE;
        } else {
            *code = OP_ALT as u8;
            PUT!(code, 1, (code as usize - last_branch as usize) as i32);
            last_branch = code;
            bc.current_branch = last_branch;
            code = code.add(1 + LINK_SIZE);
        }

        /* Set the maximum lookbehind length for the next branch (if not in a
        lookbehind the value will be zero) and then advance past the vertical bar. */

        lookbehindlength = META_DATA!(*pptr);
        pptr = pptr.add(1);
    }
}

/*************************************************
*          Check for anchored pattern            *
*************************************************/

/* Try to find out if this is an anchored regular expression. Consider each
alternative branch. If they all start with OP_SOD or OP_CIRC, or with a bracket
all of whose alternatives start with OP_SOD or OP_CIRC (recurse ad lib), then
it's anchored.

Arguments:
  code           points to start of the compiled pattern
  bracket_map    a bitmap of which brackets we are inside while testing
  cb             points to the compile data block
  atomcount      atomic group level
  inassert       TRUE if in an assertion
  dotstar_anchor TRUE if automatic anchoring optimization is enabled

Returns:     TRUE or FALSE
*/

pub(crate) unsafe fn is_anchored(
    code: PCRE2_SPTR,
    bracket_map: u32,
    cb: *mut compile_block,
    atomcount: i32,
    inassert: BOOL,
    dotstar_anchor: BOOL,
) -> BOOL {
    let mut code: PCRE2_SPTR = code;
    loop {
        let scode: PCRE2_SPTR = first_significant_code(
            code.add(_pcre2_OP_lengths_8[*code as usize] as usize),
            FALSE,
        );
        let op: u32 = *scode as u32;

        /* Non-capturing brackets */

        if op == OP_BRA || op == OP_BRAPOS || op == OP_SBRA || op == OP_SBRAPOS {
            if is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Capturing brackets */
        else if op == OP_CBRA || op == OP_CBRAPOS || op == OP_SCBRA || op == OP_SCBRAPOS {
            let n: i32 = GET2!(scode, 1 + LINK_SIZE) as i32;
            let new_map: u32 = bracket_map | (if n < 32 { 1u32 << n } else { 1 });
            if is_anchored(scode, new_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Positive forward assertion */
        else if op == OP_ASSERT || op == OP_ASSERT_NA {
            if is_anchored(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Condition. If there is no second branch, it can't be anchored. */
        else if op == OP_COND || op == OP_SCOND {
            if *scode.add(GET!(scode, 1) as usize) as u32 != OP_ALT {
                return FALSE;
            }
            if is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Atomic groups */
        else if op == OP_ONCE {
            if is_anchored(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor) == FALSE
            {
                return FALSE;
            }
        }
        /* .* is not anchored unless DOTALL is set (which generates OP_ALLANY) and
        it isn't in brackets that are or may be referenced or inside an atomic
        group or an assertion. */
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
        /* Check for explicit anchoring */
        else if op != OP_SOD && op != OP_SOM && op != OP_CIRC {
            return FALSE;
        }

        code = code.add(GET!(code, 1) as usize);

        if !(*code as u32 == OP_ALT) {
            break;
        } /* Loop for each alternative */
    }
    TRUE
}

/*************************************************
*         Check for starting with ^ or .*        *
*************************************************/

/* This is called to find out if every branch starts with ^ or .* so that
"first char" processing can be done to speed things up in multiline
matching and for non-DOTALL patterns that start with .*.

Arguments:
  code           points to start of the compiled pattern or a group
  bracket_map    a bitmap of which brackets we are inside while testing
  cb             points to the compile data
  atomcount      atomic group level
  inassert       TRUE if in an assertion
  dotstar_anchor TRUE if automatic anchoring optimization is enabled

Returns:         TRUE or FALSE
*/

pub(crate) unsafe fn is_startline(
    code: PCRE2_SPTR,
    bracket_map: u32,
    cb: *mut compile_block,
    atomcount: i32,
    inassert: BOOL,
    dotstar_anchor: BOOL,
) -> BOOL {
    let mut code: PCRE2_SPTR = code;
    loop {
        let mut scode: PCRE2_SPTR = first_significant_code(
            code.add(_pcre2_OP_lengths_8[*code as usize] as usize),
            FALSE,
        );
        let mut op: u32 = *scode as u32;

        /* If we are at the start of a conditional assertion group, *both* the
        conditional assertion *and* what follows the condition must satisfy the test
        for start of line. Other kinds of condition fail. Note that there may be an
        auto-callout at the start of a condition. */

        if op == OP_COND {
            scode = scode.add(1 + LINK_SIZE);

            if *scode as u32 == OP_CALLOUT {
                scode = scode.add(_pcre2_OP_lengths_8[OP_CALLOUT as usize] as usize);
            } else if *scode as u32 == OP_CALLOUT_STR {
                scode = scode.add(GET!(scode, 1 + 2 * LINK_SIZE) as usize);
            }

            match *scode as u32 {
                OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF | OP_FAIL | OP_FALSE | OP_TRUE => {
                    return FALSE;
                }

                _ => {
                    /* Assertion */
                    if is_startline(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor)
                        == FALSE
                    {
                        return FALSE;
                    }
                    loop {
                        scode = scode.add(GET!(scode, 1) as usize);
                        if !(*scode as u32 == OP_ALT) {
                            break;
                        }
                    }
                    scode = scode.add(1 + LINK_SIZE);
                }
            }
            scode = first_significant_code(scode, FALSE);
            op = *scode as u32;
        }

        /* Non-capturing brackets */

        if op == OP_BRA || op == OP_BRAPOS || op == OP_SBRA || op == OP_SBRAPOS {
            if is_startline(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Capturing brackets */
        else if op == OP_CBRA || op == OP_CBRAPOS || op == OP_SCBRA || op == OP_SCBRAPOS {
            let n: i32 = GET2!(scode, 1 + LINK_SIZE) as i32;
            let new_map: u32 = bracket_map | (if n < 32 { 1u32 << n } else { 1 });
            if is_startline(scode, new_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Positive forward assertions */
        else if op == OP_ASSERT || op == OP_ASSERT_NA {
            if is_startline(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Atomic brackets */
        else if op == OP_ONCE {
            if is_startline(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor)
                == FALSE
            {
                return FALSE;
            }
        }
        /* .* means "start at start or after \n" if it isn't in atomic brackets or
        brackets that may be referenced or an assertion. */
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
        /* Check for explicit circumflex; anything else gives a FALSE result. */
        else if op != OP_CIRC && op != OP_CIRCM {
            return FALSE;
        }

        /* Move on to the next alternative */

        code = code.add(GET!(code, 1) as usize);

        if !(*code as u32 == OP_ALT) {
            break;
        } /* Loop for each alternative */
    }
    TRUE
}

/*************************************************
*   Scan compiled regex for recursion reference  *
*************************************************/

/* This function scans through a compiled pattern until it finds an instance of
OP_RECURSE.

Arguments:
  code        points to start of expression
  utf         TRUE in UTF mode

Returns:      pointer to the opcode for OP_RECURSE, or NULL if not found
*/

pub(crate) unsafe fn find_recurse(code: *mut PCRE2_UCHAR, utf: BOOL) -> *mut PCRE2_UCHAR {
    let mut code: *mut PCRE2_UCHAR = code;
    loop {
        let c: PCRE2_UCHAR = *code;
        if c as u32 == OP_END {
            return core::ptr::null_mut();
        }
        if c as u32 == OP_RECURSE {
            return code;
        }

        /* XCLASS is used for classes that cannot be represented just by a bit map.
        This includes negated single high-valued characters. ECLASS is used for
        classes that use set operations internally. CALLOUT_STR is used for
        callouts with string arguments. In each case the length in the table is
        zero; the actual length is stored in the compiled code. */

        if c as u32 == OP_XCLASS || c as u32 == OP_ECLASS {
            code = code.add(GET!(code, 1) as usize);
        } else if c as u32 == OP_CALLOUT_STR {
            code = code.add(GET!(code, 1 + 2 * LINK_SIZE) as usize);
        }
        /* Otherwise, we can get the item's length from the table, except that for
        repeated character types, we have to test for \p and \P, which have an extra
        two code units of parameters, and for MARK/PRUNE/SKIP/THEN with an argument,
        we must add in its length. */
        else {
            match c as u32 {
                OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
                | OP_TYPEMINQUERY | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS | OP_TYPEPOSQUERY => {
                    if *code.add(1) as u32 == OP_PROP || *code.add(1) as u32 == OP_NOTPROP {
                        code = code.add(2);
                    }
                }

                OP_TYPEPOSUPTO | OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT => {
                    if *code.add(1 + IMM2_SIZE) as u32 == OP_PROP
                        || *code.add(1 + IMM2_SIZE) as u32 == OP_NOTPROP
                    {
                        code = code.add(2);
                    }
                }

                OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                    code = code.add(*code.add(1) as usize);
                }

                _ => {}
            }

            /* Add in the fixed length from the table */

            code = code.add(_pcre2_OP_lengths_8[c as usize] as usize);

            /* In UTF-8 and UTF-16 modes, opcodes that are followed by a character may
            be followed by a multi-unit character. The length in the table is a
            minimum, so we have to arrange to skip the extra units. */

            if utf != 0 {
                match c as u32 {
                    OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_EXACT | OP_EXACTI | OP_NOTEXACT
                    | OP_NOTEXACTI | OP_UPTO | OP_UPTOI | OP_NOTUPTO | OP_NOTUPTOI | OP_MINUPTO
                    | OP_MINUPTOI | OP_NOTMINUPTO | OP_NOTMINUPTOI | OP_POSUPTO | OP_POSUPTOI
                    | OP_NOTPOSUPTO | OP_NOTPOSUPTOI | OP_STAR | OP_STARI | OP_NOTSTAR
                    | OP_NOTSTARI | OP_MINSTAR | OP_MINSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI
                    | OP_POSSTAR | OP_POSSTARI | OP_NOTPOSSTAR | OP_NOTPOSSTARI | OP_PLUS
                    | OP_PLUSI | OP_NOTPLUS | OP_NOTPLUSI | OP_MINPLUS | OP_MINPLUSI
                    | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_POSPLUS | OP_POSPLUSI | OP_NOTPOSPLUS
                    | OP_NOTPOSPLUSI | OP_QUERY | OP_QUERYI | OP_NOTQUERY | OP_NOTQUERYI
                    | OP_MINQUERY | OP_MINQUERYI | OP_NOTMINQUERY | OP_NOTMINQUERYI
                    | OP_POSQUERY | OP_POSQUERYI | OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                        if HAS_EXTRALEN!(*code.offset(-1)) {
                            code = code.add(GET_EXTRALEN!(*code.offset(-1)) as usize);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/*************************************************
*    Check for asserted fixed first code unit    *
*************************************************/

/* During compilation, the "first code unit" settings from forward assertions
are discarded, because they can cause conflicts with actual literals that
follow. However, if we end up without a first code unit setting for an
unanchored pattern, it is worth scanning the regex to see if there is an
initial asserted first code unit.

Arguments:
  code       points to start of compiled pattern
  flags      points to the first code unit flags
  inassert   non-zero if in an assertion

Returns:     the fixed first code unit, or 0 with REQ_NONE in flags
*/

pub(crate) unsafe fn find_firstassertedcu(
    code: PCRE2_SPTR,
    flags: *mut u32,
    inassert: u32,
) -> u32 {
    let mut code: PCRE2_SPTR = code;
    let mut c: u32 = 0;
    let mut cflags: u32 = REQ_NONE;

    *flags = REQ_NONE;
    loop {
        let mut d: u32;
        let mut dflags: u32 = 0;
        let xl: i32 = if *code as u32 == OP_CBRA
            || *code as u32 == OP_SCBRA
            || *code as u32 == OP_CBRAPOS
            || *code as u32 == OP_SCBRAPOS
        {
            IMM2_SIZE as i32
        } else {
            0
        };
        let mut scode: PCRE2_SPTR =
            first_significant_code(code.add(1 + LINK_SIZE + xl as usize), TRUE);
        let op: PCRE2_UCHAR = *scode;

        match op as u32 {
            OP_BRA | OP_BRAPOS | OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS | OP_ASSERT
            | OP_ASSERT_NA | OP_ONCE | OP_SCRIPT_RUN => {
                d = find_firstassertedcu(
                    scode,
                    &mut dflags,
                    inassert
                        + (if op as u32 == OP_ASSERT || op as u32 == OP_ASSERT_NA {
                            1
                        } else {
                            0
                        }),
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
                scode = scode.add(IMM2_SIZE);
                /* Fall through */
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
                scode = scode.add(IMM2_SIZE);
                /* Fall through */
                if inassert == 0 {
                    return 0;
                }

                /* If the character is more than one code unit long, we cannot set its
                first code unit when matching caselessly. Later scanning may pick up
                multiple code units. */

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

        code = code.add(GET!(code, 1) as usize);

        if !(*code as u32 == OP_ALT) {
            break;
        }
    }

    *flags = cflags;
    c
}

/*************************************************
*             Skip in parsed pattern             *
*************************************************/

/* This function is called to skip parts of the parsed pattern when finding the
length of a lookbehind branch.

Arguments:
  pptr       current pointer to skip from
  skiptype   PSKIP_CLASS when skipping to end of class
             PSKIP_ALT when META_ALT ends the skip
             PSKIP_KET when only META_KET ends the skip

Returns:     new value of pptr
             NULL if META_END is reached - should never occur
               or for an unknown meta value - likewise
*/

pub(crate) unsafe fn parsed_skip(pptr: *mut u32, skiptype: u32) -> *mut u32 {
    let mut pptr: *mut u32 = pptr;
    let mut nestlevel: u32 = 0;

    'outer: loop {
        let mut meta: u32 = META_CODE!(*pptr);

        'sw: {
            match meta {
                /* The parsed regex is malformed; we have reached the end and did
                not find the end of the construct which we are skipping over. */
                META_END => {
                    return core::ptr::null_mut();
                }

                /* The data for these items is variable in length. */
                META_BACKREF => {
                    /* Offset is present only if group >= 10 */
                    if META_DATA!(*pptr) >= 10 {
                        pptr = pptr.add(SIZEOFFSET);
                    }
                    break 'sw;
                }

                META_ESCAPE => {
                    if *pptr - META_ESCAPE == ESC_P || *pptr - META_ESCAPE == ESC_p {
                        pptr = pptr.add(1); /* Skip prop data */
                    }
                    break 'sw;
                }

                /* Add the length of the name. */
                META_MARK | META_COMMIT_ARG | META_PRUNE_ARG | META_SKIP_ARG | META_THEN_ARG => {
                    pptr = pptr.add(*pptr.add(1) as usize);
                    break 'sw;
                }

                /* These are the "active" items in this loop. */
                META_CLASS_END => {
                    if skiptype == PSKIP_CLASS {
                        return pptr;
                    }
                    break 'sw;
                }

                META_ATOMIC | META_CAPTURE | META_COND_ASSERT | META_COND_DEFINE
                | META_COND_NAME | META_COND_NUMBER | META_COND_RNAME | META_COND_RNUMBER
                | META_COND_VERSION | META_SCS | META_LOOKAHEAD | META_LOOKAHEADNOT
                | META_LOOKAHEAD_NA | META_LOOKBEHIND | META_LOOKBEHINDNOT
                | META_LOOKBEHIND_NA | META_NOCAPTURE | META_SCRIPT_RUN => {
                    nestlevel += 1;
                    break 'sw;
                }

                META_ALT => {
                    if nestlevel == 0 && skiptype == PSKIP_ALT {
                        return pptr;
                    }
                    break 'sw;
                }

                META_KET => {
                    if nestlevel == 0 {
                        return pptr;
                    }
                    nestlevel -= 1;
                    break 'sw;
                }

                _ => {
                    /* Just skip over most items */
                    if meta < META_END {
                        /* Literal */
                        pptr = pptr.add(1);
                        continue 'outer;
                    }
                    break 'sw;
                }
            }
        }

        /* The extra data item length for each meta is in a table. */

        meta = (meta >> 16) & 0x7fff;
        if meta as usize >= core::mem::size_of_val(&meta_extra_lengths) {
            return core::ptr::null_mut();
        }
        pptr = pptr.add(meta_extra_lengths[meta as usize] as usize);

        pptr = pptr.add(1);
    }
}

/*************************************************
*       Find length of a parsed group            *
*************************************************/

/* This is called for nested groups within a branch of a lookbehind whose
length is being computed.

Arguments:
  pptrptr     pointer to pointer in the parsed pattern
  minptr      where to return the minimum length
  isinline    FALSE if a reference or recursion; TRUE for inline group
  errcodeptr  pointer to the errorcode
  lcptr       pointer to the loop counter
  group       number of captured group or -1 for a non-capturing group
  recurses    chain of recurse_check to catch mutual recursion
  cb          pointer to the compile data

Returns:      the maximum group length or a negative number
*/

pub(crate) unsafe fn get_grouplength(
    pptrptr: *mut *mut u32,
    minptr: *mut i32,
    isinline: BOOL,
    errcodeptr: *mut i32,
    lcptr: *mut i32,
    group: i32,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> i32 {
    let gi: *mut u32 = (*cb).groupinfo.offset((2 * group) as isize);
    let mut branchlength: i32;
    let mut branchminlength: i32 = 0;
    let mut grouplength: i32 = -1;
    let mut groupminlength: i32 = INT_MAX;

    /* The cache can be used only if there is no possibility of there being two
    groups with the same number. We do not need to set the end pointer for a group
    that is being processed as a back reference or recursion, but we must do so for
    an inline group. */

    if group > 0 && ((*cb).external_flags & PCRE2_DUPCAPUSED) == 0 {
        let groupinfo: u32 = *gi.add(0);
        if (groupinfo & GI_NOT_FIXED_LENGTH) != 0 {
            return -1;
        }
        if (groupinfo & GI_SET_FIXED_LENGTH) != 0 {
            if isinline != 0 {
                *pptrptr = parsed_skip(*pptrptr, PSKIP_KET);
            }
            *minptr = *gi.add(1) as i32;
            return (groupinfo & GI_FIXED_LENGTH_MASK) as i32;
        }
    }

    /* Scan the group. In this case we find the end pointer of necessity. */

    'isnotfixed: {
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
                break 'isnotfixed; /* goto ISNOTFIXED */
            }
            if branchlength > grouplength {
                grouplength = branchlength;
            }
            if branchminlength < groupminlength {
                groupminlength = branchminlength;
            }
            if **pptrptr == META_KET {
                break;
            }
            *pptrptr = (*pptrptr).add(1); /* Skip META_ALT */
        }

        if group > 0 {
            *gi.add(0) |= GI_SET_FIXED_LENGTH | (grouplength as u32);
            *gi.add(1) = groupminlength as u32;
        }

        *minptr = groupminlength;
        return grouplength;
    }

    /* ISNOTFIXED: */
    if group > 0 {
        *gi.add(0) |= GI_NOT_FIXED_LENGTH;
    }
    -1
}

/*************************************************
*        Find length of a parsed branch          *
*************************************************/

/* Return fixed maximum and minimum lengths for a branch in a lookbehind,
giving an error if the length is not limited.

Arguments:
  pptrptr     pointer to pointer in the parsed pattern
  minptr      where to return the minimum length
  errcodeptr  pointer to error code
  lcptr       pointer to loop counter
  recurses    chain of recurse_check to catch mutual recursion
  cb          pointer to compile block

Returns:      the maximum length, or a negative value on error
*/

pub(crate) unsafe fn get_branchlength(
    pptrptr: *mut *mut u32,
    minptr: *mut i32,
    errcodeptr: *mut i32,
    lcptr: *mut i32,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> i32 {
    let mut branchlength: i32 = 0;
    let mut branchminlength: i32 = 0;
    let mut grouplength: i32;
    let mut groupminlength: i32 = 0;
    let mut lastitemlength: u32 = 0;
    let mut lastitemminlength: u32 = 0;
    let mut pptr: *mut u32 = *pptrptr;
    let mut offset: PCRE2_SIZE = 0;
    let mut this_recurse: parsed_recurse_check = parsed_recurse_check {
        prev: core::ptr::null_mut(),
        groupptr: core::ptr::null_mut(),
    };

    /* A large and/or complex regex can take too long to process. */

    let lc0 = *lcptr;
    *lcptr = lc0 + 1;
    if lc0 > 2000 {
        *errcodeptr = ERR35; /* Lookbehind is too complicated */
        return -1;
    }

    /* Scan the branch, accumulating the length. */

    'mainloop: loop {
        let mut r: *mut parsed_recurse_check;
        let mut gptr: *mut u32;
        let mut gptrend: *mut u32;
        let mut escape: u32;
        let mut min: u32 = 0;
        let mut max: u32 = 0;
        let mut group: u32 = 0;
        let mut itemlength: u32 = 0;
        let mut itemminlength: u32 = 0;

        if *pptr < META_END {
            itemminlength = 1;
            itemlength = itemminlength;
        } else {
            const S_SWITCH: u32 = 0;
            const S_RECURSE_BYNAME: u32 = 1;
            const S_META_RECURSE: u32 = 2;
            const S_RECURSE_OR_BACKREF_LENGTH: u32 = 3;
            const S_CHECK_GROUP: u32 = 4;
            const S_REPETITION: u32 = 5;
            const S_ISNOTFIXED: u32 = 6;

            let mut st: u32 = S_SWITCH;
            'sm: loop {
                match st {
                    S_SWITCH => {
                        match META_CODE!(*pptr) {
                            META_KET | META_ALT => {
                                break 'mainloop; /* goto EXIT */
                            }

                            /* (*ACCEPT) and (*FAIL) terminate the branch, but we must skip to
                            the actual termination. */
                            META_ACCEPT | META_FAIL => {
                                pptr = parsed_skip(pptr, PSKIP_ALT);
                                if pptr.is_null() {
                                    /* goto PARSED_SKIP_FAILED */
                                    *errcodeptr = ERR90;
                                    return -1;
                                }
                                break 'mainloop; /* goto EXIT */
                            }

                            META_MARK | META_COMMIT_ARG | META_PRUNE_ARG | META_SKIP_ARG
                            | META_THEN_ARG => {
                                pptr = pptr.add(*pptr.add(1) as usize + 1);
                                break 'sm;
                            }

                            META_CIRCUMFLEX | META_COMMIT | META_DOLLAR | META_PRUNE
                            | META_SKIP | META_THEN => {
                                break 'sm;
                            }

                            META_OPTIONS => {
                                pptr = pptr.add(2);
                                break 'sm;
                            }

                            META_BIGVALUE => {
                                itemminlength = 1;
                                itemlength = itemminlength;
                                pptr = pptr.add(1);
                                break 'sm;
                            }

                            META_CLASS | META_CLASS_NOT => {
                                itemminlength = 1;
                                itemlength = itemminlength;
                                pptr = parsed_skip(pptr, PSKIP_CLASS);
                                if pptr.is_null() {
                                    /* goto PARSED_SKIP_FAILED */
                                    *errcodeptr = ERR90;
                                    return -1;
                                }
                                break 'sm;
                            }

                            META_CLASS_EMPTY_NOT | META_DOT => {
                                itemminlength = 1;
                                itemlength = itemminlength;
                                break 'sm;
                            }

                            META_CALLOUT_NUMBER => {
                                pptr = pptr.add(3);
                                break 'sm;
                            }

                            META_CALLOUT_STRING => {
                                pptr = pptr.add(3 + SIZEOFFSET);
                                break 'sm;
                            }

                            /* Only some escapes consume a character. Of those, \R can match
                            one or two characters, but \X is never allowed because it matches
                            an unknown number of characters. \C is allowed only in 32-bit and
                            non-UTF 8/16-bit modes. */
                            META_ESCAPE => {
                                escape = META_DATA!(*pptr);
                                if escape == ESC_X {
                                    return -1;
                                }
                                if escape == ESC_R {
                                    itemminlength = 1;
                                    itemlength = 2;
                                } else if escape > ESC_b && escape < ESC_Z {
                                    if ((*cb).external_options & PCRE2_UTF) != 0
                                        && escape == ESC_C
                                    {
                                        *errcodeptr = ERR36;
                                        return -1;
                                    }
                                    itemminlength = 1;
                                    itemlength = itemminlength;
                                    if escape == ESC_p || escape == ESC_P {
                                        pptr = pptr.add(1); /* Skip prop data */
                                    }
                                }
                                break 'sm;
                            }

                            /* Lookaheads do not contribute to the length of this branch, but
                            they may contain lookbehinds within them whose lengths need to be
                            set. */
                            META_LOOKAHEAD | META_LOOKAHEADNOT | META_LOOKAHEAD_NA | META_SCS => {
                                *errcodeptr =
                                    check_lookbehinds(pptr.add(1), &mut pptr, recurses, cb, lcptr);
                                if *errcodeptr != 0 {
                                    return -1;
                                }

                                /* Ignore any qualifiers that follow a lookahead assertion. */

                                match *pptr.add(1) {
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
                                break 'sm;
                            }

                            /* A nested lookbehind does not contribute any length to this
                            lookbehind, but must itself be checked and have its lengths set. */
                            META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA => {
                                if set_lookbehind_lengths(
                                    &mut pptr, errcodeptr, lcptr, recurses, cb,
                                ) == FALSE
                                {
                                    return -1;
                                }
                                break 'sm;
                            }

                            /* Back references and recursions are handled by very similar
                            code. */
                            META_BACKREF_BYNAME => {
                                if ((*cb).external_options & PCRE2_MATCH_UNSET_BACKREF) != 0 {
                                    st = S_ISNOTFIXED; /* goto ISNOTFIXED */
                                    continue 'sm;
                                }
                                /* Fall through */
                                st = S_RECURSE_BYNAME;
                                continue 'sm;
                            }

                            META_RECURSE_BYNAME => {
                                st = S_RECURSE_BYNAME;
                                continue 'sm;
                            }

                            /* The offset values for back references < 10 are in a separate
                            vector because otherwise they would use more than two parsed
                            pattern elements on 64-bit systems. */
                            META_BACKREF => {
                                if ((*cb).external_options & PCRE2_MATCH_UNSET_BACKREF) != 0
                                    || ((*cb).external_flags & PCRE2_DUPCAPUSED) != 0
                                {
                                    st = S_ISNOTFIXED; /* goto ISNOTFIXED */
                                    continue 'sm;
                                }
                                group = META_DATA!(*pptr);
                                if group < 10 {
                                    offset = (*cb).small_ref_offset[group as usize];
                                    st = S_RECURSE_OR_BACKREF_LENGTH;
                                    continue 'sm;
                                }

                                /* Fall through */
                                /* For groups >= 10 - picking up group twice does no harm. */
                                st = S_META_RECURSE;
                                continue 'sm;
                            }

                            /* A true recursion implies not fixed length, but a subroutine
                            call may be OK. Back reference "recursions" are also failed. */
                            META_RECURSE => {
                                st = S_META_RECURSE;
                                continue 'sm;
                            }

                            /* A (DEFINE) group is never obeyed inline and so it does not
                            contribute to the length of this branch. Skip from the following
                            item to the next unpaired ket. */
                            META_COND_DEFINE => {
                                pptr = parsed_skip(pptr.add(1), PSKIP_KET);
                                break 'sm;
                            }

                            /* Check other nested groups - advance past the initial data for
                            each type and then seek a fixed length with get_grouplength(). */
                            META_COND_NAME | META_COND_NUMBER | META_COND_RNAME
                            | META_COND_RNUMBER => {
                                pptr = pptr.add(2 + SIZEOFFSET);
                                st = S_CHECK_GROUP; /* goto CHECK_GROUP */
                                continue 'sm;
                            }

                            META_COND_ASSERT => {
                                pptr = pptr.add(1);
                                st = S_CHECK_GROUP; /* goto CHECK_GROUP */
                                continue 'sm;
                            }

                            META_COND_VERSION => {
                                pptr = pptr.add(4);
                                st = S_CHECK_GROUP; /* goto CHECK_GROUP */
                                continue 'sm;
                            }

                            META_CAPTURE => {
                                group = META_DATA!(*pptr);
                                /* Fall through */
                                pptr = pptr.add(1);
                                st = S_CHECK_GROUP;
                                continue 'sm;
                            }

                            META_ATOMIC | META_NOCAPTURE | META_SCRIPT_RUN => {
                                pptr = pptr.add(1);
                                st = S_CHECK_GROUP;
                                continue 'sm;
                            }

                            META_QUERY | META_QUERY_PLUS | META_QUERY_QUERY => {
                                min = 0;
                                max = 1;
                                st = S_REPETITION; /* goto REPETITION */
                                continue 'sm;
                            }

                            /* Exact repetition is OK; variable repetition is not. A
                            repetition of zero must subtract the length that has already been
                            added. */
                            META_MINMAX | META_MINMAX_PLUS | META_MINMAX_QUERY => {
                                min = *pptr.add(1);
                                max = *pptr.add(2);
                                pptr = pptr.add(2);
                                /* Fall through into REPETITION */
                                st = S_REPETITION;
                                continue 'sm;
                            }

                            /* Any other item means this branch does not have a fixed
                            length. */
                            _ => {
                                st = S_ISNOTFIXED;
                                continue 'sm;
                            }
                        }
                    }

                    S_RECURSE_BYNAME => {
                        {
                            let name: PCRE2_SPTR;
                            let mut is_dupname: BOOL = FALSE;
                            let ng: *mut named_group;
                            let meta_code: u32 = META_CODE!(*pptr);
                            pptr = pptr.add(1);
                            let length: u32 = *pptr;

                            GETPLUSOFFSET!(offset, pptr);
                            name = (*cb).start_pattern.add(offset);
                            ng = _pcre2_compile_find_named_group8(name, length, cb);

                            if ng.is_null() {
                                *errcodeptr = ERR15; /* Non-existent subpattern */
                                (*cb).erroroffset = offset;
                                return -1;
                            }

                            group = (*ng).number;
                            is_dupname =
                                (((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) != 0) as BOOL;

                            /* A numerical back reference can be fixed length if duplicate
                            capturing groups are not being used. A non-duplicate named back
                            reference can also be handled. */

                            if meta_code == META_RECURSE_BYNAME
                                || (is_dupname == 0
                                    && ((*cb).external_flags & PCRE2_DUPCAPUSED) == 0)
                            {
                                st = S_RECURSE_OR_BACKREF_LENGTH; /* Handle as a numbered version. */
                                continue 'sm;
                            }
                        }
                        st = S_ISNOTFIXED; /* Duplicate name or number */
                        continue 'sm;
                    }

                    S_META_RECURSE => {
                        group = META_DATA!(*pptr);
                        GETPLUSOFFSET!(offset, pptr);
                        st = S_RECURSE_OR_BACKREF_LENGTH;
                        continue 'sm;
                    }

                    S_RECURSE_OR_BACKREF_LENGTH => {
                        if group > (*cb).bracount {
                            (*cb).erroroffset = offset;
                            *errcodeptr = ERR15; /* Non-existent subpattern */
                            return -1;
                        }
                        if group == 0 {
                            st = S_ISNOTFIXED; /* Local recursion */
                            continue 'sm;
                        }
                        gptr = (*cb).parsed_pattern;
                        while *gptr != META_END {
                            if META_CODE!(*gptr) == META_BIGVALUE {
                                gptr = gptr.add(1);
                            } else if *gptr == (META_CAPTURE | group) {
                                break;
                            }
                            gptr = gptr.add(1);
                        }

                        /* We must start the search for the end of the group at the first meta
                        code inside the group. Otherwise it will be treated as an enclosed
                        group. */

                        gptrend = parsed_skip(gptr.add(1), PSKIP_KET);
                        if gptrend.is_null() {
                            /* goto PARSED_SKIP_FAILED */
                            *errcodeptr = ERR90;
                            return -1;
                        }
                        if pptr > gptr && pptr < gptrend {
                            st = S_ISNOTFIXED; /* Local recursion */
                            continue 'sm;
                        }
                        r = recurses;
                        while !r.is_null() {
                            if (*r).groupptr == gptr {
                                break;
                            }
                            r = (*r).prev;
                        }
                        if !r.is_null() {
                            st = S_ISNOTFIXED; /* Mutual recursion */
                            continue 'sm;
                        }
                        this_recurse.prev = recurses;
                        this_recurse.groupptr = gptr;

                        /* We do not need to know the position of the end of the group, that
                        is, gptr is not used after the call to get_grouplength(). */

                        gptr = gptr.add(1);
                        grouplength = get_grouplength(
                            &mut gptr,
                            &mut groupminlength,
                            FALSE,
                            errcodeptr,
                            lcptr,
                            group as i32,
                            &mut this_recurse,
                            cb,
                        );
                        if grouplength < 0 {
                            if *errcodeptr == 0 {
                                st = S_ISNOTFIXED;
                                continue 'sm;
                            }
                            return -1; /* Error already set */
                        }
                        itemlength = grouplength as u32;
                        itemminlength = groupminlength as u32;
                        break 'sm;
                    }

                    S_CHECK_GROUP => {
                        grouplength = get_grouplength(
                            &mut pptr,
                            &mut groupminlength,
                            TRUE,
                            errcodeptr,
                            lcptr,
                            group as i32,
                            recurses,
                            cb,
                        );
                        if grouplength < 0 {
                            return -1;
                        }
                        itemlength = grouplength as u32;
                        itemminlength = groupminlength as u32;
                        break 'sm;
                    }

                    S_REPETITION => {
                        if max != REPEAT_UNLIMITED {
                            if lastitemlength != 0 &&  /* Should not occur, but just in case */
                               max != 0 &&
                               ((INT_MAX - branchlength) as u32) / lastitemlength < max - 1
                            {
                                *errcodeptr = ERR87; /* Integer overflow; lookbehind too big */
                                return -1;
                            }
                            if min == 0 {
                                branchminlength =
                                    branchminlength.wrapping_sub(lastitemminlength as i32);
                            } else {
                                itemminlength = (min - 1).wrapping_mul(lastitemminlength);
                            }
                            if max == 0 {
                                branchlength = branchlength.wrapping_sub(lastitemlength as i32);
                            } else {
                                itemlength = (max - 1).wrapping_mul(lastitemlength);
                            }
                            break 'sm;
                        }
                        /* Fall through to ISNOTFIXED */
                        st = S_ISNOTFIXED;
                        continue 'sm;
                    }

                    _ => {
                        /* S_ISNOTFIXED */
                        *errcodeptr = ERR25; /* Not fixed length */
                        return -1;
                    }
                }
            }
        }

        /* Add the item length to the branchlength, checking for integer overflow and
        for the branch length exceeding the overall limit. Later, if there is at
        least one variable-length branch in the group, there is a test for the
        (smaller) variable-length branch length limit. */

        if INT_MAX - branchlength < itemlength as i32 || {
            branchlength = branchlength.wrapping_add(itemlength as i32);
            branchlength > LOOKBEHIND_MAX
        } {
            *errcodeptr = ERR87;
            return -1;
        }

        branchminlength = branchminlength.wrapping_add(itemminlength as i32);

        /* Save this item length for use if the next item is a quantifier. */

        lastitemlength = itemlength;
        lastitemminlength = itemminlength;

        pptr = pptr.add(1);
    }

    /* EXIT: */
    *pptrptr = pptr;
    *minptr = branchminlength;
    branchlength
}

/*************************************************
*        Set lengths in a lookbehind             *
*************************************************/

/* This function is called for each lookbehind, to set the lengths in its
branches. An error occurs if any branch does not have a limited maximum length
that is less than the limit (65535). On exit, the pointer must be left on the
final ket.

Arguments:
  pptrptr     pointer to pointer in the parsed pattern
  errcodeptr  pointer to error code
  lcptr       pointer to loop counter
  recurses    chain of recurse_check to catch mutual recursion
  cb          pointer to compile block

Returns:      TRUE if all is well
              FALSE otherwise, with error code and offset set
*/

pub(crate) unsafe fn set_lookbehind_lengths(
    pptrptr: *mut *mut u32,
    errcodeptr: *mut i32,
    lcptr: *mut i32,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> BOOL {
    let mut offset: PCRE2_SIZE;
    let mut bptr: *mut u32 = *pptrptr;
    let gbptr: *mut u32 = bptr;
    let mut maxlength: i32 = 0;
    let mut minlength: i32 = INT_MAX;
    let mut variable: BOOL = FALSE;

    READPLUSOFFSET!(offset, bptr); /* Offset for error messages */
    *pptrptr = (*pptrptr).add(SIZEOFFSET);

    /* Each branch can have a different maximum length, but we can keep only a
    single minimum for the whole group, because there's nowhere to save individual
    values in the META_ALT item. */

    loop {
        let branchlength: i32;
        let mut branchminlength: i32 = 0;

        *pptrptr = (*pptrptr).add(1);
        branchlength = get_branchlength(
            pptrptr,
            &mut branchminlength,
            errcodeptr,
            lcptr,
            recurses,
            cb,
        );

        if branchlength < 0 {
            /* The errorcode and offset may already be set from a nested lookbehind. */
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
        *bptr |= branchlength as u32; /* branchlength never more than 65535 */
        bptr = *pptrptr;

        if !(META_CODE!(*bptr) == META_ALT) {
            break;
        }
    }

    /* If any branch is of variable length, the whole lookbehind is of variable
    length. If the maximum length of any branch exceeds the maximum for variable
    lookbehinds, give an error. Otherwise, the minimum length is set in the word
    that follows the original group META value. For a fixed-length lookbehind, this
    is set to LOOKBEHIND_MAX, to indicate that each branch is of a fixed (but
    possibly different) length. */

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

/*************************************************
*         Check parsed pattern lookbehinds       *
*************************************************/

/* This function is called at the end of parsing a pattern if any lookbehinds
were encountered. It scans the parsed pattern for them, calling
set_lookbehind_lengths() for each one.

Arguments
  pptr      points to where to start (start of pattern or start of lookahead)
  retptr    if not NULL, return the ket pointer here
  recurses  chain of recurse_check to catch mutual recursion
  cb        points to the compile block
  lcptr     points to loop counter

Returns:    0 on success, or an errorcode (cb->erroroffset will be set)
*/

pub(crate) unsafe fn check_lookbehinds(
    pptr: *mut u32,
    retptr: *mut *mut u32,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
    lcptr: *mut i32,
) -> i32 {
    let mut pptr: *mut u32 = pptr;
    let mut errorcode: i32 = 0;
    let mut nestlevel: i32 = 0;

    (*cb).erroroffset = PCRE2_UNSET;

    while *pptr != META_END {
        if *pptr < META_END {
            /* Literal */
            pptr = pptr.add(1);
            continue;
        }

        match META_CODE!(*pptr) {
            META_ESCAPE => {
                if *pptr - META_ESCAPE == ESC_P || *pptr - META_ESCAPE == ESC_p {
                    pptr = pptr.add(1); /* Skip prop data */
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

            META_ACCEPT | META_ALT | META_ASTERISK | META_ASTERISK_PLUS | META_ASTERISK_QUERY
            | META_BACKREF | META_CIRCUMFLEX | META_CLASS | META_CLASS_EMPTY
            | META_CLASS_EMPTY_NOT | META_CLASS_END | META_CLASS_NOT | META_COMMIT
            | META_DOLLAR | META_DOT | META_FAIL | META_PLUS | META_PLUS_PLUS
            | META_PLUS_QUERY | META_PRUNE | META_QUERY | META_QUERY_PLUS | META_QUERY_QUERY
            | META_RANGE_ESCAPED | META_RANGE_LITERAL | META_SKIP | META_THEN => {}

            META_OFFSET | META_RECURSE => {
                pptr = pptr.add(SIZEOFFSET);
            }

            META_BACKREF_BYNAME | META_RECURSE_BYNAME => {
                pptr = pptr.add(1 + SIZEOFFSET);
            }

            META_COND_DEFINE => {
                pptr = pptr.add(SIZEOFFSET);
                nestlevel += 1;
            }

            META_COND_NAME | META_COND_NUMBER | META_COND_RNAME | META_COND_RNUMBER => {
                pptr = pptr.add(1 + SIZEOFFSET);
                nestlevel += 1;
            }

            META_COND_VERSION => {
                pptr = pptr.add(3);
                nestlevel += 1;
            }

            META_CALLOUT_STRING => {
                pptr = pptr.add(3 + SIZEOFFSET);
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

            /* Note that set_lookbehind_lengths() updates pptr, leaving it pointing to
            the final ket of the group, so no need to update it here. */
            META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA => {
                if set_lookbehind_lengths(&mut pptr, &mut errorcode, lcptr, recurses, cb) == FALSE
                {
                    return errorcode;
                }
            }

            /* The following erroroffset is a bogus but safe value. */
            _ => {
                (*cb).erroroffset = 0;
                return ERR70; /* Unrecognized meta code */
            }
        }

        pptr = pptr.add(1);
    }

    0
}
