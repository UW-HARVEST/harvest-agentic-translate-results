//! Translation of `c_src/src/pcre2_study.c`.
//!
//! Functions for scanning a compiled pattern and collecting data (e.g. minimum
//! matching length and the set of possible starting code units).
//!
//! Build configuration: `PCRE2_CODE_UNIT_WIDTH == 8`, `SUPPORT_UNICODE`
//! (therefore `SUPPORT_WIDE_CHARS`), no JIT, no EBCDIC, `LINK_SIZE == 2`.

#![allow(non_snake_case, non_upper_case_globals, unused_parens, unused_assignments)]

use core::ffi::c_int;

use crate::chars::*;
use crate::find_bracket::find_bracket;
use crate::internal::*;
use crate::opcodes::*;
use crate::ord2utf::ord2utf;

/* The maximum remembered capturing brackets minimum. */

const MAX_CACHE_BACKREF: usize = 128;

/* Returns from set_start_bits() */

const SSB_FAIL: c_int = 0;
const SSB_DONE: c_int = 1;
const SSB_CONTINUE: c_int = 2;
const SSB_UNKNOWN: c_int = 3;
const SSB_TOODEEP: c_int = 4;

const INT_MAX: c_int = c_int::MAX;
const UINT16_MAX: u32 = u16::MAX as u32;

/// `SET_BIT(c)` -- set a bit in the starting code unit bit map.
#[inline]
unsafe fn set_bit(re: *mut pcre2_real_code, c: u32) {
    unsafe {
        (*re).start_bitmap[(c / 8) as usize] |= 1u8 << (c & 7);
    }
}

/*************************************************
*   Find the minimum subject length for a group  *
*************************************************/

/* Scan a parenthesized group and compute the minimum length of subject that
is needed to match it. This is a lower bound; it does not mean there is a
string of that length that matches. In UTF mode, the result is in characters
rather than code units. The field in a compiled pattern for storing the minimum
length is 16-bits long (on the grounds that anything longer than that is
pathological), so we give up when we reach that amount. This also means that
integer overflow for really crazy patterns cannot happen.

Backreference minimum lengths are cached to speed up multiple references. This
function is called only when the highest back reference in the pattern is less
than or equal to MAX_CACHE_BACKREF, which is one less than the size of the
caching vector. The zeroth element contains the number of the highest set
value.

Arguments:
  re              compiled pattern block
  code            pointer to start of group (the bracket)
  startcode       pointer to start of the whole pattern's code
  utf             UTF flag
  recurses        chain of recurse_check to catch mutual recursion
  countptr        pointer to call count (to catch over complexity)
  backref_cache   vector for caching back references.

This function is no longer called when the pattern contains (*ACCEPT); however,
the old code for returning -1 is retained, just in case.

Returns:   the minimum length
           -1 \C in UTF-8 mode
              or (*ACCEPT)
              or pattern too complicated
           -2 internal error (missing capturing bracket)
           -3 internal error (opcode not listed)
*/

