//! Translation of the compiled-pattern scanning and lookbehind-length helpers
//! from `c_src/src/pcre2_compile.c` (C lines ~8896..10600).
//!
//! Built for the 8-bit library with `SUPPORT_UNICODE` (hence
//! `MAYBE_UTF_MULTI`), `LINK_SIZE == 2`, no JIT, no EBCDIC, no `PCRE2_DEBUG`.

#![allow(non_snake_case, non_upper_case_globals, unused_parens, dead_code)]

use core::ffi::c_int;

use crate::compile_branch::first_significant_code;
use crate::compile_internal::*;
use crate::compile_tables::{meta_extra_lengths, REPEAT_UNLIMITED};
use crate::internal::*;
use crate::opcodes::*;

/* These #defines are local to pcre2_compile.c. They are duplicated here (rather
than shared) because they are used only by the functions in this file and their
siblings; keeping them local keeps this module self-contained. */

/* Values greater than or equal to REQ_NONE mean "no code unit set". */
const REQ_NONE: u32 = 0xfffffffe;
const REQ_CASELESS: u32 = 0x00000001;

/* Bits used in the group-info word (cb->groupinfo) during lookbehind length
scanning. */
const GI_SET_FIXED_LENGTH: u32 = 0x80000000;
const GI_NOT_FIXED_LENGTH: u32 = 0x40000000;
const GI_FIXED_LENGTH_MASK: u32 = 0x0000ffff;

/* Skip modes for parsed_skip(). C: enum { PSKIP_ALT, PSKIP_CLASS, PSKIP_KET }; */
const PSKIP_ALT: u32 = 0;
const PSKIP_CLASS: u32 = 1;
const PSKIP_KET: u32 = 2;

const INT_MAX: c_int = c_int::MAX;

/*************************************************
*          Check for anchored pattern            *
*************************************************/

/* Try to find out if this is an anchored regular expression. See the C source
for the detailed rationale.

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
    mut code: PCRE2_SPTR,
    bracket_map: u32,
    cb: *mut compile_block,
    atomcount: c_int,
    inassert: BOOL,
    dotstar_anchor: BOOL,
) -> BOOL {
    unsafe {
        loop {
            let scode = first_significant_code(
                code.add(OP_LENGTHS[*code as usize] as usize),
                FALSE,
            );
            let op = *scode as c_int;

            /* Non-capturing brackets */

            if op == OP_BRA as c_int
                || op == OP_BRAPOS as c_int
                || op == OP_SBRA as c_int
                || op == OP_SBRAPOS as c_int
            {
                if is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor)
                    == FALSE
                {
                    return FALSE;
                }
            }
            /* Capturing brackets */
            else if op == OP_CBRA as c_int
                || op == OP_CBRAPOS as c_int
                || op == OP_SCBRA as c_int
                || op == OP_SCBRAPOS as c_int
            {
                let n = get2(scode, 1 + LINK_SIZE);
                let new_map = bracket_map | (if n < 32 { 1u32 << n } else { 1 });
                if is_anchored(scode, new_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                    return FALSE;
                }
            }
            /* Positive forward assertion */
            else if op == OP_ASSERT as c_int || op == OP_ASSERT_NA as c_int {
                if is_anchored(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor) == FALSE {
                    return FALSE;
                }
            }
            /* Condition. If there is no second branch, it can't be anchored. */
            else if op == OP_COND as c_int || op == OP_SCOND as c_int {
                if *scode.add(get(scode, 1) as usize) != OP_ALT {
                    return FALSE;
                }
                if is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor)
                    == FALSE
                {
                    return FALSE;
                }
            }
            /* Atomic groups */
            else if op == OP_ONCE as c_int {
                if is_anchored(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor)
                    == FALSE
                {
                    return FALSE;
                }
            }
            /* .* is not anchored unless DOTALL is set (which generates OP_ALLANY) and
            it isn't in brackets that are or may be referenced or inside an atomic
            group or an assertion. Also the pattern must not contain *PRUNE or *SKIP,
            because these break the feature. There is also an option that disables
            auto-anchoring. */
            else if op == OP_TYPESTAR as c_int
                || op == OP_TYPEMINSTAR as c_int
                || op == OP_TYPEPOSSTAR as c_int
            {
                if *scode.add(1) != OP_ALLANY
                    || (bracket_map & (*cb).backref_map) != 0
                    || atomcount > 0
                    || (*cb).had_pruneorskip != FALSE
                    || inassert != FALSE
                    || dotstar_anchor == FALSE
                {
                    return FALSE;
                }
            }
            /* Check for explicit anchoring */
            else if op != OP_SOD as c_int && op != OP_SOM as c_int && op != OP_CIRC as c_int {
                return FALSE;
            }

            code = code.add(get(code, 1) as usize);

            if *code != OP_ALT {
                break;
            }
        }
        TRUE
    }
}

