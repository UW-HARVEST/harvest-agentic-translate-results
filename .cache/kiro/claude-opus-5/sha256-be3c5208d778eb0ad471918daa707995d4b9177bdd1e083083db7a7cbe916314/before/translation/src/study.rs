//! Translation of `pcre2_study.c` (8-bit, UTF/UCP supported, JIT off).
//!
//! This module computes the minimum match length and the starting-code-unit
//! bitmap for a compiled pattern. It exports exactly one linker symbol,
//! `_pcre2_study_8` (the `PRIV(study)` function).

use crate::internal::*;
use crate::tables;
use core::ffi::c_int;
use core::ptr;

// ---------------------------------------------------------------------------
// Local constants
// ---------------------------------------------------------------------------

/// `XCL_LIST` for 8-bit mode: `sizeof(PCRE2_UCHAR) == 1 ? 0x10 : 0x1000`.
const XCL_LIST: i64 = 0x10;

/// The maximum remembered capturing brackets minimum.
const MAX_CACHE_BACKREF: usize = 128;

// Returns from set_start_bits()
const SSB_FAIL: c_int = 0;
const SSB_DONE: c_int = 1;
const SSB_CONTINUE: c_int = 2;
const SSB_UNKNOWN: c_int = 3;
const SSB_TOODEEP: c_int = 4;

// CHAR_* values (ASCII / non-EBCDIC). Only CHAR_LF/CHAR_NL are provided in
// `consts`, so the remaining ones we need are defined here to match
// `pcre2_internal.h`.
const CHAR_HT: u32 = 0x09;
const CHAR_LF_V: u32 = 0x0a;
const CHAR_VT: u32 = 0x0b;
const CHAR_FF: u32 = 0x0c;
const CHAR_CR: u32 = 0x0d;
const CHAR_SPACE: u32 = 0x20;
const CHAR_NEL: u32 = 0x85;
const CHAR_NBSP: u32 = 0xa0;

const LINK_SIZE: usize = LINK_SIZE_U;
const IMM2_SIZE: usize = IMM2_SIZE_U;

/// `SET_BIT(c)` — set a bit in the starting code unit bit map.
#[inline(always)]
unsafe fn set_bit(re: *mut pcre2_real_code, c: u32) {
    unsafe {
        (*re).start_bitmap[(c as usize) / 8] |= 1u8 << ((c & 7) as u8);
    }
}

// ---------------------------------------------------------------------------
// find_minlength
// ---------------------------------------------------------------------------

