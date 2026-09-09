//! Translated from pcre2_study.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

use crate::find_bracket::_pcre2_find_bracket_8;
use crate::ord2utf::_pcre2_ord2utf_8;
use crate::tables::_pcre2_OP_lengths_8;
use crate::ucd::_pcre2_ucd_caseless_sets_8;

/* Limits used by this module that come from <limits.h>/<stdint.h> in C. */

const UINT16_MAX: i32 = 0xffff;
const INT_MAX: i32 = 0x7fffffff;

/* The maximum remembered capturing brackets minimum. */

const MAX_CACHE_BACKREF: usize = 128;

/* Set a bit in the starting code unit bit map. In C this is

     #define SET_BIT(c) re->start_bitmap[(c)/8] |= (1u << ((c)&7))

   which picks up `re` from the enclosing scope; Rust macros are hygienic for
   local variables, so `re` is passed explicitly. */

macro_rules! SET_BIT {
    ($re:expr, $c:expr) => {
        (*$re).start_bitmap[(($c) as usize) / 8] |= (1u32 << (($c) as u32 & 7)) as u8
    };
}

/* PRIV(OP_lengths)[i] */

macro_rules! OP_LENGTHS {
    ($i:expr) => {
        _pcre2_OP_lengths_8[($i) as usize] as usize
    };
}

/* Returns from set_start_bits() */

const SSB_FAIL: i32 = 0;
const SSB_DONE: i32 = 1;
const SSB_CONTINUE: i32 = 2;
const SSB_UNKNOWN: i32 = 3;
const SSB_TOODEEP: i32 = 4;

/* Labels emulated in find_minlength() */

const FM_SWITCH: u32 = 0;
const FM_PROCESS_NON_CAPTURE: u32 = 1;
const FM_REPEAT_BACK_REFERENCE: u32 = 2;

/* Labels emulated in set_start_bits() */

const SB_SWITCH: u32 = 0;
const SB_BRA_GROUP: u32 = 1;
const SB_TYPESTAR: u32 = 2;
const SB_NCLASS: u32 = 3;
const SB_CLASS: u32 = 4;
const SB_HANDLE_CLASSMAP: u32 = 5;

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