unsafe fn find_minlength(
    re: *const pcre2_real_code,
    code: PCRE2_SPTR,
    startcode: PCRE2_SPTR,
    utf: BOOL,
    recurses: *const recurse_check,
    countptr: *mut c_int,
    backref_cache: *mut c_int,
) -> c_int {
    unsafe {
        let mut length: c_int = -1;
        let mut branchlength: c_int = 0;
        let mut prev_cap_recno: c_int = -1;
        let mut prev_cap_d: c_int = 0;
        let mut prev_recurse_recno: c_int = -1;
        let mut prev_recurse_d: c_int = 0;
        let mut once_fudge: u32 = 0;
        let mut had_recurse: BOOL = FALSE;
        let dupcapused: BOOL = (((*re).flags & PCRE2_DUPCAPUSED) != 0) as BOOL;
        let mut nextbranch: PCRE2_SPTR = code.add(get(code, 1) as usize);
        let mut cc: PCRE2_SPTR = code.add(1 + LINK_SIZE);
        let mut this_recurse: recurse_check = recurse_check {
            prev: core::ptr::null(),
            group: core::ptr::null(),
        };

        /* If this is a "could be empty" group, its minimum length is 0. */

        if *code >= OP_SBRA && *code <= OP_SCOND {
            return 0;
        }

        /* Skip over capturing bracket number */

        if *code == OP_CBRA || *code == OP_CBRAPOS {
            cc = cc.add(IMM2_SIZE);
        }

        /* A large and/or complex regex can take too long to process. */

        let cnt = *countptr;
        *countptr = cnt + 1;
        if cnt > 1000 {
            return -1;
        }

        /* Scan along the opcodes for this branch. If we get to the end of the branch,
        check the length against that of the other branches. If the accumulated length
        passes 16-bits, reset to that value and skip the rest of the branch. */

        loop {
            let d: c_int;
            let min: c_int;
            let op: PCRE2_UCHAR;
            let mut cs: PCRE2_SPTR;
            let mut ce: PCRE2_SPTR;

            if branchlength >= UINT16_MAX as c_int {
                branchlength = UINT16_MAX as c_int;
                cc = nextbranch;
            }

            op = *cc;
            match op {
                OP_COND | OP_SCOND => {
                    /* If there is only one branch in a condition, the implied branch has zero
                    length, so we don't add anything. This covers the DEFINE "condition"
                    automatically. If there are two branches we can treat it the same as any
                    other non-capturing subpattern. */

                    cs = cc.add(get(cc, 1) as usize);
                    if *cs != OP_ALT {
                        cc = cs.add(1 + LINK_SIZE);
                    } else {
                        /* goto PROCESS_NON_CAPTURE */
                        d = find_minlength(re, cc, startcode, utf, recurses, countptr, backref_cache);
                        if d < 0 {
                            return d;
                        }
                        branchlength += d;
                        loop {
                            cc = cc.add(get(cc, 1) as usize);
                            if *cc != OP_ALT {
                                break;
                            }
                        }
                        cc = cc.add(1 + LINK_SIZE);
                    }
                }

                OP_BRA => {
                    /* There's a special case of OP_BRA, when it is wrapped round a repeated
                    OP_RECURSE. We'd like to process the latter at this level so that
                    remembering the value works for repeated cases. So we do nothing, but
                    set a fudge value to skip over the OP_KET after the recurse. */

                    if *cc.add(1 + LINK_SIZE) == OP_RECURSE && *cc.add(2 * (1 + LINK_SIZE)) == OP_KET
                    {
                        once_fudge = (1 + LINK_SIZE) as u32;
                        cc = cc.add(1 + LINK_SIZE);
                    } else {
                        /* Fall through to PROCESS_NON_CAPTURE */
                        d = find_minlength(re, cc, startcode, utf, recurses, countptr, backref_cache);
                        if d < 0 {
                            return d;
                        }
                        branchlength += d;
                        loop {
                            cc = cc.add(get(cc, 1) as usize);
                            if *cc != OP_ALT {
                                break;
                            }
                        }
                        cc = cc.add(1 + LINK_SIZE);
                    }
                }

                OP_ONCE | OP_SCRIPT_RUN | OP_SBRA | OP_BRAPOS | OP_SBRAPOS => {
                    /* PROCESS_NON_CAPTURE */
                    d = find_minlength(re, cc, startcode, utf, recurses, countptr, backref_cache);
                    if d < 0 {
                        return d;
                    }
                    branchlength += d;
                    loop {
                        cc = cc.add(get(cc, 1) as usize);
                        if *cc != OP_ALT {
                            break;
                        }
                    }
                    cc = cc.add(1 + LINK_SIZE);
                }

                /* To save time for repeated capturing subpatterns, we remember the
                length of the previous one. Unfortunately we can't do the same for
                the unnumbered ones above. Nor can we do this if (?| is present in the
                pattern because captures with the same number are not then identical. */

                OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS => {
                    let recno = get2(cc, 1 + LINK_SIZE) as c_int;
                    if dupcapused != FALSE || recno != prev_cap_recno {
                        prev_cap_recno = recno;
                        prev_cap_d =
                            find_minlength(re, cc, startcode, utf, recurses, countptr, backref_cache);
                        if prev_cap_d < 0 {
                            return prev_cap_d;
                        }
                    }
                    branchlength += prev_cap_d;
                    loop {
                        cc = cc.add(get(cc, 1) as usize);
                        if *cc != OP_ALT {
                            break;
                        }
                    }
                    cc = cc.add(1 + LINK_SIZE);
                }

                /* ACCEPT makes things far too complicated; we have to give up. In fact,
                from 10.34 onwards, if a pattern contains (*ACCEPT), this function is not
                used. However, leave the code in place, just in case. */

                OP_ACCEPT | OP_ASSERT_ACCEPT => {
                    return -1;
                }

                /* Reached end of a branch; if it's a ket it is the end of a nested
                call. If it's ALT it is an alternation in a nested call. If it is END it's
                the end of the outer call. All can be handled by the same code. If the
                length of any branch is zero, there is no need to scan any subsequent
                branches. */

                OP_ALT | OP_KET | OP_KETRMAX | OP_KETRMIN | OP_KETRPOS | OP_END => {
                    if length < 0 || (had_recurse == FALSE && branchlength < length) {
                        length = branchlength;
                    }
                    if op != OP_ALT || length == 0 {
                        return length;
                    }
                    nextbranch = cc.add(get(cc, 1) as usize);
                    cc = cc.add(1 + LINK_SIZE);
                    branchlength = 0;
                    had_recurse = FALSE;
                }

                /* Skip over assertive subpatterns */

                OP_ASSERT | OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT | OP_ASSERT_NA
                | OP_ASSERT_SCS | OP_ASSERTBACK_NA => {
                    loop {
                        cc = cc.add(get(cc, 1) as usize);
                        if *cc != OP_ALT {
                            break;
                        }
                    }
                    /* Fall through */
                    cc = cc.add(OP_LENGTHS[*cc as usize] as usize);
                }

                /* Skip over things that don't match chars */

                OP_REVERSE | OP_VREVERSE | OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF
                | OP_FALSE | OP_TRUE | OP_CALLOUT | OP_SOD | OP_SOM | OP_EOD | OP_EODN
                | OP_CIRC | OP_CIRCM | OP_DOLL | OP_DOLLM | OP_NOT_WORD_BOUNDARY
                | OP_WORD_BOUNDARY | OP_NOT_UCP_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY => {
                    cc = cc.add(OP_LENGTHS[*cc as usize] as usize);
                }

                OP_CALLOUT_STR => {
                    cc = cc.add(get(cc, 1 + 2 * LINK_SIZE) as usize);
                }

                /* Skip over a subpattern that has a {0} or {0,x} quantifier */

                OP_BRAZERO | OP_BRAMINZERO | OP_BRAPOSZERO | OP_SKIPZERO => {
                    cc = cc.add(OP_LENGTHS[*cc as usize] as usize);
                    loop {
                        cc = cc.add(get(cc, 1) as usize);
                        if *cc != OP_ALT {
                            break;
                        }
                    }
                    cc = cc.add(1 + LINK_SIZE);
                }

                /* Handle literal characters and + repetitions */

                OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_PLUS | OP_PLUSI | OP_MINPLUS
                | OP_MINPLUSI | OP_POSPLUS | OP_POSPLUSI | OP_NOTPLUS | OP_NOTPLUSI
                | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_NOTPOSPLUS | OP_NOTPOSPLUSI => {
                    branchlength += 1;
                    cc = cc.add(2);
                    if utf != FALSE && has_extralen(*cc.sub(1) as u32) {
                        cc = cc.add(get_extralen(*cc.sub(1) as u32) as usize);
                    }
                }

                OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEPOSPLUS => {
                    branchlength += 1;
                    cc = cc.add(if *cc.add(1) == OP_PROP || *cc.add(1) == OP_NOTPROP {
                        4
                    } else {
                        2
                    });
                }

                /* Handle exact repetitions. The count is already in characters, but we
                may need to skip over a multibyte character in UTF mode.  */

                OP_EXACT | OP_EXACTI | OP_NOTEXACT | OP_NOTEXACTI => {
                    branchlength += get2(cc, 1) as c_int;
                    cc = cc.add(2 + IMM2_SIZE);
                    if utf != FALSE && has_extralen(*cc.sub(1) as u32) {
                        cc = cc.add(get_extralen(*cc.sub(1) as u32) as usize);
                    }
                }

                OP_TYPEEXACT => {
                    branchlength += get2(cc, 1) as c_int;
                    cc = cc.add(
                        2 + IMM2_SIZE
                            + (if *cc.add(1 + IMM2_SIZE) == OP_PROP
                                || *cc.add(1 + IMM2_SIZE) == OP_NOTPROP
                            {
                                2
                            } else {
                                0
                            }),
                    );
                }

                /* Handle single-char non-literal matchers */

                OP_PROP | OP_NOTPROP => {
                    cc = cc.add(2);
                    /* Fall through */
                    branchlength += 1;
                    cc = cc.add(1);
                }

                OP_NOT_DIGIT | OP_DIGIT | OP_NOT_WHITESPACE | OP_WHITESPACE | OP_NOT_WORDCHAR
                | OP_WORDCHAR | OP_ANY | OP_ALLANY | OP_EXTUNI | OP_HSPACE | OP_NOT_HSPACE
                | OP_VSPACE | OP_NOT_VSPACE => {
                    branchlength += 1;
                    cc = cc.add(1);
                }

                /* "Any newline" might match two characters, but it also might match just
                one. */

                OP_ANYNL => {
                    branchlength += 1;
                    cc = cc.add(1);
                }

                /* The single-byte matcher means we can't proceed in UTF mode. (In
                non-UTF mode \C will actually be turned into OP_ALLANY, so won't ever
                appear, but leave the code, just in case.) */

                OP_ANYBYTE => {
                    if utf != FALSE {
                        return -1;
                    }
                    branchlength += 1;
                    cc = cc.add(1);
                }

                /* For repeated character types, we have to test for \p and \P, which have
                an extra two bytes of parameters. */

                OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEQUERY | OP_TYPEMINQUERY
                | OP_TYPEPOSSTAR | OP_TYPEPOSQUERY => {
                    if *cc.add(1) == OP_PROP || *cc.add(1) == OP_NOTPROP {
                        cc = cc.add(2);
                    }
                    cc = cc.add(OP_LENGTHS[op as usize] as usize);
                }

                OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO => {
                    if *cc.add(1 + IMM2_SIZE) == OP_PROP || *cc.add(1 + IMM2_SIZE) == OP_NOTPROP {
                        cc = cc.add(2);
                    }
                    cc = cc.add(OP_LENGTHS[op as usize] as usize);
                }

                /* Check a class for variable quantification */

                OP_CLASS | OP_NCLASS | OP_XCLASS | OP_ECLASS => {
                    /* The original code caused an unsigned overflow in 64 bit systems,
                    so now we use a conditional statement. */
                    if op == OP_XCLASS || op == OP_ECLASS {
                        cc = cc.add(get(cc, 1) as usize);
                    } else {
                        cc = cc.add(OP_LENGTHS[OP_CLASS as usize] as usize);
                    }

                    match *cc {
                        OP_CRPLUS | OP_CRMINPLUS | OP_CRPOSPLUS => {
                            branchlength += 1;
                            /* Fall through */
                            cc = cc.add(1);
                        }

                        OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSSTAR
                        | OP_CRPOSQUERY => {
                            cc = cc.add(1);
                        }

                        OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                            branchlength += get2(cc, 1) as c_int;
                            cc = cc.add(1 + 2 * IMM2_SIZE);
                        }

                        _ => {
                            branchlength += 1;
                        }
                    }
                }

                /* Backreferences and subroutine calls (OP_RECURSE) are treated in the same
                way: we find the minimum length for the subpattern. A recursion
                (backreference or subroutine) causes an a flag to be set that causes the
                length of this branch to be ignored. The logic is that a recursion can only
                make sense if there is another alternative that stops the recursing. That
                will provide the minimum length (when no recursion happens).

                If PCRE2_MATCH_UNSET_BACKREF is set, a backreference to an unset bracket
                matches an empty string (by default it causes a matching failure), so in
                that case we must set the minimum length to zero.

                For backreferenes, if duplicate numbers are present in the pattern we check
                for a reference to a duplicate. If it is, we don't know which version will
                be referenced, so we have to set the minimum length to zero. */

                /* Duplicate named pattern back reference. */

                OP_DNREF | OP_DNREFI => {
                    let d_val: c_int;
                    if dupcapused == FALSE
                        && ((*re).overall_options & PCRE2_MATCH_UNSET_BACKREF) == 0
                    {
                        let mut count = get2(cc, 1 + IMM2_SIZE) as c_int;
                        let mut slot: PCRE2_SPTR = (re as *const u8)
                            .add(core::mem::size_of::<pcre2_real_code>())
                            .add(get2(cc, 1) as usize * (*re).name_entry_size as usize);

                        let mut d_local = INT_MAX;

                        /* Scan all groups with the same name; find the shortest. */

                        while {
                            let c = count;
                            count -= 1;
                            c
                        } > 0
                        {
                            let mut dd: c_int;
                            let mut i: c_int;
                            let recno = get2(slot, 0) as c_int;

                            if recno <= *backref_cache.offset(0)
                                && *backref_cache.offset(recno as isize) >= 0
                            {
                                dd = *backref_cache.offset(recno as isize);
                            } else {
                                cs = find_bracket(startcode, utf, recno);
                                ce = cs;
                                if cs.is_null() {
                                    return -2;
                                }
                                loop {
                                    ce = ce.add(get(ce, 1) as usize);
                                    if *ce != OP_ALT {
                                        break;
                                    }
                                }

                                dd = 0;
                                if dupcapused == FALSE || find_bracket(ce, utf, recno).is_null() {
                                    if cc > cs && cc < ce {
                                        /* Simple recursion */
                                        had_recurse = TRUE;
                                    } else {
                                        let mut r = recurses;
                                        while !r.is_null() {
                                            if (*r).group == cs {
                                                break;
                                            }
                                            r = (*r).prev;
                                        }
                                        if !r.is_null() {
                                            /* Mutual recursion */
                                            had_recurse = TRUE;
                                        } else {
                                            this_recurse.prev = recurses; /* No recursion */
                                            this_recurse.group = cs;
                                            dd = find_minlength(
                                                re,
                                                cs,
                                                startcode,
                                                utf,
                                                &this_recurse,
                                                countptr,
                                                backref_cache,
                                            );
                                            if dd < 0 {
                                                return dd;
                                            }
                                        }
                                    }
                                }

                                *backref_cache.offset(recno as isize) = dd;
                                i = *backref_cache.offset(0) + 1;
                                while i < recno {
                                    *backref_cache.offset(i as isize) = -1;
                                    i += 1;
                                }
                                *backref_cache.offset(0) = recno;
                            }

                            if dd < d_local {
                                d_local = dd;
                            }
                            if d_local <= 0 {
                                break; /* No point looking at any more */
                            }
                            slot = slot.add((*re).name_entry_size as usize);
                        }
                        d_val = d_local;
                    } else {
                        d_val = 0;
                    }
                    cc = cc.add(OP_LENGTHS[*cc as usize] as usize);

                    /* REPEAT_BACK_REFERENCE */
                    match *cc {
                        OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSSTAR
                        | OP_CRPOSQUERY => {
                            min = 0;
                            cc = cc.add(1);
                        }
                        OP_CRPLUS | OP_CRMINPLUS | OP_CRPOSPLUS => {
                            min = 1;
                            cc = cc.add(1);
                        }
                        OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                            min = get2(cc, 1) as c_int;
                            cc = cc.add(1 + 2 * IMM2_SIZE);
                        }
                        _ => {
                            min = 1;
                        }
                    }

                    /* Take care not to overflow: (1) min and d are ints, so check that their
                    product is not greater than INT_MAX. (2) branchlength is limited to
                    UINT16_MAX (checked at the top of the loop). */

                    if (d_val > 0 && (INT_MAX / d_val) < min)
                        || (UINT16_MAX as c_int - branchlength) < min * d_val
                    {
                        branchlength = UINT16_MAX as c_int;
                    } else {
                        branchlength += min * d_val;
                    }
                }

                /* Single back reference by number. References by name are converted to by
                number when there is no duplication. */

                OP_REF | OP_REFI => {
                    let mut d_val: c_int;
                    let recno = get2(cc, 1) as c_int;
                    if recno <= *backref_cache.offset(0)
                        && *backref_cache.offset(recno as isize) >= 0
                    {
                        d_val = *backref_cache.offset(recno as isize);
                    } else {
                        let mut i: c_int;
                        d_val = 0;

                        if ((*re).overall_options & PCRE2_MATCH_UNSET_BACKREF) == 0 {
                            cs = find_bracket(startcode, utf, recno);
                            ce = cs;
                            if cs.is_null() {
                                return -2;
                            }
                            loop {
                                ce = ce.add(get(ce, 1) as usize);
                                if *ce != OP_ALT {
                                    break;
                                }
                            }

                            if dupcapused == FALSE || find_bracket(ce, utf, recno).is_null() {
                                if cc > cs && cc < ce {
                                    /* Simple recursion */
                                    had_recurse = TRUE;
                                } else {
                                    let mut r = recurses;
                                    while !r.is_null() {
                                        if (*r).group == cs {
                                            break;
                                        }
                                        r = (*r).prev;
                                    }
                                    if !r.is_null() {
                                        /* Mutual recursion */
                                        had_recurse = TRUE;
                                    } else {
                                        /* No recursion */
                                        this_recurse.prev = recurses;
                                        this_recurse.group = cs;
                                        d_val = find_minlength(
                                            re,
                                            cs,
                                            startcode,
                                            utf,
                                            &this_recurse,
                                            countptr,
                                            backref_cache,
                                        );
                                        if d_val < 0 {
                                            return d_val;
                                        }
                                    }
                                }
                            }
                        }

                        *backref_cache.offset(recno as isize) = d_val;
                        i = *backref_cache.offset(0) + 1;
                        while i < recno {
                            *backref_cache.offset(i as isize) = -1;
                            i += 1;
                        }
                        *backref_cache.offset(0) = recno;
                    }

                    cc = cc.add(OP_LENGTHS[*cc as usize] as usize);

                    /* Handle repeated back references. REPEAT_BACK_REFERENCE */
                    match *cc {
                        OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSSTAR
                        | OP_CRPOSQUERY => {
                            min = 0;
                            cc = cc.add(1);
                        }
                        OP_CRPLUS | OP_CRMINPLUS | OP_CRPOSPLUS => {
                            min = 1;
                            cc = cc.add(1);
                        }
                        OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                            min = get2(cc, 1) as c_int;
                            cc = cc.add(1 + 2 * IMM2_SIZE);
                        }
                        _ => {
                            min = 1;
                        }
                    }

                    if (d_val > 0 && (INT_MAX / d_val) < min)
                        || (UINT16_MAX as c_int - branchlength) < min * d_val
                    {
                        branchlength = UINT16_MAX as c_int;
                    } else {
                        branchlength += min * d_val;
                    }
                }

                /* Recursion always refers to the first occurrence of a subpattern with a
                given number. Therefore, we can always make use of caching, even when the
                pattern contains multiple subpatterns with the same number. */

                OP_RECURSE => {
                    cs = startcode.add(get(cc, 1) as usize);
                    ce = cs;
                    let recno = get2(cs, 1 + LINK_SIZE) as c_int;
                    if recno == prev_recurse_recno {
                        branchlength += prev_recurse_d;
                    } else {
                        loop {
                            ce = ce.add(get(ce, 1) as usize);
                            if *ce != OP_ALT {
                                break;
                            }
                        }
                        if cc > cs && cc < ce {
                            /* Simple recursion */
                            had_recurse = TRUE;
                        } else {
                            let mut r = recurses;
                            while !r.is_null() {
                                if (*r).group == cs {
                                    break;
                                }
                                r = (*r).prev;
                            }
                            if !r.is_null() {
                                /* Mutual recursion */
                                had_recurse = TRUE;
                            } else {
                                this_recurse.prev = recurses;
                                this_recurse.group = cs;
                                prev_recurse_d = find_minlength(
                                    re,
                                    cs,
                                    startcode,
                                    utf,
                                    &this_recurse,
                                    countptr,
                                    backref_cache,
                                );
                                if prev_recurse_d < 0 {
                                    return prev_recurse_d;
                                }
                                prev_recurse_recno = recno;
                                branchlength += prev_recurse_d;
                            }
                        }
                    }
                    cc = cc.add(1 + LINK_SIZE + once_fudge as usize);
                    once_fudge = 0;
                }

                /* Anything else does not or need not match a character. We can get the
                item's length from the table, but for those that can match zero occurrences
                of a character, we must take special action for UTF-8 characters. As it
                happens, the "NOT" versions of these opcodes are used at present only for
                ASCII characters, so they could be omitted from this list. However, in
                future that may change, so we include them here so as not to leave a
                gotcha for a future maintainer. */

                OP_UPTO | OP_UPTOI | OP_NOTUPTO | OP_NOTUPTOI | OP_MINUPTO | OP_MINUPTOI
                | OP_NOTMINUPTO | OP_NOTMINUPTOI | OP_POSUPTO | OP_POSUPTOI | OP_NOTPOSUPTO
                | OP_NOTPOSUPTOI | OP_STAR | OP_STARI | OP_NOTSTAR | OP_NOTSTARI | OP_MINSTAR
                | OP_MINSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI | OP_POSSTAR | OP_POSSTARI
                | OP_NOTPOSSTAR | OP_NOTPOSSTARI | OP_QUERY | OP_QUERYI | OP_NOTQUERY
                | OP_NOTQUERYI | OP_MINQUERY | OP_MINQUERYI | OP_NOTMINQUERY | OP_NOTMINQUERYI
                | OP_POSQUERY | OP_POSQUERYI | OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                    cc = cc.add(OP_LENGTHS[op as usize] as usize);
                    if utf != FALSE && has_extralen(*cc.sub(1) as u32) {
                        cc = cc.add(get_extralen(*cc.sub(1) as u32) as usize);
                    }
                }

                /* Skip these, but we need to add in the name length. */

                OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                    cc = cc.add(OP_LENGTHS[op as usize] as usize + *cc.add(1) as usize);
                }

                /* The remaining opcodes are just skipped over. */

                OP_CLOSE | OP_COMMIT | OP_FAIL | OP_PRUNE | OP_SET_SOM | OP_SKIP | OP_THEN => {
                    cc = cc.add(OP_LENGTHS[op as usize] as usize);
                }

                /* This should not occur: we list all opcodes explicitly so that when
                new ones get added they are properly considered. */

                _ => {
                    return -3;
                }
            }
        }
    }
}

