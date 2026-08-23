/* Translated from pcre2_study.c
   8-bit code units, SUPPORT_UNICODE, SUPPORT_WIDE_CHARS, no JIT, LINK_SIZE == 2. */

#![allow(non_snake_case, non_upper_case_globals, unused_assignments, unused_mut)]

use crate::internal::*;
use crate::pcre2_pub::*;
use crate::tables::*;
use crate::ucd_data::*;
use core::ffi::{c_int, c_uint, c_void};

/* The maximum remembered capturing brackets minimum. */

const MAX_CACHE_BACKREF: usize = 128;

/* Returns from set_start_bits() */

const SSB_FAIL: c_int = 0;
const SSB_DONE: c_int = 1;
const SSB_CONTINUE: c_int = 2;
const SSB_UNKNOWN: c_int = 3;
const SSB_TOODEEP: c_int = 4;

const UINT16_MAX: c_int = 65535;
const INT_MAX: c_int = c_int::MAX;

/* Set a bit in the starting code unit bit map. */

#[inline(always)]
unsafe fn SET_BIT(re: *mut pcre2_real_code, c: u32) {
    (*re).start_bitmap[(c / 8) as usize] |= (1u32 << (c & 7)) as u8;
}

/*************************************************
*   Find the minimum subject length for a group  *
*************************************************/