/// Scan a parenthesized group and compute the minimum length of subject that
/// is needed to match it.
///
/// Returns the minimum length, or:
///   -1  `\C` in UTF-8 mode, or (*ACCEPT), or pattern too complicated
///   -2  internal error (missing capturing bracket)
///   -3  internal error (opcode not listed)
unsafe fn find_minlength(
    re: *const pcre2_real_code,
    code: PCRE2_SPTR,
    startcode: PCRE2_SPTR,
    utf: BOOL,
    recurses: *mut recurse_check,
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
        let dupcapused: BOOL = if ((*re).flags & PCRE2_DUPCAPUSED as u32) != 0 {
            TRUE
        } else {
            FALSE
        };
        let mut nextbranch: PCRE2_SPTR = code.add(GET(code, 1) as usize);
        let mut cc: PCRE2_SPTR = code.add(1 + LINK_SIZE);
        let mut this_recurse = recurse_check {
            prev: ptr::null_mut(),
            group: ptr::null(),
        };

        // If this is a "could be empty" group, its minimum length is 0.
        if *code as u32 >= OP_SBRA && *code as u32 <= OP_SCOND {
            return 0;
        }

        // Skip over capturing bracket number.
        if *code as u32 == OP_CBRA || *code as u32 == OP_CBRAPOS {
            cc = cc.add(IMM2_SIZE);
        }

        // A large and/or complex regex can take too long to process.
        let cnt = *countptr;
        *countptr = cnt + 1;
        if cnt > 1000 {
            return -1;
        }

        // Scan along the opcodes for this branch.
        loop {
            let mut d: c_int;
            let mut min: c_int;
            let mut recno: c_int;
            let op: PCRE2_UCHAR;
            let mut cs: PCRE2_SPTR;
            let mut ce: PCRE2_SPTR;

            if branchlength >= u16::MAX as c_int {
                branchlength = u16::MAX as c_int;
                cc = nextbranch;
            }

            op = *cc;
            match op as u32 {
                OP_COND | OP_SCOND => {
                    // If there is only one branch in a condition, the implied
                    // branch has zero length. Otherwise treat it like any other
                    // non-capturing subpattern.
                    cs = cc.add(GET(cc, 1) as usize);
                    if *cs as u32 != OP_ALT {
                        cc = cs.add(1 + LINK_SIZE);
                        continue;
                    }
                    // PROCESS_NON_CAPTURE
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

                OP_BRA => {
                    // Special case of OP_BRA wrapped round a repeated OP_RECURSE.
                    if *cc.add(1 + LINK_SIZE) as u32 == OP_RECURSE
                        && *cc.add(2 * (1 + LINK_SIZE)) as u32 == OP_KET
                    {
                        once_fudge = (1 + LINK_SIZE) as u32;
                        cc = cc.add(1 + LINK_SIZE);
                        continue;
                    }
                    // Fall through to non-capture processing.
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

                OP_ONCE | OP_SCRIPT_RUN | OP_SBRA | OP_BRAPOS | OP_SBRAPOS => {
                    // PROCESS_NON_CAPTURE
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

                OP_ACCEPT | OP_ASSERT_ACCEPT => {
                    return -1;
                }

                OP_ALT | OP_KET | OP_KETRMAX | OP_KETRMIN | OP_KETRPOS | OP_END => {
                    if length < 0 || (had_recurse == FALSE && branchlength < length) {
                        length = branchlength;
                    }
                    if op as u32 != OP_ALT || length == 0 {
                        return length;
                    }
                    nextbranch = cc.add(GET(cc, 1) as usize);
                    cc = cc.add(1 + LINK_SIZE);
                    branchlength = 0;
                    had_recurse = FALSE;
                }

                OP_ASSERT | OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT | OP_ASSERT_NA
                | OP_ASSERT_SCS | OP_ASSERTBACK_NA => {
                    loop {
                        cc = cc.add(GET(cc, 1) as usize);
                        if *cc as u32 != OP_ALT {
                            break;
                        }
                    }
                    // Fall through: skip over things that don't match chars.
                    cc = cc.add(tables::_pcre2_OP_lengths_8[*cc as usize] as usize);
                }

                OP_REVERSE | OP_VREVERSE | OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF | OP_FALSE
                | OP_TRUE | OP_CALLOUT | OP_SOD | OP_SOM | OP_EOD | OP_EODN | OP_CIRC
                | OP_CIRCM | OP_DOLL | OP_DOLLM | OP_NOT_WORD_BOUNDARY | OP_WORD_BOUNDARY
                | OP_NOT_UCP_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY => {
                    cc = cc.add(tables::_pcre2_OP_lengths_8[*cc as usize] as usize);
                }

                OP_CALLOUT_STR => {
                    cc = cc.add(GET(cc, 1 + 2 * LINK_SIZE) as usize);
                }

                OP_BRAZERO | OP_BRAMINZERO | OP_BRAPOSZERO | OP_SKIPZERO => {
                    cc = cc.add(tables::_pcre2_OP_lengths_8[*cc as usize] as usize);
                    loop {
                        cc = cc.add(GET(cc, 1) as usize);
                        if *cc as u32 != OP_ALT {
                            break;
                        }
                    }
                    cc = cc.add(1 + LINK_SIZE);
                }

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

                OP_PROP | OP_NOTPROP => {
                    cc = cc.add(2);
                    // Fall through to single-char non-literal matchers.
                    branchlength += 1;
                    cc = cc.add(1);
                }

                OP_NOT_DIGIT | OP_DIGIT | OP_NOT_WHITESPACE | OP_WHITESPACE | OP_NOT_WORDCHAR
                | OP_WORDCHAR | OP_ANY | OP_ALLANY | OP_EXTUNI | OP_HSPACE | OP_NOT_HSPACE
                | OP_VSPACE | OP_NOT_VSPACE => {
                    branchlength += 1;
                    cc = cc.add(1);
                }

                OP_ANYNL => {
                    branchlength += 1;
                    cc = cc.add(1);
                }

                OP_ANYBYTE => {
                    if utf != FALSE {
                        return -1;
                    }
                    branchlength += 1;
                    cc = cc.add(1);
                }

                OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEQUERY | OP_TYPEMINQUERY | OP_TYPEPOSSTAR
                | OP_TYPEPOSQUERY => {
                    if *cc.add(1) as u32 == OP_PROP || *cc.add(1) as u32 == OP_NOTPROP {
                        cc = cc.add(2);
                    }
                    cc = cc.add(tables::_pcre2_OP_lengths_8[op as usize] as usize);
                }

                OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO => {
                    if *cc.add(1 + IMM2_SIZE) as u32 == OP_PROP
                        || *cc.add(1 + IMM2_SIZE) as u32 == OP_NOTPROP
                    {
                        cc = cc.add(2);
                    }
                    cc = cc.add(tables::_pcre2_OP_lengths_8[op as usize] as usize);
                }

                OP_CLASS | OP_NCLASS | OP_XCLASS | OP_ECLASS => {
                    if op as u32 == OP_XCLASS || op as u32 == OP_ECLASS {
                        cc = cc.add(GET(cc, 1) as usize);
                    } else {
                        cc = cc.add(tables::_pcre2_OP_lengths_8[OP_CLASS as usize] as usize);
                    }

                    match *cc as u32 {
                        OP_CRPLUS | OP_CRMINPLUS | OP_CRPOSPLUS => {
                            branchlength += 1;
                            cc = cc.add(1);
                        }
                        OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSSTAR
                        | OP_CRPOSQUERY => {
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

                OP_DNREF | OP_DNREFI => {
                    if dupcapused == FALSE
                        && ((*re).overall_options & PCRE2_MATCH_UNSET_BACKREF as u32) == 0
                    {
                        let mut count = GET2(cc, 1 + IMM2_SIZE) as c_int;
                        let mut slot: PCRE2_SPTR = (re as *const u8)
                            .add(core::mem::size_of::<pcre2_real_code>())
                            .add((GET2(cc, 1) as usize) * (*re).name_entry_size as usize);

                        d = c_int::MAX;

                        // Scan all groups with the same name; find the shortest.
                        while count > 0 {
                            count -= 1;
                            let mut dd: c_int;
                            recno = GET2(slot, 0) as c_int;

                            if recno <= *backref_cache.add(0)
                                && *backref_cache.add(recno as usize) >= 0
                            {
                                dd = *backref_cache.add(recno as usize);
                            } else {
                                cs = tables_find_bracket(startcode, utf, recno);
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
                                    || tables_find_bracket(ce, utf, recno).is_null()
                                {
                                    if cc > cs && cc < ce {
                                        // Simple recursion
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
                                            // Mutual recursion
                                            had_recurse = TRUE;
                                        } else {
                                            this_recurse.prev = recurses;
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
                                let mut i = *backref_cache.add(0) + 1;
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
                                break;
                            }
                            slot = slot.add((*re).name_entry_size as usize);
                        }
                    } else {
                        d = 0;
                    }
                    cc = cc.add(tables::_pcre2_OP_lengths_8[*cc as usize] as usize);
                    // goto REPEAT_BACK_REFERENCE
                    min = repeat_back_reference(&mut cc);
                    apply_backref(&mut branchlength, d, min);
                }

                OP_REF | OP_REFI => {
                    recno = GET2(cc, 1) as c_int;
                    if recno <= *backref_cache.add(0) && *backref_cache.add(recno as usize) >= 0 {
                        d = *backref_cache.add(recno as usize);
                    } else {
                        d = 0;

                        if ((*re).overall_options & PCRE2_MATCH_UNSET_BACKREF as u32) == 0 {
                            cs = tables_find_bracket(startcode, utf, recno);
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
                                || tables_find_bracket(ce, utf, recno).is_null()
                            {
                                if cc > cs && cc < ce {
                                    // Simple recursion
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
                                        // Mutual recursion
                                        had_recurse = TRUE;
                                    } else {
                                        // No recursion
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
                        let mut i = *backref_cache.add(0) + 1;
                        while i < recno {
                            *backref_cache.add(i as usize) = -1;
                            i += 1;
                        }
                        *backref_cache.add(0) = recno;
                    }

                    cc = cc.add(tables::_pcre2_OP_lengths_8[*cc as usize] as usize);

                    // REPEAT_BACK_REFERENCE
                    min = repeat_back_reference(&mut cc);
                    apply_backref(&mut branchlength, d, min);
                }

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
                        if cc > cs && cc < ce {
                            // Simple recursion
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
                                // Mutual recursion
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

                OP_UPTO | OP_UPTOI | OP_NOTUPTO | OP_NOTUPTOI | OP_MINUPTO | OP_MINUPTOI
                | OP_NOTMINUPTO | OP_NOTMINUPTOI | OP_POSUPTO | OP_POSUPTOI | OP_NOTPOSUPTO
                | OP_NOTPOSUPTOI | OP_STAR | OP_STARI | OP_NOTSTAR | OP_NOTSTARI | OP_MINSTAR
                | OP_MINSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI | OP_POSSTAR | OP_POSSTARI
                | OP_NOTPOSSTAR | OP_NOTPOSSTARI | OP_QUERY | OP_QUERYI | OP_NOTQUERY
                | OP_NOTQUERYI | OP_MINQUERY | OP_MINQUERYI | OP_NOTMINQUERY | OP_NOTMINQUERYI
                | OP_POSQUERY | OP_POSQUERYI | OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                    cc = cc.add(tables::_pcre2_OP_lengths_8[op as usize] as usize);
                    if utf != FALSE && HAS_EXTRALEN(*cc.offset(-1) as u32) {
                        cc = cc.add(GET_EXTRALEN(*cc.offset(-1) as u32) as usize);
                    }
                }

                OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                    cc = cc.add(
                        tables::_pcre2_OP_lengths_8[op as usize] as usize + *cc.add(1) as usize,
                    );
                }

                OP_CLOSE | OP_COMMIT | OP_FAIL | OP_PRUNE | OP_SET_SOM | OP_SKIP | OP_THEN => {
                    cc = cc.add(tables::_pcre2_OP_lengths_8[op as usize] as usize);
                }

                _ => {
                    return -3;
                }
            }
        }
    }
}

/// Handle the `REPEAT_BACK_REFERENCE` switch, advancing `cc` and returning the
/// computed `min`.
#[inline]
unsafe fn repeat_back_reference(cc: &mut PCRE2_SPTR) -> c_int {
    unsafe {
        let min: c_int;
        match **cc as u32 {
            OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSSTAR
            | OP_CRPOSQUERY => {
                min = 0;
                *cc = cc.add(1);
            }
            OP_CRPLUS | OP_CRMINPLUS | OP_CRPOSPLUS => {
                min = 1;
                *cc = cc.add(1);
            }
            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                min = GET2(*cc, 1) as c_int;
                *cc = cc.add(1 + 2 * IMM2_SIZE);
            }
            _ => {
                min = 1;
            }
        }
        min
    }
}

/// Apply the min*d contribution to `branchlength`, taking care not to overflow,
/// matching the C code following `REPEAT_BACK_REFERENCE`.
#[inline]
fn apply_backref(branchlength: &mut c_int, d: c_int, min: c_int) {
    if (d > 0 && (c_int::MAX / d) < min) || (u16::MAX as c_int - *branchlength) < min * d {
        *branchlength = u16::MAX as c_int;
    } else {
        *branchlength += min * d;
    }
}

/// Thin wrapper over the exported `find_bracket` to keep call sites tidy.
#[inline(always)]
unsafe fn tables_find_bracket(code: PCRE2_SPTR, utf: BOOL, number: c_int) -> PCRE2_SPTR {
    unsafe { crate::find_bracket::_pcre2_find_bracket_8(code, utf, number) }
}

// ---------------------------------------------------------------------------
// set_table_bit
// ---------------------------------------------------------------------------

/// Given a character, set its first code unit's bit in the table, and also the
/// corresponding bit for the other version of a letter if we are caseless.
///
/// Returns the pointer after the character.
unsafe fn set_table_bit(
    re: *mut pcre2_real_code,
    p: PCRE2_SPTR,
    caseless: BOOL,
    utf: BOOL,
    ucp: BOOL,
) -> PCRE2_SPTR {
    unsafe {
        let mut p = p;
        let mut c: u32 = *p as u32; // First code unit
        p = p.add(1);

        // 8-bit mode: SET_BIT(c) directly.
        set_bit(re, c);

        // In UTF-8 mode, pick up the remaining code units to find the end of the
        // character, even when caseless.
        if utf != FALSE {
            if c >= 0xc0 {
                c = GETUTF8INC(c, &mut p);
            }
        }

        // If caseless, handle the other case of the character.
        if caseless != FALSE {
            if utf != FALSE || ucp != FALSE {
                c = UCD_OTHERCASE(c);
                // 8-bit mode:
                if utf != FALSE {
                    let mut buff = [0u8; 6];
                    let _ = crate::ord2utf::_pcre2_ord2utf_8(c, buff.as_mut_ptr());
                    set_bit(re, buff[0] as u32);
                } else if c < 256 {
                    set_bit(re, c);
                }
            } else {
                // Not UTF or UCP
                if MAX_255(c) {
                    set_bit(re, *(*re).tables.add(fcc_offset as usize + c as usize) as u32);
                }
            }
        }

        p
    }
}

// ---------------------------------------------------------------------------
// set_type_bits / set_nottype_bits
// ---------------------------------------------------------------------------

/// Set starting bits for a (positive) character type.
unsafe fn set_type_bits(re: *mut pcre2_real_code, cbit_type: c_int, table_limit: u32) {
    unsafe {
        let mut c: u32 = 0;
        while c < table_limit {
            (*re).start_bitmap[c as usize] |=
                *(*re).tables.add(c as usize + cbits_offset as usize + cbit_type as usize);
            c += 1;
        }
        // SUPPORT_UNICODE && 8-bit
        if table_limit == 32 {
            return;
        }
        c = 128;
        while c < 256 {
            if (*(*re).tables.add(cbits_offset as usize + (c / 8) as usize) & (1u8 << (c & 7)))
                != 0
            {
                let mut buff = [0u8; 6];
                let _ = crate::ord2utf::_pcre2_ord2utf_8(c, buff.as_mut_ptr());
                set_bit(re, buff[0] as u32);
            }
            c += 1;
        }
    }
}

/// Set starting bits for a negative character type such as `\D`.
unsafe fn set_nottype_bits(re: *mut pcre2_real_code, cbit_type: c_int, table_limit: u32) {
    unsafe {
        let mut c: u32 = 0;
        while c < table_limit {
            (*re).start_bitmap[c as usize] |= !(*(*re)
                .tables
                .add(c as usize + cbits_offset as usize + cbit_type as usize));
            c += 1;
        }
        // SUPPORT_UNICODE && 8-bit
        if table_limit != 32 {
            c = 24;
            while c < 32 {
                (*re).start_bitmap[c as usize] = 0xff;
                c += 1;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// study_char_list
// ---------------------------------------------------------------------------

/// Set starting bits for a character list (8-bit UTF only). Enumerates all
/// characters and ranges in the list and sets the corresponding starting bits.
unsafe fn study_char_list(
    code: PCRE2_SPTR,
    start_bitmap: *mut u8,
    char_lists_end: *const u8,
) {
    unsafe {
        let mut code = code;
        let mut char_list_add: u32 = XCL_CHAR_LIST_LOW_16_ADD as u32;
        let mut range_start: u32 = !0u32;
        let mut range_end: u32;
        let mut next_char: *const u8;
        let mut start_buffer = [0u8; 6];
        let mut end_buffer = [0u8; 6];
        let mut start: u8;
        let mut end: u8;

        let mut ty: u32 = ((*code.add(0) as u32) << 8) | *code.add(1) as u32;
        code = code.add(2);

        // Align characters.
        next_char = char_lists_end.sub((GET(code, 0) as usize) << 1);
        ty &= XCL_TYPE_MASK as u32;
        let mut list_ind: u32 = 0;

        if (ty & XCL_BEGIN_WITH_RANGE as u32) != 0 {
            range_start = XCL_CHAR_LIST_LOW_16_START as u32;
        }

        while ty > 0 {
            let mut item_count: u32 = ty & XCL_ITEM_COUNT_MASK as u32;

            if item_count == XCL_ITEM_COUNT_MASK as u32 {
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

                if (range_end & XCL_CHAR_END as u32) != 0 {
                    range_end = char_list_add + (range_end >> XCL_CHAR_SHIFT as u32);

                    let _ = crate::ord2utf::_pcre2_ord2utf_8(range_end, end_buffer.as_mut_ptr());
                    end = end_buffer[0];

                    if range_start < range_end {
                        let _ = crate::ord2utf::_pcre2_ord2utf_8(
                            range_start,
                            start_buffer.as_mut_ptr(),
                        );
                        start = start_buffer[0];
                        while start <= end {
                            *start_bitmap.add((start / 8) as usize) |= 1u8 << (start & 7);
                            if start == u8::MAX {
                                break;
                            }
                            start += 1;
                        }
                    } else {
                        *start_bitmap.add((end / 8) as usize) |= 1u8 << (end & 7);
                    }

                    range_start = !0u32;
                } else {
                    range_start = char_list_add + (range_end >> XCL_CHAR_SHIFT as u32);
                }

                item_count -= 1;
            }

            list_ind += 1;
            ty >>= XCL_TYPE_BIT_LEN as u32;

            if range_start == !0u32 {
                if (ty & XCL_BEGIN_WITH_RANGE as u32) != 0 {
                    if list_ind == 1 {
                        range_start = XCL_CHAR_LIST_HIGH_16_START as u32;
                    } else {
                        range_start = XCL_CHAR_LIST_LOW_32_START as u32;
                    }
                }
            } else if (ty & XCL_BEGIN_WITH_RANGE as u32) == 0 {
                let _ =
                    crate::ord2utf::_pcre2_ord2utf_8(range_start, start_buffer.as_mut_ptr());

                if list_ind == 1 {
                    range_end = XCL_CHAR_LIST_LOW_16_END as u32;
                } else {
                    range_end = XCL_CHAR_LIST_HIGH_16_END as u32;
                }

                let _ = crate::ord2utf::_pcre2_ord2utf_8(range_end, end_buffer.as_mut_ptr());
                end = end_buffer[0];

                start = start_buffer[0];
                while start <= end {
                    *start_bitmap.add((start / 8) as usize) |= 1u8 << (start & 7);
                    if start == u8::MAX {
                        break;
                    }
                    start += 1;
                }

                range_start = !0u32;
            }

            if list_ind == 1 {
                char_list_add = XCL_CHAR_LIST_HIGH_16_ADD as u32;
            } else {
                char_list_add = XCL_CHAR_LIST_LOW_32_ADD as u32;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// set_start_bits
// ---------------------------------------------------------------------------

/// Scan a compiled expression recursively and build a bitmap of the set of
/// possible starting code units whose values are less than 256.
///
/// See the enum `SSB_*` for the return values.
unsafe fn set_start_bits(
    re: *mut pcre2_real_code,
    code: PCRE2_SPTR,
    utf: BOOL,
    ucp: BOOL,
    depthptr: *mut c_int,
) -> c_int {
    unsafe {
        let mut c: u32;
        let mut yield_: c_int = SSB_DONE;

        // SUPPORT_UNICODE && 8-bit
        let table_limit: u32 = if utf != FALSE { 16 } else { 32 };

        let mut code = code;

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

            while try_next != FALSE {
                let rc: c_int;
                let mut ncode: PCRE2_SPTR;
                let mut classmap: *const u8 = ptr::null();
                let mut xclassflags: PCRE2_UCHAR = 0;

                match *tcode as u32 {
                    // Unknown opcode.
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

                    OP_CIRC => {
                        tcode = tcode.add(tables::_pcre2_OP_lengths_8[OP_CIRC as usize] as usize);
                    }

                    OP_PROP => {
                        if *tcode.add(1) as u32 != PT_CLIST as u32 {
                            return SSB_FAIL;
                        }
                        {
                            let mut p: *const u32 = tables::_pcre2_ucd_caseless_sets_8
                                .as_ptr()
                                .add(*tcode.add(2) as usize);
                            loop {
                                c = *p;
                                p = p.add(1);
                                if c >= NOTACHAR as u32 {
                                    break;
                                }
                                // 8-bit UTF
                                if utf != FALSE {
                                    let mut buff = [0u8; 6];
                                    let _ =
                                        crate::ord2utf::_pcre2_ord2utf_8(c, buff.as_mut_ptr());
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

                    OP_WORD_BOUNDARY | OP_NOT_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY
                    | OP_NOT_UCP_WORD_BOUNDARY => {
                        tcode = tcode.add(1);
                    }

                    OP_ASSERT | OP_ASSERT_NA => {
                        ncode = tcode.add(GET(tcode, 1) as usize);
                        while *ncode as u32 == OP_ALT {
                            ncode = ncode.add(GET(ncode, 1) as usize);
                        }
                        ncode = ncode.add(1 + LINK_SIZE);

                        // Skip irrelevant items.
                        let mut done = false;
                        while !done {
                            match *ncode as u32 {
                                OP_ASSERT | OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT
                                | OP_ASSERT_NA | OP_ASSERTBACK_NA | OP_ASSERT_SCS => {
                                    ncode = ncode.add(GET(ncode, 1) as usize);
                                    while *ncode as u32 == OP_ALT {
                                        ncode = ncode.add(GET(ncode, 1) as usize);
                                    }
                                    ncode = ncode.add(1 + LINK_SIZE);
                                }
                                OP_WORD_BOUNDARY | OP_NOT_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY
                                | OP_NOT_UCP_WORD_BOUNDARY => {
                                    ncode = ncode.add(1);
                                }
                                OP_CALLOUT => {
                                    ncode = ncode
                                        .add(tables::_pcre2_OP_lengths_8[OP_CALLOUT as usize]
                                            as usize);
                                }
                                OP_CALLOUT_STR => {
                                    ncode = ncode.add(GET(ncode, 1 + 2 * LINK_SIZE) as usize);
                                }
                                _ => {
                                    done = true;
                                }
                            }
                        }

                        // Now check the next significant item.
                        let mut fell_through = false;
                        match *ncode as u32 {
                            OP_PROP => {
                                if *ncode.add(1) as u32 != PT_CLIST as u32 {
                                    // break out of the inner switch (default)
                                } else {
                                    tcode = ncode;
                                    continue; // with following significant opcode
                                }
                            }
                            OP_ANYNL | OP_CHAR | OP_CHARI | OP_EXACT | OP_EXACTI | OP_HSPACE
                            | OP_MINPLUS | OP_MINPLUSI | OP_PLUS | OP_PLUSI | OP_POSPLUS
                            | OP_POSPLUSI | OP_VSPACE | OP_DIGIT | OP_NOT_DIGIT | OP_WORDCHAR
                            | OP_NOT_WORDCHAR | OP_WHITESPACE | OP_NOT_WHITESPACE => {
                                tcode = ncode;
                                continue; // with following significant opcode
                            }
                            _ => {}
                        }
                        let _ = fell_through;

                        // Fall through to group-bracket handling.
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
                            return rc;
                        }
                    }

                    OP_BRA | OP_SBRA | OP_CBRA | OP_SCBRA | OP_BRAPOS | OP_SBRAPOS
                    | OP_CBRAPOS | OP_SCBRAPOS | OP_ONCE | OP_SCRIPT_RUN => {
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
                            return rc;
                        }
                    }

                    OP_ALT => {
                        yield_ = SSB_CONTINUE;
                        try_next = FALSE;
                    }

                    OP_KET | OP_KETRMAX | OP_KETRMIN | OP_KETRPOS => {
                        return SSB_CONTINUE;
                    }

                    OP_CALLOUT => {
                        tcode =
                            tcode.add(tables::_pcre2_OP_lengths_8[OP_CALLOUT as usize] as usize);
                    }

                    OP_CALLOUT_STR => {
                        tcode = tcode.add(GET(tcode, 1 + 2 * LINK_SIZE) as usize);
                    }

                    OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT | OP_ASSERTBACK_NA
                    | OP_ASSERT_SCS => {
                        loop {
                            tcode = tcode.add(GET(tcode, 1) as usize);
                            if *tcode as u32 != OP_ALT {
                                break;
                            }
                        }
                        tcode = tcode.add(1 + LINK_SIZE);
                    }

                    OP_BRAZERO | OP_BRAMINZERO | OP_BRAPOSZERO => {
                        tcode = tcode.add(1);
                        rc = set_start_bits(re, tcode, utf, ucp, depthptr);
                        if rc == SSB_FAIL || rc == SSB_UNKNOWN || rc == SSB_TOODEEP {
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

                    OP_STAR | OP_MINSTAR | OP_POSSTAR | OP_QUERY | OP_MINQUERY | OP_POSQUERY => {
                        tcode = set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                    }

                    OP_STARI | OP_MINSTARI | OP_POSSTARI | OP_QUERYI | OP_MINQUERYI
                    | OP_POSQUERYI => {
                        tcode = set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                    }

                    OP_UPTO | OP_MINUPTO | OP_POSUPTO => {
                        tcode = set_table_bit(re, tcode.add(1 + IMM2_SIZE), FALSE, utf, ucp);
                    }

                    OP_UPTOI | OP_MINUPTOI | OP_POSUPTOI => {
                        tcode = set_table_bit(re, tcode.add(1 + IMM2_SIZE), TRUE, utf, ucp);
                    }

                    OP_EXACT => {
                        tcode = tcode.add(IMM2_SIZE);
                        let _ = set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                        try_next = FALSE;
                    }

                    OP_CHAR | OP_PLUS | OP_MINPLUS | OP_POSPLUS => {
                        let _ = set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                        try_next = FALSE;
                    }

                    OP_EXACTI => {
                        tcode = tcode.add(IMM2_SIZE);
                        let _ = set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                        try_next = FALSE;
                    }

                    OP_CHARI | OP_PLUSI | OP_MINPLUSI | OP_POSPLUSI => {
                        let _ = set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                        try_next = FALSE;
                    }

                    OP_HSPACE => {
                        set_bit(re, CHAR_HT);
                        set_bit(re, CHAR_SPACE);
                        // 8-bit
                        if utf != FALSE {
                            set_bit(re, 0xC2); // For U+00A0
                            set_bit(re, 0xE1); // For U+1680, U+180E
                            set_bit(re, 0xE2); // For U+2000 - U+200A, U+202F, U+205F
                            set_bit(re, 0xE3); // For U+3000
                        } else {
                            set_bit(re, CHAR_NBSP);
                        }
                        try_next = FALSE;
                    }

                    OP_ANYNL | OP_VSPACE => {
                        set_bit(re, CHAR_LF_V);
                        set_bit(re, CHAR_VT);
                        set_bit(re, CHAR_FF);
                        set_bit(re, CHAR_CR);
                        // 8-bit
                        if utf != FALSE {
                            set_bit(re, 0xC2); // For U+0085 (NEL)
                            set_bit(re, 0xE2); // For U+2028, U+2029
                        } else {
                            set_bit(re, CHAR_NEL);
                        }
                        try_next = FALSE;
                    }

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

                    OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEPOSPLUS => {
                        tcode = tcode.add(1);
                    }

                    OP_TYPEEXACT => {
                        tcode = tcode.add(1 + IMM2_SIZE);
                    }

                    OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO | OP_TYPESTAR
                    | OP_TYPEMINSTAR | OP_TYPEPOSSTAR | OP_TYPEQUERY | OP_TYPEMINQUERY
                    | OP_TYPEPOSQUERY => {
                        // TYPEUPTO/MINUPTO/POSUPTO skip IMM2_SIZE first.
                        if *tcode as u32 == OP_TYPEUPTO
                            || *tcode as u32 == OP_TYPEMINUPTO
                            || *tcode as u32 == OP_TYPEPOSUPTO
                        {
                            tcode = tcode.add(IMM2_SIZE);
                        }

                        match *tcode.add(1) as u32 {
                            OP_HSPACE => {
                                set_bit(re, CHAR_HT);
                                set_bit(re, CHAR_SPACE);
                                if utf != FALSE {
                                    set_bit(re, 0xC2);
                                    set_bit(re, 0xE1);
                                    set_bit(re, 0xE2);
                                    set_bit(re, 0xE3);
                                } else {
                                    set_bit(re, CHAR_NBSP);
                                }
                            }
                            OP_ANYNL | OP_VSPACE => {
                                set_bit(re, CHAR_LF_V);
                                set_bit(re, CHAR_VT);
                                set_bit(re, CHAR_FF);
                                set_bit(re, CHAR_CR);
                                if utf != FALSE {
                                    set_bit(re, 0xC2);
                                    set_bit(re, 0xE2);
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
                            // default, OP_ANY, OP_ALLANY
                            _ => {
                                return SSB_FAIL;
                            }
                        }

                        tcode = tcode.add(2);
                    }

                    // Set-based ECLASS: give up.
                    OP_ECLASS => {
                        return SSB_FAIL;
                    }

                    // XCLASS, NCLASS and CLASS share fall-through code in C.
                    // `goto_handle_classmap` marks the `goto HANDLE_CLASSMAP`
                    // that skips the CLASS-label tcode adjustment.
                    OP_XCLASS | OP_NCLASS | OP_CLASS => {
                        let mut goto_handle_classmap = false;

                        // --- case OP_XCLASS ---
                        if *tcode as u32 == OP_XCLASS {
                            xclassflags = *tcode.add(1 + LINK_SIZE);
                            if (xclassflags as u32 & XCL_HASPROP as u32) != 0
                                || (xclassflags as u32 & (XCL_MAP as u32 | XCL_NOT as u32))
                                    == XCL_NOT as u32
                            {
                                return SSB_FAIL;
                            }

                            classmap = if (xclassflags as u32 & XCL_MAP as u32) == 0 {
                                ptr::null()
                            } else {
                                tcode.add(1 + LINK_SIZE + 1)
                            };

                            // 8-bit UTF: scan the character list and set bits
                            // for leading bytes, then jump to the map.
                            if utf != FALSE && (xclassflags as u32 & XCL_NOT as u32) == 0 {
                                let mut b: u8;
                                let mut e: u8;
                                let mut p: PCRE2_SPTR = tcode.add(
                                    1 + LINK_SIZE + 1 + if classmap.is_null() { 0 } else { 32 },
                                );
                                tcode = tcode.add(GET(tcode, 1) as usize);

                                if *p as u32 >= XCL_LIST as u32 {
                                    study_char_list(
                                        p,
                                        (*re).start_bitmap.as_mut_ptr(),
                                        (re as *const u8).add((*re).code_start),
                                    );
                                    goto_handle_classmap = true;
                                } else {
                                    loop {
                                        let tag = *p as u32;
                                        p = p.add(1);
                                        if tag == XCL_SINGLE as u32 {
                                            b = *p;
                                            p = p.add(1);
                                            while (*p & 0xc0) == 0x80 {
                                                p = p.add(1);
                                            }
                                            (*re).start_bitmap[(b / 8) as usize] |=
                                                1u8 << (b & 7);
                                        } else if tag == XCL_RANGE as u32 {
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
                                                if b == u8::MAX {
                                                    break;
                                                }
                                                b += 1;
                                            }
                                        } else if tag == XCL_END as u32 {
                                            goto_handle_classmap = true;
                                            break;
                                        } else {
                                            return SSB_UNKNOWN;
                                        }
                                    }
                                }
                            }
                            // Otherwise fall through into the NCLASS code below.
                        }

                        // --- case OP_NCLASS (also fall-through target of the
                        // positive XCLASS branch when it did not jump) ---
                        if !goto_handle_classmap {
                            // In C, `case OP_XCLASS:` falls through into
                            // `case OP_NCLASS:` whenever it did not jump to
                            // HANDLE_CLASSMAP, so this block runs for both.
                            // `case OP_CLASS:` is a separate label and skips it.
                            if *tcode as u32 == OP_NCLASS || *tcode as u32 == OP_XCLASS {
                                if utf != FALSE {
                                    (*re).start_bitmap[24] |= 0xf0; // 0xc4 - 0xc8
                                    for i in 25..32 {
                                        (*re).start_bitmap[i] = 0xff; // 0xc9 - 0xff
                                    }
                                }
                            }

                            // --- case OP_CLASS ---
                            if *tcode as u32 == OP_XCLASS {
                                tcode = tcode.add(GET(tcode, 1) as usize);
                            } else {
                                tcode = tcode.add(1);
                                classmap = tcode;
                                tcode = tcode.add(32);
                            }
                        }

                        // HANDLE_CLASSMAP
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
                                        let d = (c >> 6) | 0xc0;
                                        (*re).start_bitmap[(d / 8) as usize] |= 1u8 << (d & 7);
                                        c = (c & 0xc0) + 0x40 - 1;
                                    }
                                    c += 1;
                                }
                            } else {
                                c = 0;
                                while c < 32 {
                                    (*re).start_bitmap[c as usize] |= *classmap.add(c as usize);
                                    c += 1;
                                }
                            }
                        }

                        // Act on what follows the class.
                        match *tcode as u32 {
                            OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY
                            | OP_CRPOSSTAR | OP_CRPOSQUERY => {
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
                    }

                    // If we reach something we don't understand.
                    _ => {
                        return SSB_UNKNOWN;
                    }
                }
                let _ = xclassflags;
            }

            code = code.add(GET(code, 1) as usize); // Advance to next branch
            if *code as u32 != OP_ALT {
                break;
            }
        }

        yield_
    }
}

// ---------------------------------------------------------------------------
// PRIV(study)  ->  _pcre2_study_8
// ---------------------------------------------------------------------------

/// Study a compiled expression, producing the starting-code-unit bitmap and the
/// minimum match length.
///
/// Returns 0 normally; non-zero should never normally occur:
///   1 unknown opcode in set_start_bits
///   2 missing capturing bracket
///   3 unknown opcode in find_minlength
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_study_8(re: *mut pcre2_real_code) -> c_int {
    unsafe {
        let mut count: c_int = 0;
        let utf: BOOL = if ((*re).overall_options & PCRE2_UTF as u32) != 0 {
            TRUE
        } else {
            FALSE
        };
        let ucp: BOOL = if ((*re).overall_options & PCRE2_UCP as u32) != 0 {
            TRUE
        } else {
            FALSE
        };

        // Find start of compiled code.
        let code: PCRE2_SPTR = (re as *const u8).add((*re).code_start);

        // For a pattern that has a first code unit, or a multiline pattern that
        // matches only at "line start", there is no point in seeking a list of
        // starting code units.
        if ((*re).flags & (PCRE2_FIRSTSET as u32 | PCRE2_STARTLINE as u32)) == 0 {
            let mut depth: c_int = 0;
            let rc = set_start_bits(re, code, utf, ucp, &mut depth);
            if rc == SSB_UNKNOWN {
                return 1;
            }

            // If a list of starting code units was set up, scan the list to see
            // if only one or two were listed.
            if rc == SSB_DONE {
                let mut a: c_int = -1;
                let mut b: c_int = -1;
                let mut flags: u32 = PCRE2_FIRSTMAPSET as u32;
                let mut done = false;

                let mut i: usize = 0;
                let mut pidx: usize = 0;
                while i < 256 {
                    let x = (*re).start_bitmap[pidx];
                    if x != 0 {
                        let y = x & (!x).wrapping_add(1); // Least significant bit
                        if y != x {
                            done = true; // More than one bit set
                            break;
                        }

                        // Compute the character value.
                        let mut cc_val: c_int = i as c_int;
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

                        // 8-bit UTF: only values < 128 can be used.
                        if utf != FALSE && cc_val > 127 {
                            done = true;
                            break;
                        }

                        if a < 0 {
                            a = cc_val; // First one found, save in a
                        } else if b < 0 {
                            // Second one found
                            let mut d = TABLE_GET(
                                cc_val as u32,
                                (*re).tables.add(fcc_offset as usize),
                                cc_val as u32,
                            ) as c_int;

                            if utf != FALSE || ucp != FALSE {
                                if UCD_CASESET(cc_val as u32) != 0 {
                                    done = true; // Multiple case set
                                    break;
                                }
                                if cc_val > 127 {
                                    d = UCD_OTHERCASE(cc_val as u32) as c_int;
                                }
                            }

                            if d != a {
                                done = true; // Not the other case of a
                                break;
                            }
                            b = cc_val; // Save second in b
                        } else {
                            done = true; // More than two characters found
                            break;
                        }
                    }
                    pidx += 1;
                    i += 8;
                }

                if !done {
                    // Replace the start code unit bits with a first code unit.
                    if a >= 0 {
                        if ((*re).flags & PCRE2_LASTSET as u32) != 0
                            && ((*re).last_codeunit == a as u32
                                || (b >= 0 && (*re).last_codeunit == b as u32))
                        {
                            (*re).flags &=
                                !(PCRE2_LASTSET as u32 | PCRE2_LASTCASELESS as u32);
                            (*re).last_codeunit = 0;
                        }
                        (*re).first_codeunit = a as u32;
                        flags = PCRE2_FIRSTSET as u32;
                        if b >= 0 {
                            flags |= PCRE2_FIRSTCASELESS as u32;
                        }
                    }
                }

                // DONE:
                (*re).flags |= flags;
            }
        }

        // Find the minimum length of subject string.
        if ((*re).flags & (PCRE2_MATCH_EMPTY as u32 | PCRE2_HASACCEPT as u32)) == 0
            && (*re).top_backref as usize <= MAX_CACHE_BACKREF
        {
            let mut backref_cache: [c_int; MAX_CACHE_BACKREF + 1] = [0; MAX_CACHE_BACKREF + 1];
            backref_cache[0] = 0; // Highest one that is set
            let min = find_minlength(
                re,
                code,
                code,
                utf,
                ptr::null_mut(),
                &mut count,
                backref_cache.as_mut_ptr(),
            );
            match min {
                -1 => {
                    // Leave minlength unchanged (will be zero)
                }
                -2 => {
                    return 2; // missing capturing bracket
                }
                -3 => {
                    return 3; // unrecognized opcode
                }
                _ => {
                    (*re).minlength = if min > u16::MAX as c_int {
                        u16::MAX
                    } else {
                        min as u16
                    };
                }
            }
        }

        0
    }
}