/*************************************************
*      Set a bit and maybe its alternate case    *
*************************************************/

/* Given a character, set its first code unit's bit in the table, and also the
corresponding bit for the other version of a letter if we are caseless.

Arguments:
  re            points to the regex block
  p             points to the first code unit of the character
  caseless      TRUE if caseless
  utf           TRUE for UTF mode
  ucp           TRUE for UCP mode

Returns:        pointer after the character
*/

unsafe fn set_table_bit(
    re: *mut pcre2_real_code,
    mut p: PCRE2_SPTR,
    caseless: BOOL,
    utf: BOOL,
    ucp: BOOL,
) -> PCRE2_SPTR {
    unsafe {
        let mut c: u32 = *p as u32; /* First code unit */
        p = p.add(1);

        set_bit(re, c);

        /* In UTF-8 mode, pick up the remaining code units in order to find
        the end of the character, even when caseless. */

        if utf != FALSE {
            if c >= 0xc0 {
                /* GETUTF8INC(c, p) */
                let (ch, len) = getutf8len(c, p.sub(1));
                c = ch;
                p = p.add(len as usize);
            }
        }

        /* If caseless, handle the other case of the character. */

        if caseless != FALSE {
            if utf != FALSE || ucp != FALSE {
                c = ucd_othercase(c);
                if utf != FALSE {
                    let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
                    ord2utf(c, buff.as_mut_ptr());
                    set_bit(re, buff[0] as u32);
                } else if c < 256 {
                    set_bit(re, c);
                }
            } else
            /* Not UTF or UCP */
            if max_255(c) {
                set_bit(re, *(*re).tables.add(fcc_offset + c as usize) as u32);
            }
        }

        p
    }
}