/* Scan a parenthesized group and compute the minimum length of subject that
is needed to match it. This is a lower bound; it does not mean there is a
string of that length that matches. In UTF mode, the result is in characters
rather than code units.

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
    recurses: *mut recurse_check,
    countptr: *mut c_int,
    backref_cache: *mut c_int,
) -> c_int {
    let mut length: c_int = -1;
    let mut branchlength: c_int = 0;
    let mut prev_cap_recno: c_int = -1;
    let mut prev_cap_d: c_int = 0;
    let mut prev_recurse_recno: c_int = -1;
    let mut prev_recurse_d: c_int = 0;
    let mut once_fudge: u32 = 0;
    let mut had_recurse: BOOL = FALSE;
    let dupcapused: BOOL = if ((*re).flags & PCRE2_DUPCAPUSED) != 0 {
        TRUE
    } else {
        FALSE
    };
    let mut nextbranch: PCRE2_SPTR = code.add(GET(code, 1) as usize);
    let mut cc: PCRE2_SPTR = code.add(1 + LINK_SIZE);
    let mut this_recurse: recurse_check = recurse_check {
        prev: core::ptr::null_mut(),
        group: core::ptr::null(),
    };

    /* If this is a "could be empty" group, its minimum length is 0. */

    if *code as u32 >= OP_SBRA && *code as u32 <= OP_SCOND {
        return 0;
    }

    /* Skip over capturing bracket number */

    if *code as u32 == OP_CBRA || *code as u32 == OP_CBRAPOS {
        cc = cc.add(IMM2_SIZE);
    }

    /* A large and/or complex regex can take too long to process. */

    {
        let oldcount = *countptr;
        *countptr = oldcount + 1;
        if oldcount > 1000 {
            return -1;
        }
    }

    /* Scan along the opcodes for this branch. If we get to the end of the branch,
    check the length against that of the other branches. If the accumulated length
    passes 16-bits, reset to that value and skip the rest of the branch. */

    loop {
        let mut d: c_int = 0;
        let mut min: c_int = 0;
        let mut recno: c_int = 0;
        let op: u32;
        let mut cs: PCRE2_SPTR = core::ptr::null();
        let mut ce: PCRE2_SPTR = core::ptr::null();

        if branchlength >= UINT16_MAX {
            branchlength = UINT16_MAX;
            cc = nextbranch;
        }

        op = *cc as u32;

        'sw: {
            'process_non_capture: {
                'repeat_back_reference: {
                    match op {
                        OP_COND | OP_SCOND => {
                            /* If there is only one branch in a condition, the implied branch has
                            zero length, so we don't add anything. This covers the DEFINE
                            "condition" automatically. If there are two branches we can treat it
                            the same as any other non-capturing subpattern. */

                            cs = cc.add(GET(cc, 1) as usize);
                            if *cs as u32 != OP_ALT {
                                cc = cs.add(1 + LINK_SIZE);
                                break 'sw;
                            }
                            break 'process_non_capture; /* goto PROCESS_NON_CAPTURE */
                        }

                        OP_BRA => {
                            /* There's a special case of OP_BRA, when it is wrapped round a
                            repeated OP_RECURSE. We'd like to process the latter at this level so
                            that remembering the value works for repeated cases. So we do nothing,
                            but set a fudge value to skip over the OP_KET after the recurse. */

                            if *cc.add(1 + LINK_SIZE) as u32 == OP_RECURSE
                                && *cc.add(2 * (1 + LINK_SIZE)) as u32 == OP_KET
                            {
                                once_fudge = (1 + LINK_SIZE) as u32;
                                cc = cc.add(1 + LINK_SIZE);
                                break 'sw;
                            }
                            /* Fall through */
                            break 'process_non_capture;
                        }

                        OP_ONCE | OP_SCRIPT_RUN | OP_SBRA | OP_BRAPOS | OP_SBRAPOS => {
                            break 'process_non_capture;
                        }

                        /* To save time for repeated capturing subpatterns, we remember the
                        length of the previous one. */
                        OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS => {
                            recno = GET2(cc, 1 + LINK_SIZE) as c_int;
                            if dupcapused != FALSE || recno != prev_cap_recno {
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
                                cc = cc.add(GET(cc, 1) as usize);
                                if *cc as u32 != OP_ALT {
                                    break;
                                }
                            }
                            cc = cc.add(1 + LINK_SIZE);
                        }

                        /* ACCEPT makes things far too complicated; we have to give up. */
                        OP_ACCEPT | OP_ASSERT_ACCEPT => {
                            return -1;
                        }

                        /* Reached end of a branch. */
                        OP_ALT | OP_KET | OP_KETRMAX | OP_KETRMIN | OP_KETRPOS | OP_END => {
                            if length < 0 || (had_recurse == FALSE && branchlength < length) {
                                length = branchlength;
                            }
                            if op != OP_ALT || length == 0 {
                                return length;
                            }
                            nextbranch = cc.add(GET(cc, 1) as usize);
                            cc = cc.add(1 + LINK_SIZE);
                            branchlength = 0;
                            had_recurse = FALSE;
                        }

                        /* Skip over assertive subpatterns */
                        OP_ASSERT | OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT
                        | OP_ASSERT_NA | OP_ASSERT_SCS | OP_ASSERTBACK_NA => {
                            loop {
                                cc = cc.add(GET(cc, 1) as usize);
                                if *cc as u32 != OP_ALT {
                                    break;
                                }
                            }
                            /* Fall through */
                            cc = cc.add(_pcre2_OP_lengths_8[*cc as usize] as usize);
                        }

                        /* Skip over things that don't match chars */
                        OP_REVERSE | OP_VREVERSE | OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF
                        | OP_FALSE | OP_TRUE | OP_CALLOUT | OP_SOD | OP_SOM | OP_EOD | OP_EODN
                        | OP_CIRC | OP_CIRCM | OP_DOLL | OP_DOLLM | OP_NOT_WORD_BOUNDARY
                        | OP_WORD_BOUNDARY | OP_NOT_UCP_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY => {
                            cc = cc.add(_pcre2_OP_lengths_8[*cc as usize] as usize);
                        }

                        OP_CALLOUT_STR => {
                            cc = cc.add(GET(cc, 1 + 2 * LINK_SIZE) as usize);
                        }

                        /* Skip over a subpattern that has a {0} or {0,x} quantifier */
                        OP_BRAZERO | OP_BRAMINZERO | OP_BRAPOSZERO | OP_SKIPZERO => {
                            cc = cc.add(_pcre2_OP_lengths_8[*cc as usize] as usize);
                            loop {
                                cc = cc.add(GET(cc, 1) as usize);
                                if *cc as u32 != OP_ALT {
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
                            if utf != FALSE && HAS_EXTRALEN(*cc.offset(-1) as u32) {
                                cc = cc.add(GET_EXTRALEN(*cc.offset(-1) as u32) as usize);
                            }
                        }

                        OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEPOSPLUS => {
                            branchlength += 1;
                            cc = cc.add(
                                if *cc.add(1) as u32 == OP_PROP || *cc.add(1) as u32 == OP_NOTPROP {
                                    4
                                } else {
                                    2
                                },
                            );
                        }

                        /* Handle exact repetitions. */
                        OP_EXACT | OP_EXACTI | OP_NOTEXACT | OP_NOTEXACTI => {
                            branchlength += GET2(cc, 1) as c_int;
                            cc = cc.add(2 + IMM2_SIZE);
                            if utf != FALSE && HAS_EXTRALEN(*cc.offset(-1) as u32) {
                                cc = cc.add(GET_EXTRALEN(*cc.offset(-1) as u32) as usize);
                            }
                        }

                        OP_TYPEEXACT => {
                            branchlength += GET2(cc, 1) as c_int;
                            cc = cc.add(
                                2 + IMM2_SIZE
                                    + if *cc.add(1 + IMM2_SIZE) as u32 == OP_PROP
                                        || *cc.add(1 + IMM2_SIZE) as u32 == OP_NOTPROP
                                    {
                                        2
                                    } else {
                                        0
                                    },
                            );
                        }

                        /* Handle single-char non-literal matchers */
                        OP_PROP | OP_NOTPROP => {
                            cc = cc.add(2);
                            /* Fall through */
                            branchlength += 1;
                            cc = cc.add(1);
                        }

                        OP_NOT_DIGIT | OP_DIGIT | OP_NOT_WHITESPACE | OP_WHITESPACE
                        | OP_NOT_WORDCHAR | OP_WORDCHAR | OP_ANY | OP_ALLANY | OP_EXTUNI
                        | OP_HSPACE | OP_NOT_HSPACE | OP_VSPACE | OP_NOT_VSPACE => {
                            branchlength += 1;
                            cc = cc.add(1);
                        }

                        /* "Any newline" might match two characters, but it also might match
                        just one. */
                        OP_ANYNL => {
                            branchlength += 1;
                            cc = cc.add(1);
                        }

                        /* The single-byte matcher means we can't proceed in UTF mode. */
                        OP_ANYBYTE => {
                            if utf != FALSE {
                                return -1;
                            }
                            branchlength += 1;
                            cc = cc.add(1);
                        }

                        /* For repeated character types, we have to test for \p and \P. */
                        OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEQUERY | OP_TYPEMINQUERY
                        | OP_TYPEPOSSTAR | OP_TYPEPOSQUERY => {
                            if *cc.add(1) as u32 == OP_PROP || *cc.add(1) as u32 == OP_NOTPROP {
                                cc = cc.add(2);
                            }
                            cc = cc.add(_pcre2_OP_lengths_8[op as usize] as usize);
                        }

                        OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO => {
                            if *cc.add(1 + IMM2_SIZE) as u32 == OP_PROP
                                || *cc.add(1 + IMM2_SIZE) as u32 == OP_NOTPROP
                            {
                                cc = cc.add(2);
                            }
                            cc = cc.add(_pcre2_OP_lengths_8[op as usize] as usize);
                        }

                        /* Check a class for variable quantification */
                        OP_CLASS | OP_NCLASS | OP_XCLASS | OP_ECLASS => {
                            /* The original code caused an unsigned overflow in 64 bit systems,
                            so now we use a conditional statement. */
                            if op == OP_XCLASS || op == OP_ECLASS {
                                cc = cc.add(GET(cc, 1) as usize);
                            } else {
                                cc = cc.add(_pcre2_OP_lengths_8[OP_CLASS as usize] as usize);
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
                                    branchlength += GET2(cc, 1) as c_int;
                                    cc = cc.add(1 + 2 * IMM2_SIZE);
                                }

                                _ => {
                                    branchlength += 1;
                                }
                            }
                        }

                        /* Duplicate named pattern back reference. */
                        OP_DNREF | OP_DNREFI => {
                            if dupcapused == FALSE
                                && ((*re).overall_options & PCRE2_MATCH_UNSET_BACKREF) == 0
                            {
                                let mut count: c_int = GET2(cc, 1 + IMM2_SIZE) as c_int;
                                let mut slot: PCRE2_SPTR = ((re as *const u8)
                                    .add(core::mem::size_of::<pcre2_real_code>()))
                                    as PCRE2_SPTR;
                                slot = slot.add(
                                    (GET2(cc, 1) as usize) * ((*re).name_entry_size as usize),
                                );

                                d = INT_MAX;

                                /* Scan all groups with the same name; find the shortest. */

                                loop {
                                    let oldcount = count;
                                    count -= 1;
                                    if !(oldcount > 0) {
                                        break;
                                    }

                                    let mut dd: c_int;
                                    let mut i: c_int;
                                    recno = GET2(slot, 0) as c_int;

                                    if recno <= *backref_cache.offset(0)
                                        && *backref_cache.offset(recno as isize) >= 0
                                    {
                                        dd = *backref_cache.offset(recno as isize);
                                    } else {
                                        cs = crate::find_bracket::_pcre2_find_bracket_8(
                                            startcode, utf, recno,
                                        );
                                        ce = cs;
                                        if cs.is_null() {
                                            return -2;
                                        }
                                        loop {
                                            ce = ce.add(GET(ce, 1) as usize);
                                            if *ce as u32 != OP_ALT {
                                                break;
                                            }
                                        }

                                        dd = 0;
                                        if dupcapused == FALSE
                                            || crate::find_bracket::_pcre2_find_bracket_8(
                                                ce, utf, recno,
                                            )
                                            .is_null()
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

                                        *backref_cache.offset(recno as isize) = dd;
                                        i = *backref_cache.offset(0) + 1;
                                        while i < recno {
                                            *backref_cache.offset(i as isize) = -1;
                                            i += 1;
                                        }
                                        *backref_cache.offset(0) = recno;
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
                            cc = cc.add(_pcre2_OP_lengths_8[*cc as usize] as usize);
                            break 'repeat_back_reference; /* goto REPEAT_BACK_REFERENCE */
                        }

                        /* Single back reference by number. */
                        OP_REF | OP_REFI => {
                            recno = GET2(cc, 1) as c_int;
                            if recno <= *backref_cache.offset(0)
                                && *backref_cache.offset(recno as isize) >= 0
                            {
                                d = *backref_cache.offset(recno as isize);
                            } else {
                                let mut i: c_int;
                                d = 0;

                                if ((*re).overall_options & PCRE2_MATCH_UNSET_BACKREF) == 0 {
                                    cs = crate::find_bracket::_pcre2_find_bracket_8(
                                        startcode, utf, recno,
                                    );
                                    ce = cs;
                                    if cs.is_null() {
                                        return -2;
                                    }
                                    loop {
                                        ce = ce.add(GET(ce, 1) as usize);
                                        if *ce as u32 != OP_ALT {
                                            break;
                                        }
                                    }

                                    if dupcapused == FALSE
                                        || crate::find_bracket::_pcre2_find_bracket_8(
                                            ce, utf, recno,
                                        )
                                        .is_null()
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

                                *backref_cache.offset(recno as isize) = d;
                                i = *backref_cache.offset(0) + 1;
                                while i < recno {
                                    *backref_cache.offset(i as isize) = -1;
                                    i += 1;
                                }
                                *backref_cache.offset(0) = recno;
                            }

                            cc = cc.add(_pcre2_OP_lengths_8[*cc as usize] as usize);
                            break 'repeat_back_reference;
                        }

                        /* Recursion always refers to the first occurrence of a subpattern with
                        a given number. */
                        OP_RECURSE => {
                            cs = startcode.add(GET(cc, 1) as usize);
                            ce = cs;
                            recno = GET2(cs, 1 + LINK_SIZE) as c_int;
                            if recno == prev_recurse_recno {
                                branchlength += prev_recurse_d;
                            } else {
                                loop {
                                    ce = ce.add(GET(ce, 1) as usize);
                                    if *ce as u32 != OP_ALT {
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
                        }

                        /* Anything else does not or need not match a character. */
                        OP_UPTO | OP_UPTOI | OP_NOTUPTO | OP_NOTUPTOI | OP_MINUPTO
                        | OP_MINUPTOI | OP_NOTMINUPTO | OP_NOTMINUPTOI | OP_POSUPTO
                        | OP_POSUPTOI | OP_NOTPOSUPTO | OP_NOTPOSUPTOI | OP_STAR | OP_STARI
                        | OP_NOTSTAR | OP_NOTSTARI | OP_MINSTAR | OP_MINSTARI | OP_NOTMINSTAR
                        | OP_NOTMINSTARI | OP_POSSTAR | OP_POSSTARI | OP_NOTPOSSTAR
                        | OP_NOTPOSSTARI | OP_QUERY | OP_QUERYI | OP_NOTQUERY | OP_NOTQUERYI
                        | OP_MINQUERY | OP_MINQUERYI | OP_NOTMINQUERY | OP_NOTMINQUERYI
                        | OP_POSQUERY | OP_POSQUERYI | OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                            cc = cc.add(_pcre2_OP_lengths_8[op as usize] as usize);
                            if utf != FALSE && HAS_EXTRALEN(*cc.offset(-1) as u32) {
                                cc = cc.add(GET_EXTRALEN(*cc.offset(-1) as u32) as usize);
                            }
                        }

                        /* Skip these, but we need to add in the name length. */
                        OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                            cc = cc.add(
                                _pcre2_OP_lengths_8[op as usize] as usize + *cc.add(1) as usize,
                            );
                        }

                        /* The remaining opcodes are just skipped over. */
                        OP_CLOSE | OP_COMMIT | OP_FAIL | OP_PRUNE | OP_SET_SOM | OP_SKIP
                        | OP_THEN => {
                            cc = cc.add(_pcre2_OP_lengths_8[op as usize] as usize);
                        }

                        /* This should not occur: we list all opcodes explicitly so that when
                        new ones get added they are properly considered. */
                        _ => {
                            return -3;
                        }
                    }

                    break 'sw;
                }

                /* REPEAT_BACK_REFERENCE: Handle repeated back references */

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
                        min = GET2(cc, 1) as c_int;
                        cc = cc.add(1 + 2 * IMM2_SIZE);
                    }

                    _ => {
                        min = 1;
                    }
                }

                /* Take care not to overflow. */

                if (d > 0 && (INT_MAX / d) < min)
                    || UINT16_MAX.wrapping_sub(branchlength) < min.wrapping_mul(d)
                {
                    branchlength = UINT16_MAX;
                } else {
                    branchlength += min * d;
                }

                break 'sw;
            }

            /* PROCESS_NON_CAPTURE: */

            d = find_minlength(re, cc, startcode, utf, recurses, countptr, backref_cache);
            if d < 0 {
                return d;
            }
            branchlength += d;
            loop {
                cc = cc.add(GET(cc, 1) as usize);
                if *cc as u32 != OP_ALT {
                    break;
                }
            }
            cc = cc.add(1 + LINK_SIZE);
        }
    }
}

