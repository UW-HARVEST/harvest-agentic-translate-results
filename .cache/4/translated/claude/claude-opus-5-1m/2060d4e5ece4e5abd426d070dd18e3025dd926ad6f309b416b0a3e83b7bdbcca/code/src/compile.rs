// Translated from pcre2_compile.c lines 8895-11350
use crate::compile_branch::*;
use crate::compile_h::*;
use crate::compile_parse::*;
use crate::compile_tables::*;
use crate::compile_util::*;
use crate::internal::*;
use crate::pcre2_pub::*;
use crate::tables::*;
use crate::ucd_data::*;
use crate::ucp::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

/*************************************************
*   Check for anchored pattern                   *
*************************************************/

/* Try to find out if this is an anchored regular expression. Consider each
alternative branch. If they all start with OP_SOD or OP_CIRC, or with a bracket
all of whose alternatives start with OP_SOD or OP_CIRC (recurse ad lib), then
it's anchored. However, if this is a multiline pattern, then only OP_SOD will
be found, because ^ generates OP_CIRCM in that case.

We can also consider a regex to be anchored if OP_SOM starts all its branches.
This is the code for \G, which means "match at start of match position, taking
into account the match offset".

A branch is also implicitly anchored if it starts with .* and DOTALL is set,
because that will try the rest of the pattern at all possible matching points,
so there is no point trying again.... er ....

.... except when the .* appears inside capturing parentheses, and there is a
subsequent back reference to those parentheses. We haven't enough information
to catch that case precisely.

At first, the best we could do was to detect when .* was in capturing brackets
and the highest back reference was greater than or equal to that level.
However, by keeping a bitmap of the first 31 back references, we can catch some
of the more common cases more precisely.

Also, the code cannot be anchored if a *SKIP or *PRUNE is present, because
this prevents the number of characters it matches from being adjusted.

Arguments:
  code           points to start of the compiled pattern
  bracket_map    a bitmap of which brackets we are inside while testing; this
                   handles up to substring 31; after that we just have to take
                   the less precise approach
  cb             points to the compile data block
  atomcount      atomic group level
  inassert       TRUE if in an assertion
  dotstar_anchor TRUE if automatic anchoring optimization is enabled

Returns:     TRUE or FALSE
*/