/*************************************************
*     Set bits for a positive character type     *
*************************************************/

/* This function sets starting bits for a character type. In UTF-8 mode, we can
only do a direct setting for bytes less than 128, as otherwise there can be
confusion with bytes in the middle of UTF-8 characters. In a "traditional"
environment, the tables will only recognize ASCII characters anyway, but in at
least one Windows environment, some higher bytes bits were set in the tables.
So we deal with that case by considering the UTF-8 encoding.

Arguments:
  re             the regex block
  cbit type      the type of character wanted
  table_limit    32 for non-UTF-8; 16 for UTF-8

Returns:         nothing
*/

unsafe fn set_type_bits(re: *mut pcre2_real_code, cbit_type: c_int, table_limit: u32) {
    unsafe {
        let mut c: u32 = 0;
        while c < table_limit {
            (*re).start_bitmap[c as usize] |=
                *(*re).tables.add(c as usize + cbits_offset + cbit_type as usize);
            c += 1;
        }
        if table_limit == 32 {
            return;
        }
        c = 128;
        while c < 256 {
            if (*(*re).tables.add(cbits_offset + (c / 8) as usize) & (1u8 << (c & 7))) != 0 {
                let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
                ord2utf(c, buff.as_mut_ptr());
                set_bit(re, buff[0] as u32);
            }
            c += 1;
        }
    }
}

/*************************************************
*     Set bits for a negative character type     *
*************************************************/

/* This function sets starting bits for a negative character type such as \D.
In UTF-8 mode, we can only do a direct setting for bytes less than 128, as
otherwise there can be confusion with bytes in the middle of UTF-8 characters.
Unlike in the positive case, where we can set appropriate starting bits for
specific high-valued UTF-8 characters, in this case we have to set the bits for
all high-valued characters. The lowest is 0xc2, but we overkill by starting at
0xc0 (192) for simplicity.

Arguments:
  re             the regex block
  cbit type      the type of character wanted
  table_limit    32 for non-UTF-8; 16 for UTF-8

Returns:         nothing
*/

unsafe fn set_nottype_bits(re: *mut pcre2_real_code, cbit_type: c_int, table_limit: u32) {
    unsafe {
        let mut c: u32 = 0;
        while c < table_limit {
            (*re).start_bitmap[c as usize] |=
                !(*(*re).tables.add(c as usize + cbits_offset + cbit_type as usize));
            c += 1;
        }
        if table_limit != 32 {
            c = 24;
            while c < 32 {
                (*re).start_bitmap[c as usize] = 0xff;
                c += 1;
            }
        }
    }
}

/*************************************************
*     Set starting bits for a character list.    *
*************************************************/

/* This function sets starting bits for a character list. It enumerates
all characters and character ranges in the character list, and sets
the starting bits accordingly.

Arguments:
  code           pointer to the code
  start_bitmap   pointer to the starting bitmap

Returns:         nothing
*/

unsafe fn study_char_list(
    mut code: PCRE2_SPTR,
    start_bitmap: *mut u8,
    char_lists_end: *const u8,
) {
    unsafe {
        let mut type_: u32;
        let mut list_ind: u32;
        let mut char_list_add: u32 = XCL_CHAR_LIST_LOW_16_ADD;
        let mut range_start: u32 = !0u32;
        let mut range_end: u32;
        let mut next_char: *const u8;
        let mut start_buffer: [PCRE2_UCHAR; 6] = [0; 6];
        let mut end_buffer: [PCRE2_UCHAR; 6] = [0; 6];
        let mut start: PCRE2_UCHAR;
        let mut end: PCRE2_UCHAR;

        /* Only needed in 8-bit mode at the moment. */
        type_ = ((*code.add(0) as u32) << 8) | *code.add(1) as u32;
        code = code.add(2);

        /* Align characters. */
        next_char = char_lists_end.sub((get(code, 0) as usize) << 1);
        type_ &= XCL_TYPE_MASK;
        list_ind = 0;

        if (type_ & XCL_BEGIN_WITH_RANGE) != 0 {
            range_start = XCL_CHAR_LIST_LOW_16_START;
        }

        while type_ > 0 {
            let mut item_count = type_ & XCL_ITEM_COUNT_MASK;

            if item_count == XCL_ITEM_COUNT_MASK {
                if list_ind <= 1 {
                    item_count = (next_char as *const u16).read_unaligned() as u32;
                    next_char = next_char.add(2);
                } else {
                    item_count = (next_char as *const u32).read_unaligned();
                    next_char = next_char.add(4);
                }
            }

            while item_count > 0 {
                if list_ind <= 1 {
                    range_end = (next_char as *const u16).read_unaligned() as u32;
                    next_char = next_char.add(2);
                } else {
                    range_end = (next_char as *const u32).read_unaligned();
                    next_char = next_char.add(4);
                }

                if (range_end & XCL_CHAR_END) != 0 {
                    range_end = char_list_add + (range_end >> XCL_CHAR_SHIFT);

                    ord2utf(range_end, end_buffer.as_mut_ptr());
                    end = end_buffer[0];

                    if range_start < range_end {
                        ord2utf(range_start, start_buffer.as_mut_ptr());
                        start = start_buffer[0];
                        while start <= end {
                            *start_bitmap.add((start / 8) as usize) |= 1u8 << (start & 7);
                            start += 1;
                        }
                    } else {
                        *start_bitmap.add((end / 8) as usize) |= 1u8 << (end & 7);
                    }

                    range_start = !0u32;
                } else {
                    range_start = char_list_add + (range_end >> XCL_CHAR_SHIFT);
                }

                item_count -= 1;
            }

            list_ind += 1;
            type_ >>= XCL_TYPE_BIT_LEN;

            if range_start == !0u32 {
                if (type_ & XCL_BEGIN_WITH_RANGE) != 0 {
                    /* In 8 bit mode XCL_CHAR_LIST_HIGH_32_START is not possible. */
                    if list_ind == 1 {
                        range_start = XCL_CHAR_LIST_HIGH_16_START;
                    } else {
                        range_start = XCL_CHAR_LIST_LOW_32_START;
                    }
                }
            } else if (type_ & XCL_BEGIN_WITH_RANGE) == 0 {
                ord2utf(range_start, start_buffer.as_mut_ptr());

                /* In 8 bit mode XCL_CHAR_LIST_LOW_32_END and
                XCL_CHAR_LIST_HIGH_32_END are not possible. */
                if list_ind == 1 {
                    range_end = XCL_CHAR_LIST_LOW_16_END;
                } else {
                    range_end = XCL_CHAR_LIST_HIGH_16_END;
                }

                ord2utf(range_end, end_buffer.as_mut_ptr());
                end = end_buffer[0];

                start = start_buffer[0];
                while start <= end {
                    *start_bitmap.add((start / 8) as usize) |= 1u8 << (start & 7);
                    start += 1;
                }

                range_start = !0u32;
            }

            /* In 8 bit mode XCL_CHAR_LIST_HIGH_32_ADD is not possible. */
            if list_ind == 1 {
                char_list_add = XCL_CHAR_LIST_HIGH_16_ADD;
            } else {
                char_list_add = XCL_CHAR_LIST_LOW_32_ADD;
            }
        }
    }
}