/*************************************************
*      Set a bit and maybe its alternate case    *
*************************************************/

/* Given a character, set its first code unit's bit in the table, and also the
corresponding bit for the other version of a letter if we are caseless.

Returns:        pointer after the character
*/

unsafe fn set_table_bit(
    re: *mut pcre2_real_code,
    p: PCRE2_SPTR,
    caseless: BOOL,
    utf: BOOL,
    ucp: BOOL,
) -> PCRE2_SPTR {
    let mut p = p;
    let mut c: u32 = {
        let v = *p as u32;
        p = p.add(1);
        v
    }; /* First code unit */

    SET_BIT(re, c);

    /* In UTF-8 mode, pick up the remaining code units in order to find the end of
    the character, even when caseless. */

    if utf != FALSE {
        if c >= 0xc0 {
            let r = getutf8inc(c, p);
            c = r.0;
            p = r.1;
        }
    }

    /* If caseless, handle the other case of the character. */

    if caseless != FALSE {
        if utf != FALSE || ucp != FALSE {
            c = UCD_OTHERCASE(c);
            if utf != FALSE {
                let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
                crate::ord2utf::_pcre2_ord2utf_8(c, buff.as_mut_ptr());
                SET_BIT(re, buff[0] as u32);
            } else if c < 256 {
                SET_BIT(re, c);
            }
        } else {
            /* Not UTF or UCP */
            if MAX_255(c) {
                SET_BIT(re, *(*re).tables.add(fcc_offset + c as usize) as u32);
            }
        }
    }

    p
}