pub(crate) unsafe fn is_anchored(code: PCRE2_SPTR, bracket_map: u32, cb: *mut compile_block, atomcount: c_int, inassert: BOOL, dotstar_anchor: BOOL) -> BOOL {
    let mut code = code;
    loop {
        let scode: PCRE2_SPTR = first_significant_code(
            code.add(_pcre2_OP_lengths_8[*code as usize] as usize), FALSE);
        let op: c_int = *scode as c_int;

        /* Non-capturing brackets */

        if op as u32 == OP_BRA || op as u32 == OP_BRAPOS ||
           op as u32 == OP_SBRA || op as u32 == OP_SBRAPOS
        {
            if is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }

        /* Capturing brackets */

        else if op as u32 == OP_CBRA || op as u32 == OP_CBRAPOS ||
                op as u32 == OP_SCBRA || op as u32 == OP_SCBRAPOS
        {
            let n: c_int = GET2(scode, 1 + LINK_SIZE) as c_int;
            let new_map: u32 = bracket_map | (if n < 32 { 1u32 << n } else { 1 });
            if is_anchored(scode, new_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }

        /* Positive forward assertion */

        else if op as u32 == OP_ASSERT || op as u32 == OP_ASSERT_NA {
            if is_anchored(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }

        /* Condition. If there is no second branch, it can't be anchored. */

        else if op as u32 == OP_COND || op as u32 == OP_SCOND {
            if *scode.add(GET(scode, 1) as usize) as u32 != OP_ALT { return FALSE; }
            if is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }

        /* Atomic groups */

        else if op as u32 == OP_ONCE {
            if is_anchored(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }

        /* .* is not anchored unless DOTALL is set (which generates OP_ALLANY) and
        it isn't in brackets that are or may be referenced or inside an atomic
        group or an assertion. Also the pattern must not contain *PRUNE or *SKIP,
        because these break the feature. Consider, for example, /(?s).*?(*PRUNE)b/
        with the subject "aab", which matches "b", i.e. not at the start of a line.
        There is also an option that disables auto-anchoring. */

        else if op as u32 == OP_TYPESTAR || op as u32 == OP_TYPEMINSTAR ||
                op as u32 == OP_TYPEPOSSTAR
        {
            if *scode.add(1) as u32 != OP_ALLANY || (bracket_map & (*cb).backref_map) != 0 ||
               atomcount > 0 || (*cb).had_pruneorskip != 0 || inassert != 0 ||
               dotstar_anchor == 0
            {
                return FALSE;
            }
        }

        /* Check for explicit anchoring */

        else if op as u32 != OP_SOD && op as u32 != OP_SOM && op as u32 != OP_CIRC {
            return FALSE;
        }

        code = code.add(GET(code, 1) as usize);

        if !(*code as u32 == OP_ALT) { break; } /* Loop for each alternative */
    }
    return TRUE;
}

/*************************************************
*         Check for starting with ^ or .*        *
*************************************************/

/* This is called to find out if every branch starts with ^ or .* so that
"first char" processing can be done to speed things up in multiline
matching and for non-DOTALL patterns that start with .* (which must start at
the beginning or after \n).

Arguments:
  code           points to start of the compiled pattern or a group
  bracket_map    a bitmap of which brackets we are inside while testing
  cb             points to the compile data
  atomcount      atomic group level
  inassert       TRUE if in an assertion
  dotstar_anchor TRUE if automatic anchoring optimization is enabled

Returns:         TRUE or FALSE
*/

pub(crate) unsafe fn is_startline(code: PCRE2_SPTR, bracket_map: c_uint, cb: *mut compile_block, atomcount: c_int, inassert: BOOL, dotstar_anchor: BOOL) -> BOOL {
    let mut code = code;
    loop {
        let mut scode: PCRE2_SPTR = first_significant_code(
            code.add(_pcre2_OP_lengths_8[*code as usize] as usize), FALSE);
        let mut op: c_int = *scode as c_int;

        /* If we are at the start of a conditional assertion group, *both* the
        conditional assertion *and* what follows the condition must satisfy the test
        for start of line. Other kinds of condition fail. Note that there may be an
        auto-callout at the start of a condition. */

        if op as u32 == OP_COND {
            scode = scode.add(1 + LINK_SIZE);

            if *scode as u32 == OP_CALLOUT {
                scode = scode.add(_pcre2_OP_lengths_8[OP_CALLOUT as usize] as usize);
            } else if *scode as u32 == OP_CALLOUT_STR {
                scode = scode.add(GET(scode, 1 + 2 * LINK_SIZE) as usize);
            }

            match *scode as u32 {
                OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF | OP_FAIL | OP_FALSE | OP_TRUE => {
                    return FALSE;
                }

                _ => {
                    /* Assertion */
                    if is_startline(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor) == FALSE {
                        return FALSE;
                    }
                    loop {
                        scode = scode.add(GET(scode, 1) as usize);
                        if !(*scode as u32 == OP_ALT) { break; }
                    }
                    scode = scode.add(1 + LINK_SIZE);
                }
            }
            scode = first_significant_code(scode, FALSE);
            op = *scode as c_int;
        }

        /* Non-capturing brackets */

        if op as u32 == OP_BRA || op as u32 == OP_BRAPOS ||
           op as u32 == OP_SBRA || op as u32 == OP_SBRAPOS
        {
            if is_startline(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }

        /* Capturing brackets */

        else if op as u32 == OP_CBRA || op as u32 == OP_CBRAPOS ||
                op as u32 == OP_SCBRA || op as u32 == OP_SCBRAPOS
        {
            let n: c_int = GET2(scode, 1 + LINK_SIZE) as c_int;
            let new_map: c_uint = bracket_map | (if n < 32 { 1u32 << n } else { 1 });
            if is_startline(scode, new_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }

        /* Positive forward assertions */

        else if op as u32 == OP_ASSERT || op as u32 == OP_ASSERT_NA {
            if is_startline(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }

        /* Atomic brackets */

        else if op as u32 == OP_ONCE {
            if is_startline(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }

        /* .* means "start at start or after \n" if it isn't in atomic brackets or
        brackets that may be referenced or an assertion, and as long as the pattern
        does not contain *PRUNE or *SKIP. */

        else if op as u32 == OP_TYPESTAR || op as u32 == OP_TYPEMINSTAR ||
                op as u32 == OP_TYPEPOSSTAR
        {
            if *scode.add(1) as u32 != OP_ANY || (bracket_map & (*cb).backref_map) != 0 ||
               atomcount > 0 || (*cb).had_pruneorskip != 0 || inassert != 0 ||
               dotstar_anchor == 0
            {
                return FALSE;
            }
        }

        /* Check for explicit circumflex; anything else gives a FALSE result. */

        else if op as u32 != OP_CIRC && op as u32 != OP_CIRCM {
            return FALSE;
        }

        /* Move on to the next alternative */

        code = code.add(GET(code, 1) as usize);

        if !(*code as u32 == OP_ALT) { break; } /* Loop for each alternative */
    }
    return TRUE;
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
    let mut code = code;
    loop {
        let c: PCRE2_UCHAR = *code;
        if c as u32 == OP_END { return core::ptr::null_mut(); }
        if c as u32 == OP_RECURSE { return code; }

        /* XCLASS is used for classes that cannot be represented just by a bit map.
        This includes negated single high-valued characters. ECLASS is used for
        classes that use set operations internally. CALLOUT_STR is used for
        callouts with string arguments. In each case the length in the table is
        zero; the actual length is stored in the compiled code. */

        if c as u32 == OP_XCLASS || c as u32 == OP_ECLASS {
            code = code.add(GET(code, 1) as usize);
        } else if c as u32 == OP_CALLOUT_STR {
            code = code.add(GET(code, 1 + 2 * LINK_SIZE) as usize);
        }

        /* Otherwise, we can get the item's length from the table, except that for
        repeated character types, we have to test for \p and \P, which have an extra
        two code units of parameters, and for MARK/PRUNE/SKIP/THEN with an argument,
        we must add in its length. */

        else {
            match c as u32 {
                OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS |
                OP_TYPEQUERY | OP_TYPEMINQUERY | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS |
                OP_TYPEPOSQUERY => {
                    if *code.add(1) as u32 == OP_PROP || *code.add(1) as u32 == OP_NOTPROP {
                        code = code.add(2);
                    }
                }

                OP_TYPEPOSUPTO | OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT => {
                    if *code.add(1 + IMM2_SIZE) as u32 == OP_PROP ||
                       *code.add(1 + IMM2_SIZE) as u32 == OP_NOTPROP
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
                    OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI |
                    OP_EXACT | OP_EXACTI | OP_NOTEXACT | OP_NOTEXACTI |
                    OP_UPTO | OP_UPTOI | OP_NOTUPTO | OP_NOTUPTOI |
                    OP_MINUPTO | OP_MINUPTOI | OP_NOTMINUPTO | OP_NOTMINUPTOI |
                    OP_POSUPTO | OP_POSUPTOI | OP_NOTPOSUPTO | OP_NOTPOSUPTOI |
                    OP_STAR | OP_STARI | OP_NOTSTAR | OP_NOTSTARI |
                    OP_MINSTAR | OP_MINSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI |
                    OP_POSSTAR | OP_POSSTARI | OP_NOTPOSSTAR | OP_NOTPOSSTARI |
                    OP_PLUS | OP_PLUSI | OP_NOTPLUS | OP_NOTPLUSI |
                    OP_MINPLUS | OP_MINPLUSI | OP_NOTMINPLUS | OP_NOTMINPLUSI |
                    OP_POSPLUS | OP_POSPLUSI | OP_NOTPOSPLUS | OP_NOTPOSPLUSI |
                    OP_QUERY | OP_QUERYI | OP_NOTQUERY | OP_NOTQUERYI |
                    OP_MINQUERY | OP_MINQUERYI | OP_NOTMINQUERY | OP_NOTMINQUERYI |
                    OP_POSQUERY | OP_POSQUERYI | OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                        if HAS_EXTRALEN(*code.offset(-1) as u32) {
                            code = code.add(GET_EXTRALEN(*code.offset(-1) as u32) as usize);
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

pub(crate) unsafe fn find_firstassertedcu(code: PCRE2_SPTR, flags: *mut u32, inassert: u32) -> u32 {
    let mut code = code;
    let mut c: u32 = 0;
    let mut cflags: u32 = REQ_NONE;

    *flags = REQ_NONE;
    loop {
        let d: u32;
        let mut dflags: u32 = 0;
        let xl: c_int = if *code as u32 == OP_CBRA || *code as u32 == OP_SCBRA ||
                           *code as u32 == OP_CBRAPOS || *code as u32 == OP_SCBRAPOS
                        { IMM2_SIZE as c_int } else { 0 };
        let mut scode: PCRE2_SPTR =
            first_significant_code(code.add(1 + LINK_SIZE + xl as usize), TRUE);
        let op: PCRE2_UCHAR = *scode;

        'sw: {
            match op as u32 {
                OP_BRA | OP_BRAPOS | OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS |
                OP_ASSERT | OP_ASSERT_NA | OP_ONCE | OP_SCRIPT_RUN => {
                    d = find_firstassertedcu(scode, &mut dflags, inassert +
                        (if op as u32 == OP_ASSERT || op as u32 == OP_ASSERT_NA { 1 } else { 0 }));
                    if dflags >= REQ_NONE { return 0; }
                    if cflags >= REQ_NONE { c = d; cflags = dflags; }
                    else if c != d || cflags != dflags { return 0; }
                    break 'sw;
                }

                OP_EXACT | OP_CHAR | OP_PLUS | OP_MINPLUS | OP_POSPLUS => {
                    if op as u32 == OP_EXACT { scode = scode.add(IMM2_SIZE); }
                    /* Fall through */
                    if inassert == 0 { return 0; }
                    if cflags >= REQ_NONE { c = *scode.add(1) as u32; cflags = 0; }
                    else if c != *scode.add(1) as u32 { return 0; }
                    break 'sw;
                }

                OP_EXACTI | OP_CHARI | OP_PLUSI | OP_MINPLUSI | OP_POSPLUSI => {
                    if op as u32 == OP_EXACTI { scode = scode.add(IMM2_SIZE); }
                    /* Fall through */
                    if inassert == 0 { return 0; }

                    /* If the character is more than one code unit long, we cannot set
                    its first code unit when matching caselessly. Later scanning may
                    pick up multiple code units. */

                    if *scode.add(1) as u32 >= 0x80 { return 0; }

                    if cflags >= REQ_NONE { c = *scode.add(1) as u32; cflags = REQ_CASELESS; }
                    else if c != *scode.add(1) as u32 { return 0; }
                    break 'sw;
                }

                _ => {
                    return 0;
                }
            }
        }

        code = code.add(GET(code, 1) as usize);

        if !(*code as u32 == OP_ALT) { break; }
    }

    *flags = cflags;
    return c;
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
    let mut pptr = pptr;
    let mut nestlevel: u32 = 0;

    loop {
        let mut meta: u32 = META_CODE(*pptr);
        let mut do_continue: bool = false;

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
                    if META_DATA(*pptr) >= 10 { pptr = pptr.add(SIZEOFFSET); }
                    break 'sw;
                }

                META_ESCAPE => {
                    if (*pptr).wrapping_sub(META_ESCAPE) == ESC_P as u32 ||
                       (*pptr).wrapping_sub(META_ESCAPE) == ESC_p as u32
                    {
                        pptr = pptr.add(1); /* Skip prop data */
                    }
                    break 'sw;
                }

                /* Add the length of the name. */
                META_MARK | META_COMMIT_ARG | META_PRUNE_ARG | META_SKIP_ARG |
                META_THEN_ARG => {
                    pptr = pptr.add(*pptr.add(1) as usize);
                    break 'sw;
                }

                /* These are the "active" items in this loop. */

                META_CLASS_END => {
                    if skiptype == PSKIP_CLASS { return pptr; }
                    break 'sw;
                }

                META_ATOMIC | META_CAPTURE | META_COND_ASSERT | META_COND_DEFINE |
                META_COND_NAME | META_COND_NUMBER | META_COND_RNAME |
                META_COND_RNUMBER | META_COND_VERSION | META_SCS | META_LOOKAHEAD |
                META_LOOKAHEADNOT | META_LOOKAHEAD_NA | META_LOOKBEHIND |
                META_LOOKBEHINDNOT | META_LOOKBEHIND_NA | META_NOCAPTURE |
                META_SCRIPT_RUN => {
                    nestlevel += 1;
                    break 'sw;
                }

                META_ALT => {
                    if nestlevel == 0 && skiptype == PSKIP_ALT { return pptr; }
                    break 'sw;
                }

                META_KET => {
                    if nestlevel == 0 { return pptr; }
                    nestlevel -= 1;
                    break 'sw;
                }

                _ => {
                    /* Just skip over most items */
                    if meta < META_END { do_continue = true; break 'sw; } /* Literal */
                    break 'sw;
                }
            }
        }

        if !do_continue {
            /* The extra data item length for each meta is in a table. */

            meta = (meta >> 16) & 0x7fff;
            if meta >= meta_extra_lengths.len() as u32 { return core::ptr::null_mut(); }
            pptr = pptr.add(meta_extra_lengths[meta as usize] as usize);
        }

        pptr = pptr.add(1);
    }
}

/*************************************************
*       Find length of a parsed group            *
*************************************************/

/* This is called for nested groups within a branch of a lookbehind whose
length is being computed. On entry, the pointer must be at the first element
after the group initializing code. On exit it points to OP_KET. Caching is used
to improve processing speed when the same capturing group occurs many times.

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

pub(crate) unsafe fn get_grouplength(pptrptr: *mut *mut u32, minptr: *mut c_int, isinline: BOOL, errcodeptr: *mut c_int, lcptr: *mut c_int, group: c_int, recurses: *mut parsed_recurse_check, cb: *mut compile_block) -> c_int {
    let gi: *mut u32 = (*cb).groupinfo.wrapping_offset((2 * group) as isize);
    let mut branchlength: c_int;
    let mut branchminlength: c_int = 0;
    let mut grouplength: c_int = -1;
    let mut groupminlength: c_int = c_int::MAX;

    /* The cache can be used only if there is no possibility of there being two
    groups with the same number. We do not need to set the end pointer for a group
    that is being processed as a back reference or recursion, but we must do so for
    an inline group. */

    if group > 0 && ((*cb).external_flags & PCRE2_DUPCAPUSED) == 0 {
        let groupinfo: u32 = *gi;
        if (groupinfo & GI_NOT_FIXED_LENGTH) != 0 { return -1; }
        if (groupinfo & GI_SET_FIXED_LENGTH) != 0 {
            if isinline != 0 { *pptrptr = parsed_skip(*pptrptr, PSKIP_KET); }
            *minptr = *gi.add(1) as c_int;
            return (groupinfo & GI_FIXED_LENGTH_MASK) as c_int;
        }
    }

    /* Scan the group. In this case we find the end pointer of necessity. */

    'isnotfixed: {
        loop {
            branchlength = get_branchlength(pptrptr, &mut branchminlength, errcodeptr,
                lcptr, recurses, cb);
            if branchlength < 0 { break 'isnotfixed; }
            if branchlength > grouplength { grouplength = branchlength; }
            if branchminlength < groupminlength { groupminlength = branchminlength; }
            if **pptrptr == META_KET { break; }
            *pptrptr = (*pptrptr).add(1); /* Skip META_ALT */
        }

        if group > 0 {
            *gi = *gi | (GI_SET_FIXED_LENGTH | grouplength as u32);
            *gi.add(1) = groupminlength as u32;
        }

        *minptr = groupminlength;
        return grouplength;
    }

    /* ISNOTFIXED: */
    if group > 0 { *gi = *gi | GI_NOT_FIXED_LENGTH; }
    return -1;
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

pub(crate) unsafe fn get_branchlength(pptrptr: *mut *mut u32, minptr: *mut c_int, errcodeptr: *mut c_int, lcptr: *mut c_int, recurses: *mut parsed_recurse_check, cb: *mut compile_block) -> c_int {
    let mut branchlength: c_int = 0;
    let mut branchminlength: c_int = 0;
    let mut grouplength: c_int;
    let mut groupminlength: c_int = 0;
    let mut lastitemlength: u32 = 0;
    let mut lastitemminlength: u32 = 0;
    let mut pptr: *mut u32 = *pptrptr;
    let mut offset: PCRE2_SIZE = 0;
    let mut this_recurse: parsed_recurse_check = parsed_recurse_check {
        prev: core::ptr::null_mut(),
        groupptr: core::ptr::null_mut(),
    };

    /* A large and/or complex regex can take too long to process. This can happen
    more often when (?| groups are present in the pattern because their length
    cannot be cached. */

    if { let v = *lcptr; *lcptr = v + 1; v } > 2000 {
        *errcodeptr = ERR(35); /* Lookbehind is too complicated */
        return -1;
    }

    /* Scan the branch, accumulating the length. */

    'exit_l: {
        loop {
            let mut r: *mut parsed_recurse_check = core::ptr::null_mut();
            let mut gptr: *mut u32 = core::ptr::null_mut();
            let gptrend: *mut u32;
            let escape: u32;
            let mut min: u32 = 0;
            let mut max: u32 = 0;
            let mut group: u32 = 0;
            let mut itemlength: u32 = 0;
            let mut itemminlength: u32 = 0;

            if *pptr < META_END {
                itemlength = 1;
                itemminlength = 1;
            } else {
                'sw: {
                'recurse_or_backref: {
                'check_group: {
                'repetition: {
                    match META_CODE(*pptr) {
                        META_KET | META_ALT => {
                            break 'exit_l;
                        }

                        /* (*ACCEPT) and (*FAIL) terminate the branch, but we must skip
                        to the actual termination. */

                        META_ACCEPT | META_FAIL => {
                            pptr = parsed_skip(pptr, PSKIP_ALT);
                            if pptr.is_null() {
                                /* PARSED_SKIP_FAILED */
                                *errcodeptr = ERR(90);
                                return -1;
                            }
                            break 'exit_l;
                        }

                        META_MARK | META_COMMIT_ARG | META_PRUNE_ARG | META_SKIP_ARG |
                        META_THEN_ARG => {
                            pptr = pptr.add((*pptr.add(1)).wrapping_add(1) as usize);
                            break 'sw;
                        }

                        META_CIRCUMFLEX | META_COMMIT | META_DOLLAR | META_PRUNE |
                        META_SKIP | META_THEN => {
                            break 'sw;
                        }

                        META_OPTIONS => {
                            pptr = pptr.add(2);
                            break 'sw;
                        }

                        META_BIGVALUE => {
                            itemlength = 1;
                            itemminlength = 1;
                            pptr = pptr.add(1);
                            break 'sw;
                        }

                        META_CLASS | META_CLASS_NOT => {
                            itemlength = 1;
                            itemminlength = 1;
                            pptr = parsed_skip(pptr, PSKIP_CLASS);
                            if pptr.is_null() {
                                /* PARSED_SKIP_FAILED */
                                *errcodeptr = ERR(90);
                                return -1;
                            }
                            break 'sw;
                        }

                        META_CLASS_EMPTY_NOT | META_DOT => {
                            itemlength = 1;
                            itemminlength = 1;
                            break 'sw;
                        }

                        META_CALLOUT_NUMBER => {
                            pptr = pptr.add(3);
                            break 'sw;
                        }

                        META_CALLOUT_STRING => {
                            pptr = pptr.add(3 + SIZEOFFSET);
                            break 'sw;
                        }

                        /* Only some escapes consume a character. Of those, \R can match
                        one or two characters, but \X is never allowed because it matches
                        an unknown number of characters. \C is allowed only in 32-bit and
                        non-UTF 8/16-bit modes. */

                        META_ESCAPE => {
                            escape = META_DATA(*pptr);
                            if escape == ESC_X as u32 { return -1; }
                            if escape == ESC_R as u32 {
                                itemminlength = 1;
                                itemlength = 2;
                            } else if escape > ESC_b as u32 && escape < ESC_Z as u32 {
                                if ((*cb).external_options & PCRE2_UTF) != 0 &&
                                   escape == ESC_C as u32
                                {
                                    *errcodeptr = ERR(36);
                                    return -1;
                                }
                                itemlength = 1;
                                itemminlength = 1;
                                if escape == ESC_p as u32 || escape == ESC_P as u32 {
                                    pptr = pptr.add(1); /* Skip prop data */
                                }
                            }
                            break 'sw;
                        }

                        /* Lookaheads do not contribute to the length of this branch, but
                        they may contain lookbehinds within them whose lengths need to be
                        set. */

                        META_LOOKAHEAD | META_LOOKAHEADNOT | META_LOOKAHEAD_NA |
                        META_SCS => {
                            let p1 = pptr.add(1);
                            *errcodeptr = check_lookbehinds(p1, &mut pptr, recurses, cb, lcptr);
                            if *errcodeptr != 0 { return -1; }

                            /* Ignore any qualifiers that follow a lookahead assertion. */

                            match *pptr.add(1) {
                                META_ASTERISK | META_ASTERISK_PLUS | META_ASTERISK_QUERY |
                                META_PLUS | META_PLUS_PLUS | META_PLUS_QUERY |
                                META_QUERY | META_QUERY_PLUS | META_QUERY_QUERY => {
                                    pptr = pptr.add(1);
                                }

                                META_MINMAX | META_MINMAX_PLUS | META_MINMAX_QUERY => {
                                    pptr = pptr.add(3);
                                }

                                _ => {}
                            }
                            break 'sw;
                        }

                        /* A nested lookbehind does not contribute any length to this
                        lookbehind, but must itself be checked and have its lengths set. */

                        META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA => {
                            if set_lookbehind_lengths(&mut pptr, errcodeptr, lcptr, recurses, cb)
                                == FALSE
                            {
                                return -1;
                            }
                            break 'sw;
                        }

                        /* Back references and recursions are handled by very similar
                        code. */

                        META_BACKREF_BYNAME | META_RECURSE_BYNAME => {
                            if META_CODE(*pptr) == META_BACKREF_BYNAME &&
                               ((*cb).external_options & PCRE2_MATCH_UNSET_BACKREF) != 0
                            {
                                /* ISNOTFIXED */
                                *errcodeptr = ERR(25);
                                return -1;
                            }
                            /* Fall through */
                            {
                                let name: PCRE2_SPTR;
                                let is_dupname: BOOL;
                                let ng: *mut named_group;
                                let meta_code: u32 = META_CODE(*pptr);
                                pptr = pptr.add(1);
                                let length: u32 = *pptr;

                                /* GETPLUSOFFSET(offset, pptr) */
                                offset = ((*pptr.add(1) as usize) << 32) |
                                         (*pptr.add(2) as usize);
                                pptr = pptr.add(2);

                                name = (*cb).start_pattern.add(offset);
                                ng = crate::compile_cgroup::_pcre2_compile_find_named_group8(
                                    name, length, cb);

                                if ng.is_null() {
                                    *errcodeptr = ERR(15); /* Non-existent subpattern */
                                    (*cb).erroroffset = offset;
                                    return -1;
                                }

                                group = (*ng).number;
                                is_dupname =
                                    (((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) != 0) as BOOL;

                                /* A numerical back reference can be fixed length if
                                duplicate capturing groups are not being used. A
                                non-duplicate named back reference can also be handled. */

                                if meta_code == META_RECURSE_BYNAME ||
                                   (is_dupname == FALSE &&
                                    ((*cb).external_flags & PCRE2_DUPCAPUSED) == 0)
                                {
                                    /* Handle as a numbered version. */
                                    break 'recurse_or_backref;
                                }
                            }
                            /* Duplicate name or number: ISNOTFIXED */
                            *errcodeptr = ERR(25);
                            return -1;
                        }

                        /* The offset values for back references < 10 are in a separate
                        vector. */

                        META_BACKREF => {
                            if ((*cb).external_options & PCRE2_MATCH_UNSET_BACKREF) != 0 ||
                               ((*cb).external_flags & PCRE2_DUPCAPUSED) != 0
                            {
                                /* ISNOTFIXED */
                                *errcodeptr = ERR(25);
                                return -1;
                            }
                            group = META_DATA(*pptr);
                            if group < 10 {
                                offset = (*cb).small_ref_offset[group as usize];
                                break 'recurse_or_backref;
                            }

                            /* Fall through: for groups >= 10 - picking up group twice
                            does no harm. */

                            group = META_DATA(*pptr);
                            /* GETPLUSOFFSET(offset, pptr) */
                            offset = ((*pptr.add(1) as usize) << 32) | (*pptr.add(2) as usize);
                            pptr = pptr.add(2);
                            break 'recurse_or_backref;
                        }

                        /* A true recursion implies not fixed length, but a subroutine
                        call may be OK. Back reference "recursions" are also failed. */

                        META_RECURSE => {
                            group = META_DATA(*pptr);
                            /* GETPLUSOFFSET(offset, pptr) */
                            offset = ((*pptr.add(1) as usize) << 32) | (*pptr.add(2) as usize);
                            pptr = pptr.add(2);
                            break 'recurse_or_backref;
                        }

                        /* A (DEFINE) group is never obeyed inline and so it does not
                        contribute to the length of this branch. */

                        META_COND_DEFINE => {
                            pptr = parsed_skip(pptr.add(1), PSKIP_KET);
                            break 'sw;
                        }

                        /* Check other nested groups. */

                        META_COND_NAME | META_COND_NUMBER | META_COND_RNAME |
                        META_COND_RNUMBER => {
                            pptr = pptr.add(2 + SIZEOFFSET);
                            break 'check_group;
                        }

                        META_COND_ASSERT => {
                            pptr = pptr.add(1);
                            break 'check_group;
                        }

                        META_COND_VERSION => {
                            pptr = pptr.add(4);
                            break 'check_group;
                        }

                        META_CAPTURE => {
                            group = META_DATA(*pptr);
                            /* Fall through */
                            pptr = pptr.add(1);
                            break 'check_group;
                        }

                        META_ATOMIC | META_NOCAPTURE | META_SCRIPT_RUN => {
                            pptr = pptr.add(1);
                            break 'check_group;
                        }

                        META_QUERY | META_QUERY_PLUS | META_QUERY_QUERY => {
                            min = 0;
                            max = 1;
                            break 'repetition;
                        }

                        /* Exact repetition is OK; variable repetition is not. A
                        repetition of zero must subtract the length that has already
                        been added. */

                        META_MINMAX | META_MINMAX_PLUS | META_MINMAX_QUERY => {
                            min = *pptr.add(1);
                            max = *pptr.add(2);
                            pptr = pptr.add(2);
                            break 'repetition;
                        }

                        /* Any other item means this branch does not have a fixed
                        length. */

                        _ => {
                            /* ISNOTFIXED */
                            *errcodeptr = ERR(25); /* Not fixed length */
                            return -1;
                        }
                    }
                }
                /* REPETITION: */
                if max != REPEAT_UNLIMITED {
                    if lastitemlength != 0 && /* Should not occur, but just in case */
                       max != 0 &&
                       ((c_int::MAX - branchlength) as u32) / lastitemlength <
                           max.wrapping_sub(1)
                    {
                        *errcodeptr = ERR(87); /* Integer overflow; lookbehind too big */
                        return -1;
                    }
                    if min == 0 {
                        branchminlength =
                            branchminlength.wrapping_sub(lastitemminlength as c_int);
                    } else {
                        itemminlength = min.wrapping_sub(1).wrapping_mul(lastitemminlength);
                    }
                    if max == 0 {
                        branchlength = branchlength.wrapping_sub(lastitemlength as c_int);
                    } else {
                        itemlength = max.wrapping_sub(1).wrapping_mul(lastitemlength);
                    }
                    break 'sw;
                }
                /* Fall through to ISNOTFIXED */
                *errcodeptr = ERR(25); /* Not fixed length */
                return -1;
                }
                /* CHECK_GROUP: */
                grouplength = get_grouplength(&mut pptr, &mut groupminlength, TRUE,
                    errcodeptr, lcptr, group as c_int, recurses, cb);
                if grouplength < 0 { return -1; }
                itemlength = grouplength as u32;
                itemminlength = groupminlength as u32;
                break 'sw;
                }
                /* RECURSE_OR_BACKREF_LENGTH: */
                if group > (*cb).bracount {
                    (*cb).erroroffset = offset;
                    *errcodeptr = ERR(15); /* Non-existent subpattern */
                    return -1;
                }
                if group == 0 {
                    /* Local recursion: ISNOTFIXED */
                    *errcodeptr = ERR(25);
                    return -1;
                }
                gptr = (*cb).parsed_pattern;
                loop {
                    if *gptr == META_END { break; }
                    if META_CODE(*gptr) == META_BIGVALUE {
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
                    /* PARSED_SKIP_FAILED */
                    *errcodeptr = ERR(90);
                    return -1;
                }
                if pptr > gptr && pptr < gptrend {
                    /* Local recursion: ISNOTFIXED */
                    *errcodeptr = ERR(25);
                    return -1;
                }
                r = recurses;
                while !r.is_null() {
                    if (*r).groupptr == gptr { break; }
                    r = (*r).prev;
                }
                if !r.is_null() {
                    /* Mutual recursion: ISNOTFIXED */
                    *errcodeptr = ERR(25);
                    return -1;
                }
                this_recurse.prev = recurses;
                this_recurse.groupptr = gptr;

                /* We do not need to know the position of the end of the group, that is,
                gptr is not used after the call to get_grouplength(). */

                gptr = gptr.add(1);
                grouplength = get_grouplength(&mut gptr, &mut groupminlength, FALSE,
                    errcodeptr, lcptr, group as c_int, &mut this_recurse, cb);
                if grouplength < 0 {
                    if *errcodeptr == 0 {
                        /* ISNOTFIXED */
                        *errcodeptr = ERR(25);
                        return -1;
                    }
                    return -1; /* Error already set */
                }
                itemlength = grouplength as u32;
                itemminlength = groupminlength as u32;
                break 'sw;
                }
            }

            /* Add the item length to the branchlength, checking for integer overflow
            and for the branch length exceeding the overall limit. */

            if (c_int::MAX - branchlength) < itemlength as c_int || {
                branchlength = branchlength.wrapping_add(itemlength as c_int);
                branchlength > LOOKBEHIND_MAX
            } {
                *errcodeptr = ERR(87);
                return -1;
            }

            branchminlength = branchminlength.wrapping_add(itemminlength as c_int);

            /* Save this item length for use if the next item is a quantifier. */

            lastitemlength = itemlength;
            lastitemminlength = itemminlength;

            pptr = pptr.add(1);
        }
    }

    /* EXIT: */
    *pptrptr = pptr;
    *minptr = branchminlength;
    return branchlength;
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

pub(crate) unsafe fn set_lookbehind_lengths(pptrptr: *mut *mut u32, errcodeptr: *mut c_int, lcptr: *mut c_int, recurses: *mut parsed_recurse_check, cb: *mut compile_block) -> BOOL {
    let offset: PCRE2_SIZE;
    let mut bptr: *mut u32 = *pptrptr;
    let gbptr: *mut u32 = bptr;
    let mut maxlength: c_int = 0;
    let mut minlength: c_int = c_int::MAX;
    let mut variable: BOOL = FALSE;

    /* READPLUSOFFSET(offset, bptr) -- Offset for error messages */
    offset = ((*bptr.add(1) as usize) << 32) | (*bptr.add(2) as usize);
    *pptrptr = (*pptrptr).add(SIZEOFFSET);

    /* Each branch can have a different maximum length, but we can keep only a
    single minimum for the whole group, because there's nowhere to save individual
    values in the META_ALT item. */

    loop {
        let branchlength: c_int;
        let mut branchminlength: c_int = 0;

        *pptrptr = (*pptrptr).add(1);
        branchlength = get_branchlength(pptrptr, &mut branchminlength, errcodeptr, lcptr,
            recurses, cb);

        if branchlength < 0 {
            /* The errorcode and offset may already be set from a nested lookbehind. */
            if *errcodeptr == 0 { *errcodeptr = ERR(25); }
            if (*cb).erroroffset == PCRE2_UNSET { (*cb).erroroffset = offset; }
            return FALSE;
        }

        if branchlength != branchminlength { variable = TRUE; }
        if branchminlength < minlength { minlength = branchminlength; }
        if branchlength > maxlength { maxlength = branchlength; }
        if branchlength > (*cb).max_lookbehind { (*cb).max_lookbehind = branchlength; }
        *bptr = *bptr | branchlength as u32; /* branchlength never more than 65535 */
        bptr = *pptrptr;

        if !(META_CODE(*bptr) == META_ALT) { break; }
    }

    /* If any branch is of variable length, the whole lookbehind is of variable
    length. */

    if variable != FALSE {
        *gbptr.add(1) = minlength as u32;
        if (maxlength as PCRE2_SIZE) > (*cb).max_varlookbehind as PCRE2_SIZE {
            *errcodeptr = ERR(100);
            (*cb).erroroffset = offset;
            return FALSE;
        }
    } else {
        *gbptr.add(1) = LOOKBEHIND_MAX as u32;
    }

    return TRUE;
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

pub(crate) unsafe fn check_lookbehinds(pptr: *mut u32, retptr: *mut *mut u32, recurses: *mut parsed_recurse_check, cb: *mut compile_block, lcptr: *mut c_int) -> c_int {
    let mut pptr = pptr;
    let mut errorcode: c_int = 0;
    let mut nestlevel: c_int = 0;

    (*cb).erroroffset = PCRE2_UNSET;

    while *pptr != META_END {
        if *pptr < META_END {
            pptr = pptr.add(1);
            continue; /* Literal */
        }

        match META_CODE(*pptr) {
            META_ESCAPE => {
                if (*pptr).wrapping_sub(META_ESCAPE) == ESC_P as u32 ||
                   (*pptr).wrapping_sub(META_ESCAPE) == ESC_p as u32
                {
                    pptr = pptr.add(1); /* Skip prop data */
                }
            }

            META_KET => {
                nestlevel -= 1;
                if nestlevel < 0 {
                    if !retptr.is_null() { *retptr = pptr; }
                    return 0;
                }
            }

            META_ATOMIC | META_CAPTURE | META_COND_ASSERT | META_SCS | META_LOOKAHEAD |
            META_LOOKAHEADNOT | META_LOOKAHEAD_NA | META_NOCAPTURE | META_SCRIPT_RUN => {
                nestlevel += 1;
            }

            META_ACCEPT | META_ALT | META_ASTERISK | META_ASTERISK_PLUS |
            META_ASTERISK_QUERY | META_BACKREF | META_CIRCUMFLEX | META_CLASS |
            META_CLASS_EMPTY | META_CLASS_EMPTY_NOT | META_CLASS_END | META_CLASS_NOT |
            META_COMMIT | META_DOLLAR | META_DOT | META_FAIL | META_PLUS |
            META_PLUS_PLUS | META_PLUS_QUERY | META_PRUNE | META_QUERY |
            META_QUERY_PLUS | META_QUERY_QUERY | META_RANGE_ESCAPED |
            META_RANGE_LITERAL | META_SKIP | META_THEN => {}

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

            META_BIGVALUE | META_POSIX | META_POSIX_NEG | META_CAPTURE_NAME |
            META_CAPTURE_NUMBER => {
                pptr = pptr.add(1);
            }

            META_MINMAX | META_MINMAX_QUERY | META_MINMAX_PLUS | META_OPTIONS => {
                pptr = pptr.add(2);
            }

            META_CALLOUT_NUMBER => {
                pptr = pptr.add(3);
            }

            META_MARK | META_COMMIT_ARG | META_PRUNE_ARG | META_SKIP_ARG |
            META_THEN_ARG => {
                pptr = pptr.add(1 + *pptr.add(1) as usize);
            }

            /* Note that set_lookbehind_lengths() updates pptr, leaving it pointing to
            the final ket of the group, so no need to update it here. */

            META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA => {
                if set_lookbehind_lengths(&mut pptr, &mut errorcode, lcptr, recurses, cb)
                    == FALSE
                {
                    return errorcode;
                }
            }

            /* The following erroroffset is a bogus but safe value. */

            _ => {
                (*cb).erroroffset = 0;
                return ERR(70); /* Unrecognized meta code */
            }
        }

        pptr = pptr.add(1);
    }

    return 0;
}

/*************************************************
*     External function to compile a pattern     *
*************************************************/

/* This function reads a regular expression in the form of a string and returns
a pointer to a block of store holding a compiled version of the expression.

Arguments:
  pattern       the regular expression
  patlen        the length of the pattern, or PCRE2_ZERO_TERMINATED
  options       option bits
  errorptr      pointer to errorcode
  erroroffset   pointer to error offset
  ccontext      points to a compile context or is NULL

Returns:        pointer to compiled data block, or NULL on error,
                with errorcode and erroroffset set
*/

const RSCAN_CACHE_SIZE: usize = 8;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_8(
    pattern: PCRE2_SPTR,
    patlen: PCRE2_SIZE,
    options: u32,
    errorptr: *mut c_int,
    erroroffset: *mut PCRE2_SIZE,
    ccontext: *mut pcre2_real_compile_context,
) -> *mut pcre2_real_code {
    let mut pattern = pattern;
    let mut patlen = patlen;
    let mut options = options;
    let mut ccontext = ccontext;

    let mut utf: BOOL = FALSE; /* Set TRUE for UTF mode */
    let mut ucp: BOOL = FALSE; /* Set TRUE for UCP mode */
    let mut has_lookbehind: BOOL = FALSE; /* Set TRUE if a lookbehind is found */
    let zero_terminated: BOOL; /* Set TRUE for zero-terminated pattern */
    let mut re: *mut pcre2_real_code = core::ptr::null_mut(); /* What we will return */
    let mut cb: compile_block = core::mem::zeroed(); /* "Static" compile-time data */
    let mut tables: *const u8 = core::ptr::null(); /* Char tables base pointer */

    let null_str: [PCRE2_UCHAR; 1] = [0xcd]; /* Dummy for handling null inputs */
    let mut code: *mut PCRE2_UCHAR = core::ptr::null_mut(); /* Current pointer in compiled code */
    let mut codestart: *mut PCRE2_UCHAR = core::ptr::null_mut(); /* Start of compiled code */
    let mut ptr: PCRE2_SPTR = core::ptr::null(); /* Current pointer in pattern */
    let mut pptr: *mut u32 = core::ptr::null_mut(); /* Current pointer in parsed pattern */

    let mut length: PCRE2_SIZE = 1; /* Allow for final END opcode */
    let mut usedlength: PCRE2_SIZE = 0; /* Actual length used */
    let mut re_blocksize: PCRE2_SIZE = 0; /* Size of memory block */
    let mut parsed_size_needed: PCRE2_SIZE = 0; /* Needed for parsed pattern */

    let mut firstcuflags: u32 = 0;
    let mut reqcuflags: u32 = 0; /* Type of first/req code unit */
    let mut firstcu: u32 = 0;
    let mut reqcu: u32 = 0; /* Value of first/req code unit */
    let mut setflags: u32 = 0; /* NL and BSR set flags */
    let mut xoptions: u32 = 0; /* Flags from context, modified */

    let mut skipatstart: u32 = 0; /* When checking (*UTF) etc */
    let mut limit_heap: u32 = u32::MAX;
    let mut limit_match: u32 = u32::MAX; /* Unset match limits */
    let mut limit_depth: u32 = u32::MAX;

    let mut newline: c_int = 0; /* Unset; can be set by the pattern */
    let mut bsr: c_int = 0; /* Unset; can be set by the pattern */
    let mut errorcode: c_int = 0; /* Initialize to avoid compiler warn */
    let mut regexrc: c_int = 0; /* Return from compile */

    let mut i: u32; /* Local loop counter */

    /* Enable all optimizations by default. */
    let mut optim_flags: u32 = if !ccontext.is_null() {
        (*ccontext).optimization_flags
    } else {
        PCRE2_OPTIMIZATION_ALL
    };

    /* Comments at the head of this file explain about these variables. */

    let mut stack_groupinfo: [u32; GROUPINFO_DEFAULT_SIZE] = [0; GROUPINFO_DEFAULT_SIZE];
    let mut stack_parsed_pattern: [u32; PARSED_PATTERN_DEFAULT_SIZE] =
        [0; PARSED_PATTERN_DEFAULT_SIZE];
    let mut named_groups: [named_group; NAMED_GROUP_LIST_SIZE] = [named_group {
        name: core::ptr::null(),
        number: 0,
        length: 0,
        hash_dup: 0,
    }; NAMED_GROUP_LIST_SIZE];

    /* The workspace is used in different ways in the different compiling phases.
    It needs to be 16-bit aligned for the preliminary parsing scan. */

    let mut c16workspace: [u32; C16_WORK_SIZE] = [0; C16_WORK_SIZE];
    let cworkspace: *mut PCRE2_UCHAR = c16workspace.as_mut_ptr() as *mut PCRE2_UCHAR;

    /* -------------- Check arguments and set up the pattern ----------------- */

    /* There must be error code and offset pointers. */

    if errorptr.is_null() {
        if !erroroffset.is_null() { *erroroffset = 0; }
        return core::ptr::null_mut();
    }
    if erroroffset.is_null() {
        if !errorptr.is_null() { *errorptr = ERR(120); }
        return core::ptr::null_mut();
    }
    *errorptr = ERR(0);
    *erroroffset = 0;

    /* There must be a pattern, but NULL is allowed with zero length. */

    if pattern.is_null() {
        if patlen == 0 {
            pattern = null_str.as_ptr();
        } else {
            *errorptr = ERR(16);
            return core::ptr::null_mut();
        }
    }

    /* A NULL compile context means "use a default context" */

    if ccontext.is_null() {
        ccontext = &raw mut crate::context::_pcre2_default_compile_context_8;
    }

    /* PCRE2_MATCH_INVALID_UTF implies UTF */

    if (options & PCRE2_MATCH_INVALID_UTF) != 0 { options |= PCRE2_UTF; }

    /* Check that all undefined public option bits are zero. */

    if (options & !PUBLIC_COMPILE_OPTIONS) != 0
        || ((*ccontext).extra_options & !PUBLIC_COMPILE_EXTRA_OPTIONS) != 0
    {
        *errorptr = ERR(17);
        return core::ptr::null_mut();
    }

    if (options & PCRE2_LITERAL) != 0
        && ((options & !PUBLIC_LITERAL_COMPILE_OPTIONS) != 0
            || ((*ccontext).extra_options & !PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS) != 0)
    {
        *errorptr = ERR(92);
        return core::ptr::null_mut();
    }

    /* A zero-terminated pattern is indicated by the special length value
    PCRE2_ZERO_TERMINATED. Check for an overlong pattern. */

    zero_terminated = (patlen == PCRE2_ZERO_TERMINATED) as BOOL;
    if zero_terminated != FALSE {
        patlen = crate::string_utils::_pcre2_strlen_8(pattern);
    }
    let _ = zero_terminated; /* Silence compiler; only used if Valgrind enabled */

    if patlen > (*ccontext).max_pattern_length {
        *errorptr = ERR(88);
        return core::ptr::null_mut();
    }

    /* Optimization flags in 'options' can override those in the compile context. */

    if (options & PCRE2_NO_AUTO_POSSESS) != 0 { optim_flags &= !PCRE2_OPTIM_AUTO_POSSESS; }
    if (options & PCRE2_NO_DOTSTAR_ANCHOR) != 0 { optim_flags &= !PCRE2_OPTIM_DOTSTAR_ANCHOR; }
    if (options & PCRE2_NO_START_OPTIMIZE) != 0 { optim_flags &= !PCRE2_OPTIM_START_OPTIMIZE; }

    /* From here on, all returns from this function should end up going via the
    EXIT label. */

    'exit_l: {
    'had_error: {
    'had_early_error: {
    'had_cb_error: {

        /* ------------ Initialize the "static" compile data -------------- */

        tables = if !(*ccontext).tables.is_null() {
            (*ccontext).tables
        } else {
            _pcre2_default_tables_8.as_ptr()
        };

        cb.lcc = tables.add(lcc_offset); /* Individual */
        cb.fcc = tables.add(fcc_offset); /*   character */
        cb.cbits = tables.add(cbits_offset); /*      tables */
        cb.ctypes = tables.add(ctypes_offset);

        cb.assert_depth = 0;
        cb.bracount = 0;
        cb.cx = ccontext;
        cb.dupnames = FALSE;
        cb.end_pattern = pattern.add(patlen);
        cb.erroroffset = 0;
        cb.external_flags = 0;
        cb.external_options = options;
        cb.groupinfo = stack_groupinfo.as_mut_ptr();
        cb.had_recurse = FALSE;
        cb.lastcapture = 0;
        cb.max_lookbehind = 0; /* Max encountered */
        cb.max_varlookbehind = (*ccontext).max_varlookbehind; /* Limit */
        cb.name_entry_size = 0;
        cb.name_table = core::ptr::null_mut();
        cb.named_groups = named_groups.as_mut_ptr();
        cb.named_group_list_size = NAMED_GROUP_LIST_SIZE as u32;
        cb.names_found = 0;
        cb.parens_depth = 0;
        cb.parsed_pattern = stack_parsed_pattern.as_mut_ptr();
        cb.req_varyopt = 0;
        cb.start_code = cworkspace;
        cb.start_pattern = pattern;
        cb.start_workspace = cworkspace;
        cb.workspace_size = COMPILE_WORK_SIZE;
        cb.first_data = core::ptr::null_mut();
        cb.last_data = core::ptr::null_mut();
        cb.char_lists_size = 0;

        /* Maximum back reference and backref bitmap. */

        cb.top_backref = 0;
        cb.backref_map = 0;

        i = 0;
        while i < 10 {
            cb.small_ref_offset[i as usize] = PCRE2_UNSET;
            i += 1;
        }

        /* --------------- Start looking at the pattern --------------- */

        xoptions = (*ccontext).extra_options;
        ptr = pattern;
        skipatstart = 0;

        if (options & PCRE2_LITERAL) == 0 {
            while patlen.wrapping_sub(skipatstart as usize) >= 2
                && *ptr.add(skipatstart as usize) as u32 == CHAR_LEFT_PARENTHESIS
                && *ptr.add(skipatstart as usize + 1) as u32 == CHAR_ASTERISK
            {
                i = 0;
                while (i as usize) < pso_list.len() {
                    let p: *const pso = pso_list.as_ptr().add(i as usize);

                    if patlen
                        .wrapping_sub(skipatstart as usize)
                        .wrapping_sub(2)
                        >= (*p).length as usize
                        && crate::string_utils::_pcre2_strncmp_c8_8(
                            ptr.add(skipatstart as usize + 2),
                            (*p).name,
                            (*p).length as usize,
                        ) == 0
                    {
                        let mut c: u32;
                        let mut pp: u32;

                        skipatstart += (*p).length as u32 + 2;
                        match (*p).type_ {
                            PSO_OPT => {
                                cb.external_options |= (*p).value;
                            }

                            PSO_XOPT => {
                                xoptions |= (*p).value;
                            }

                            PSO_FLG => {
                                setflags |= (*p).value;
                            }

                            PSO_NL => {
                                newline = (*p).value as c_int;
                                setflags |= PCRE2_NL_SET;
                            }

                            PSO_BSR => {
                                bsr = (*p).value as c_int;
                                setflags |= PCRE2_BSR_SET;
                            }

                            PSO_LIMM | PSO_LIMD | PSO_LIMH => {
                                c = 0;
                                pp = skipatstart;
                                while (pp as usize) < patlen
                                    && IS_DIGIT(*ptr.add(pp as usize) as u32)
                                {
                                    if c > u32::MAX / 10 - 1 { break; } /* Integer overflow */
                                    let d = *ptr.add(pp as usize) as u32;
                                    pp += 1;
                                    c = c * 10 + (d - CHAR_0);
                                }
                                if pp as usize >= patlen
                                    || pp == skipatstart
                                    || *ptr.add(pp as usize) as u32 != CHAR_RIGHT_PARENTHESIS
                                {
                                    errorcode = ERR(60);
                                    ptr = ptr.add(pp as usize);
                                    utf = FALSE; /* Used by HAD_EARLY_ERROR */
                                    break 'had_early_error;
                                }
                                if (*p).type_ == PSO_LIMH {
                                    limit_heap = c;
                                } else if (*p).type_ == PSO_LIMM {
                                    limit_match = c;
                                } else {
                                    limit_depth = c;
                                }
                                pp += 1;
                                skipatstart = pp;
                            }

                            PSO_OPTMZ => {
                                optim_flags &= !((*p).value);

                                /* For backward compatibility the three original VERBs to
                                disable optimizations need to also update the corresponding
                                bit in the external options. */

                                match (*p).value {
                                    PCRE2_OPTIM_AUTO_POSSESS => {
                                        cb.external_options |= PCRE2_NO_AUTO_POSSESS;
                                    }

                                    PCRE2_OPTIM_DOTSTAR_ANCHOR => {
                                        cb.external_options |= PCRE2_NO_DOTSTAR_ANCHOR;
                                    }

                                    PCRE2_OPTIM_START_OPTIMIZE => {
                                        cb.external_options |= PCRE2_NO_START_OPTIMIZE;
                                    }

                                    _ => {}
                                }
                            }

                            _ => {}
                        }
                        break; /* Out of the table scan loop */
                    }
                    i += 1;
                }
                if (i as usize) >= pso_list.len() { break; } /* Out of pso loop */
            }
        }

        /* End of pattern-start options; advance to start of real regex. */

        ptr = ptr.add(skipatstart as usize);

        /* Check UTF. */

        utf = ((cb.external_options & PCRE2_UTF) != 0) as BOOL;
        if utf != FALSE {
            if (options & PCRE2_NEVER_UTF) != 0 {
                errorcode = ERR(74);
                break 'had_early_error;
            }
            if (options & PCRE2_NO_UTF_CHECK) == 0 {
                errorcode = crate::valid_utf::_pcre2_valid_utf_8(pattern, patlen, erroroffset);
                if errorcode != 0 {
                    break 'had_error; /* Offset was set by valid_utf() */
                }
            }
        }

        /* Check UCP lockout. */

        ucp = ((cb.external_options & PCRE2_UCP) != 0) as BOOL;
        if ucp != FALSE && (cb.external_options & PCRE2_NEVER_UCP) != 0 {
            errorcode = ERR(75);
            break 'had_early_error;
        }

        /* PCRE2_EXTRA_TURKISH_CASING checks */

        if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0 {
            if utf == FALSE && ucp == FALSE {
                errorcode = ERR(104);
                break 'had_early_error;
            }

            if utf == FALSE {
                errorcode = ERR(105);
                break 'had_early_error;
            }

            if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0 {
                errorcode = ERR(106);
                break 'had_early_error;
            }
        }

        /* Process the BSR setting. */

        if bsr == 0 { bsr = (*ccontext).bsr_convention as c_int; }

        /* Process the newline setting. */

        if newline == 0 { newline = (*ccontext).newline_convention as c_int; }
        cb.nltype = NLTYPE_FIXED;
        match newline as u32 {
            PCRE2_NEWLINE_CR => {
                cb.nllen = 1;
                cb.nl[0] = CHAR_CR as PCRE2_UCHAR;
            }

            PCRE2_NEWLINE_LF => {
                cb.nllen = 1;
                cb.nl[0] = CHAR_NL as PCRE2_UCHAR;
            }

            PCRE2_NEWLINE_NUL => {
                cb.nllen = 1;
                cb.nl[0] = CHAR_NUL as PCRE2_UCHAR;
            }

            PCRE2_NEWLINE_CRLF => {
                cb.nllen = 2;
                cb.nl[0] = CHAR_CR as PCRE2_UCHAR;
                cb.nl[1] = CHAR_NL as PCRE2_UCHAR;
            }

            PCRE2_NEWLINE_ANY => {
                cb.nltype = NLTYPE_ANY;
            }

            PCRE2_NEWLINE_ANYCRLF => {
                cb.nltype = NLTYPE_ANYCRLF;
            }

            _ => {
                errorcode = ERR(56);
                break 'had_early_error;
            }
        }

        /* Ensure that the parsed pattern buffer is big enough. */

        parsed_size_needed =
            max_parsed_pattern(ptr, cb.end_pattern, utf, options) as PCRE2_SIZE;

        /* Allow for 2x uint32_t at the start and 2 at the end, for
        PCRE2_EXTRA_MATCH_WORD or PCRE2_EXTRA_MATCH_LINE (which are exclusive). */

        if ((*ccontext).extra_options & (PCRE2_EXTRA_MATCH_WORD | PCRE2_EXTRA_MATCH_LINE)) != 0 {
            parsed_size_needed += 4;
        }

        /* When PCRE2_AUTO_CALLOUT is set we allow for one callout at the end. */

        if (options & PCRE2_AUTO_CALLOUT) != 0 { parsed_size_needed += 4; }

        parsed_size_needed += 1; /* For the final META_END */

        if parsed_size_needed > PARSED_PATTERN_DEFAULT_SIZE {
            let heap_parsed_pattern: *mut u32 = ((*ccontext).memctl.malloc.unwrap())(
                parsed_size_needed * core::mem::size_of::<u32>(),
                (*ccontext).memctl.memory_data,
            ) as *mut u32;
            if heap_parsed_pattern.is_null() {
                *errorptr = ERR(21);
                break 'exit_l;
            }
            cb.parsed_pattern = heap_parsed_pattern;
        }
        cb.parsed_pattern_end = cb.parsed_pattern.add(parsed_size_needed);

        /* Do the parsing scan. */

        errorcode = parse_regex(ptr, cb.external_options, xoptions, &mut has_lookbehind, &mut cb);
        if errorcode != 0 { break 'had_cb_error; }

        /* If there are any lookbehinds, scan the parsed pattern to figure out their
        lengths. */

        if has_lookbehind != FALSE {
            let mut loopcount: c_int = 0;
            if cb.bracount >= (GROUPINFO_DEFAULT_SIZE / 2) as u32 {
                cb.groupinfo = ((*ccontext).memctl.malloc.unwrap())(
                    ((2 * (cb.bracount + 1)) as usize) * core::mem::size_of::<u32>(),
                    (*ccontext).memctl.memory_data,
                ) as *mut u32;
                if cb.groupinfo.is_null() {
                    errorcode = ERR(21);
                    cb.erroroffset = 0;
                    break 'had_cb_error;
                }
            }
            memset(
                cb.groupinfo as *mut c_void,
                0,
                ((2 * cb.bracount + 1) as usize) * core::mem::size_of::<u32>(),
            );
            errorcode = check_lookbehinds(
                cb.parsed_pattern,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut cb,
                &mut loopcount,
            );
            if errorcode != 0 { break 'had_cb_error; }
        }

        /* Pretend to compile the pattern while actually just accumulating the amount
        of memory required in the 'length' variable. */

        cb.erroroffset = patlen; /* For any subsequent errors that do not set it */
        pptr = cb.parsed_pattern;
        code = cworkspace;
        *code = OP_BRA as PCRE2_UCHAR;

        compile_regex(
            cb.external_options,
            xoptions,
            &mut code,
            &mut pptr,
            &mut errorcode,
            0,
            &mut firstcu,
            &mut firstcuflags,
            &mut reqcu,
            &mut reqcuflags,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            &mut cb,
            &mut length,
        );

        if errorcode != 0 { break 'had_cb_error; } /* Offset is in cb.erroroffset */

        /* This should be caught in compile_regex(), but just in case... */

        if length > MAX_PATTERN_SIZE
            || MAX_PATTERN_SIZE - length
                < (cb.char_lists_size / core::mem::size_of::<PCRE2_UCHAR>())
        {
            errorcode = ERR(20);
            cb.erroroffset = 0;
            break 'had_cb_error;
        }

        /* Compute the size of, then, if not too large, get and initialize the data
        block for storing the compiled pattern and names table. */

        re_blocksize = CU2BYTES(
            (cb.names_found as PCRE2_SIZE) * (cb.name_entry_size as PCRE2_SIZE),
        );

        if cb.char_lists_size != 0 {
            /* Align to 32 bit first. This ensures the allocated area will also be
            32 bit aligned. */
            re_blocksize = CLIST_ALIGN_TO(re_blocksize, core::mem::size_of::<u32>());
            re_blocksize += cb.char_lists_size;
        }

        re_blocksize += CU2BYTES(length);

        if re_blocksize > (*ccontext).max_pattern_compiled_length {
            errorcode = ERR(101);
            cb.erroroffset = 0;
            break 'had_cb_error;
        }

        re_blocksize += core::mem::size_of::<pcre2_real_code>();
        re = ((*ccontext).memctl.malloc.unwrap())(re_blocksize, (*ccontext).memctl.memory_data)
            as *mut pcre2_real_code;
        if re.is_null() {
            errorcode = ERR(21);
            cb.erroroffset = 0;
            break 'had_cb_error;
        }

        /* The compiler may put padding at the end of the pcre2_real_code structure.
        To avoid reading undefined bytes we explicitly write to the last 8 bytes of
        the structure before setting the fields. */

        memset(
            (re as *mut u8).add(core::mem::size_of::<pcre2_real_code>() - 8) as *mut c_void,
            0,
            8,
        );
        (*re).memctl = (*ccontext).memctl;
        (*re).tables = tables;
        (*re).executable_jit = core::ptr::null_mut();
        memset((*re).start_bitmap.as_mut_ptr() as *mut c_void, 0, 32);
        (*re).blocksize = re_blocksize;
        (*re).code_start = re_blocksize - CU2BYTES(length);
        (*re).magic_number = MAGIC_NUMBER;
        (*re).compile_options = options;
        (*re).overall_options = cb.external_options;
        (*re).extra_options = xoptions;
        (*re).flags = 1u32 | cb.external_flags | setflags;
        (*re).limit_heap = limit_heap;
        (*re).limit_match = limit_match;
        (*re).limit_depth = limit_depth;
        (*re).first_codeunit = 0;
        (*re).last_codeunit = 0;
        (*re).bsr_convention = bsr as u16;
        (*re).newline_convention = newline as u16;
        (*re).max_lookbehind = 0;
        (*re).minlength = 0;
        (*re).top_bracket = 0;
        (*re).top_backref = 0;
        (*re).name_entry_size = cb.name_entry_size;
        (*re).name_count = cb.names_found;
        (*re).optimization_flags = optim_flags;

        /* The basic block is immediately followed by the name table, and the compiled
        code follows after that. */

        codestart = ((re as *mut u8).add((*re).code_start)) as *mut PCRE2_UCHAR;

        /* Update the compile data block for the actual compile. */

        cb.parens_depth = 0;
        cb.assert_depth = 0;
        cb.lastcapture = 0;
        cb.name_table =
            ((re as *mut u8).add(core::mem::size_of::<pcre2_real_code>())) as *mut PCRE2_UCHAR;
        cb.start_code = codestart;
        cb.req_varyopt = 0;
        cb.had_accept = FALSE;
        cb.had_pruneorskip = FALSE;
        cb.char_lists_size = 0;

        /* If any named groups were found, create the name/number table from the list
        created in the pre-pass. */

        if cb.names_found > 0 {
            let mut ng: *mut named_group = cb.named_groups;
            let mut tablecount: u32 = 0;

            /* Length 0 represents duplicates, and they have already been handled. */
            i = 0;
            while i < cb.names_found as u32 {
                if (*ng).length > 0 {
                    tablecount = crate::compile_cgroup::_pcre2_compile_add_name_to_table8(
                        &mut cb, ng, tablecount,
                    );
                }
                i += 1;
                ng = ng.add(1);
            }
        }

        /* Set up a starting, non-extracting bracket, then compile the expression. */

        pptr = cb.parsed_pattern;
        code = codestart;
        *code = OP_BRA as PCRE2_UCHAR;
        regexrc = compile_regex(
            (*re).overall_options,
            (*re).extra_options,
            &mut code,
            &mut pptr,
            &mut errorcode,
            0,
            &mut firstcu,
            &mut firstcuflags,
            &mut reqcu,
            &mut reqcuflags,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            &mut cb,
            core::ptr::null_mut(),
        );
        if regexrc < 0 { (*re).flags |= PCRE2_MATCH_EMPTY; }
        (*re).top_bracket = cb.bracount as u16;
        (*re).top_backref = cb.top_backref as u16;
        (*re).max_lookbehind = cb.max_lookbehind as u16;

        if cb.had_accept != FALSE {
            reqcu = 0; /* Must disable after (*ACCEPT) */
            reqcuflags = REQ_NONE;
            (*re).flags |= PCRE2_HASACCEPT; /* Disables minimum length */
        }

        /* Fill in the final opcode and check for disastrous overflow. */

        *code = OP_END as PCRE2_UCHAR;
        code = code.add(1);
        usedlength = code.offset_from(codestart) as PCRE2_SIZE;
        if usedlength > length {
            errorcode = ERR(23); /* Overflow of code block - internal error */
            cb.erroroffset = 0;
            break 'had_cb_error;
        }

        (*re).blocksize -= CU2BYTES(length - usedlength);

        /* Scan the pattern for recursion/subroutine calls and convert the group
        numbers into offsets. */

        if errorcode == 0 && cb.had_recurse != FALSE {
            let mut rcode: *mut PCRE2_UCHAR;
            let mut rgroup: PCRE2_SPTR;
            let mut ccount: c_uint = 0;
            let mut start: c_int = RSCAN_CACHE_SIZE as c_int;
            let mut rc: [recurse_cache; RSCAN_CACHE_SIZE] = [recurse_cache {
                group: core::ptr::null(),
                groupnumber: 0,
            }; RSCAN_CACHE_SIZE];

            rcode = find_recurse(codestart, utf);
            while !rcode.is_null() {
                let mut p: c_int;
                let groupnumber: c_int;

                groupnumber = GET(rcode, 1) as c_int;
                if groupnumber == 0 {
                    rgroup = codestart as PCRE2_SPTR;
                } else {
                    let mut search_from: PCRE2_SPTR = codestart as PCRE2_SPTR;
                    rgroup = core::ptr::null();
                    i = 0;
                    p = start;
                    while i < ccount {
                        if groupnumber == rc[p as usize].groupnumber {
                            rgroup = rc[p as usize].group;
                            break;
                        }

                        /* Group n+1 must always start to the right of group n, so we
                        can save search time below when the new group number is greater
                        than any of the previously found groups. */

                        if groupnumber > rc[p as usize].groupnumber {
                            search_from = rc[p as usize].group;
                        }
                        i += 1;
                        p = (p + 1) & 7;
                    }

                    if rgroup.is_null() {
                        rgroup = crate::find_bracket::_pcre2_find_bracket_8(
                            search_from, utf, groupnumber,
                        );
                        if rgroup.is_null() {
                            errorcode = ERR(53);
                            break;
                        }

                        start -= 1;
                        if start < 0 { start = RSCAN_CACHE_SIZE as c_int - 1; }
                        rc[start as usize].groupnumber = groupnumber;
                        rc[start as usize].group = rgroup;
                        if (ccount as usize) < RSCAN_CACHE_SIZE { ccount += 1; }
                    }
                }

                PUT(
                    rcode,
                    1,
                    rgroup.offset_from(codestart as *const PCRE2_UCHAR) as u32,
                );

                rcode = find_recurse(rcode.add(1 + LINK_SIZE), utf);
            }
        }

        /* Unless disabled, check whether any single character iterators can be
        auto-possessified. */

        if errorcode == 0 && (optim_flags & PCRE2_OPTIM_AUTO_POSSESS) != 0 {
            let temp: *mut PCRE2_UCHAR = codestart;
            let possessify_rc: c_int = crate::auto_possess::_pcre2_auto_possessify_8(temp, &cb);
            if possessify_rc != 0 {
                errorcode = ERR(80);
                cb.erroroffset = 0;
            }
        }

        /* Failed to compile, or error while post-processing. */

        if errorcode != 0 { break 'had_cb_error; }

        /* Successful compile. If the anchored option was not passed, set it if
        we can determine that the pattern is anchored. */

        if ((*re).overall_options & PCRE2_ANCHORED) == 0 {
            let dotstar_anchor: BOOL = ((optim_flags & PCRE2_OPTIM_DOTSTAR_ANCHOR) != 0) as BOOL;
            if is_anchored(codestart as PCRE2_SPTR, 0, &mut cb, 0, FALSE, dotstar_anchor) != FALSE {
                (*re).overall_options |= PCRE2_ANCHORED;
            }
        }

        /* Set up the first code unit or startline flag, the required code unit, and
        then study the pattern. */

        if (optim_flags & PCRE2_OPTIM_START_OPTIMIZE) != 0 {
            let mut minminlength: c_int = 0; /* For minimal minlength from first/required CU */
            let study_rc: c_int;

            /* If we do not have a first code unit, see if there is one that is
            asserted. */

            if firstcuflags >= REQ_NONE {
                let mut assertedcuflags: u32 = 0;
                let assertedcu: u32 =
                    find_firstassertedcu(codestart as PCRE2_SPTR, &mut assertedcuflags, 0);
                if assertedcuflags < REQ_NONE && assertedcu != reqcu {
                    firstcu = assertedcu;
                    firstcuflags = assertedcuflags;
                }
            }

            /* Save the data for a first code unit. */

            if firstcuflags < REQ_NONE {
                (*re).first_codeunit = firstcu;
                (*re).flags |= PCRE2_FIRSTSET;
                minminlength += 1;

                /* Handle caseless first code units. */

                if (firstcuflags & REQ_CASELESS) != 0 {
                    if firstcu < 128 || (utf == FALSE && ucp == FALSE && firstcu < 255) {
                        if *cb.fcc.add(firstcu as usize) as u32 != firstcu {
                            (*re).flags |= PCRE2_FIRSTCASELESS;
                        }
                    }
                    /* The first code unit is > 128 in UTF or UCP mode, or > 255
                    otherwise. In 8-bit UTF mode, code units in the range 128-255 are
                    introductory code units and cannot have another case, but if UCP is
                    set they may do. */
                    else if ucp != FALSE && utf == FALSE && UCD_OTHERCASE(firstcu) != firstcu {
                        (*re).flags |= PCRE2_FIRSTCASELESS;
                    }
                }
            }
            /* When there is no first code unit, for non-anchored patterns, see if we
            can set the PCRE2_STARTLINE flag. */
            else if ((*re).overall_options & PCRE2_ANCHORED) == 0 {
                let dotstar_anchor: BOOL =
                    ((optim_flags & PCRE2_OPTIM_DOTSTAR_ANCHOR) != 0) as BOOL;
                if is_startline(codestart as PCRE2_SPTR, 0, &mut cb, 0, FALSE, dotstar_anchor)
                    != FALSE
                {
                    (*re).flags |= PCRE2_STARTLINE;
                }
            }

            /* Handle the "required code unit", if one is set. */

            if reqcuflags < REQ_NONE {
                if ((*re).overall_options & PCRE2_UTF) == 0 ||   /* Not UTF */
                   firstcuflags >= REQ_NONE ||                   /* First not set */
                   (firstcu & 0x80) == 0 ||                      /* First is ASCII */
                   (reqcu & 0x80) == 0
                /* Req is ASCII */
                {
                    minminlength += 1;
                }

                /* In the case of an anchored pattern, set up the value only if it
                follows a variable length item in the pattern. */

                if ((*re).overall_options & PCRE2_ANCHORED) == 0
                    || (reqcuflags & REQ_VARY) != 0
                {
                    (*re).last_codeunit = reqcu;
                    (*re).flags |= PCRE2_LASTSET;

                    /* Handle caseless required code units as for first code units. */

                    if (reqcuflags & REQ_CASELESS) != 0 {
                        if reqcu < 128 || (utf == FALSE && ucp == FALSE && reqcu < 255) {
                            if *cb.fcc.add(reqcu as usize) as u32 != reqcu {
                                (*re).flags |= PCRE2_LASTCASELESS;
                            }
                        } else if ucp != FALSE && utf == FALSE && UCD_OTHERCASE(reqcu) != reqcu {
                            (*re).flags |= PCRE2_LASTCASELESS;
                        }
                    }
                }
            }

            /* Study the compiled pattern to set up information such as a bitmap of
            starting code units and a minimum matching length. */

            study_rc = crate::study::_pcre2_study_8(re);
            if study_rc != 0 {
                errorcode = ERR(31);
                cb.erroroffset = 0;
                break 'had_cb_error;
            }

            /* If study() set a bitmap of starting code units, it implies a minimum
            length of at least one. */

            if ((*re).flags & PCRE2_FIRSTMAPSET) != 0 && minminlength == 0 {
                minminlength = 1;
            }

            /* If the minimum length set (or not set) by study() is less than the
            minimum implied by required code units, override it. */

            if ((*re).minlength as c_int) < minminlength {
                (*re).minlength = minminlength as u16;
            }
        } /* End of start-of-match optimizations. */

        /* Control ends up here in all cases. */

        break 'exit_l;
    }
    /* HAD_CB_ERROR: */
    ptr = pattern.add(cb.erroroffset);
    }
    /* HAD_EARLY_ERROR: */
    *erroroffset = ptr.offset_from(pattern) as PCRE2_SIZE;
    }
    /* HAD_ERROR: */
    *errorptr = errorcode;
    pcre2_code_free_8(re);
    re = core::ptr::null_mut();

    if !cb.first_data.is_null() {
        let mut current_data: *mut compile_data = cb.first_data;
        loop {
            let next_data: *mut compile_data = (*current_data).next;
            ((*cb.cx).memctl.free.unwrap())(
                current_data as *mut c_void,
                (*cb.cx).memctl.memory_data,
            );
            current_data = next_data;
            if current_data.is_null() { break; }
        }
    }

    break 'exit_l;
    }

    /* EXIT: */
    if cb.parsed_pattern != stack_parsed_pattern.as_mut_ptr() {
        ((*ccontext).memctl.free.unwrap())(
            cb.parsed_pattern as *mut c_void,
            (*ccontext).memctl.memory_data,
        );
    }
    if cb.named_group_list_size as usize > NAMED_GROUP_LIST_SIZE {
        ((*ccontext).memctl.free.unwrap())(
            cb.named_groups as *mut c_void,
            (*ccontext).memctl.memory_data,
        );
    }
    if cb.groupinfo != stack_groupinfo.as_mut_ptr() {
        ((*ccontext).memctl.free.unwrap())(
            cb.groupinfo as *mut c_void,
            (*ccontext).memctl.memory_data,
        );
    }

    return re; /* Will be NULL after an error */
}