/*************************************************
*         Check for starting with ^ or .*        *
*************************************************/

/* This is called to find out if every branch starts with ^ or .* so that
"first char" processing can be done to speed things up in multiline matching
and for non-DOTALL patterns that start with .*.

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
    mut code: PCRE2_SPTR,
    bracket_map: u32,
    cb: *mut compile_block,
    atomcount: c_int,
    inassert: BOOL,
    dotstar_anchor: BOOL,
) -> BOOL {
    unsafe {
        loop {
            let mut scode = first_significant_code(
                code.add(OP_LENGTHS[*code as usize] as usize),
                FALSE,
            );
            let mut op = *scode as c_int;

            /* If we are at the start of a conditional assertion group, *both* the
            conditional assertion *and* what follows the condition must satisfy the
            test for start of line. Other kinds of condition fail. Note that there
            may be an auto-callout at the start of a condition. */

            if op == OP_COND as c_int {
                scode = scode.add(1 + LINK_SIZE);

                if *scode == OP_CALLOUT {
                    scode = scode.add(OP_LENGTHS[OP_CALLOUT as usize] as usize);
                } else if *scode == OP_CALLOUT_STR {
                    scode = scode.add(get(scode, 1 + 2 * LINK_SIZE) as usize);
                }

                match *scode {
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
                            scode = scode.add(get(scode, 1) as usize);
                            if *scode != OP_ALT {
                                break;
                            }
                        }
                        scode = scode.add(1 + LINK_SIZE);
                    }
                }
                scode = first_significant_code(scode, FALSE);
                op = *scode as c_int;
            }

            /* Non-capturing brackets */

            if op == OP_BRA as c_int
                || op == OP_BRAPOS as c_int
                || op == OP_SBRA as c_int
                || op == OP_SBRAPOS as c_int
            {
                if is_startline(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor)
                    == FALSE
                {
                    return FALSE;
                }
            }
            /* Capturing brackets */
            else if op == OP_CBRA as c_int
                || op == OP_CBRAPOS as c_int
                || op == OP_SCBRA as c_int
                || op == OP_SCBRAPOS as c_int
            {
                let n = get2(scode, 1 + LINK_SIZE);
                let new_map = bracket_map | (if n < 32 { 1u32 << n } else { 1 });
                if is_startline(scode, new_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                    return FALSE;
                }
            }
            /* Positive forward assertions */
            else if op == OP_ASSERT as c_int || op == OP_ASSERT_NA as c_int {
                if is_startline(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor) == FALSE {
                    return FALSE;
                }
            }
            /* Atomic brackets */
            else if op == OP_ONCE as c_int {
                if is_startline(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor)
                    == FALSE
                {
                    return FALSE;
                }
            }
            /* .* means "start at start or after \n" if it isn't in atomic brackets or
            brackets that may be referenced or an assertion, and as long as the pattern
            does not contain *PRUNE or *SKIP. There is also an option that disables this
            optimization. */
            else if op == OP_TYPESTAR as c_int
                || op == OP_TYPEMINSTAR as c_int
                || op == OP_TYPEPOSSTAR as c_int
            {
                if *scode.add(1) != OP_ANY
                    || (bracket_map & (*cb).backref_map) != 0
                    || atomcount > 0
                    || (*cb).had_pruneorskip != FALSE
                    || inassert != FALSE
                    || dotstar_anchor == FALSE
                {
                    return FALSE;
                }
            }
            /* Check for explicit circumflex; anything else gives a FALSE result. Note
            in particular that this includes atomic brackets OP_ONCE because the number
            of characters matched by .* cannot be adjusted inside them. */
            else if op != OP_CIRC as c_int && op != OP_CIRCM as c_int {
                return FALSE;
            }

            /* Move on to the next alternative */

            code = code.add(get(code, 1) as usize);

            if *code != OP_ALT {
                break;
            }
        }
        TRUE
    }
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
pub(crate) unsafe fn find_recurse(mut code: *mut PCRE2_UCHAR, utf: BOOL) -> *mut PCRE2_UCHAR {
    unsafe {
        loop {
            let c = *code;
            if c == OP_END {
                return core::ptr::null_mut();
            }
            if c == OP_RECURSE {
                return code;
            }

            /* XCLASS is used for classes that cannot be represented just by a bit map.
            This includes negated single high-valued characters. ECLASS is used for
            classes that use set operations internally. CALLOUT_STR is used for
            callouts with string arguments. In each case the length in the table is
            zero; the actual length is stored in the compiled code. */

            if c == OP_XCLASS || c == OP_ECLASS {
                code = code.add(get(code, 1) as usize);
            } else if c == OP_CALLOUT_STR {
                code = code.add(get(code, 1 + 2 * LINK_SIZE) as usize);
            }
            /* Otherwise, we can get the item's length from the table, except that for
            repeated character types, we have to test for \p and \P, which have an extra
            two code units of parameters, and for MARK/PRUNE/SKIP/THEN with an argument,
            we must add in its length. */
            else {
                match c {
                    OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
                    | OP_TYPEMINQUERY | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS | OP_TYPEPOSQUERY => {
                        if *code.add(1) == OP_PROP || *code.add(1) == OP_NOTPROP {
                            code = code.add(2);
                        }
                    }

                    OP_TYPEPOSUPTO | OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT => {
                        if *code.add(1 + IMM2_SIZE) == OP_PROP
                            || *code.add(1 + IMM2_SIZE) == OP_NOTPROP
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

                code = code.add(OP_LENGTHS[c as usize] as usize);

                /* In UTF-8 and UTF-16 modes, opcodes that are followed by a character
                may be followed by a multi-unit character. The length in the table is a
                minimum, so we have to arrange to skip the extra units. MAYBE_UTF_MULTI
                is defined in 8-bit mode. */

                if utf != FALSE {
                    match c {
                        OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_EXACT | OP_EXACTI
                        | OP_NOTEXACT | OP_NOTEXACTI | OP_UPTO | OP_UPTOI | OP_NOTUPTO
                        | OP_NOTUPTOI | OP_MINUPTO | OP_MINUPTOI | OP_NOTMINUPTO
                        | OP_NOTMINUPTOI | OP_POSUPTO | OP_POSUPTOI | OP_NOTPOSUPTO
                        | OP_NOTPOSUPTOI | OP_STAR | OP_STARI | OP_NOTSTAR | OP_NOTSTARI
                        | OP_MINSTAR | OP_MINSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI
                        | OP_POSSTAR | OP_POSSTARI | OP_NOTPOSSTAR | OP_NOTPOSSTARI | OP_PLUS
                        | OP_PLUSI | OP_NOTPLUS | OP_NOTPLUSI | OP_MINPLUS | OP_MINPLUSI
                        | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_POSPLUS | OP_POSPLUSI
                        | OP_NOTPOSPLUS | OP_NOTPOSPLUSI | OP_QUERY | OP_QUERYI | OP_NOTQUERY
                        | OP_NOTQUERYI | OP_MINQUERY | OP_MINQUERYI | OP_NOTMINQUERY
                        | OP_NOTMINQUERYI | OP_POSQUERY | OP_POSQUERYI | OP_NOTPOSQUERY
                        | OP_NOTPOSQUERYI => {
                            if has_extralen(*code.sub(1) as u32) {
                                code = code.add(get_extralen(*code.sub(1) as u32) as usize);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/*************************************************
*    Check for asserted fixed first code unit    *
*************************************************/

/* During compilation, the "first code unit" settings from forward assertions
are discarded. However, if we end up without a first code unit setting for an
unanchored pattern, it is worth scanning the regex to see if there is an initial
asserted first code unit.

Arguments:
  code       points to start of compiled pattern
  flags      points to the first code unit flags
  inassert   non-zero if in an assertion

Returns:     the fixed first code unit, or 0 with REQ_NONE in flags
*/
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
            let xl = if *code == OP_CBRA
                || *code == OP_SCBRA
                || *code == OP_CBRAPOS
                || *code == OP_SCBRAPOS
            {
                IMM2_SIZE
            } else {
                0
            };
            let mut scode = first_significant_code(code.add(1 + LINK_SIZE + xl), TRUE);
            let op = *scode;

            match op {
                OP_BRA | OP_BRAPOS | OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS | OP_ASSERT
                | OP_ASSERT_NA | OP_ONCE | OP_SCRIPT_RUN => {
                    let mut dflags: u32 = 0;
                    let d = find_firstassertedcu(
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
                    /* If the character is more than one code unit long, we cannot set
                    its first code unit when matching caselessly. */
                    if *scode.add(1) >= 0x80 {
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
                    /* If the character is more than one code unit long, we cannot set
                    its first code unit when matching caselessly. Later scanning may
                    pick up multiple code units. */
                    if *scode.add(1) >= 0x80 {
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

            code = code.add(get(code, 1) as usize);

            if *code != OP_ALT {
                break;
            }
        }

        *flags = cflags;
        c
    }
}

/*************************************************
*             Skip in parsed pattern             *
*************************************************/

/* This function is called to skip parts of the parsed pattern when finding the
length of a lookbehind branch. See the C source for the full contract.

Arguments:
  pptr       current pointer to skip from
  skiptype   PSKIP_CLASS when skipping to end of class
             PSKIP_ALT when META_ALT ends the skip
             PSKIP_KET when only META_KET ends the skip

Returns:     new value of pptr
             NULL if META_END is reached - should never occur
               or for an unknown meta value - likewise
*/
pub(crate) unsafe fn parsed_skip(mut pptr: *mut u32, skiptype: u32) -> *mut u32 {
    unsafe {
        let mut nestlevel: u32 = 0;

        loop {
            let mut meta = meta_code(*pptr);

            match meta {
                META_END => {
                    /* The parsed regex is malformed; we have reached the end and did
                    not find the end of the construct which we are skipping over. */
                    return core::ptr::null_mut();
                }

                /* The data for these items is variable in length. */
                META_BACKREF => {
                    /* Offset is present only if group >= 10 */
                    if meta_data(*pptr) >= 10 {
                        pptr = pptr.add(SIZEOFFSET);
                    }
                }

                META_ESCAPE => {
                    if *pptr - META_ESCAPE == ESC_P as u32 || *pptr - META_ESCAPE == ESC_p as u32 {
                        pptr = pptr.add(1); /* Skip prop data */
                    }
                }

                META_MARK | META_COMMIT_ARG | META_PRUNE_ARG | META_SKIP_ARG | META_THEN_ARG => {
                    /* Add the length of the name. */
                    pptr = pptr.add(*pptr.add(1) as usize);
                }

                /* These are the "active" items in this loop. */
                META_CLASS_END => {
                    if skiptype == PSKIP_CLASS {
                        return pptr;
                    }
                }

                META_ATOMIC | META_CAPTURE | META_COND_ASSERT | META_COND_DEFINE
                | META_COND_NAME | META_COND_NUMBER | META_COND_RNAME | META_COND_RNUMBER
                | META_COND_VERSION | META_SCS | META_LOOKAHEAD | META_LOOKAHEADNOT
                | META_LOOKAHEAD_NA | META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA
                | META_NOCAPTURE | META_SCRIPT_RUN => {
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
                    /* Just skip over most items */
                    if meta < META_END {
                        pptr = pptr.add(1);
                        continue; /* Literal */
                    }
                }
            }

            /* The extra data item length for each meta is in a table. */

            meta = (meta >> 16) & 0x7fff;
            if meta as usize >= meta_extra_lengths.len() {
                return core::ptr::null_mut();
            }
            pptr = pptr.add(meta_extra_lengths[meta as usize] as usize);

            pptr = pptr.add(1);
        }
    }
}

/*************************************************
*       Find length of a parsed group            *
*************************************************/

/* This is called for nested groups within a branch of a lookbehind whose
length is being computed. On entry, the pointer must be at the first element
after the group initializing code. On exit it points to OP_KET.

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
    minptr: *mut c_int,
    isinline: BOOL,
    errcodeptr: *mut c_int,
    lcptr: *mut c_int,
    group: c_int,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> c_int {
    unsafe {
        let gi = (*cb).groupinfo.add(2 * group as usize);
        let mut grouplength: c_int = -1;
        let mut groupminlength: c_int = INT_MAX;

        /* The cache can be used only if there is no possibility of there being two
        groups with the same number. We do not need to set the end pointer for a group
        that is being processed as a back reference or recursion, but we must do so for
        an inline group. */

        if group > 0 && ((*cb).external_flags & PCRE2_DUPCAPUSED) == 0 {
            let groupinfo = *gi.add(0);
            if (groupinfo & GI_NOT_FIXED_LENGTH) != 0 {
                return -1;
            }
            if (groupinfo & GI_SET_FIXED_LENGTH) != 0 {
                if isinline != FALSE {
                    *pptrptr = parsed_skip(*pptrptr, PSKIP_KET);
                }
                *minptr = *gi.add(1) as c_int;
                return (groupinfo & GI_FIXED_LENGTH_MASK) as c_int;
            }
        }

        /* Scan the group. In this case we find the end pointer of necessity. */

        loop {
            let mut branchminlength: c_int = 0;
            let branchlength = get_branchlength(
                pptrptr,
                &mut branchminlength,
                errcodeptr,
                lcptr,
                recurses,
                cb,
            );
            if branchlength < 0 {
                /* ISNOTFIXED */
                if group > 0 {
                    *gi.add(0) |= GI_NOT_FIXED_LENGTH;
                }
                return -1;
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
            *gi.add(0) |= (GI_SET_FIXED_LENGTH | grouplength as u32);
            *gi.add(1) = groupminlength as u32;
        }

        *minptr = groupminlength;
        grouplength
    }
}

/*************************************************
*        Find length of a parsed branch          *
*************************************************/

/* Return fixed maximum and minimum lengths for a branch in a lookbehind,
giving an error if the length is not limited. On entry, *pptrptr points to the
first element inside the branch. On exit it is set to point to the ALT or KET.

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
        let mut pptr = *pptrptr;
        let mut offset: PCRE2_SIZE = 0;
        let mut this_recurse: parsed_recurse_check =
            parsed_recurse_check { prev: core::ptr::null(), groupptr: core::ptr::null() };

        /* A large and/or complex regex can take too long to process. */

        let cur = *lcptr;
        *lcptr = cur + 1;
        if cur > 2000 {
            *errcodeptr = ERR35; /* Lookbehind is too complicated */
            return -1;
        }

        /* Scan the branch, accumulating the length. */

        'mainloop: loop {
            let mut group: u32 = 0;
            let mut itemlength: u32 = 0;
            let mut itemminlength: u32 = 0;

            if *pptr < META_END {
                itemlength = 1;
                itemminlength = 1;
            } else {
                /* Labelled targets for the goto-driven flow of the original C. */
                #[derive(PartialEq)]
                enum Flow {
                    Normal,
                    Repetition,
                    RecurseOrBackref,
                    CheckGroup,
                    IsNotFixed,
                }
                let mut flow = Flow::Normal;

                // Placeholders carried between goto targets.
                let mut min_v: u32 = 0;
                let mut max_v: u32 = 0;

                match meta_code(*pptr) {
                    META_KET | META_ALT => break 'mainloop, /* goto EXIT */

                    /* (*ACCEPT) and (*FAIL) terminate the branch, but we must skip to
                    the actual termination. */
                    META_ACCEPT | META_FAIL => {
                        pptr = parsed_skip(pptr, PSKIP_ALT);
                        if pptr.is_null() {
                            /* PARSED_SKIP_FAILED */
                            *errcodeptr = ERR90;
                            return -1;
                        }
                        break 'mainloop; /* goto EXIT */
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
                        pptr = pptr.add(3 + SIZEOFFSET);
                    }

                    /* Only some escapes consume a character. \R can match one or two
                    characters, but \X is never allowed. \C is allowed only in 32-bit
                    and non-UTF 8/16-bit modes. */
                    META_ESCAPE => {
                        let escape = meta_data(*pptr);
                        if escape == ESC_X as u32 {
                            return -1;
                        }
                        if escape == ESC_R as u32 {
                            itemminlength = 1;
                            itemlength = 2;
                        } else if escape > ESC_b as u32 && escape < ESC_Z as u32 {
                            if ((*cb).external_options & PCRE2_UTF) != 0 && escape == ESC_C as u32 {
                                *errcodeptr = ERR36;
                                return -1;
                            }
                            itemlength = 1;
                            itemminlength = 1;
                            if escape == ESC_p as u32 || escape == ESC_P as u32 {
                                pptr = pptr.add(1); /* Skip prop data */
                            }
                        }
                    }

                    /* Lookaheads do not contribute to the length of this branch, but
                    they may contain lookbehinds within them whose lengths need to be
                    set. */
                    META_LOOKAHEAD | META_LOOKAHEADNOT | META_LOOKAHEAD_NA | META_SCS => {
                        *errcodeptr = check_lookbehinds(pptr.add(1), &mut pptr, recurses, cb, lcptr);
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
                    }

                    /* A nested lookbehind does not contribute any length to this
                    lookbehind, but must itself be checked and have its lengths set.
                    set_lookbehind_lengths() updates pptr, leaving it pointing to the
                    final ket of the group. */
                    META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA => {
                        if set_lookbehind_lengths(&mut pptr, errcodeptr, lcptr, recurses, cb)
                            == FALSE
                        {
                            return -1;
                        }
                    }

                    /* Back references and recursions are handled by very similar code. */
                    META_BACKREF_BYNAME => {
                        if ((*cb).external_options & PCRE2_MATCH_UNSET_BACKREF) != 0 {
                            flow = Flow::IsNotFixed;
                        } else {
                            /* Fall through to META_RECURSE_BYNAME handling. */
                            let meta_c = meta_code(*pptr);
                            pptr = pptr.add(1);
                            let length = *pptr;

                            offset = getplusoffset(&mut pptr);
                            let name = (*cb).start_pattern.add(offset);
                            let ng = crate::compile_cgroup::find_named_group(name, length, cb);

                            if ng.is_null() {
                                *errcodeptr = ERR15; /* Non-existent subpattern */
                                (*cb).erroroffset = offset;
                                return -1;
                            }

                            group = (*ng).number;
                            let is_dupname = ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) != 0;

                            if meta_c == META_RECURSE_BYNAME
                                || (!is_dupname
                                    && ((*cb).external_flags & PCRE2_DUPCAPUSED) == 0)
                            {
                                flow = Flow::RecurseOrBackref;
                            } else {
                                flow = Flow::IsNotFixed; /* Duplicate name or number */
                            }
                        }
                    }

                    META_RECURSE_BYNAME => {
                        let meta_c = meta_code(*pptr);
                        pptr = pptr.add(1);
                        let length = *pptr;

                        offset = getplusoffset(&mut pptr);
                        let name = (*cb).start_pattern.add(offset);
                        let ng = crate::compile_cgroup::find_named_group(name, length, cb);

                        if ng.is_null() {
                            *errcodeptr = ERR15; /* Non-existent subpattern */
                            (*cb).erroroffset = offset;
                            return -1;
                        }

                        group = (*ng).number;
                        let is_dupname = ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) != 0;

                        if meta_c == META_RECURSE_BYNAME
                            || (!is_dupname && ((*cb).external_flags & PCRE2_DUPCAPUSED) == 0)
                        {
                            flow = Flow::RecurseOrBackref;
                        } else {
                            flow = Flow::IsNotFixed;
                        }
                    }

                    META_BACKREF => {
                        if ((*cb).external_options & PCRE2_MATCH_UNSET_BACKREF) != 0
                            || ((*cb).external_flags & PCRE2_DUPCAPUSED) != 0
                        {
                            flow = Flow::IsNotFixed;
                        } else {
                            group = meta_data(*pptr);
                            if group < 10 {
                                offset = (*cb).small_ref_offset[group as usize];
                                flow = Flow::RecurseOrBackref;
                            } else {
                                /* Fall through to META_RECURSE. For groups >= 10 -
                                picking up group twice does no harm. */
                                group = meta_data(*pptr);
                                offset = getplusoffset(&mut pptr);
                                flow = Flow::RecurseOrBackref;
                            }
                        }
                    }

                    /* A true recursion implies not fixed length, but a subroutine call
                    may be OK. Back reference "recursions" are also failed. */
                    META_RECURSE => {
                        group = meta_data(*pptr);
                        offset = getplusoffset(&mut pptr);
                        flow = Flow::RecurseOrBackref;
                    }

                    /* A (DEFINE) group is never obeyed inline and so it does not
                    contribute to the length of this branch. Skip from the following
                    item to the next unpaired ket. */
                    META_COND_DEFINE => {
                        pptr = parsed_skip(pptr.add(1), PSKIP_KET);
                    }

                    /* Check other nested groups - advance past the initial data for
                    each type and then seek a fixed length with get_grouplength(). */
                    META_COND_NAME | META_COND_NUMBER | META_COND_RNAME | META_COND_RNUMBER => {
                        pptr = pptr.add(2 + SIZEOFFSET);
                        flow = Flow::CheckGroup;
                    }

                    META_COND_ASSERT => {
                        pptr = pptr.add(1);
                        flow = Flow::CheckGroup;
                    }

                    META_COND_VERSION => {
                        pptr = pptr.add(4);
                        flow = Flow::CheckGroup;
                    }

                    META_CAPTURE => {
                        group = meta_data(*pptr);
                        /* Fall through */
                        pptr = pptr.add(1);
                        flow = Flow::CheckGroup;
                    }

                    META_ATOMIC | META_NOCAPTURE | META_SCRIPT_RUN => {
                        pptr = pptr.add(1);
                        flow = Flow::CheckGroup;
                    }

                    META_QUERY | META_QUERY_PLUS | META_QUERY_QUERY => {
                        min_v = 0;
                        max_v = 1;
                        flow = Flow::Repetition;
                    }

                    /* Exact repetition is OK; variable repetition is not. A repetition
                    of zero must subtract the length that has already been added. */
                    META_MINMAX | META_MINMAX_PLUS | META_MINMAX_QUERY => {
                        min_v = *pptr.add(1);
                        max_v = *pptr.add(2);
                        pptr = pptr.add(2);
                        flow = Flow::Repetition;
                    }

                    _ => {
                        flow = Flow::IsNotFixed;
                    }
                }

                /* Handle the goto targets in dependency order. */

                if flow == Flow::RecurseOrBackref {
                    /* RECURSE_OR_BACKREF_LENGTH */
                    if group > (*cb).bracount {
                        (*cb).erroroffset = offset;
                        *errcodeptr = ERR15; /* Non-existent subpattern */
                        return -1;
                    }
                    if group == 0 {
                        flow = Flow::IsNotFixed; /* Local recursion */
                    } else {
                        let mut gptr = (*cb).parsed_pattern;
                        while *gptr != META_END {
                            if meta_code(*gptr) == META_BIGVALUE {
                                gptr = gptr.add(1);
                            } else if *gptr == (META_CAPTURE | group) {
                                break;
                            }
                            gptr = gptr.add(1);
                        }

                        /* We must start the search for the end of the group at the
                        first meta code inside the group. */
                        let gptrend = parsed_skip(gptr.add(1), PSKIP_KET);
                        if gptrend.is_null() {
                            *errcodeptr = ERR90;
                            return -1;
                        }
                        if pptr > gptr && pptr < gptrend {
                            flow = Flow::IsNotFixed; /* Local recursion */
                        } else {
                            let mut r = recurses;
                            while !r.is_null() {
                                if (*r).groupptr == gptr as *const u32 {
                                    break;
                                }
                                r = (*r).prev as *mut parsed_recurse_check;
                            }
                            if !r.is_null() {
                                flow = Flow::IsNotFixed; /* Mutual recursion */
                            } else {
                                this_recurse.prev = recurses as *const parsed_recurse_check;
                                this_recurse.groupptr = gptr as *const u32;

                                /* We do not need to know the position of the end of the
                                group. Setting the second argument FALSE stops it
                                scanning for the end when the length is cached. */
                                let mut gp = gptr.add(1);
                                grouplength = get_grouplength(
                                    &mut gp,
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
                                        flow = Flow::IsNotFixed;
                                    } else {
                                        return -1; /* Error already set */
                                    }
                                } else {
                                    itemlength = grouplength as u32;
                                    itemminlength = groupminlength as u32;
                                }
                            }
                        }
                    }
                }

                if flow == Flow::CheckGroup {
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

                if flow == Flow::Repetition {
                    /* REPETITION */
                    let min = min_v;
                    let max = max_v;
                    if max != REPEAT_UNLIMITED {
                        if lastitemlength != 0 /* Should not occur, but just in case */
                            && max != 0
                            && (INT_MAX - branchlength) as u32 / lastitemlength < max - 1
                        {
                            *errcodeptr = ERR87; /* Integer overflow; lookbehind too big */
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
                    } else {
                        /* Fall through to default: not fixed length. */
                        flow = Flow::IsNotFixed;
                    }
                }

                if flow == Flow::IsNotFixed {
                    /* ISNOTFIXED */
                    *errcodeptr = ERR25; /* Not fixed length */
                    return -1;
                }
            }

            /* Add the item length to the branchlength, checking for integer overflow
            and for the branch length exceeding the overall limit. */

            if INT_MAX - branchlength < itemlength as c_int
                || {
                    branchlength += itemlength as c_int;
                    branchlength > LOOKBEHIND_MAX
                }
            {
                *errcodeptr = ERR87;
                return -1;
            }

            branchminlength += itemminlength as c_int;

            /* Save this item length for use if the next item is a quantifier. */

            lastitemlength = itemlength;
            lastitemminlength = itemminlength;

            pptr = pptr.add(1);
        }

        /* EXIT */
        *pptrptr = pptr;
        *minptr = branchminlength;
        branchlength
    }
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
    errcodeptr: *mut c_int,
    lcptr: *mut c_int,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> BOOL {
    unsafe {
        let mut bptr = *pptrptr;
        let gbptr = bptr;
        let mut maxlength: c_int = 0;
        let mut minlength: c_int = INT_MAX;
        let mut variable: BOOL = FALSE;

        let offset = readplusoffset(bptr); /* Offset for error messages */
        *pptrptr = (*pptrptr).add(SIZEOFFSET);

        /* Each branch can have a different maximum length, but we can keep only a
        single minimum for the whole group, because there's nowhere to save
        individual values in the META_ALT item. */

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
                /* The errorcode and offset may already be set from a nested
                lookbehind. */
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

            if meta_code(*bptr) != META_ALT {
                break;
            }
        }

        /* If any branch is of variable length, the whole lookbehind is of variable
        length. If the maximum length of any branch exceeds the maximum for variable
        lookbehinds, give an error. Otherwise, the minimum length is set in the word
        that follows the original group META value. For a fixed-length lookbehind,
        this is set to LOOKBEHIND_MAX. */

        if variable != FALSE {
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

/*************************************************
*         Check parsed pattern lookbehinds       *
*************************************************/

/* This function is called at the end of parsing a pattern if any lookbehinds
were encountered. It scans the parsed pattern for them, calling
set_lookbehind_lengths() for each one.

This function is called recursively from get_branchlength() for lookaheads in
order to process any lookbehinds that they may contain. It stops when it hits a
non-nested closing parenthesis in this case, returning a pointer to it.

Arguments
  pptr      points to where to start (start of pattern or start of lookahead)
  retptr    if not NULL, return the ket pointer here
  recurses  chain of recurse_check to catch mutual recursion
  cb        points to the compile block
  lcptr     points to loop counter

Returns:    0 on success, or an errorcode (cb->erroroffset will be set)
*/
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

        while *pptr != META_END {
            if *pptr < META_END {
                pptr = pptr.add(1);
                continue; /* Literal */
            }

            match meta_code(*pptr) {
                META_ESCAPE => {
                    if *pptr - META_ESCAPE == ESC_P as u32 || *pptr - META_ESCAPE == ESC_p as u32 {
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

                META_ACCEPT | META_ALT | META_ASTERISK | META_ASTERISK_PLUS
                | META_ASTERISK_QUERY | META_BACKREF | META_CIRCUMFLEX | META_CLASS
                | META_CLASS_EMPTY | META_CLASS_EMPTY_NOT | META_CLASS_END | META_CLASS_NOT
                | META_COMMIT | META_DOLLAR | META_DOT | META_FAIL | META_PLUS
                | META_PLUS_PLUS | META_PLUS_QUERY | META_PRUNE | META_QUERY | META_QUERY_PLUS
                | META_QUERY_QUERY | META_RANGE_ESCAPED | META_RANGE_LITERAL | META_SKIP
                | META_THEN => {}

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

                /* set_lookbehind_lengths() updates pptr, leaving it pointing to the
                final ket of the group. */
                META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA => {
                    if set_lookbehind_lengths(&mut pptr, &mut errorcode, lcptr, recurses, cb)
                        == FALSE
                    {
                        return errorcode;
                    }
                }

                _ => {
                    /* This branch should be avoided by providing a proper
                    implementation for all supported cases. */
                    (*cb).erroroffset = 0;
                    return ERR70; /* Unrecognized meta code */
                }
            }

            pptr = pptr.add(1);
        }

        0
    }
}