pub(crate) unsafe fn find_minlength(
    re: *const pcre2_real_code,
    code: PCRE2_SPTR,
    startcode: PCRE2_SPTR,
    utf: BOOL,
    recurses: *mut recurse_check,
    countptr: *mut i32,
    backref_cache: *mut i32,
) -> i32 {
    let mut length: i32 = -1;
    let mut branchlength: i32 = 0;
    let mut prev_cap_recno: i32 = -1;
    let mut prev_cap_d: i32 = 0;
    let mut prev_recurse_recno: i32 = -1;
    let mut prev_recurse_d: i32 = 0;
    let mut once_fudge: u32 = 0;
    let mut had_recurse: BOOL = FALSE;
    let dupcapused: BOOL = (((*re).flags & PCRE2_DUPCAPUSED) != 0) as BOOL;
    let mut nextbranch: PCRE2_SPTR = code.add(GET!(code, 1) as usize);
    let mut cc: PCRE2_SPTR = code.add(1 + LINK_SIZE);
    let mut this_recurse: recurse_check = recurse_check {
        prev: core::ptr::null_mut(),
        group: core::ptr::null(),
    };

    /* If this is a "could be empty" group, its minimum length is 0. */

    if (*code as u32) >= OP_SBRA && (*code as u32) <= OP_SCOND {
        return 0;
    }

    /* Skip over capturing bracket number */

    if (*code as u32) == OP_CBRA || (*code as u32) == OP_CBRAPOS {
        cc = cc.add(IMM2_SIZE);
    }

    /* A large and/or complex regex can take too long to process. */

    if {
        let t_ = *countptr;
        *countptr = t_ + 1;
        t_
    } > 1000
    {
        return -1;
    }

    /* Scan along the opcodes for this branch. If we get to the end of the branch,
    check the length against that of the other branches. If the accumulated length
    passes 16-bits, reset to that value and skip the rest of the branch. */

    loop {
        let mut d: i32 = 0;
        let mut min: i32 = 0;
        let mut recno: i32 = 0;
        let op: u8;
        let mut cs: PCRE2_SPTR = core::ptr::null();
        let mut ce: PCRE2_SPTR = core::ptr::null();

        if branchlength >= UINT16_MAX {
            branchlength = UINT16_MAX;
            cc = nextbranch;
        }

        op = *cc;
        let mut state: u32 = FM_SWITCH;
        'sm: loop {
            match state {
                FM_SWITCH => match op as u32 {
                    OP_COND | OP_SCOND => {
                        /* If there is only one branch in a condition, the implied branch has
                        zero length, so we don't add anything. This covers the DEFINE
                        "condition" automatically. If there are two branches we can treat it
                        the same as any other non-capturing subpattern. */

                        cs = cc.add(GET!(cc, 1) as usize);
                        if (*cs as u32) != OP_ALT {
                            cc = cs.add(1 + LINK_SIZE);
                            break 'sm;
                        }
                        /* goto PROCESS_NON_CAPTURE */
                        state = FM_PROCESS_NON_CAPTURE;
                        continue 'sm;
                    }

                    OP_BRA => {
                        /* There's a special case of OP_BRA, when it is wrapped round a
                        repeated OP_RECURSE. We'd like to process the latter at this level so
                        that remembering the value works for repeated cases. So we do nothing,
                        but set a fudge value to skip over the OP_KET after the recurse. */

                        if (*cc.add(1 + LINK_SIZE) as u32) == OP_RECURSE
                            && (*cc.add(2 * (1 + LINK_SIZE)) as u32) == OP_KET
                        {
                            once_fudge = (1 + LINK_SIZE) as u32;
                            cc = cc.add(1 + LINK_SIZE);
                            break 'sm;
                        }
                        /* Fall through */
                        state = FM_PROCESS_NON_CAPTURE;
                        continue 'sm;
                    }

                    OP_ONCE | OP_SCRIPT_RUN | OP_SBRA | OP_BRAPOS | OP_SBRAPOS => {
                        state = FM_PROCESS_NON_CAPTURE;
                        continue 'sm;
                    }

                    /* To save time for repeated capturing subpatterns, we remember the
                    length of the previous one. Unfortunately we can't do the same for
                    the unnumbered ones above. Nor can we do this if (?| is present in the
                    pattern because captures with the same number are not then identical. */

                    OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS => {
                        recno = GET2!(cc, 1 + LINK_SIZE) as i32;
                        if dupcapused != 0 || recno != prev_cap_recno {
                            prev_cap_recno = recno;
                            prev_cap_d = find_minlength(
                                re,
                                cc,
                                startcode,
                                utf,
                                recurses,
                                countptr,
                                backref_cache,
                            );
                            if prev_cap_d < 0 {
                                return prev_cap_d;
                            }
                        }
                        branchlength += prev_cap_d;
                        loop {
                            cc = cc.add(GET!(cc, 1) as usize);
                            if (*cc as u32) != OP_ALT {
                                break;
                            }
                        }
                        cc = cc.add(1 + LINK_SIZE);
                        break 'sm;
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
                        if length < 0 || (had_recurse == 0 && branchlength < length) {
                            length = branchlength;
                        }
                        if (op as u32) != OP_ALT || length == 0 {
                            return length;
                        }
                        nextbranch = cc.add(GET!(cc, 1) as usize);
                        cc = cc.add(1 + LINK_SIZE);
                        branchlength = 0;
                        had_recurse = FALSE;
                        break 'sm;
                    }

                    /* Skip over assertive subpatterns */

                    OP_ASSERT | OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT
                    | OP_ASSERT_NA | OP_ASSERT_SCS | OP_ASSERTBACK_NA => {
                        loop {
                            cc = cc.add(GET!(cc, 1) as usize);
                            if (*cc as u32) != OP_ALT {
                                break;
                            }
                        }
                        /* Fall through */
                        cc = cc.add(OP_LENGTHS!(*cc));
                        break 'sm;
                    }

                    /* Skip over things that don't match chars */

                    OP_REVERSE | OP_VREVERSE | OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF
                    | OP_FALSE | OP_TRUE | OP_CALLOUT | OP_SOD | OP_SOM | OP_EOD | OP_EODN
                    | OP_CIRC | OP_CIRCM | OP_DOLL | OP_DOLLM | OP_NOT_WORD_BOUNDARY
                    | OP_WORD_BOUNDARY | OP_NOT_UCP_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY => {
                        cc = cc.add(OP_LENGTHS!(*cc));
                        break 'sm;
                    }

                    OP_CALLOUT_STR => {
                        cc = cc.add(GET!(cc, 1 + 2 * LINK_SIZE) as usize);
                        break 'sm;
                    }

                    /* Skip over a subpattern that has a {0} or {0,x} quantifier */

                    OP_BRAZERO | OP_BRAMINZERO | OP_BRAPOSZERO | OP_SKIPZERO => {
                        cc = cc.add(OP_LENGTHS!(*cc));
                        loop {
                            cc = cc.add(GET!(cc, 1) as usize);
                            if (*cc as u32) != OP_ALT {
                                break;
                            }
                        }
                        cc = cc.add(1 + LINK_SIZE);
                        break 'sm;
                    }

                    /* Handle literal characters and + repetitions */

                    OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_PLUS | OP_PLUSI | OP_MINPLUS
                    | OP_MINPLUSI | OP_POSPLUS | OP_POSPLUSI | OP_NOTPLUS | OP_NOTPLUSI
                    | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_NOTPOSPLUS | OP_NOTPOSPLUSI => {
                        branchlength += 1;
                        cc = cc.add(2);
                        if utf != 0 && HAS_EXTRALEN!(*cc.offset(-1)) {
                            cc = cc.add(GET_EXTRALEN!(*cc.offset(-1)) as usize);
                        }
                        break 'sm;
                    }

                    OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEPOSPLUS => {
                        branchlength += 1;
                        cc = cc.add(
                            if (*cc.add(1) as u32) == OP_PROP || (*cc.add(1) as u32) == OP_NOTPROP {
                                4
                            } else {
                                2
                            },
                        );
                        break 'sm;
                    }

                    /* Handle exact repetitions. The count is already in characters, but we
                    may need to skip over a multibyte character in UTF mode.  */

                    OP_EXACT | OP_EXACTI | OP_NOTEXACT | OP_NOTEXACTI => {
                        branchlength += GET2!(cc, 1) as i32;
                        cc = cc.add(2 + IMM2_SIZE);
                        if utf != 0 && HAS_EXTRALEN!(*cc.offset(-1)) {
                            cc = cc.add(GET_EXTRALEN!(*cc.offset(-1)) as usize);
                        }
                        break 'sm;
                    }

                    OP_TYPEEXACT => {
                        branchlength += GET2!(cc, 1) as i32;
                        cc = cc.add(
                            2 + IMM2_SIZE
                                + if (*cc.add(1 + IMM2_SIZE) as u32) == OP_PROP
                                    || (*cc.add(1 + IMM2_SIZE) as u32) == OP_NOTPROP
                                {
                                    2
                                } else {
                                    0
                                },
                        );
                        break 'sm;
                    }

                    /* Handle single-char non-literal matchers */

                    OP_PROP | OP_NOTPROP => {
                        cc = cc.add(2);
                        /* Fall through */
                        branchlength += 1;
                        cc = cc.add(1);
                        break 'sm;
                    }

                    OP_NOT_DIGIT | OP_DIGIT | OP_NOT_WHITESPACE | OP_WHITESPACE
                    | OP_NOT_WORDCHAR | OP_WORDCHAR | OP_ANY | OP_ALLANY | OP_EXTUNI
                    | OP_HSPACE | OP_NOT_HSPACE | OP_VSPACE | OP_NOT_VSPACE => {
                        branchlength += 1;
                        cc = cc.add(1);
                        break 'sm;
                    }

                    /* "Any newline" might match two characters, but it also might match just
                    one. */

                    OP_ANYNL => {
                        branchlength += 1;
                        cc = cc.add(1);
                        break 'sm;
                    }

                    /* The single-byte matcher means we can't proceed in UTF mode. (In
                    non-UTF mode \C will actually be turned into OP_ALLANY, so won't ever
                    appear, but leave the code, just in case.) */

                    OP_ANYBYTE => {
                        if utf != 0 {
                            return -1;
                        }
                        branchlength += 1;
                        cc = cc.add(1);
                        break 'sm;
                    }

                    /* For repeated character types, we have to test for \p and \P, which have
                    an extra two bytes of parameters. */

                    OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEQUERY | OP_TYPEMINQUERY
                    | OP_TYPEPOSSTAR | OP_TYPEPOSQUERY => {
                        if (*cc.add(1) as u32) == OP_PROP || (*cc.add(1) as u32) == OP_NOTPROP {
                            cc = cc.add(2);
                        }
                        cc = cc.add(OP_LENGTHS!(op));
                        break 'sm;
                    }

                    OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO => {
                        if (*cc.add(1 + IMM2_SIZE) as u32) == OP_PROP
                            || (*cc.add(1 + IMM2_SIZE) as u32) == OP_NOTPROP
                        {
                            cc = cc.add(2);
                        }
                        cc = cc.add(OP_LENGTHS!(op));
                        break 'sm;
                    }

                    /* Check a class for variable quantification */

                    OP_CLASS | OP_NCLASS | OP_XCLASS | OP_ECLASS => {
                        /* The original code caused an unsigned overflow in 64 bit systems,
                        so now we use a conditional statement. */
                        if (op as u32) == OP_XCLASS || (op as u32) == OP_ECLASS {
                            cc = cc.add(GET!(cc, 1) as usize);
                        } else {
                            cc = cc.add(OP_LENGTHS!(OP_CLASS));
                        }

                        match *cc as u32 {
                            OP_CRPLUS | OP_CRMINPLUS | OP_CRPOSPLUS => {
                                branchlength += 1;
                                /* Fall through */
                                cc = cc.add(1);
                            }

                            OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY
                            | OP_CRPOSSTAR | OP_CRPOSQUERY => {
                                cc = cc.add(1);
                            }

                            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                branchlength += GET2!(cc, 1) as i32;
                                cc = cc.add(1 + 2 * IMM2_SIZE);
                            }

                            _ => {
                                branchlength += 1;
                            }
                        }
                        break 'sm;
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
                        if dupcapused == 0
                            && ((*re).overall_options & PCRE2_MATCH_UNSET_BACKREF) == 0
                        {
                            let mut count: i32 = GET2!(cc, 1 + IMM2_SIZE) as i32;
                            let mut slot: PCRE2_SPTR = (re as *const u8)
                                .add(core::mem::size_of::<pcre2_real_code>())
                                .add((GET2!(cc, 1) * (*re).name_entry_size as u32) as usize);

                            d = INT_MAX;

                            /* Scan all groups with the same name; find the shortest. */

                            loop {
                                let t_ = count;
                                count -= 1;
                                if !(t_ > 0) {
                                    break;
                                }

                                let mut dd: i32;
                                let mut i: i32;
                                recno = GET2!(slot, 0) as i32;

                                if recno <= *backref_cache.add(0)
                                    && *backref_cache.add(recno as usize) >= 0
                                {
                                    dd = *backref_cache.add(recno as usize);
                                } else {
                                    cs = _pcre2_find_bracket_8(startcode, utf, recno);
                                    ce = cs;
                                    if cs.is_null() {
                                        return -2;
                                    }
                                    loop {
                                        ce = ce.add(GET!(ce, 1) as usize);
                                        if (*ce as u32) != OP_ALT {
                                            break;
                                        }
                                    }

                                    dd = 0;
                                    if dupcapused == 0
                                        || _pcre2_find_bracket_8(ce, utf, recno).is_null()
                                    {
                                        if cc > cs && cc < ce
                                        /* Simple recursion */
                                        {
                                            had_recurse = TRUE;
                                        } else {
                                            let mut r: *mut recurse_check = recurses;
                                            r = recurses;
                                            while !r.is_null() {
                                                if (*r).group == cs {
                                                    break;
                                                }
                                                r = (*r).prev;
                                            }
                                            if !r.is_null()
                                            /* Mutual recursion */
                                            {
                                                had_recurse = TRUE;
                                            } else {
                                                this_recurse.prev = recurses; /* No recursion */
                                                this_recurse.group = cs;
                                                dd = find_minlength(
                                                    re,
                                                    cs,
                                                    startcode,
                                                    utf,
                                                    &mut this_recurse,
                                                    countptr,
                                                    backref_cache,
                                                );
                                                if dd < 0 {
                                                    return dd;
                                                }
                                            }
                                        }
                                    }

                                    *backref_cache.add(recno as usize) = dd;
                                    i = *backref_cache.add(0) + 1;
                                    while i < recno {
                                        *backref_cache.add(i as usize) = -1;
                                        i += 1;
                                    }
                                    *backref_cache.add(0) = recno;
                                }

                                if dd < d {
                                    d = dd;
                                }
                                if d <= 0 {
                                    break; /* No point looking at any more */
                                }
                                slot = slot.add((*re).name_entry_size as usize);
                            }
                        } else {
                            d = 0;
                        }
                        cc = cc.add(OP_LENGTHS!(*cc));
                        /* goto REPEAT_BACK_REFERENCE */
                        state = FM_REPEAT_BACK_REFERENCE;
                        continue 'sm;
                    }

                    /* Single back reference by number. References by name are converted to by
                    number when there is no duplication. */

                    OP_REF | OP_REFI => {
                        recno = GET2!(cc, 1) as i32;
                        if recno <= *backref_cache.add(0)
                            && *backref_cache.add(recno as usize) >= 0
                        {
                            d = *backref_cache.add(recno as usize);
                        } else {
                            let mut i: i32;
                            d = 0;

                            if ((*re).overall_options & PCRE2_MATCH_UNSET_BACKREF) == 0 {
                                cs = _pcre2_find_bracket_8(startcode, utf, recno);
                                ce = cs;
                                if cs.is_null() {
                                    return -2;
                                }
                                loop {
                                    ce = ce.add(GET!(ce, 1) as usize);
                                    if (*ce as u32) != OP_ALT {
                                        break;
                                    }
                                }

                                if dupcapused == 0
                                    || _pcre2_find_bracket_8(ce, utf, recno).is_null()
                                {
                                    if cc > cs && cc < ce
                                    /* Simple recursion */
                                    {
                                        had_recurse = TRUE;
                                    } else {
                                        let mut r: *mut recurse_check = recurses;
                                        r = recurses;
                                        while !r.is_null() {
                                            if (*r).group == cs {
                                                break;
                                            }
                                            r = (*r).prev;
                                        }
                                        if !r.is_null()
                                        /* Mutual recursion */
                                        {
                                            had_recurse = TRUE;
                                        } else
                                        /* No recursion */
                                        {
                                            this_recurse.prev = recurses;
                                            this_recurse.group = cs;
                                            d = find_minlength(
                                                re,
                                                cs,
                                                startcode,
                                                utf,
                                                &mut this_recurse,
                                                countptr,
                                                backref_cache,
                                            );
                                            if d < 0 {
                                                return d;
                                            }
                                        }
                                    }
                                }
                            }

                            *backref_cache.add(recno as usize) = d;
                            i = *backref_cache.add(0) + 1;
                            while i < recno {
                                *backref_cache.add(i as usize) = -1;
                                i += 1;
                            }
                            *backref_cache.add(0) = recno;
                        }

                        cc = cc.add(OP_LENGTHS!(*cc));

                        /* Handle repeated back references */

                        /* Fall through to REPEAT_BACK_REFERENCE */
                        state = FM_REPEAT_BACK_REFERENCE;
                        continue 'sm;
                    }

                    /* Recursion always refers to the first occurrence of a subpattern with a
                    given number. Therefore, we can always make use of caching, even when the
                    pattern contains multiple subpatterns with the same number. */

                    OP_RECURSE => {
                        ce = startcode.add(GET!(cc, 1) as usize);
                        cs = ce;
                        recno = GET2!(cs, 1 + LINK_SIZE) as i32;
                        if recno == prev_recurse_recno {
                            branchlength += prev_recurse_d;
                        } else {
                            loop {
                                ce = ce.add(GET!(ce, 1) as usize);
                                if (*ce as u32) != OP_ALT {
                                    break;
                                }
                            }
                            if cc > cs && cc < ce
                            /* Simple recursion */
                            {
                                had_recurse = TRUE;
                            } else {
                                let mut r: *mut recurse_check = recurses;
                                r = recurses;
                                while !r.is_null() {
                                    if (*r).group == cs {
                                        break;
                                    }
                                    r = (*r).prev;
                                }
                                if !r.is_null()
                                /* Mutual recursion */
                                {
                                    had_recurse = TRUE;
                                } else {
                                    this_recurse.prev = recurses;
                                    this_recurse.group = cs;
                                    prev_recurse_d = find_minlength(
                                        re,
                                        cs,
                                        startcode,
                                        utf,
                                        &mut this_recurse,
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
                        break 'sm;
                    }

                    /* Anything else does not or need not match a character. We can get the
                    item's length from the table, but for those that can match zero occurrences
                    of a character, we must take special action for UTF-8 characters. As it
                    happens, the "NOT" versions of these opcodes are used at present only for
                    ASCII characters, so they could be omitted from this list. However, in
                    future that may change, so we include them here so as not to leave a
                    gotcha for a future maintainer. */

                    OP_UPTO | OP_UPTOI | OP_NOTUPTO | OP_NOTUPTOI | OP_MINUPTO | OP_MINUPTOI
                    | OP_NOTMINUPTO | OP_NOTMINUPTOI | OP_POSUPTO | OP_POSUPTOI
                    | OP_NOTPOSUPTO | OP_NOTPOSUPTOI | OP_STAR | OP_STARI | OP_NOTSTAR
                    | OP_NOTSTARI | OP_MINSTAR | OP_MINSTARI | OP_NOTMINSTAR
                    | OP_NOTMINSTARI | OP_POSSTAR | OP_POSSTARI | OP_NOTPOSSTAR
                    | OP_NOTPOSSTARI | OP_QUERY | OP_QUERYI | OP_NOTQUERY | OP_NOTQUERYI
                    | OP_MINQUERY | OP_MINQUERYI | OP_NOTMINQUERY | OP_NOTMINQUERYI
                    | OP_POSQUERY | OP_POSQUERYI | OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                        cc = cc.add(OP_LENGTHS!(op));
                        if utf != 0 && HAS_EXTRALEN!(*cc.offset(-1)) {
                            cc = cc.add(GET_EXTRALEN!(*cc.offset(-1)) as usize);
                        }
                        break 'sm;
                    }

                    /* Skip these, but we need to add in the name length. */

                    OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                        cc = cc.add(OP_LENGTHS!(op) + *cc.add(1) as usize);
                        break 'sm;
                    }

                    /* The remaining opcodes are just skipped over. */

                    OP_CLOSE | OP_COMMIT | OP_FAIL | OP_PRUNE | OP_SET_SOM | OP_SKIP
                    | OP_THEN => {
                        cc = cc.add(OP_LENGTHS!(op));
                        break 'sm;
                    }

                    /* This should not occur: we list all opcodes explicitly so that when
                    new ones get added they are properly considered. */

                    _ => {
                        /* PCRE2_DEBUG_UNREACHABLE */
                        return -3;
                    }
                },

                FM_PROCESS_NON_CAPTURE => {
                    d = find_minlength(re, cc, startcode, utf, recurses, countptr, backref_cache);
                    if d < 0 {
                        return d;
                    }
                    branchlength += d;
                    loop {
                        cc = cc.add(GET!(cc, 1) as usize);
                        if (*cc as u32) != OP_ALT {
                            break;
                        }
                    }
                    cc = cc.add(1 + LINK_SIZE);
                    break 'sm;
                }

                FM_REPEAT_BACK_REFERENCE => {
                    match *cc as u32 {
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
                            min = GET2!(cc, 1) as i32;
                            cc = cc.add(1 + 2 * IMM2_SIZE);
                        }

                        _ => {
                            min = 1;
                        }
                    }

                    /* Take care not to overflow: (1) min and d are ints, so check that their
                    product is not greater than INT_MAX. (2) branchlength is limited to
                    UINT16_MAX (checked at the top of the loop). */

                    if (d > 0 && (INT_MAX / d) < min)
                        || UINT16_MAX - branchlength < min.wrapping_mul(d)
                    {
                        branchlength = UINT16_MAX;
                    } else {
                        branchlength += min.wrapping_mul(d);
                    }
                    break 'sm;
                }

                _ => {
                    break 'sm;
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

pub(crate) unsafe fn set_table_bit(
    re: *mut pcre2_real_code,
    p: PCRE2_SPTR,
    caseless: BOOL,
    utf: BOOL,
    ucp: BOOL,
) -> PCRE2_SPTR {
    let mut p: PCRE2_SPTR = p;
    let mut c: u32 = {
        let t_ = *p as u32;
        p = p.add(1);
        t_
    }; /* First code unit */

    /* In 16-bit and 32-bit modes, code units greater than 0xff set the bit for
    0xff. */

    SET_BIT!(re, c);

    /* In UTF-8 or UTF-16 mode, pick up the remaining code units in order to find
    the end of the character, even when caseless. */

    if utf != 0 {
        if c >= 0xc0 {
            GETUTF8INC!(c, p);
        }
    }

    /* If caseless, handle the other case of the character. */

    if caseless != 0 {
        if utf != 0 || ucp != 0 {
            c = UCD_OTHERCASE!(c);
            if utf != 0 {
                let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
                _pcre2_ord2utf_8(c, buff.as_mut_ptr());
                SET_BIT!(re, buff[0]);
            } else if c < 256 {
                SET_BIT!(re, c);
            }
        }
        /* Not UTF or UCP */
        else if MAX_255!(c) != 0 {
            SET_BIT!(re, *(*re).tables.add(fcc_offset + c as usize));
        }
    }

    p
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

pub(crate) unsafe fn set_type_bits(
    re: *mut pcre2_real_code,
    cbit_type: i32,
    table_limit: u32,
) {
    let mut c: u32;
    c = 0;
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
        if ((*(*re).tables.add(cbits_offset + (c / 8) as usize) as u32) & (1u32 << (c & 7))) != 0 {
            let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
            _pcre2_ord2utf_8(c, buff.as_mut_ptr());
            SET_BIT!(re, buff[0]);
        }
        c += 1;
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

pub(crate) unsafe fn set_nottype_bits(
    re: *mut pcre2_real_code,
    cbit_type: i32,
    table_limit: u32,
) {
    let mut c: u32;
    c = 0;
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

pub(crate) unsafe fn study_char_list(
    code: PCRE2_SPTR,
    start_bitmap: *mut u8,
    char_lists_end: *const u8,
) {
    let mut code: PCRE2_SPTR = code;
    let mut type_: u32;
    let mut list_ind: u32;
    let mut char_list_add: u32 = XCL_CHAR_LIST_LOW_16_ADD;
    let mut range_start: u32 = !(0 as u32);
    let mut range_end: u32 = 0;
    let mut next_char: *const u8;
    let mut start_buffer: [PCRE2_UCHAR; 6] = [0; 6];
    let mut end_buffer: [PCRE2_UCHAR; 6] = [0; 6];
    let mut start: PCRE2_UCHAR;
    let mut end: PCRE2_UCHAR;

    /* Only needed in 8-bit mode at the moment. */
    type_ = ((*code.add(0) as u32) << 8) | (*code.add(1) as u32);
    code = code.add(2);

    /* Align characters. */
    next_char = char_lists_end.sub((GET!(code, 0) << 1) as usize);
    type_ &= XCL_TYPE_MASK;
    list_ind = 0;

    if (type_ & XCL_BEGIN_WITH_RANGE) != 0 {
        range_start = XCL_CHAR_LIST_LOW_16_START;
    }

    while type_ > 0 {
        let mut item_count: u32 = type_ & XCL_ITEM_COUNT_MASK;

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

                _pcre2_ord2utf_8(range_end, end_buffer.as_mut_ptr());
                end = end_buffer[0];

                if range_start < range_end {
                    _pcre2_ord2utf_8(range_start, start_buffer.as_mut_ptr());
                    start = start_buffer[0];
                    while start <= end {
                        *start_bitmap.add((start / 8) as usize) |=
                            (1u32 << (start as u32 & 7)) as u8;
                        start = start.wrapping_add(1);
                    }
                } else {
                    *start_bitmap.add((end / 8) as usize) |= (1u32 << (end as u32 & 7)) as u8;
                }

                range_start = !(0 as u32);
            } else {
                range_start = char_list_add + (range_end >> XCL_CHAR_SHIFT);
            }

            item_count -= 1;
        }

        list_ind += 1;
        type_ >>= XCL_TYPE_BIT_LEN;

        if range_start == !(0 as u32) {
            if (type_ & XCL_BEGIN_WITH_RANGE) != 0 {
                /* In 8 bit mode XCL_CHAR_LIST_HIGH_32_START is not possible. */
                if list_ind == 1 {
                    range_start = XCL_CHAR_LIST_HIGH_16_START;
                } else {
                    range_start = XCL_CHAR_LIST_LOW_32_START;
                }
            }
        } else if (type_ & XCL_BEGIN_WITH_RANGE) == 0 {
            _pcre2_ord2utf_8(range_start, start_buffer.as_mut_ptr());

            /* In 8 bit mode XCL_CHAR_LIST_LOW_32_END and
            XCL_CHAR_LIST_HIGH_32_END are not possible. */
            if list_ind == 1 {
                range_end = XCL_CHAR_LIST_LOW_16_END;
            } else {
                range_end = XCL_CHAR_LIST_HIGH_16_END;
            }

            _pcre2_ord2utf_8(range_end, end_buffer.as_mut_ptr());
            end = end_buffer[0];

            start = start_buffer[0];
            while start <= end {
                *start_bitmap.add((start / 8) as usize) |= (1u32 << (start as u32 & 7)) as u8;
                start = start.wrapping_add(1);
            }

            range_start = !(0 as u32);
        }

        /* In 8 bit mode XCL_CHAR_LIST_HIGH_32_ADD is not possible. */
        if list_ind == 1 {
            char_list_add = XCL_CHAR_LIST_HIGH_16_ADD;
        } else {
            char_list_add = XCL_CHAR_LIST_LOW_32_ADD;
        }
    }
}

/*************************************************
*      Create bitmap of starting code units      *
*************************************************/

/* This function scans a compiled unanchored expression recursively and
attempts to build a bitmap of the set of possible starting code units whose
values are less than 256. In 16-bit and 32-bit mode, values above 255 all cause
the 255 bit to be set. When calling set[_not]_type_bits() in UTF-8 (sic) mode
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

pub(crate) unsafe fn set_start_bits(
    re: *mut pcre2_real_code,
    code: PCRE2_SPTR,
    utf: BOOL,
    ucp: BOOL,
    depthptr: *mut i32,
) -> i32 {
    let mut code: PCRE2_SPTR = code;
    let mut c: u32 = 0;
    let mut yield_: i32 = SSB_DONE;

    let table_limit: i32 = if utf != 0 { 16 } else { 32 };

    *depthptr += 1;
    if *depthptr > 1000 {
        return SSB_TOODEEP;
    }

    loop {
        let mut try_next: BOOL = TRUE;
        let mut tcode: PCRE2_SPTR = code.add(1 + LINK_SIZE);

        if (*code as u32) == OP_CBRA
            || (*code as u32) == OP_SCBRA
            || (*code as u32) == OP_CBRAPOS
            || (*code as u32) == OP_SCBRAPOS
        {
            tcode = tcode.add(IMM2_SIZE);
        }

        'try_next: while try_next != 0
        /* Loop for items in this branch */
        {
            let mut rc: i32 = 0;
            let mut ncode: PCRE2_SPTR = core::ptr::null();
            let mut classmap: *const u8 = core::ptr::null();
            let mut xclassflags: PCRE2_UCHAR = 0;

            let mut state: u32 = SB_SWITCH;
            'sm: loop {
                match state {
                    SB_SWITCH => match *tcode as u32 {
                        /* Fail for a valid opcode that implies no starting bits. */

                        OP_ACCEPT | OP_ASSERT_ACCEPT | OP_ALLANY | OP_ANY | OP_ANYBYTE
                        | OP_CIRCM | OP_CLOSE | OP_COMMIT | OP_COMMIT_ARG | OP_COND | OP_CREF
                        | OP_FALSE | OP_TRUE | OP_DNCREF | OP_DNREF | OP_DNREFI | OP_DNRREF
                        | OP_DOLL | OP_DOLLM | OP_END | OP_EOD | OP_EODN | OP_EXTUNI | OP_FAIL
                        | OP_MARK | OP_NOT | OP_NOTEXACT | OP_NOTEXACTI | OP_NOTI
                        | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_NOTMINQUERY | OP_NOTMINQUERYI
                        | OP_NOTMINSTAR | OP_NOTMINSTARI | OP_NOTMINUPTO | OP_NOTMINUPTOI
                        | OP_NOTPLUS | OP_NOTPLUSI | OP_NOTPOSPLUS | OP_NOTPOSPLUSI
                        | OP_NOTPOSQUERY | OP_NOTPOSQUERYI | OP_NOTPOSSTAR | OP_NOTPOSSTARI
                        | OP_NOTPOSUPTO | OP_NOTPOSUPTOI | OP_NOTPROP | OP_NOTQUERY
                        | OP_NOTQUERYI | OP_NOTSTAR | OP_NOTSTARI | OP_NOTUPTO | OP_NOTUPTOI
                        | OP_NOT_HSPACE | OP_NOT_VSPACE | OP_PRUNE | OP_PRUNE_ARG | OP_RECURSE
                        | OP_REF | OP_REFI | OP_REVERSE | OP_VREVERSE | OP_RREF | OP_SCOND
                        | OP_SET_SOM | OP_SKIP | OP_SKIP_ARG | OP_SOD | OP_SOM | OP_THEN
                        | OP_THEN_ARG => {
                            return SSB_FAIL;
                        }

                        /* OP_CIRC happens only at the start of an anchored branch (multiline ^
                        uses OP_CIRCM). Skip over it. */

                        OP_CIRC => {
                            tcode = tcode.add(OP_LENGTHS!(OP_CIRC));
                            break 'sm;
                        }

                        /* A "real" property test implies no starting bits, but the fake property
                        PT_CLIST identifies a list of characters. These lists are short, as they
                        are used for characters with more than one "other case", so there is no
                        point in recognizing them for OP_NOTPROP. */

                        OP_PROP => {
                            if (*tcode.add(1) as u32) != PT_CLIST {
                                return SSB_FAIL;
                            }
                            {
                                let mut p: *const u32 = _pcre2_ucd_caseless_sets_8
                                    .as_ptr()
                                    .add(*tcode.add(2) as usize);
                                loop {
                                    c = {
                                        let t_ = *p;
                                        p = p.add(1);
                                        t_
                                    };
                                    if !(c < NOTACHAR) {
                                        break;
                                    }
                                    if utf != 0 {
                                        let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
                                        _pcre2_ord2utf_8(c, buff.as_mut_ptr());
                                        c = buff[0] as u32;
                                    }
                                    if c > 0xff {
                                        SET_BIT!(re, 0xff);
                                    } else {
                                        SET_BIT!(re, c);
                                    }
                                }
                            }
                            try_next = FALSE;
                            break 'sm;
                        }

                        /* We can ignore word boundary tests. */

                        OP_WORD_BOUNDARY | OP_NOT_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY
                        | OP_NOT_UCP_WORD_BOUNDARY => {
                            tcode = tcode.add(1);
                            break 'sm;
                        }

                        /* For a positive lookahead assertion, inspect what immediately follows,
                        ignoring intermediate assertions and callouts. If the next item is one
                        that sets a mandatory character, skip this assertion. Otherwise, treat it
                        the same as other bracket groups. */

                        OP_ASSERT | OP_ASSERT_NA => {
                            ncode = tcode.add(GET!(tcode, 1) as usize);
                            while (*ncode as u32) == OP_ALT {
                                ncode = ncode.add(GET!(ncode, 1) as usize);
                            }
                            ncode = ncode.add(1 + LINK_SIZE);

                            /* Skip irrelevant items */

                            {
                                let mut done: BOOL = FALSE;
                                while done == 0 {
                                    match *ncode as u32 {
                                        OP_ASSERT | OP_ASSERT_NOT | OP_ASSERTBACK
                                        | OP_ASSERTBACK_NOT | OP_ASSERT_NA | OP_ASSERTBACK_NA
                                        | OP_ASSERT_SCS => {
                                            ncode = ncode.add(GET!(ncode, 1) as usize);
                                            while (*ncode as u32) == OP_ALT {
                                                ncode = ncode.add(GET!(ncode, 1) as usize);
                                            }
                                            ncode = ncode.add(1 + LINK_SIZE);
                                        }

                                        OP_WORD_BOUNDARY | OP_NOT_WORD_BOUNDARY
                                        | OP_UCP_WORD_BOUNDARY | OP_NOT_UCP_WORD_BOUNDARY => {
                                            ncode = ncode.add(1);
                                        }

                                        OP_CALLOUT => {
                                            ncode = ncode.add(OP_LENGTHS!(OP_CALLOUT));
                                        }

                                        OP_CALLOUT_STR => {
                                            ncode =
                                                ncode.add(GET!(ncode, 1 + 2 * LINK_SIZE) as usize);
                                        }

                                        _ => {
                                            done = TRUE;
                                        }
                                    }
                                }
                            }

                            /* Now check the next significant item. */

                            match *ncode as u32 {
                                OP_PROP => {
                                    if (*ncode.add(1) as u32) != PT_CLIST {
                                        /* break */
                                    } else {
                                        /* Fall through */
                                        tcode = ncode;
                                        continue 'try_next; /* With the following significant opcode */
                                    }
                                }

                                OP_ANYNL | OP_CHAR | OP_CHARI | OP_EXACT | OP_EXACTI | OP_HSPACE
                                | OP_MINPLUS | OP_MINPLUSI | OP_PLUS | OP_PLUSI | OP_POSPLUS
                                | OP_POSPLUSI | OP_VSPACE
                                /* Note that these types will only be present in non-UCP mode. */
                                | OP_DIGIT | OP_NOT_DIGIT | OP_WORDCHAR | OP_NOT_WORDCHAR
                                | OP_WHITESPACE | OP_NOT_WHITESPACE => {
                                    tcode = ncode;
                                    continue 'try_next; /* With the following significant opcode */
                                }

                                _ => {}
                            }
                            /* Fall through */
                            state = SB_BRA_GROUP;
                            continue 'sm;
                        }

                        /* For a group bracket or a positive assertion without an immediately
                        following mandatory setting, recurse to set bits from within the
                        subpattern. If it can't find anything, we have to give up. If it finds
                        some mandatory character(s), we are done for this branch. Otherwise,
                        carry on scanning after the subpattern. */

                        OP_BRA | OP_SBRA | OP_CBRA | OP_SCBRA | OP_BRAPOS | OP_SBRAPOS
                        | OP_CBRAPOS | OP_SCBRAPOS | OP_ONCE | OP_SCRIPT_RUN => {
                            state = SB_BRA_GROUP;
                            continue 'sm;
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
                            break 'sm;
                        }

                        OP_KET | OP_KETRMAX | OP_KETRMIN | OP_KETRPOS => {
                            return SSB_CONTINUE;
                        }

                        /* Skip over callout */

                        OP_CALLOUT => {
                            tcode = tcode.add(OP_LENGTHS!(OP_CALLOUT));
                            break 'sm;
                        }

                        OP_CALLOUT_STR => {
                            tcode = tcode.add(GET!(tcode, 1 + 2 * LINK_SIZE) as usize);
                            break 'sm;
                        }

                        /* Skip over lookbehind, negative lookahead, and scan substring
                        assertions */

                        OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT | OP_ASSERTBACK_NA
                        | OP_ASSERT_SCS => {
                            loop {
                                tcode = tcode.add(GET!(tcode, 1) as usize);
                                if (*tcode as u32) != OP_ALT {
                                    break;
                                }
                            }
                            tcode = tcode.add(1 + LINK_SIZE);
                            break 'sm;
                        }

                        /* BRAZERO does the bracket, but carries on. */

                        OP_BRAZERO | OP_BRAMINZERO | OP_BRAPOSZERO => {
                            tcode = tcode.add(1);
                            rc = set_start_bits(re, tcode, utf, ucp, depthptr);
                            if rc == SSB_FAIL || rc == SSB_UNKNOWN || rc == SSB_TOODEEP {
                                return rc;
                            }
                            loop {
                                tcode = tcode.add(GET!(tcode, 1) as usize);
                                if (*tcode as u32) != OP_ALT {
                                    break;
                                }
                            }
                            tcode = tcode.add(1 + LINK_SIZE);
                            break 'sm;
                        }

                        /* SKIPZERO skips the bracket. */

                        OP_SKIPZERO => {
                            tcode = tcode.add(1);
                            loop {
                                tcode = tcode.add(GET!(tcode, 1) as usize);
                                if (*tcode as u32) != OP_ALT {
                                    break;
                                }
                            }
                            tcode = tcode.add(1 + LINK_SIZE);
                            break 'sm;
                        }

                        /* Single-char * or ? sets the bit and tries the next item */

                        OP_STAR | OP_MINSTAR | OP_POSSTAR | OP_QUERY | OP_MINQUERY
                        | OP_POSQUERY => {
                            tcode = set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                            break 'sm;
                        }

                        OP_STARI | OP_MINSTARI | OP_POSSTARI | OP_QUERYI | OP_MINQUERYI
                        | OP_POSQUERYI => {
                            tcode = set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                            break 'sm;
                        }

                        /* Single-char upto sets the bit and tries the next */

                        OP_UPTO | OP_MINUPTO | OP_POSUPTO => {
                            tcode = set_table_bit(re, tcode.add(1 + IMM2_SIZE), FALSE, utf, ucp);
                            break 'sm;
                        }

                        OP_UPTOI | OP_MINUPTOI | OP_POSUPTOI => {
                            tcode = set_table_bit(re, tcode.add(1 + IMM2_SIZE), TRUE, utf, ucp);
                            break 'sm;
                        }

                        /* At least one single char sets the bit and stops */

                        OP_EXACT => {
                            tcode = tcode.add(IMM2_SIZE);
                            /* Fall through */
                            set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                            try_next = FALSE;
                            break 'sm;
                        }

                        OP_CHAR | OP_PLUS | OP_MINPLUS | OP_POSPLUS => {
                            set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                            try_next = FALSE;
                            break 'sm;
                        }

                        OP_EXACTI => {
                            tcode = tcode.add(IMM2_SIZE);
                            /* Fall through */
                            set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                            try_next = FALSE;
                            break 'sm;
                        }

                        OP_CHARI | OP_PLUSI | OP_MINPLUSI | OP_POSPLUSI => {
                            set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                            try_next = FALSE;
                            break 'sm;
                        }

                        /* Special spacing and line-terminating items. These recognize specific
                        lists of characters. The difference between VSPACE and ANYNL is that the
                        latter can match the two-character CRLF sequence, but that is not
                        relevant for finding the first character, so their code here is
                        identical. */

                        OP_HSPACE => {
                            SET_BIT!(re, 0x09); /* CHAR_HT */
                            SET_BIT!(re, 0x20); /* CHAR_SPACE */

                            /* For the 8-bit library in UTF-8 mode, set the bits for the first code
                            units of horizontal space characters. */

                            if utf != 0 {
                                SET_BIT!(re, 0xC2); /* For U+00A0 */
                                SET_BIT!(re, 0xE1); /* For U+1680, U+180E */
                                SET_BIT!(re, 0xE2); /* For U+2000 - U+200A, U+202F, U+205F */
                                SET_BIT!(re, 0xE3); /* For U+3000 */
                            }
                            /* For the 8-bit library not in UTF-8 mode, set the bit for NBSP. */
                            else {
                                SET_BIT!(re, 0xA0); /* CHAR_NBSP */
                            }

                            try_next = FALSE;
                            break 'sm;
                        }

                        OP_ANYNL | OP_VSPACE => {
                            SET_BIT!(re, 0x0A); /* CHAR_LF */
                            SET_BIT!(re, 0x0B); /* CHAR_VT */
                            SET_BIT!(re, 0x0C); /* CHAR_FF */
                            SET_BIT!(re, 0x0D); /* CHAR_CR */

                            /* For the 8-bit library in UTF-8 mode, set the bits for the first code
                            units of vertical space characters. */

                            if utf != 0 {
                                SET_BIT!(re, 0xC2); /* For U+0085 (NEL) */
                                SET_BIT!(re, 0xE2); /* For U+2028, U+2029 */
                            }
                            /* For the 8-bit library not in UTF-8 mode, set the bit for NEL. */
                            else {
                                SET_BIT!(re, 0x85); /* CHAR_NEL */
                            }

                            try_next = FALSE;
                            break 'sm;
                        }

                        /* Single character types set the bits and stop. Note that if PCRE2_UCP
                        is set, we do not see these opcodes because \d etc are converted to
                        properties. Therefore, these apply in the case when only characters less
                        than 256 are recognized to match the types. */

                        OP_NOT_DIGIT => {
                            set_nottype_bits(re, cbit_digit as i32, table_limit as u32);
                            try_next = FALSE;
                            break 'sm;
                        }

                        OP_DIGIT => {
                            set_type_bits(re, cbit_digit as i32, table_limit as u32);
                            try_next = FALSE;
                            break 'sm;
                        }

                        OP_NOT_WHITESPACE => {
                            set_nottype_bits(re, cbit_space as i32, table_limit as u32);
                            try_next = FALSE;
                            break 'sm;
                        }

                        OP_WHITESPACE => {
                            set_type_bits(re, cbit_space as i32, table_limit as u32);
                            try_next = FALSE;
                            break 'sm;
                        }

                        OP_NOT_WORDCHAR => {
                            set_nottype_bits(re, cbit_word as i32, table_limit as u32);
                            try_next = FALSE;
                            break 'sm;
                        }

                        OP_WORDCHAR => {
                            set_type_bits(re, cbit_word as i32, table_limit as u32);
                            try_next = FALSE;
                            break 'sm;
                        }

                        /* One or more character type fudges the pointer and restarts, knowing
                        it will hit a single character type and stop there. */

                        OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEPOSPLUS => {
                            tcode = tcode.add(1);
                            break 'sm;
                        }

                        OP_TYPEEXACT => {
                            tcode = tcode.add(1 + IMM2_SIZE);
                            break 'sm;
                        }

                        /* Zero or more repeats of character types set the bits and then
                        try again. */

                        OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO => {
                            tcode = tcode.add(IMM2_SIZE);
                            /* Fall through */
                            state = SB_TYPESTAR;
                            continue 'sm;
                        }

                        OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPOSSTAR | OP_TYPEQUERY
                        | OP_TYPEMINQUERY | OP_TYPEPOSQUERY => {
                            state = SB_TYPESTAR;
                            continue 'sm;
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

                        OP_XCLASS => {
                            xclassflags = *tcode.add(1 + LINK_SIZE);
                            if (xclassflags as u32 & XCL_HASPROP) != 0
                                || (xclassflags as u32 & (XCL_MAP | XCL_NOT)) == XCL_NOT
                            {
                                return SSB_FAIL;
                            }

                            /* We have a positive XCLASS or a negative one without a map. Set up the
                            map pointer if there is one, and fall through. */

                            classmap = if (xclassflags as u32 & XCL_MAP) == 0 {
                                core::ptr::null()
                            } else {
                                tcode.add(1 + LINK_SIZE + 1)
                            };

                            /* In UTF-8 mode, scan the character list and set bits for leading bytes,
                            then jump to handle the map. */

                            if utf != 0 && (xclassflags as u32 & XCL_NOT) == 0 {
                                let mut b: PCRE2_UCHAR;
                                let mut e: PCRE2_UCHAR;
                                let mut p: PCRE2_SPTR = tcode.add(
                                    1 + LINK_SIZE
                                        + 1
                                        + if classmap.is_null() { 0 } else { 32 },
                                );
                                tcode = tcode.add(GET!(tcode, 1) as usize);

                                if (*p as u32) >= XCL_LIST {
                                    study_char_list(
                                        p,
                                        (*re).start_bitmap.as_mut_ptr(),
                                        (re as *const u8).add((*re).code_start),
                                    );
                                    /* goto HANDLE_CLASSMAP */
                                    state = SB_HANDLE_CLASSMAP;
                                    continue 'sm;
                                }

                                loop {
                                    let v_ = {
                                        let t_ = *p;
                                        p = p.add(1);
                                        t_
                                    };
                                    match v_ as u32 {
                                        XCL_SINGLE => {
                                            b = {
                                                let t_ = *p;
                                                p = p.add(1);
                                                t_
                                            };
                                            while (*p & 0xc0) == 0x80 {
                                                p = p.add(1);
                                            }
                                            (*re).start_bitmap[(b / 8) as usize] |=
                                                (1u32 << (b as u32 & 7)) as u8;
                                        }

                                        XCL_RANGE => {
                                            b = {
                                                let t_ = *p;
                                                p = p.add(1);
                                                t_
                                            };
                                            while (*p & 0xc0) == 0x80 {
                                                p = p.add(1);
                                            }
                                            e = {
                                                let t_ = *p;
                                                p = p.add(1);
                                                t_
                                            };
                                            while (*p & 0xc0) == 0x80 {
                                                p = p.add(1);
                                            }
                                            while b <= e {
                                                (*re).start_bitmap[(b / 8) as usize] |=
                                                    (1u32 << (b as u32 & 7)) as u8;
                                                b = b.wrapping_add(1);
                                            }
                                        }

                                        XCL_END => {
                                            /* goto HANDLE_CLASSMAP */
                                            state = SB_HANDLE_CLASSMAP;
                                            continue 'sm;
                                        }

                                        _ => {
                                            /* PCRE2_DEBUG_UNREACHABLE */
                                            return SSB_UNKNOWN; /* Internal error, should not occur */
                                        }
                                    }
                                }
                            }

                            /* Fall through */
                            state = SB_NCLASS;
                            continue 'sm;
                        }

                        /* Enter here for a negative non-XCLASS. In the 8-bit library, if we are
                        in UTF mode, any byte with a value >= 0xc4 is a potentially valid starter
                        because it starts a character with a value > 255. In 8-bit non-UTF mode,
                        there is no difference between CLASS and NCLASS. In all other wide
                        character modes, set the 0xFF bit to indicate code units >= 255. */

                        OP_NCLASS => {
                            state = SB_NCLASS;
                            continue 'sm;
                        }

                        /* Enter here for a positive non-XCLASS. If we have fallen through from
                        an XCLASS, classmap will already be set; just advance the code pointer.
                        Otherwise, set up classmap for a non-XCLASS and advance past it. */

                        OP_CLASS => {
                            state = SB_CLASS;
                            continue 'sm;
                        }

                        /* If we reach something we don't understand, it means a new opcode has
                        been created that hasn't been added to this function. Hopefully this
                        problem will be discovered during testing. */

                        _ => {
                            return SSB_UNKNOWN;
                        }
                    },

                    SB_BRA_GROUP => {
                        rc = set_start_bits(re, tcode, utf, ucp, depthptr);
                        if rc == SSB_DONE {
                            try_next = FALSE;
                        } else if rc == SSB_CONTINUE {
                            loop {
                                tcode = tcode.add(GET!(tcode, 1) as usize);
                                if (*tcode as u32) != OP_ALT {
                                    break;
                                }
                            }
                            tcode = tcode.add(1 + LINK_SIZE);
                        } else {
                            return rc; /* FAIL, UNKNOWN, or TOODEEP */
                        }
                        break 'sm;
                    }

                    SB_TYPESTAR => {
                        match *tcode.add(1) as u32 {
                            OP_HSPACE => {
                                SET_BIT!(re, 0x09); /* CHAR_HT */
                                SET_BIT!(re, 0x20); /* CHAR_SPACE */

                                /* For the 8-bit library in UTF-8 mode, set the bits for the first
                                code units of horizontal space characters. */

                                if utf != 0 {
                                    SET_BIT!(re, 0xC2); /* For U+00A0 */
                                    SET_BIT!(re, 0xE1); /* For U+1680, U+180E */
                                    SET_BIT!(re, 0xE2); /* For U+2000 - U+200A, U+202F, U+205F */
                                    SET_BIT!(re, 0xE3); /* For U+3000 */
                                }
                                /* For the 8-bit library not in UTF-8 mode, set the bit for NBSP. */
                                else {
                                    SET_BIT!(re, 0xA0); /* CHAR_NBSP */
                                }
                            }

                            OP_ANYNL | OP_VSPACE => {
                                SET_BIT!(re, 0x0A); /* CHAR_LF */
                                SET_BIT!(re, 0x0B); /* CHAR_VT */
                                SET_BIT!(re, 0x0C); /* CHAR_FF */
                                SET_BIT!(re, 0x0D); /* CHAR_CR */

                                /* For the 8-bit library in UTF-8 mode, set the bits for the first
                                code units of vertical space characters. */

                                if utf != 0 {
                                    SET_BIT!(re, 0xC2); /* For U+0085 (NEL) */
                                    SET_BIT!(re, 0xE2); /* For U+2028, U+2029 */
                                }
                                /* For the 8-bit library not in UTF-8 mode, set the bit for NEL. */
                                else {
                                    SET_BIT!(re, 0x85); /* CHAR_NEL */
                                }
                            }

                            OP_NOT_DIGIT => {
                                set_nottype_bits(re, cbit_digit as i32, table_limit as u32);
                            }

                            OP_DIGIT => {
                                set_type_bits(re, cbit_digit as i32, table_limit as u32);
                            }

                            OP_NOT_WHITESPACE => {
                                set_nottype_bits(re, cbit_space as i32, table_limit as u32);
                            }

                            OP_WHITESPACE => {
                                set_type_bits(re, cbit_space as i32, table_limit as u32);
                            }

                            OP_NOT_WORDCHAR => {
                                set_nottype_bits(re, cbit_word as i32, table_limit as u32);
                            }

                            OP_WORDCHAR => {
                                set_type_bits(re, cbit_word as i32, table_limit as u32);
                            }

                            /* default, OP_ANY and OP_ALLANY */
                            _ => {
                                return SSB_FAIL;
                            }
                        }

                        tcode = tcode.add(2);
                        break 'sm;
                    }

                    SB_NCLASS => {
                        if utf != 0 {
                            (*re).start_bitmap[24] |= 0xf0; /* Bits for 0xc4 - 0xc8 */
                            core::ptr::write_bytes(
                                (*re).start_bitmap.as_mut_ptr().add(25),
                                0xff,
                                7,
                            ); /* Bits for 0xc9 - 0xff */
                        }
                        /* Fall through */
                        state = SB_CLASS;
                        continue 'sm;
                    }

                    SB_CLASS => {
                        if (*tcode as u32) == OP_XCLASS {
                            tcode = tcode.add(GET!(tcode, 1) as usize);
                        } else {
                            tcode = tcode.add(1);
                            classmap = tcode;
                            tcode = tcode.add(32);
                        }
                        /* Fall through to HANDLE_CLASSMAP */
                        state = SB_HANDLE_CLASSMAP;
                        continue 'sm;
                    }

                    SB_HANDLE_CLASSMAP => {
                        /* When wide characters are supported, classmap may be NULL. In UTF-8
                        (sic) mode, the bits in a class bit map correspond to character values,
                        not to byte values. However, the bit map we are constructing is for byte
                        values. So we have to do a conversion for characters whose code point is
                        greater than 127. In fact, there are only two possible starting bytes for
                        characters in the range 128 - 255. */

                        if !classmap.is_null() {
                            if utf != 0 {
                                c = 0;
                                while c < 16 {
                                    (*re).start_bitmap[c as usize] |= *classmap.add(c as usize);
                                    c += 1;
                                }
                                c = 128;
                                while c < 256 {
                                    if ((*classmap.add((c / 8) as usize) as u32)
                                        & (1u32 << (c & 7)))
                                        != 0
                                    {
                                        let d: i32 = ((c >> 6) | 0xc0) as i32; /* Set bit for this starter */
                                        (*re).start_bitmap[(d / 8) as usize] |=
                                            (1u32 << (d as u32 & 7)) as u8; /* and then skip on to the */
                                        c = (c & 0xc0) + 0x40 - 1; /* next relevant character. */
                                    }
                                    c += 1;
                                }
                            }
                            /* In all modes except UTF-8, the two bit maps are compatible. */
                            else {
                                c = 0;
                                while c < 32 {
                                    (*re).start_bitmap[c as usize] |= *classmap.add(c as usize);
                                    c += 1;
                                }
                            }
                        }

                        /* Act on what follows the class. For a zero minimum repeat, continue;
                        otherwise stop processing. */

                        match *tcode as u32 {
                            OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY
                            | OP_CRPOSSTAR | OP_CRPOSQUERY => {
                                tcode = tcode.add(1);
                            }

                            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                if GET2!(tcode, 1) == 0 {
                                    tcode = tcode.add(1 + 2 * IMM2_SIZE);
                                } else {
                                    try_next = FALSE;
                                }
                            }

                            _ => {
                                try_next = FALSE;
                            }
                        }
                        break 'sm; /* End of class handling case */
                    }

                    _ => {
                        break 'sm;
                    }
                }
            }
        } /* End of try_next loop */

        code = code.add(GET!(code, 1) as usize); /* Advance to next branch */
        if (*code as u32) != OP_ALT {
            break;
        }
    }

    yield_
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_study_8(re: *mut pcre2_real_code) -> i32 {
    let mut count: i32 = 0;
    let code: *mut PCRE2_UCHAR;
    let utf: BOOL = (((*re).overall_options & PCRE2_UTF) != 0) as BOOL;
    let ucp: BOOL = (((*re).overall_options & PCRE2_UCP) != 0) as BOOL;

    /* Find start of compiled code */

    code = (re as *mut u8).add((*re).code_start) as *mut PCRE2_UCHAR;

    /* For a pattern that has a first code unit, or a multiline pattern that
    matches only at "line start", there is no point in seeking a list of starting
    code units. */

    if ((*re).flags & (PCRE2_FIRSTSET | PCRE2_STARTLINE)) == 0 {
        let mut depth: i32 = 0;
        let rc: i32 = set_start_bits(re, code as PCRE2_SPTR, utf, ucp, &mut depth);
        if rc == SSB_UNKNOWN {
            /* PCRE2_DEBUG_UNREACHABLE */
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
            let mut i: i32;
            let mut a: i32 = -1;
            let mut b: i32 = -1;
            let mut p: *mut u8 = (*re).start_bitmap.as_mut_ptr();
            let mut flags: u32 = PCRE2_FIRSTMAPSET;

            'DONE: {
                i = 0;
                while i < 256 {
                    let x: u8 = *p;
                    if x != 0 {
                        let mut c: i32;
                        let y: u8 = ((x as u32) & ((!(x as u32)).wrapping_add(1))) as u8; /* Least significant bit */
                        if y != x {
                            break 'DONE; /* More than one bit set */
                        }

                        /* Compute the character value */

                        c = i;
                        match x {
                            1 => {}
                            2 => c += 1,
                            4 => c += 2,
                            8 => c += 3,
                            16 => c += 4,
                            32 => c += 5,
                            64 => c += 6,
                            128 => c += 7,
                            _ => {}
                        }

                        /* c contains the code unit value, in the range 0-255. In 8-bit UTF
                        mode, only values < 128 can be used. In all the other cases, c is a
                        character value. */

                        if utf != 0 && c > 127 {
                            break 'DONE;
                        }

                        if a < 0 {
                            a = c; /* First one found, save in a */
                        } else if b < 0
                        /* Second one found */
                        {
                            let mut d: i32 =
                                TABLE_GET!(c as u32, (*re).tables.add(fcc_offset), c) as i32;

                            if utf != 0 || ucp != 0 {
                                if UCD_CASESET!(c) != 0 {
                                    break 'DONE; /* Multiple case set */
                                }
                                if c > 127 {
                                    d = UCD_OTHERCASE!(c) as i32;
                                }
                            }

                            if d != a {
                                break 'DONE; /* Not the other case of a */
                            }
                            b = c; /* Save second in b */
                        } else {
                            break 'DONE; /* More than two characters found */
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
            }

            /* DONE: */
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
        && ((*re).top_backref as i32) <= MAX_CACHE_BACKREF as i32
    {
        let min: i32;
        let mut backref_cache: [i32; MAX_CACHE_BACKREF + 1] = [0; MAX_CACHE_BACKREF + 1];
        backref_cache[0] = 0; /* Highest one that is set */
        min = find_minlength(
            re,
            code as PCRE2_SPTR,
            code as PCRE2_SPTR,
            utf,
            core::ptr::null_mut(),
            &mut count,
            backref_cache.as_mut_ptr(),
        );
        match min {
            -1 =>
            /* \C in UTF mode or over-complex regex */
            {
                /* Leave minlength unchanged (will be zero) */
            }

            -2 => {
                /* PCRE2_DEBUG_UNREACHABLE */
                return 2; /* missing capturing bracket */
            }

            -3 => {
                /* PCRE2_DEBUG_UNREACHABLE */
                return 3; /* unrecognized opcode */
            }

            _ => {
                (*re).minlength = if min > UINT16_MAX {
                    UINT16_MAX as u16
                } else {
                    min as u16
                };
            }
        }
    }

    0
}

/* End of pcre2_study.c */