/*************************************************
*     Set bits for a positive character type     *
*************************************************/

unsafe fn set_type_bits(re: *mut pcre2_real_code, cbit_type: c_int, table_limit: c_uint) {
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
        if (*(*re).tables.add(cbits_offset + (c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0 {
            let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
            crate::ord2utf::_pcre2_ord2utf_8(c, buff.as_mut_ptr());
            SET_BIT(re, buff[0] as u32);
        }
        c += 1;
    }
}

/*************************************************
*     Set bits for a negative character type     *
*************************************************/

unsafe fn set_nottype_bits(re: *mut pcre2_real_code, cbit_type: c_int, table_limit: c_uint) {
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
the starting bits accordingly. */

unsafe fn study_char_list(code: PCRE2_SPTR, start_bitmap: *mut u8, char_lists_end: *const u8) {
    let mut code = code;
    let mut type_: u32;
    let mut list_ind: u32;
    let mut char_list_add: u32 = XCL_CHAR_LIST_LOW_16_ADD;
    let mut range_start: u32 = !(0u32);
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
    next_char = char_lists_end.sub((GET(code, 0) as usize) << 1);
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
                range_end = char_list_add.wrapping_add(range_end >> XCL_CHAR_SHIFT);

                crate::ord2utf::_pcre2_ord2utf_8(range_end, end_buffer.as_mut_ptr());
                end = end_buffer[0];

                if range_start < range_end {
                    crate::ord2utf::_pcre2_ord2utf_8(range_start, start_buffer.as_mut_ptr());
                    start = start_buffer[0];
                    while start <= end {
                        *start_bitmap.add((start / 8) as usize) |= (1u32 << (start & 7)) as u8;
                        start = start.wrapping_add(1);
                    }
                } else {
                    *start_bitmap.add((end / 8) as usize) |= (1u32 << (end & 7)) as u8;
                }

                range_start = !(0u32);
            } else {
                range_start = char_list_add.wrapping_add(range_end >> XCL_CHAR_SHIFT);
            }

            item_count -= 1;
        }

        list_ind += 1;
        type_ >>= XCL_TYPE_BIT_LEN;

        if range_start == !(0u32) {
            if (type_ & XCL_BEGIN_WITH_RANGE) != 0 {
                /* In 8 bit mode XCL_CHAR_LIST_HIGH_32_START is not possible. */
                if list_ind == 1 {
                    range_start = XCL_CHAR_LIST_HIGH_16_START;
                } else {
                    range_start = XCL_CHAR_LIST_LOW_32_START;
                }
            }
        } else if (type_ & XCL_BEGIN_WITH_RANGE) == 0 {
            crate::ord2utf::_pcre2_ord2utf_8(range_start, start_buffer.as_mut_ptr());

            /* In 8 bit mode XCL_CHAR_LIST_LOW_32_END and
            XCL_CHAR_LIST_HIGH_32_END are not possible. */
            if list_ind == 1 {
                range_end = XCL_CHAR_LIST_LOW_16_END;
            } else {
                range_end = XCL_CHAR_LIST_HIGH_16_END;
            }

            crate::ord2utf::_pcre2_ord2utf_8(range_end, end_buffer.as_mut_ptr());
            end = end_buffer[0];

            start = start_buffer[0];
            while start <= end {
                *start_bitmap.add((start / 8) as usize) |= (1u32 << (start & 7)) as u8;
                start = start.wrapping_add(1);
            }

            range_start = !(0u32);
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

/* Returns:       SSB_FAIL     => Failed to find any starting code units
                  SSB_DONE     => Found mandatory starting code units
                  SSB_CONTINUE => Found optional starting code units
                  SSB_UNKNOWN  => Hit an unrecognized opcode
                  SSB_TOODEEP  => Recursion is too deep
*/

unsafe fn set_start_bits(
    re: *mut pcre2_real_code,
    code: PCRE2_SPTR,
    utf: BOOL,
    ucp: BOOL,
    depthptr: *mut c_int,
) -> c_int {
    let mut code = code;
    let mut c: u32 = 0;
    let mut yield_: c_int = SSB_DONE;

    let table_limit: c_int = if utf != FALSE { 16 } else { 32 };

    *depthptr += 1;
    if *depthptr > 1000 {
        return SSB_TOODEEP;
    }

    loop {
        let mut try_next: BOOL = TRUE;
        let mut tcode: PCRE2_SPTR = code.add(1 + LINK_SIZE);

        if *code as u32 == OP_CBRA
            || *code as u32 == OP_SCBRA
            || *code as u32 == OP_CBRAPOS
            || *code as u32 == OP_SCBRAPOS
        {
            tcode = tcode.add(IMM2_SIZE);
        }

        'try_next_loop: while try_next != FALSE
        /* Loop for items in this branch */
        {
            let mut rc: c_int;
            let mut ncode: PCRE2_SPTR;
            let mut classmap: *const u8 = core::ptr::null();
            let mut xclassflags: PCRE2_UCHAR;

            'sw: {
                'handle_classmap: {
                    'class_entry: {
                        'nclass_entry: {
                            'type_star: {
                                'bra_entry: {
                                    match *tcode as u32 {
                                        /* Fail for a valid opcode that implies no starting
                                        bits. */
                                        OP_ACCEPT
                                        | OP_ASSERT_ACCEPT
                                        | OP_ALLANY
                                        | OP_ANY
                                        | OP_ANYBYTE
                                        | OP_CIRCM
                                        | OP_CLOSE
                                        | OP_COMMIT
                                        | OP_COMMIT_ARG
                                        | OP_COND
                                        | OP_CREF
                                        | OP_FALSE
                                        | OP_TRUE
                                        | OP_DNCREF
                                        | OP_DNREF
                                        | OP_DNREFI
                                        | OP_DNRREF
                                        | OP_DOLL
                                        | OP_DOLLM
                                        | OP_END
                                        | OP_EOD
                                        | OP_EODN
                                        | OP_EXTUNI
                                        | OP_FAIL
                                        | OP_MARK
                                        | OP_NOT
                                        | OP_NOTEXACT
                                        | OP_NOTEXACTI
                                        | OP_NOTI
                                        | OP_NOTMINPLUS
                                        | OP_NOTMINPLUSI
                                        | OP_NOTMINQUERY
                                        | OP_NOTMINQUERYI
                                        | OP_NOTMINSTAR
                                        | OP_NOTMINSTARI
                                        | OP_NOTMINUPTO
                                        | OP_NOTMINUPTOI
                                        | OP_NOTPLUS
                                        | OP_NOTPLUSI
                                        | OP_NOTPOSPLUS
                                        | OP_NOTPOSPLUSI
                                        | OP_NOTPOSQUERY
                                        | OP_NOTPOSQUERYI
                                        | OP_NOTPOSSTAR
                                        | OP_NOTPOSSTARI
                                        | OP_NOTPOSUPTO
                                        | OP_NOTPOSUPTOI
                                        | OP_NOTPROP
                                        | OP_NOTQUERY
                                        | OP_NOTQUERYI
                                        | OP_NOTSTAR
                                        | OP_NOTSTARI
                                        | OP_NOTUPTO
                                        | OP_NOTUPTOI
                                        | OP_NOT_HSPACE
                                        | OP_NOT_VSPACE
                                        | OP_PRUNE
                                        | OP_PRUNE_ARG
                                        | OP_RECURSE
                                        | OP_REF
                                        | OP_REFI
                                        | OP_REVERSE
                                        | OP_VREVERSE
                                        | OP_RREF
                                        | OP_SCOND
                                        | OP_SET_SOM
                                        | OP_SKIP
                                        | OP_SKIP_ARG
                                        | OP_SOD
                                        | OP_SOM
                                        | OP_THEN
                                        | OP_THEN_ARG => {
                                            return SSB_FAIL;
                                        }

                                        /* OP_CIRC happens only at the start of an anchored
                                        branch (multiline ^ uses OP_CIRCM). Skip over it. */
                                        OP_CIRC => {
                                            tcode = tcode.add(
                                                _pcre2_OP_lengths_8[OP_CIRC as usize] as usize,
                                            );
                                        }

                                        /* A "real" property test implies no starting bits, but
                                        the fake property PT_CLIST identifies a list of
                                        characters. */
                                        OP_PROP => {
                                            if *tcode.add(1) as u32 != PT_CLIST {
                                                return SSB_FAIL;
                                            }
                                            {
                                                let mut p: *const u32 =
                                                    _pcre2_ucd_caseless_sets_8.as_ptr()
                                                        .add(*tcode.add(2) as usize);
                                                loop {
                                                    c = *p;
                                                    p = p.add(1);
                                                    if !(c < NOTACHAR) {
                                                        break;
                                                    }
                                                    if utf != FALSE {
                                                        let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
                                                        crate::ord2utf::_pcre2_ord2utf_8(
                                                            c,
                                                            buff.as_mut_ptr(),
                                                        );
                                                        c = buff[0] as u32;
                                                    }
                                                    if c > 0xff {
                                                        SET_BIT(re, 0xff);
                                                    } else {
                                                        SET_BIT(re, c);
                                                    }
                                                }
                                            }
                                            try_next = FALSE;
                                        }

                                        /* We can ignore word boundary tests. */
                                        OP_WORD_BOUNDARY
                                        | OP_NOT_WORD_BOUNDARY
                                        | OP_UCP_WORD_BOUNDARY
                                        | OP_NOT_UCP_WORD_BOUNDARY => {
                                            tcode = tcode.add(1);
                                        }

                                        /* For a positive lookahead assertion, inspect what
                                        immediately follows, ignoring intermediate assertions
                                        and callouts. */
                                        OP_ASSERT | OP_ASSERT_NA => {
                                            ncode = tcode.add(GET(tcode, 1) as usize);
                                            while *ncode as u32 == OP_ALT {
                                                ncode = ncode.add(GET(ncode, 1) as usize);
                                            }
                                            ncode = ncode.add(1 + LINK_SIZE);

                                            /* Skip irrelevant items */

                                            {
                                                let mut done: BOOL = FALSE;
                                                while done == FALSE {
                                                    match *ncode as u32 {
                                                        OP_ASSERT
                                                        | OP_ASSERT_NOT
                                                        | OP_ASSERTBACK
                                                        | OP_ASSERTBACK_NOT
                                                        | OP_ASSERT_NA
                                                        | OP_ASSERTBACK_NA
                                                        | OP_ASSERT_SCS => {
                                                            ncode =
                                                                ncode.add(GET(ncode, 1) as usize);
                                                            while *ncode as u32 == OP_ALT {
                                                                ncode = ncode
                                                                    .add(GET(ncode, 1) as usize);
                                                            }
                                                            ncode = ncode.add(1 + LINK_SIZE);
                                                        }

                                                        OP_WORD_BOUNDARY
                                                        | OP_NOT_WORD_BOUNDARY
                                                        | OP_UCP_WORD_BOUNDARY
                                                        | OP_NOT_UCP_WORD_BOUNDARY => {
                                                            ncode = ncode.add(1);
                                                        }

                                                        OP_CALLOUT => {
                                                            ncode = ncode.add(
                                                                _pcre2_OP_lengths_8
                                                                    [OP_CALLOUT as usize]
                                                                    as usize,
                                                            );
                                                        }

                                                        OP_CALLOUT_STR => {
                                                            ncode = ncode.add(GET(
                                                                ncode,
                                                                1 + 2 * LINK_SIZE,
                                                            ) as usize);
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
                                                    if *ncode.add(1) as u32 == PT_CLIST {
                                                        tcode = ncode;
                                                        continue 'try_next_loop;
                                                    }
                                                }

                                                OP_ANYNL
                                                | OP_CHAR
                                                | OP_CHARI
                                                | OP_EXACT
                                                | OP_EXACTI
                                                | OP_HSPACE
                                                | OP_MINPLUS
                                                | OP_MINPLUSI
                                                | OP_PLUS
                                                | OP_PLUSI
                                                | OP_POSPLUS
                                                | OP_POSPLUSI
                                                | OP_VSPACE
                                                /* Note that these types will only be present
                                                in non-UCP mode. */
                                                | OP_DIGIT
                                                | OP_NOT_DIGIT
                                                | OP_WORDCHAR
                                                | OP_NOT_WORDCHAR
                                                | OP_WHITESPACE
                                                | OP_NOT_WHITESPACE => {
                                                    tcode = ncode;
                                                    continue 'try_next_loop;
                                                }

                                                _ => {}
                                            }

                                            /* Fall through */
                                            break 'bra_entry;
                                        }

                                        /* For a group bracket or a positive assertion without
                                        an immediately following mandatory setting, recurse to
                                        set bits from within the subpattern. */
                                        OP_BRA | OP_SBRA | OP_CBRA | OP_SCBRA | OP_BRAPOS
                                        | OP_SBRAPOS | OP_CBRAPOS | OP_SCBRAPOS | OP_ONCE
                                        | OP_SCRIPT_RUN => {
                                            break 'bra_entry;
                                        }

                                        /* If we hit ALT or KET, it means we haven't found
                                        anything mandatory in this branch. */
                                        OP_ALT => {
                                            yield_ = SSB_CONTINUE;
                                            try_next = FALSE;
                                        }

                                        OP_KET | OP_KETRMAX | OP_KETRMIN | OP_KETRPOS => {
                                            return SSB_CONTINUE;
                                        }

                                        /* Skip over callout */
                                        OP_CALLOUT => {
                                            tcode = tcode.add(
                                                _pcre2_OP_lengths_8[OP_CALLOUT as usize] as usize,
                                            );
                                        }

                                        OP_CALLOUT_STR => {
                                            tcode = tcode
                                                .add(GET(tcode, 1 + 2 * LINK_SIZE) as usize);
                                        }

                                        /* Skip over lookbehind, negative lookahead, and scan
                                        substring assertions */
                                        OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT
                                        | OP_ASSERTBACK_NA | OP_ASSERT_SCS => {
                                            loop {
                                                tcode = tcode.add(GET(tcode, 1) as usize);
                                                if *tcode as u32 != OP_ALT {
                                                    break;
                                                }
                                            }
                                            tcode = tcode.add(1 + LINK_SIZE);
                                        }

                                        /* BRAZERO does the bracket, but carries on. */
                                        OP_BRAZERO | OP_BRAMINZERO | OP_BRAPOSZERO => {
                                            tcode = tcode.add(1);
                                            rc = set_start_bits(re, tcode, utf, ucp, depthptr);
                                            if rc == SSB_FAIL
                                                || rc == SSB_UNKNOWN
                                                || rc == SSB_TOODEEP
                                            {
                                                return rc;
                                            }
                                            loop {
                                                tcode = tcode.add(GET(tcode, 1) as usize);
                                                if *tcode as u32 != OP_ALT {
                                                    break;
                                                }
                                            }
                                            tcode = tcode.add(1 + LINK_SIZE);
                                        }

                                        /* SKIPZERO skips the bracket. */
                                        OP_SKIPZERO => {
                                            tcode = tcode.add(1);
                                            loop {
                                                tcode = tcode.add(GET(tcode, 1) as usize);
                                                if *tcode as u32 != OP_ALT {
                                                    break;
                                                }
                                            }
                                            tcode = tcode.add(1 + LINK_SIZE);
                                        }

                                        /* Single-char * or ? sets the bit and tries the next
                                        item */
                                        OP_STAR | OP_MINSTAR | OP_POSSTAR | OP_QUERY
                                        | OP_MINQUERY | OP_POSQUERY => {
                                            tcode =
                                                set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                                        }

                                        OP_STARI | OP_MINSTARI | OP_POSSTARI | OP_QUERYI
                                        | OP_MINQUERYI | OP_POSQUERYI => {
                                            tcode =
                                                set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                                        }

                                        /* Single-char upto sets the bit and tries the next */
                                        OP_UPTO | OP_MINUPTO | OP_POSUPTO => {
                                            tcode = set_table_bit(
                                                re,
                                                tcode.add(1 + IMM2_SIZE),
                                                FALSE,
                                                utf,
                                                ucp,
                                            );
                                        }

                                        OP_UPTOI | OP_MINUPTOI | OP_POSUPTOI => {
                                            tcode = set_table_bit(
                                                re,
                                                tcode.add(1 + IMM2_SIZE),
                                                TRUE,
                                                utf,
                                                ucp,
                                            );
                                        }

                                        /* At least one single char sets the bit and stops */
                                        OP_EXACT => {
                                            tcode = tcode.add(IMM2_SIZE);
                                            /* Fall through */
                                            let _ =
                                                set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                                            try_next = FALSE;
                                        }

                                        OP_CHAR | OP_PLUS | OP_MINPLUS | OP_POSPLUS => {
                                            let _ =
                                                set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                                            try_next = FALSE;
                                        }

                                        OP_EXACTI => {
                                            tcode = tcode.add(IMM2_SIZE);
                                            /* Fall through */
                                            let _ =
                                                set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                                            try_next = FALSE;
                                        }

                                        OP_CHARI | OP_PLUSI | OP_MINPLUSI | OP_POSPLUSI => {
                                            let _ =
                                                set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                                            try_next = FALSE;
                                        }

                                        /* Special spacing and line-terminating items. */
                                        OP_HSPACE => {
                                            SET_BIT(re, CHAR_HT);
                                            SET_BIT(re, CHAR_SPACE);

                                            /* For the 8-bit library in UTF-8 mode, set the bits
                                            for the first code units of horizontal space
                                            characters. */

                                            if utf != FALSE {
                                                SET_BIT(re, 0xC2); /* For U+00A0 */
                                                SET_BIT(re, 0xE1); /* For U+1680, U+180E */
                                                SET_BIT(re, 0xE2); /* For U+2000 - U+200A, ... */
                                                SET_BIT(re, 0xE3); /* For U+3000 */
                                            } else {
                                                /* For the 8-bit library not in UTF-8 mode, set
                                                the bit for NBSP. */
                                                SET_BIT(re, CHAR_NBSP);
                                            }

                                            try_next = FALSE;
                                        }

                                        OP_ANYNL | OP_VSPACE => {
                                            SET_BIT(re, CHAR_LF);
                                            SET_BIT(re, CHAR_VT);
                                            SET_BIT(re, CHAR_FF);
                                            SET_BIT(re, CHAR_CR);

                                            if utf != FALSE {
                                                SET_BIT(re, 0xC2); /* For U+0085 (NEL) */
                                                SET_BIT(re, 0xE2); /* For U+2028, U+2029 */
                                            } else {
                                                SET_BIT(re, CHAR_NEL);
                                            }

                                            try_next = FALSE;
                                        }

                                        /* Single character types set the bits and stop. */
                                        OP_NOT_DIGIT => {
                                            set_nottype_bits(
                                                re,
                                                cbit_digit as c_int,
                                                table_limit as c_uint,
                                            );
                                            try_next = FALSE;
                                        }

                                        OP_DIGIT => {
                                            set_type_bits(
                                                re,
                                                cbit_digit as c_int,
                                                table_limit as c_uint,
                                            );
                                            try_next = FALSE;
                                        }

                                        OP_NOT_WHITESPACE => {
                                            set_nottype_bits(
                                                re,
                                                cbit_space as c_int,
                                                table_limit as c_uint,
                                            );
                                            try_next = FALSE;
                                        }

                                        OP_WHITESPACE => {
                                            set_type_bits(
                                                re,
                                                cbit_space as c_int,
                                                table_limit as c_uint,
                                            );
                                            try_next = FALSE;
                                        }

                                        OP_NOT_WORDCHAR => {
                                            set_nottype_bits(
                                                re,
                                                cbit_word as c_int,
                                                table_limit as c_uint,
                                            );
                                            try_next = FALSE;
                                        }

                                        OP_WORDCHAR => {
                                            set_type_bits(
                                                re,
                                                cbit_word as c_int,
                                                table_limit as c_uint,
                                            );
                                            try_next = FALSE;
                                        }

                                        /* One or more character type fudges the pointer and
                                        restarts, knowing it will hit a single character type
                                        and stop there. */
                                        OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEPOSPLUS => {
                                            tcode = tcode.add(1);
                                        }

                                        OP_TYPEEXACT => {
                                            tcode = tcode.add(1 + IMM2_SIZE);
                                        }

                                        /* Zero or more repeats of character types set the bits
                                        and then try again. */
                                        OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO => {
                                            tcode = tcode.add(IMM2_SIZE);
                                            /* Fall through */
                                            break 'type_star;
                                        }

                                        OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPOSSTAR
                                        | OP_TYPEQUERY | OP_TYPEMINQUERY | OP_TYPEPOSQUERY => {
                                            break 'type_star;
                                        }

                                        /* Set-based ECLASS: treat it the same as a "complex"
                                        XCLASS; give up. */
                                        OP_ECLASS => {
                                            return SSB_FAIL;
                                        }

                                        /* Extended class. */
                                        OP_XCLASS => {
                                            xclassflags = *tcode.add(1 + LINK_SIZE);
                                            if (xclassflags as u32 & XCL_HASPROP) != 0
                                                || (xclassflags as u32 & (XCL_MAP | XCL_NOT))
                                                    == XCL_NOT
                                            {
                                                return SSB_FAIL;
                                            }

                                            /* We have a positive XCLASS or a negative one
                                            without a map. Set up the map pointer if there is
                                            one, and fall through. */

                                            classmap = if (xclassflags as u32 & XCL_MAP) == 0 {
                                                core::ptr::null()
                                            } else {
                                                tcode.add(1 + LINK_SIZE + 1) as *const u8
                                            };

                                            /* In UTF-8 mode, scan the character list and set
                                            bits for leading bytes, then jump to handle the
                                            map. */

                                            if utf != FALSE
                                                && (xclassflags as u32 & XCL_NOT) == 0
                                            {
                                                let mut b: PCRE2_UCHAR;
                                                let mut e: PCRE2_UCHAR;
                                                let mut p: PCRE2_SPTR = tcode.add(
                                                    1 + LINK_SIZE
                                                        + 1
                                                        + if classmap.is_null() { 0 } else { 32 },
                                                );
                                                tcode = tcode.add(GET(tcode, 1) as usize);

                                                if *p as u32 >= XCL_LIST {
                                                    study_char_list(
                                                        p,
                                                        (*re).start_bitmap.as_mut_ptr(),
                                                        (re as *const u8).add((*re).code_start),
                                                    );
                                                    break 'handle_classmap;
                                                }

                                                loop {
                                                    let v = {
                                                        let t = *p;
                                                        p = p.add(1);
                                                        t
                                                    } as u32;
                                                    match v {
                                                        XCL_SINGLE => {
                                                            b = {
                                                                let t = *p;
                                                                p = p.add(1);
                                                                t
                                                            };
                                                            while (*p & 0xc0) == 0x80 {
                                                                p = p.add(1);
                                                            }
                                                            (*re).start_bitmap
                                                                [(b / 8) as usize] |=
                                                                (1u32 << (b & 7)) as u8;
                                                        }

                                                        XCL_RANGE => {
                                                            b = {
                                                                let t = *p;
                                                                p = p.add(1);
                                                                t
                                                            };
                                                            while (*p & 0xc0) == 0x80 {
                                                                p = p.add(1);
                                                            }
                                                            e = {
                                                                let t = *p;
                                                                p = p.add(1);
                                                                t
                                                            };
                                                            while (*p & 0xc0) == 0x80 {
                                                                p = p.add(1);
                                                            }
                                                            while b <= e {
                                                                (*re).start_bitmap
                                                                    [(b / 8) as usize] |=
                                                                    (1u32 << (b & 7)) as u8;
                                                                b = b.wrapping_add(1);
                                                            }
                                                        }

                                                        XCL_END => {
                                                            break 'handle_classmap;
                                                        }

                                                        _ => {
                                                            /* Internal error, should not
                                                            occur */
                                                            return SSB_UNKNOWN;
                                                        }
                                                    }
                                                }
                                            }

                                            /* Fall through */
                                            break 'nclass_entry;
                                        }

                                        OP_NCLASS => {
                                            break 'nclass_entry;
                                        }

                                        OP_CLASS => {
                                            break 'class_entry;
                                        }

                                        /* If we reach something we don't understand, it means
                                        a new opcode has been created that hasn't been added to
                                        this function. */
                                        _ => {
                                            return SSB_UNKNOWN;
                                        }
                                    }

                                    break 'sw;
                                }

                                /* Group bracket / positive assertion handling. */

                                rc = set_start_bits(re, tcode, utf, ucp, depthptr);
                                if rc == SSB_DONE {
                                    try_next = FALSE;
                                } else if rc == SSB_CONTINUE {
                                    loop {
                                        tcode = tcode.add(GET(tcode, 1) as usize);
                                        if *tcode as u32 != OP_ALT {
                                            break;
                                        }
                                    }
                                    tcode = tcode.add(1 + LINK_SIZE);
                                } else {
                                    return rc; /* FAIL, UNKNOWN, or TOODEEP */
                                }

                                break 'sw;
                            }

                            /* Repeated character types. */

                            match *tcode.add(1) as u32 {
                                OP_HSPACE => {
                                    SET_BIT(re, CHAR_HT);
                                    SET_BIT(re, CHAR_SPACE);

                                    if utf != FALSE {
                                        SET_BIT(re, 0xC2); /* For U+00A0 */
                                        SET_BIT(re, 0xE1); /* For U+1680, U+180E */
                                        SET_BIT(re, 0xE2); /* For U+2000 - U+200A, ... */
                                        SET_BIT(re, 0xE3); /* For U+3000 */
                                    } else {
                                        SET_BIT(re, CHAR_NBSP);
                                    }
                                }

                                OP_ANYNL | OP_VSPACE => {
                                    SET_BIT(re, CHAR_LF);
                                    SET_BIT(re, CHAR_VT);
                                    SET_BIT(re, CHAR_FF);
                                    SET_BIT(re, CHAR_CR);

                                    if utf != FALSE {
                                        SET_BIT(re, 0xC2); /* For U+0085 (NEL) */
                                        SET_BIT(re, 0xE2); /* For U+2028, U+2029 */
                                    } else {
                                        SET_BIT(re, CHAR_NEL);
                                    }
                                }

                                OP_NOT_DIGIT => {
                                    set_nottype_bits(re, cbit_digit as c_int, table_limit as c_uint);
                                }

                                OP_DIGIT => {
                                    set_type_bits(re, cbit_digit as c_int, table_limit as c_uint);
                                }

                                OP_NOT_WHITESPACE => {
                                    set_nottype_bits(re, cbit_space as c_int, table_limit as c_uint);
                                }

                                OP_WHITESPACE => {
                                    set_type_bits(re, cbit_space as c_int, table_limit as c_uint);
                                }

                                OP_NOT_WORDCHAR => {
                                    set_nottype_bits(re, cbit_word as c_int, table_limit as c_uint);
                                }

                                OP_WORDCHAR => {
                                    set_type_bits(re, cbit_word as c_int, table_limit as c_uint);
                                }

                                /* default, OP_ANY, OP_ALLANY */
                                _ => {
                                    return SSB_FAIL;
                                }
                            }

                            tcode = tcode.add(2);
                            break 'sw;
                        }

                        /* OP_NCLASS: Enter here for a negative non-XCLASS. In the 8-bit
                        library, if we are in UTF mode, any byte with a value >= 0xc4 is a
                        potentially valid starter because it starts a character with a value
                        > 255. In 8-bit non-UTF mode, there is no difference between CLASS
                        and NCLASS. */

                        if utf != FALSE {
                            (*re).start_bitmap[24] |= 0xf0; /* Bits for 0xc4 - 0xc8 */
                            memset(
                                (*re).start_bitmap.as_mut_ptr().add(25) as *mut c_void,
                                0xff,
                                7,
                            ); /* Bits for 0xc9 - 0xff */
                        }
                        /* Fall through */
                    }

                    /* OP_CLASS: Enter here for a positive non-XCLASS. If we have fallen
                    through from an XCLASS, classmap will already be set; just advance the
                    code pointer. Otherwise, set up classmap for a non-XCLASS and advance
                    past it. */

                    if *tcode as u32 == OP_XCLASS {
                        tcode = tcode.add(GET(tcode, 1) as usize);
                    } else {
                        tcode = tcode.add(1);
                        classmap = tcode as *const u8;
                        tcode = tcode.add(32);
                    }

                    /* Fall through to HANDLE_CLASSMAP */
                }

                /* HANDLE_CLASSMAP: */

                if !classmap.is_null() {
                    if utf != FALSE {
                        c = 0;
                        while c < 16 {
                            (*re).start_bitmap[c as usize] |= *classmap.add(c as usize);
                            c += 1;
                        }
                        c = 128;
                        while c < 256 {
                            if (*classmap.add((c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0 {
                                let d: c_int = ((c >> 6) | 0xc0) as c_int; /* Set bit for this starter */
                                (*re).start_bitmap[(d / 8) as usize] |= (1u32 << (d & 7)) as u8; /* and then skip on to the */
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

                match *tcode as u32 {
                    OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSSTAR
                    | OP_CRPOSQUERY => {
                        tcode = tcode.add(1);
                    }

                    OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                        if GET2(tcode, 1) == 0 {
                            tcode = tcode.add(1 + 2 * IMM2_SIZE);
                        } else {
                            try_next = FALSE;
                        }
                    }

                    _ => {
                        try_next = FALSE;
                    }
                }
            } /* End of switch for opcodes */
        } /* End of try_next loop */

        code = code.add(GET(code, 1) as usize); /* Advance to next branch */
        if *code as u32 != OP_ALT {
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

Returns:   0 normally; non-zero should never normally occur
           1 unknown opcode in set_start_bits
           2 missing capturing bracket
           3 unknown opcode in find_minlength
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_study_8(re: *mut pcre2_real_code) -> c_int {
    let mut count: c_int = 0;
    let code: *mut PCRE2_UCHAR;
    let utf: BOOL = if ((*re).overall_options & PCRE2_UTF) != 0 {
        TRUE
    } else {
        FALSE
    };
    let ucp: BOOL = if ((*re).overall_options & PCRE2_UCP) != 0 {
        TRUE
    } else {
        FALSE
    };

    /* Find start of compiled code */

    code = (re as *mut u8).add((*re).code_start) as *mut PCRE2_UCHAR;

    /* For a pattern that has a first code unit, or a multiline pattern that
    matches only at "line start", there is no point in seeking a list of starting
    code units. */

    if ((*re).flags & (PCRE2_FIRSTSET | PCRE2_STARTLINE)) == 0 {
        let mut depth: c_int = 0;
        let rc: c_int = set_start_bits(re, code, utf, ucp, &mut depth);
        if rc == SSB_UNKNOWN {
            return 1;
        }

        /* If a list of starting code units was set up, scan the list to see if only
        one or two were listed. */

        if rc == SSB_DONE {
            let mut i: c_int;
            let mut a: c_int = -1;
            let mut b: c_int = -1;
            let mut p: *mut u8 = (*re).start_bitmap.as_mut_ptr();
            let mut flags: u32 = PCRE2_FIRSTMAPSET;

            'done: {
                i = 0;
                while i < 256 {
                    let x: u8 = *p;
                    if x != 0 {
                        let mut c: c_int;
                        let y: u8 = x & x.wrapping_neg(); /* Least significant bit */
                        if y != x {
                            break 'done; /* More than one bit set */
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
                        mode, only values < 128 can be used. */

                        if utf != FALSE && c > 127 {
                            break 'done;
                        }
                        if a < 0 {
                            a = c; /* First one found, save in a */
                        } else if b < 0
                        /* Second one found */
                        {
                            let mut d: c_int = TABLE_GET(
                                c as c_uint as u32,
                                (*re).tables.add(fcc_offset),
                                c as u32,
                            ) as c_int;

                            if utf != FALSE || ucp != FALSE {
                                if UCD_CASESET(c as u32) != 0 {
                                    break 'done; /* Multiple case set */
                                }
                                if c > 127 {
                                    d = UCD_OTHERCASE(c as u32) as c_int;
                                }
                            }

                            if d != a {
                                break 'done; /* Not the other case of a */
                            }
                            b = c; /* Save second in b */
                        } else {
                            break 'done; /* More than two characters found */
                        }
                    }
                    p = p.add(1);
                    i += 8;
                }

                /* Replace the start code unit bits with a first code unit. */

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

    /* Find the minimum length of subject string. */

    if ((*re).flags & (PCRE2_MATCH_EMPTY | PCRE2_HASACCEPT)) == 0
        && ((*re).top_backref as usize) <= MAX_CACHE_BACKREF
    {
        let min: c_int;
        let mut backref_cache: [c_int; MAX_CACHE_BACKREF + 1] = [0; MAX_CACHE_BACKREF + 1];
        backref_cache[0] = 0; /* Highest one that is set */
        min = find_minlength(
            re,
            code,
            code,
            utf,
            core::ptr::null_mut(),
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
                (*re).minlength = (if min > UINT16_MAX { UINT16_MAX } else { min }) as u16;
            }
        }
    }

    0
}

/* End of pcre2_study.c */