/*************************************************
*      Create bitmap of starting code units      *
*************************************************/

/* This function scans a compiled unanchored expression recursively and
attempts to build a bitmap of the set of possible starting code units whose
values are less than 256. When calling set[_not]_type_bits() in UTF-8 mode
we pass a value of 16 rather than 32 as the final argument. (See comments in
those functions for the reason.)

The SSB_CONTINUE return is useful for parenthesized groups in patterns such as
(a*)b where the group provides some optional starting code units but scanning
must continue at the outer level to find at least one mandatory code unit. At
the outermost level, this function fails unless the result is SSB_DONE.

We restrict recursion (for nested groups) to 1000 to avoid stack overflow
issues.

Arguments:
  re           points to the compiled regex block
  code         points to an expression
  utf          TRUE if in UTF mode
  ucp          TRUE if in UCP mode
  depthptr     pointer to recurse depth

Returns:       SSB_FAIL     => Failed to find any starting code units
               SSB_DONE     => Found mandatory starting code units
               SSB_CONTINUE => Found optional starting code units
               SSB_UNKNOWN  => Hit an unrecognized opcode
               SSB_TOODEEP  => Recursion is too deep
*/

unsafe fn set_start_bits(
    re: *mut pcre2_real_code,
    mut code: PCRE2_SPTR,
    utf: BOOL,
    ucp: BOOL,
    depthptr: *mut c_int,
) -> c_int {
    unsafe {
        let mut c: u32;
        let mut yield_: c_int = SSB_DONE;

        let table_limit: u32 = if utf != FALSE { 16 } else { 32 };

        *depthptr += 1;
        if *depthptr > 1000 {
            return SSB_TOODEEP;
        }

        loop {
            let mut try_next: BOOL = TRUE;
            let mut tcode: PCRE2_SPTR = code.add(1 + LINK_SIZE);

            if *code == OP_CBRA || *code == OP_SCBRA || *code == OP_CBRAPOS || *code == OP_SCBRAPOS
            {
                tcode = tcode.add(IMM2_SIZE);
            }

            while try_next != FALSE {
                /* Loop for items in this branch */
                let mut classmap: *const u8;

                match *tcode {
                    /* If we reach something we don't understand, it means a new opcode has
                    been created that hasn't been added to this function. Hopefully this
                    problem will be discovered during testing. */

                    /* Fail for a valid opcode that implies no starting bits. */
                    OP_ACCEPT | OP_ASSERT_ACCEPT | OP_ALLANY | OP_ANY | OP_ANYBYTE | OP_CIRCM
                    | OP_CLOSE | OP_COMMIT | OP_COMMIT_ARG | OP_COND | OP_CREF | OP_FALSE
                    | OP_TRUE | OP_DNCREF | OP_DNREF | OP_DNREFI | OP_DNRREF | OP_DOLL
                    | OP_DOLLM | OP_END | OP_EOD | OP_EODN | OP_EXTUNI | OP_FAIL | OP_MARK
                    | OP_NOT | OP_NOTEXACT | OP_NOTEXACTI | OP_NOTI | OP_NOTMINPLUS
                    | OP_NOTMINPLUSI | OP_NOTMINQUERY | OP_NOTMINQUERYI | OP_NOTMINSTAR
                    | OP_NOTMINSTARI | OP_NOTMINUPTO | OP_NOTMINUPTOI | OP_NOTPLUS
                    | OP_NOTPLUSI | OP_NOTPOSPLUS | OP_NOTPOSPLUSI | OP_NOTPOSQUERY
                    | OP_NOTPOSQUERYI | OP_NOTPOSSTAR | OP_NOTPOSSTARI | OP_NOTPOSUPTO
                    | OP_NOTPOSUPTOI | OP_NOTPROP | OP_NOTQUERY | OP_NOTQUERYI | OP_NOTSTAR
                    | OP_NOTSTARI | OP_NOTUPTO | OP_NOTUPTOI | OP_NOT_HSPACE | OP_NOT_VSPACE
                    | OP_PRUNE | OP_PRUNE_ARG | OP_RECURSE | OP_REF | OP_REFI | OP_REVERSE
                    | OP_VREVERSE | OP_RREF | OP_SCOND | OP_SET_SOM | OP_SKIP | OP_SKIP_ARG
                    | OP_SOD | OP_SOM | OP_THEN | OP_THEN_ARG => {
                        return SSB_FAIL;
                    }

                    /* OP_CIRC happens only at the start of an anchored branch (multiline ^
                    uses OP_CIRCM). Skip over it. */
                    OP_CIRC => {
                        tcode = tcode.add(OP_LENGTHS[OP_CIRC as usize] as usize);
                    }

                    /* A "real" property test implies no starting bits, but the fake property
                    PT_CLIST identifies a list of characters. These lists are short, as they
                    are used for characters with more than one "other case", so there is no
                    point in recognizing them for OP_NOTPROP. */
                    OP_PROP => {
                        if *tcode.add(1) as u32 != PT_CLIST {
                            return SSB_FAIL;
                        }
                        {
                            let mut p: *const u32 =
                                UCD_CASELESS_SETS.as_ptr().add(*tcode.add(2) as usize);
                            loop {
                                c = *p;
                                p = p.add(1);
                                if c >= NOTACHAR {
                                    break;
                                }
                                if utf != FALSE {
                                    let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
                                    ord2utf(c, buff.as_mut_ptr());
                                    c = buff[0] as u32;
                                }
                                if c > 0xff {
                                    set_bit(re, 0xff);
                                } else {
                                    set_bit(re, c);
                                }
                            }
                        }
                        try_next = FALSE;
                    }

                    /* We can ignore word boundary tests. */
                    OP_WORD_BOUNDARY | OP_NOT_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY
                    | OP_NOT_UCP_WORD_BOUNDARY => {
                        tcode = tcode.add(1);
                    }

                    /* For a positive lookahead assertion, inspect what immediately follows,
                    ignoring intermediate assertions and callouts. If the next item is one
                    that sets a mandatory character, skip this assertion. Otherwise, treat it
                    the same as other bracket groups. */
                    OP_ASSERT | OP_ASSERT_NA => {
                        let rc: c_int;
                        let mut ncode: PCRE2_SPTR;
                        ncode = tcode.add(get(tcode, 1) as usize);
                        while *ncode == OP_ALT {
                            ncode = ncode.add(get(ncode, 1) as usize);
                        }
                        ncode = ncode.add(1 + LINK_SIZE);

                        /* Skip irrelevant items */
                        let mut done = false;
                        while !done {
                            match *ncode {
                                OP_ASSERT | OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT
                                | OP_ASSERT_NA | OP_ASSERTBACK_NA | OP_ASSERT_SCS => {
                                    ncode = ncode.add(get(ncode, 1) as usize);
                                    while *ncode == OP_ALT {
                                        ncode = ncode.add(get(ncode, 1) as usize);
                                    }
                                    ncode = ncode.add(1 + LINK_SIZE);
                                }

                                OP_WORD_BOUNDARY | OP_NOT_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY
                                | OP_NOT_UCP_WORD_BOUNDARY => {
                                    ncode = ncode.add(1);
                                }

                                OP_CALLOUT => {
                                    ncode = ncode.add(OP_LENGTHS[OP_CALLOUT as usize] as usize);
                                }

                                OP_CALLOUT_STR => {
                                    ncode = ncode.add(get(ncode, 1 + 2 * LINK_SIZE) as usize);
                                }

                                _ => {
                                    done = true;
                                }
                            }
                        }

                        /* Now check the next significant item. */
                        match *ncode {
                            OP_PROP if *ncode.add(1) as u32 == PT_CLIST => {
                                tcode = ncode;
                                continue; /* With the following significant opcode */
                            }
                            OP_ANYNL | OP_CHAR | OP_CHARI | OP_EXACT | OP_EXACTI | OP_HSPACE
                            | OP_MINPLUS | OP_MINPLUSI | OP_PLUS | OP_PLUSI | OP_POSPLUS
                            | OP_POSPLUSI | OP_VSPACE | OP_DIGIT | OP_NOT_DIGIT | OP_WORDCHAR
                            | OP_NOT_WORDCHAR | OP_WHITESPACE | OP_NOT_WHITESPACE => {
                                tcode = ncode;
                                continue; /* With the following significant opcode */
                            }
                            _ => {
                                /* Fall through to bracket handling. */
                            }
                        }

                        rc = set_start_bits(re, tcode, utf, ucp, depthptr);
                        if rc == SSB_DONE {
                            try_next = FALSE;
                        } else if rc == SSB_CONTINUE {
                            loop {
                                tcode = tcode.add(get(tcode, 1) as usize);
                                if *tcode != OP_ALT {
                                    break;
                                }
                            }
                            tcode = tcode.add(1 + LINK_SIZE);
                        } else {
                            return rc; /* FAIL, UNKNOWN, or TOODEEP */
                        }
                    }

                    /* For a group bracket or a positive assertion without an immediately
                    following mandatory setting, recurse to set bits from within the
                    subpattern. If it can't find anything, we have to give up. If it finds
                    some mandatory character(s), we are done for this branch. Otherwise,
                    carry on scanning after the subpattern. */
                    OP_BRA | OP_SBRA | OP_CBRA | OP_SCBRA | OP_BRAPOS | OP_SBRAPOS | OP_CBRAPOS
                    | OP_SCBRAPOS | OP_ONCE | OP_SCRIPT_RUN => {
                        let rc: c_int;
                        rc = set_start_bits(re, tcode, utf, ucp, depthptr);
                        if rc == SSB_DONE {
                            try_next = FALSE;
                        } else if rc == SSB_CONTINUE {
                            loop {
                                tcode = tcode.add(get(tcode, 1) as usize);
                                if *tcode != OP_ALT {
                                    break;
                                }
                            }
                            tcode = tcode.add(1 + LINK_SIZE);
                        } else {
                            return rc; /* FAIL, UNKNOWN, or TOODEEP */
                        }
                    }

                    /* If we hit ALT or KET, it means we haven't found anything mandatory in
                    this branch, though we might have found something optional. For ALT, we
                    continue with the next alternative, but we have to arrange that the final
                    result from subpattern is SSB_CONTINUE rather than SSB_DONE. For KET,
                    return SSB_CONTINUE: if this is the top level, that indicates failure,
                    but after a nested subpattern, it causes scanning to continue. */
                    OP_ALT => {
                        yield_ = SSB_CONTINUE;
                        try_next = FALSE;
                    }

                    OP_KET | OP_KETRMAX | OP_KETRMIN | OP_KETRPOS => {
                        return SSB_CONTINUE;
                    }

                    /* Skip over callout */
                    OP_CALLOUT => {
                        tcode = tcode.add(OP_LENGTHS[OP_CALLOUT as usize] as usize);
                    }

                    OP_CALLOUT_STR => {
                        tcode = tcode.add(get(tcode, 1 + 2 * LINK_SIZE) as usize);
                    }

                    /* Skip over lookbehind, negative lookahead, and scan substring
                    assertions */
                    OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT | OP_ASSERTBACK_NA
                    | OP_ASSERT_SCS => {
                        loop {
                            tcode = tcode.add(get(tcode, 1) as usize);
                            if *tcode != OP_ALT {
                                break;
                            }
                        }
                        tcode = tcode.add(1 + LINK_SIZE);
                    }

                    /* BRAZERO does the bracket, but carries on. */
                    OP_BRAZERO | OP_BRAMINZERO | OP_BRAPOSZERO => {
                        let rc: c_int;
                        tcode = tcode.add(1);
                        rc = set_start_bits(re, tcode, utf, ucp, depthptr);
                        if rc == SSB_FAIL || rc == SSB_UNKNOWN || rc == SSB_TOODEEP {
                            return rc;
                        }
                        loop {
                            tcode = tcode.add(get(tcode, 1) as usize);
                            if *tcode != OP_ALT {
                                break;
                            }
                        }
                        tcode = tcode.add(1 + LINK_SIZE);
                    }

                    /* SKIPZERO skips the bracket. */
                    OP_SKIPZERO => {
                        tcode = tcode.add(1);
                        loop {
                            tcode = tcode.add(get(tcode, 1) as usize);
                            if *tcode != OP_ALT {
                                break;
                            }
                        }
                        tcode = tcode.add(1 + LINK_SIZE);
                    }

                    /* Single-char * or ? sets the bit and tries the next item */
                    OP_STAR | OP_MINSTAR | OP_POSSTAR | OP_QUERY | OP_MINQUERY | OP_POSQUERY => {
                        tcode = set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                    }

                    OP_STARI | OP_MINSTARI | OP_POSSTARI | OP_QUERYI | OP_MINQUERYI
                    | OP_POSQUERYI => {
                        tcode = set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                    }

                    /* Single-char upto sets the bit and tries the next */
                    OP_UPTO | OP_MINUPTO | OP_POSUPTO => {
                        tcode = set_table_bit(re, tcode.add(1 + IMM2_SIZE), FALSE, utf, ucp);
                    }

                    OP_UPTOI | OP_MINUPTOI | OP_POSUPTOI => {
                        tcode = set_table_bit(re, tcode.add(1 + IMM2_SIZE), TRUE, utf, ucp);
                    }

                    /* At least one single char sets the bit and stops */
                    OP_EXACT => {
                        tcode = tcode.add(IMM2_SIZE);
                        /* Fall through */
                        set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                        try_next = FALSE;
                    }
                    OP_CHAR | OP_PLUS | OP_MINPLUS | OP_POSPLUS => {
                        set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                        try_next = FALSE;
                    }

                    OP_EXACTI => {
                        tcode = tcode.add(IMM2_SIZE);
                        /* Fall through */
                        set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                        try_next = FALSE;
                    }
                    OP_CHARI | OP_PLUSI | OP_MINPLUSI | OP_POSPLUSI => {
                        set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                        try_next = FALSE;
                    }

                    /* Special spacing and line-terminating items. These recognize specific
                    lists of characters. The difference between VSPACE and ANYNL is that the
                    latter can match the two-character CRLF sequence, but that is not
                    relevant for finding the first character, so their code here is
                    identical. */
                    OP_HSPACE => {
                        set_bit(re, CHAR_HT);
                        set_bit(re, CHAR_SPACE);

                        /* For the 8-bit library in UTF-8 mode, set the bits for the first code
                        units of horizontal space characters. */
                        if utf != FALSE {
                            set_bit(re, 0xC2); /* For U+00A0 */
                            set_bit(re, 0xE1); /* For U+1680, U+180E */
                            set_bit(re, 0xE2); /* For U+2000 - U+200A, U+202F, U+205F */
                            set_bit(re, 0xE3); /* For U+3000 */
                        } else {
                            /* For the 8-bit library not in UTF-8 mode, set the bit for NBSP. */
                            set_bit(re, CHAR_NBSP);
                        }

                        try_next = FALSE;
                    }

                    OP_ANYNL | OP_VSPACE => {
                        set_bit(re, CHAR_LF);
                        set_bit(re, CHAR_VT);
                        set_bit(re, CHAR_FF);
                        set_bit(re, CHAR_CR);

                        /* For the 8-bit library in UTF-8 mode, set the bits for the first code
                        units of vertical space characters. */
                        if utf != FALSE {
                            set_bit(re, 0xC2); /* For U+0085 (NEL) */
                            set_bit(re, 0xE2); /* For U+2028, U+2029 */
                        } else {
                            /* For the 8-bit library not in UTF-8 mode, set the bit for NEL. */
                            set_bit(re, CHAR_NEL);
                        }

                        try_next = FALSE;
                    }

                    /* Single character types set the bits and stop. Note that if PCRE2_UCP
                    is set, we do not see these opcodes because \d etc are converted to
                    properties. Therefore, these apply in the case when only characters less
                    than 256 are recognized to match the types. */
                    OP_NOT_DIGIT => {
                        set_nottype_bits(re, cbit_digit as c_int, table_limit);
                        try_next = FALSE;
                    }

                    OP_DIGIT => {
                        set_type_bits(re, cbit_digit as c_int, table_limit);
                        try_next = FALSE;
                    }

                    OP_NOT_WHITESPACE => {
                        set_nottype_bits(re, cbit_space as c_int, table_limit);
                        try_next = FALSE;
                    }

                    OP_WHITESPACE => {
                        set_type_bits(re, cbit_space as c_int, table_limit);
                        try_next = FALSE;
                    }

                    OP_NOT_WORDCHAR => {
                        set_nottype_bits(re, cbit_word as c_int, table_limit);
                        try_next = FALSE;
                    }

                    OP_WORDCHAR => {
                        set_type_bits(re, cbit_word as c_int, table_limit);
                        try_next = FALSE;
                    }

                    /* One or more character type fudges the pointer and restarts, knowing
                    it will hit a single character type and stop there. */
                    OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEPOSPLUS => {
                        tcode = tcode.add(1);
                    }

                    OP_TYPEEXACT => {
                        tcode = tcode.add(1 + IMM2_SIZE);
                    }

                    /* Zero or more repeats of character types set the bits and then
                    try again. */
                    OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO | OP_TYPESTAR
                    | OP_TYPEMINSTAR | OP_TYPEPOSSTAR | OP_TYPEQUERY | OP_TYPEMINQUERY
                    | OP_TYPEPOSQUERY => {
                        if *tcode == OP_TYPEUPTO
                            || *tcode == OP_TYPEMINUPTO
                            || *tcode == OP_TYPEPOSUPTO
                        {
                            tcode = tcode.add(IMM2_SIZE);
                        }

                        match *tcode.add(1) {
                            OP_ANY | OP_ALLANY => {
                                return SSB_FAIL;
                            }

                            OP_HSPACE => {
                                set_bit(re, CHAR_HT);
                                set_bit(re, CHAR_SPACE);
                                if utf != FALSE {
                                    set_bit(re, 0xC2); /* For U+00A0 */
                                    set_bit(re, 0xE1); /* For U+1680, U+180E */
                                    set_bit(re, 0xE2); /* For U+2000 - U+200A, U+202F, U+205F */
                                    set_bit(re, 0xE3); /* For U+3000 */
                                } else {
                                    set_bit(re, CHAR_NBSP);
                                }
                            }

                            OP_ANYNL | OP_VSPACE => {
                                set_bit(re, CHAR_LF);
                                set_bit(re, CHAR_VT);
                                set_bit(re, CHAR_FF);
                                set_bit(re, CHAR_CR);
                                if utf != FALSE {
                                    set_bit(re, 0xC2); /* For U+0085 (NEL) */
                                    set_bit(re, 0xE2); /* For U+2028, U+2029 */
                                } else {
                                    set_bit(re, CHAR_NEL);
                                }
                            }

                            OP_NOT_DIGIT => {
                                set_nottype_bits(re, cbit_digit as c_int, table_limit);
                            }

                            OP_DIGIT => {
                                set_type_bits(re, cbit_digit as c_int, table_limit);
                            }

                            OP_NOT_WHITESPACE => {
                                set_nottype_bits(re, cbit_space as c_int, table_limit);
                            }

                            OP_WHITESPACE => {
                                set_type_bits(re, cbit_space as c_int, table_limit);
                            }

                            OP_NOT_WORDCHAR => {
                                set_nottype_bits(re, cbit_word as c_int, table_limit);
                            }

                            OP_WORDCHAR => {
                                set_type_bits(re, cbit_word as c_int, table_limit);
                            }

                            _ => {
                                return SSB_FAIL;
                            }
                        }

                        tcode = tcode.add(2);
                    }

                    /* Set-based ECLASS: treat it the same as a "complex" XCLASS; give up. */
                    OP_ECLASS => {
                        return SSB_FAIL;
                    }

                    /* Extended class: if there are any property checks, or if this is a
                    negative XCLASS without a map, give up. If there are no property checks,
                    there must be wide characters on the XCLASS list, because otherwise an
                    XCLASS would not have been created. This means that code points >= 255
                    are potential starters. In the UTF-8 case we can scan them and set bits
                    for the relevant leading bytes. */
                    /* The XCLASS, NCLASS and CLASS cases fall through into each other in
                    the C code (via `goto HANDLE_CLASSMAP` and PCRE2_FALLTHROUGH). We model
                    this with a labelled block: `handle_classmap` is set true when the UTF-8
                    XCLASS path has already advanced `tcode` and jumped to HANDLE_CLASSMAP;
                    otherwise control falls through the NCLASS bit-setting (guarded by
                    `entered_nclass`) into the OP_CLASS advance code and then the classmap
                    handling. */
                    OP_XCLASS | OP_NCLASS | OP_CLASS => {
                        let op = *tcode;
                        let mut handle_classmap = false;
                        /* Both OP_NCLASS and a fallen-through OP_XCLASS enter the NCLASS
                        bit-setting code in C (OP_XCLASS's case precedes OP_NCLASS's, and
                        the non-UTF / negative-with-map XCLASS path falls through into it).
                        A plain OP_CLASS does not. */
                        let entered_nclass = op == OP_NCLASS || op == OP_XCLASS;
                        classmap = core::ptr::null();

                        'class_body: {
                            if op == OP_XCLASS {
                                let xclassflags: PCRE2_UCHAR;
                                xclassflags = *tcode.add(1 + LINK_SIZE);
                                if (xclassflags as u32 & XCL_HASPROP) != 0
                                    || (xclassflags as u32 & (XCL_MAP | XCL_NOT)) == XCL_NOT
                                {
                                    return SSB_FAIL;
                                }

                                /* We have a positive XCLASS or a negative one without a map.
                                Set up the map pointer if there is one, and fall through. */
                                classmap = if (xclassflags as u32 & XCL_MAP) == 0 {
                                    core::ptr::null()
                                } else {
                                    tcode.add(1 + LINK_SIZE + 1)
                                };

                                /* In UTF-8 mode, scan the character list and set bits for
                                leading bytes, then jump to handle the map. */
                                if utf != FALSE && (xclassflags as u32 & XCL_NOT) == 0 {
                                    let mut b: PCRE2_UCHAR;
                                    let mut e: PCRE2_UCHAR;
                                    let mut p: PCRE2_SPTR = tcode.add(
                                        1 + LINK_SIZE
                                            + 1
                                            + (if classmap.is_null() { 0 } else { 32 }),
                                    );
                                    tcode = tcode.add(get(tcode, 1) as usize);

                                    if *p as u32 >= XCL_LIST {
                                        study_char_list(
                                            p,
                                            (*re).start_bitmap.as_mut_ptr(),
                                            (re as *const u8).add((*re).code_start),
                                        );
                                        handle_classmap = true; /* goto HANDLE_CLASSMAP */
                                        break 'class_body;
                                    }

                                    'scan: loop {
                                        let item = *p;
                                        p = p.add(1);
                                        match item {
                                            v if v as u32 == XCL_SINGLE => {
                                                b = *p;
                                                p = p.add(1);
                                                while (*p & 0xc0) == 0x80 {
                                                    p = p.add(1);
                                                }
                                                (*re).start_bitmap[(b / 8) as usize] |=
                                                    1u8 << (b & 7);
                                            }

                                            v if v as u32 == XCL_RANGE => {
                                                b = *p;
                                                p = p.add(1);
                                                while (*p & 0xc0) == 0x80 {
                                                    p = p.add(1);
                                                }
                                                e = *p;
                                                p = p.add(1);
                                                while (*p & 0xc0) == 0x80 {
                                                    p = p.add(1);
                                                }
                                                while b <= e {
                                                    (*re).start_bitmap[(b / 8) as usize] |=
                                                        1u8 << (b & 7);
                                                    b += 1;
                                                }
                                            }

                                            v if v as u32 == XCL_END => {
                                                handle_classmap = true; /* goto HANDLE_CLASSMAP */
                                                break 'scan;
                                            }

                                            _ => {
                                                /* Internal error, should not occur */
                                                return SSB_UNKNOWN;
                                            }
                                        }
                                    }
                                    break 'class_body;
                                }

                                /* Fall through (non-UTF, or negative XCLASS with a map): in C
                                this drops into the OP_NCLASS case body (setting the high-byte
                                bits when in UTF mode) and then into OP_CLASS. `entered_nclass`
                                is true for OP_XCLASS, so the block below runs. */
                            }

                            /* Enter here for a negative non-XCLASS (or fall-through from
                            XCLASS). In the 8-bit library, if we are in UTF mode, any byte with
                            a value >= 0xc4 is a potentially valid starter because it starts a
                            character with a value > 255. */
                            if entered_nclass {
                                if utf != FALSE {
                                    (*re).start_bitmap[24] |= 0xf0; /* Bits for 0xc4 - 0xc8 */
                                    /* Bits for 0xc9 - 0xff */
                                    core::ptr::write_bytes(
                                        (*re).start_bitmap.as_mut_ptr().add(25),
                                        0xff,
                                        7,
                                    );
                                }
                            }

                            /* Enter here for a positive non-XCLASS. If we have fallen through
                            from an XCLASS, classmap will already be set; just advance the code
                            pointer. Otherwise, set up classmap for a non-XCLASS and advance
                            past it. */
                            if *tcode == OP_XCLASS {
                                tcode = tcode.add(get(tcode, 1) as usize);
                            } else {
                                tcode = tcode.add(1);
                                classmap = tcode;
                                tcode = tcode.add(32 / core::mem::size_of::<PCRE2_UCHAR>());
                            }
                        } /* 'class_body: HANDLE_CLASSMAP falls here */

                        let _ = handle_classmap;

                        /* When wide characters are supported, classmap may be NULL. In UTF-8
                        (sic) mode, the bits in a class bit map correspond to character values,
                        not to byte values. However, the bit map we are constructing is for
                        byte values. So we have to do a conversion for characters whose code
                        point is greater than 127. In fact, there are only two possible
                        starting bytes for characters in the range 128 - 255. */

                        if !classmap.is_null() {
                            if utf != FALSE {
                                c = 0;
                                while c < 16 {
                                    (*re).start_bitmap[c as usize] |= *classmap.add(c as usize);
                                    c += 1;
                                }
                                c = 128;
                                while c < 256 {
                                    if (*classmap.add((c / 8) as usize) & (1u8 << (c & 7))) != 0 {
                                        let d = (c >> 6) | 0xc0; /* Set bit for this starter */
                                        (*re).start_bitmap[(d / 8) as usize] |= 1u8 << (d & 7); /* and then skip on to the */
                                        c = (c & 0xc0) + 0x40 - 1; /* next relevant character. */
                                    }
                                    c += 1;
                                }
                            } else {
                                /* In all modes except UTF-8, the two bit maps are compatible. */
                                c = 0;
                                while c < 32 {
                                    (*re).start_bitmap[c as usize] |= *classmap.add(c as usize);
                                    c += 1;
                                }
                            }
                        }

                        /* Act on what follows the class. For a zero minimum repeat, continue;
                        otherwise stop processing. */

                        match *tcode {
                            OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSSTAR
                            | OP_CRPOSQUERY => {
                                tcode = tcode.add(1);
                            }

                            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                if get2(tcode, 1) == 0 {
                                    tcode = tcode.add(1 + 2 * IMM2_SIZE);
                                } else {
                                    try_next = FALSE;
                                }
                            }

                            _ => {
                                try_next = FALSE;
                            }
                        }
                    }

                    /* If we reach something we don't understand, it means a new opcode has
                    been created that hasn't been added to this function. */
                    _ => {
                        return SSB_UNKNOWN;
                    }
                }
            } /* End of try_next loop */

            code = code.add(get(code, 1) as usize); /* Advance to next branch */

            if *code != OP_ALT {
                break;
            }
        }

        yield_
    }
}

/*************************************************
*          Study a compiled expression           *
*************************************************/

/* This function is handed a compiled expression that it must study to produce
information that will speed up the matching.

Argument:
  re       points to the compiled expression

Returns:   0 normally; non-zero should never normally occur
           1 unknown opcode in set_start_bits
           2 missing capturing bracket
           3 unknown opcode in find_minlength
*/

pub unsafe fn study(re: *mut pcre2_real_code) -> c_int {
    unsafe {
        let mut count: c_int = 0;
        let code: *mut PCRE2_UCHAR;
        let utf: BOOL = (((*re).overall_options & PCRE2_UTF) != 0) as BOOL;
        let ucp: BOOL = (((*re).overall_options & PCRE2_UCP) != 0) as BOOL;

        /* Find start of compiled code */

        code = (re as *mut u8).add((*re).code_start) as *mut PCRE2_UCHAR;

        /* For a pattern that has a first code unit, or a multiline pattern that
        matches only at "line start", there is no point in seeking a list of starting
        code units. */

        if ((*re).flags & (PCRE2_FIRSTSET | PCRE2_STARTLINE)) == 0 {
            let mut depth: c_int = 0;
            let rc = set_start_bits(re, code, utf, ucp, &mut depth);
            if rc == SSB_UNKNOWN {
                return 1;
            }

            /* If a list of starting code units was set up, scan the list to see if only
            one or two were listed. Having only one listed is rare because usually a
            single starting code unit will have been recognized and PCRE2_FIRSTSET set.
            If two are listed, see if they are caseless versions of the same character;
            if so we can replace the list with a caseless first code unit. This gives
            better performance and is plausibly worth doing for patterns such as [Ww]ord
            or (word|WORD). */

            if rc == SSB_DONE {
                let mut i: c_int;
                let mut a: c_int = -1;
                let mut b: c_int = -1;
                let mut p: *mut u8 = (*re).start_bitmap.as_mut_ptr();
                let mut flags: u32 = PCRE2_FIRSTMAPSET;

                'outer: {
                    i = 0;
                    while i < 256 {
                        let x: u8 = *p;
                        if x != 0 {
                            let mut cc_val: c_int;
                            let y: u8 = x & (!x).wrapping_add(1); /* Least significant bit */
                            if y != x {
                                break 'outer; /* More than one bit set: goto DONE */
                            }

                            /* Compute the character value */

                            cc_val = i;
                            match x {
                                1 => {}
                                2 => cc_val += 1,
                                4 => cc_val += 2,
                                8 => cc_val += 3,
                                16 => cc_val += 4,
                                32 => cc_val += 5,
                                64 => cc_val += 6,
                                128 => cc_val += 7,
                                _ => {}
                            }

                            /* c contains the code unit value, in the range 0-255. In 8-bit UTF
                            mode, only values < 128 can be used. */

                            if utf != FALSE && cc_val > 127 {
                                break 'outer; /* goto DONE */
                            }

                            if a < 0 {
                                a = cc_val; /* First one found, save in a */
                            } else if b < 0 {
                                /* Second one found */
                                let mut d: c_int = table_get(
                                    cc_val as u32,
                                    (*re).tables.add(fcc_offset),
                                    cc_val as u32,
                                ) as c_int;

                                if utf != FALSE || ucp != FALSE {
                                    if ucd_caseset(cc_val as u32) != 0 {
                                        break 'outer; /* Multiple case set: goto DONE */
                                    }
                                    if cc_val > 127 {
                                        d = ucd_othercase(cc_val as u32) as c_int;
                                    }
                                }

                                if d != a {
                                    break 'outer; /* Not the other case of a: goto DONE */
                                }
                                b = cc_val; /* Save second in b */
                            } else {
                                break 'outer; /* More than two characters found: goto DONE */
                            }
                        }

                        p = p.add(1);
                        i += 8;
                    }

                    /* Replace the start code unit bits with a first code unit. If it is the
                    same as a required later code unit, then clear the required later code
                    unit. This is because a search for a required code unit starts after an
                    explicit first code unit, but at a code unit found from the bitmap.
                    Patterns such as /a*a/ don't work if both the start unit and required
                    unit are the same. */

                    if a >= 0 {
                        if ((*re).flags & PCRE2_LASTSET) != 0
                            && ((*re).last_codeunit == a as u32
                                || (b >= 0 && (*re).last_codeunit == b as u32))
                        {
                            (*re).flags &= !(PCRE2_LASTSET | PCRE2_LASTCASELESS);
                            (*re).last_codeunit = 0;
                        }
                        (*re).first_codeunit = a as u32;
                        flags = PCRE2_FIRSTSET;
                        if b >= 0 {
                            flags |= PCRE2_FIRSTCASELESS;
                        }
                    }
                } /* DONE */

                (*re).flags |= flags;
            }
        }

        /* Find the minimum length of subject string. If the pattern can match an empty
        string, the minimum length is already known. If the pattern contains (*ACCEPT)
        all bets are off, and we don't even try to find a minimum length. If there are
        more back references than the size of the vector we are going to cache them in,
        do nothing. A pattern that complicated will probably take a long time to
        analyze and may in any case turn out to be too complicated. Note that back
        reference minima are held as 16-bit numbers. */

        if ((*re).flags & (PCRE2_MATCH_EMPTY | PCRE2_HASACCEPT)) == 0
            && (*re).top_backref as usize <= MAX_CACHE_BACKREF
        {
            let min: c_int;
            let mut backref_cache: [c_int; MAX_CACHE_BACKREF + 1] = [0; MAX_CACHE_BACKREF + 1];
            backref_cache[0] = 0; /* Highest one that is set */
            min = find_minlength(
                re,
                code,
                code,
                utf,
                core::ptr::null(),
                &mut count,
                backref_cache.as_mut_ptr(),
            );
            match min {
                -1 => {
                    /* \C in UTF mode or over-complex regex */
                    /* Leave minlength unchanged (will be zero) */
                }

                -2 => {
                    return 2; /* missing capturing bracket */
                }

                -3 => {
                    return 3; /* unrecognized opcode */
                }

                _ => {
                    (*re).minlength = if min > UINT16_MAX as c_int {
                        UINT16_MAX as u16
                    } else {
                        min as u16
                    };
                }
            }
        }

        0
    }
}

/// Exported as `_pcre2_study_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_study_8(re: *mut pcre2_real_code) -> c_int {
    unsafe { study(re) }
}
