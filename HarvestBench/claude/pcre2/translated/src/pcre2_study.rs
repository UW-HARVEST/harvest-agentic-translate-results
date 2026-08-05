// Translated from pcre2_study.c (PCRE2 10.48, 8-bit, SUPPORT_UNICODE, no JIT,
// LINK_SIZE=2, IMM2_SIZE=2). Behavior preserved byte-for-byte.

use core::ffi::c_int;
use crate::pcre2_internal::*;
use crate::pcre2_find_bracket::_pcre2_find_bracket_8;
use crate::pcre2_ord2utf::_pcre2_ord2utf_8;

// The maximum remembered capturing brackets minimum.
const MAX_CACHE_BACKREF: usize = 128;

// Returns from set_start_bits()
const SSB_FAIL: c_int = 0;
const SSB_DONE: c_int = 1;
const SSB_CONTINUE: c_int = 2;
const SSB_UNKNOWN: c_int = 3;
const SSB_TOODEEP: c_int = 4;

// Set a bit in the starting code unit bit map.
#[inline]
unsafe fn set_bit(re: *mut pcre2_real_code, c: u32) {
    (*re).start_bitmap[(c / 8) as usize] |= 1u8 << (c & 7);
}

#[inline]
unsafe fn op_len(op: u8) -> usize {
    _pcre2_OP_lengths_8[op as usize] as usize
}

/*************************************************
*   Find the minimum subject length for a group  *
*************************************************/

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
    let dupcapused: BOOL = if ((*re).flags & PCRE2_DUPCAPUSED) != 0 { TRUE } else { FALSE };
    let mut nextbranch: PCRE2_SPTR = code.add(GET(code, 1) as usize);
    let mut cc: PCRE2_SPTR = code.add(1 + LINK_SIZE);
    let mut this_recurse = recurse_check { prev: core::ptr::null_mut(), group: core::ptr::null() };

    // If this is a "could be empty" group, its minimum length is 0.
    if *code >= OP_SBRA && *code <= OP_SCOND {
        return 0;
    }

    // Skip over capturing bracket number
    if *code == OP_CBRA || *code == OP_CBRAPOS {
        cc = cc.add(IMM2_SIZE);
    }

    // A large and/or complex regex can take too long to process.
    {
        let v = *countptr;
        *countptr += 1;
        if v > 1000 {
            return -1;
        }
    }

    loop {
        if branchlength >= u16::MAX as c_int {
            branchlength = u16::MAX as c_int;
            cc = nextbranch;
        }

        let op: u8 = *cc;
        match op {
            OP_COND | OP_SCOND => {
                let cs = cc.add(GET(cc, 1) as usize);
                if *cs != OP_ALT {
                    cc = cs.add(1 + LINK_SIZE);
                } else {
                    // PROCESS_NON_CAPTURE
                    let d = find_minlength(re, cc, startcode, utf, recurses, countptr, backref_cache);
                    if d < 0 {
                        return d;
                    }
                    branchlength += d;
                    loop {
                        cc = cc.add(GET(cc, 1) as usize);
                        if *cc != OP_ALT { break; }
                    }
                    cc = cc.add(1 + LINK_SIZE);
                }
            }

            OP_BRA => {
                if *cc.add(1 + LINK_SIZE) == OP_RECURSE && *cc.add(2 * (1 + LINK_SIZE)) == OP_KET {
                    once_fudge = (1 + LINK_SIZE) as u32;
                    cc = cc.add(1 + LINK_SIZE);
                } else {
                    // Fall through to PROCESS_NON_CAPTURE
                    let d = find_minlength(re, cc, startcode, utf, recurses, countptr, backref_cache);
                    if d < 0 {
                        return d;
                    }
                    branchlength += d;
                    loop {
                        cc = cc.add(GET(cc, 1) as usize);
                        if *cc != OP_ALT { break; }
                    }
                    cc = cc.add(1 + LINK_SIZE);
                }
            }

            OP_ONCE | OP_SCRIPT_RUN | OP_SBRA | OP_BRAPOS | OP_SBRAPOS => {
                // PROCESS_NON_CAPTURE
                let d = find_minlength(re, cc, startcode, utf, recurses, countptr, backref_cache);
                if d < 0 {
                    return d;
                }
                branchlength += d;
                loop {
                    cc = cc.add(GET(cc, 1) as usize);
                    if *cc != OP_ALT { break; }
                }
                cc = cc.add(1 + LINK_SIZE);
            }

            OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS => {
                let recno = GET2(cc, 1 + LINK_SIZE) as c_int;
                if dupcapused != FALSE || recno != prev_cap_recno {
                    prev_cap_recno = recno;
                    prev_cap_d = find_minlength(re, cc, startcode, utf, recurses, countptr, backref_cache);
                    if prev_cap_d < 0 {
                        return prev_cap_d;
                    }
                }
                branchlength += prev_cap_d;
                loop {
                    cc = cc.add(GET(cc, 1) as usize);
                    if *cc != OP_ALT { break; }
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
                if op != OP_ALT || length == 0 {
                    return length;
                }
                nextbranch = cc.add(GET(cc, 1) as usize);
                cc = cc.add(1 + LINK_SIZE);
                branchlength = 0;
                had_recurse = FALSE;
            }

            OP_ASSERT | OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT
            | OP_ASSERT_NA | OP_ASSERT_SCS | OP_ASSERTBACK_NA => {
                loop {
                    cc = cc.add(GET(cc, 1) as usize);
                    if *cc != OP_ALT { break; }
                }
                // Fall through
                cc = cc.add(op_len(*cc));
            }

            OP_REVERSE | OP_VREVERSE | OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF
            | OP_FALSE | OP_TRUE | OP_CALLOUT | OP_SOD | OP_SOM | OP_EOD | OP_EODN
            | OP_CIRC | OP_CIRCM | OP_DOLL | OP_DOLLM | OP_NOT_WORD_BOUNDARY
            | OP_WORD_BOUNDARY | OP_NOT_UCP_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY => {
                cc = cc.add(op_len(*cc));
            }

            OP_CALLOUT_STR => {
                cc = cc.add(GET(cc, 1 + 2 * LINK_SIZE) as usize);
            }

            OP_BRAZERO | OP_BRAMINZERO | OP_BRAPOSZERO | OP_SKIPZERO => {
                cc = cc.add(op_len(op));
                loop {
                    cc = cc.add(GET(cc, 1) as usize);
                    if *cc != OP_ALT { break; }
                }
                cc = cc.add(1 + LINK_SIZE);
            }

            OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_PLUS | OP_PLUSI | OP_MINPLUS
            | OP_MINPLUSI | OP_POSPLUS | OP_POSPLUSI | OP_NOTPLUS | OP_NOTPLUSI
            | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_NOTPOSPLUS | OP_NOTPOSPLUSI => {
                branchlength += 1;
                cc = cc.add(2);
                if utf != FALSE && HAS_EXTRALEN(*cc.sub(1) as u32) {
                    cc = cc.add(GET_EXTRALEN(*cc.sub(1) as u32) as usize);
                }
            }

            OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEPOSPLUS => {
                branchlength += 1;
                cc = cc.add(if *cc.add(1) == OP_PROP || *cc.add(1) == OP_NOTPROP { 4 } else { 2 });
            }

            OP_EXACT | OP_EXACTI | OP_NOTEXACT | OP_NOTEXACTI => {
                branchlength += GET2(cc, 1) as c_int;
                cc = cc.add(2 + IMM2_SIZE);
                if utf != FALSE && HAS_EXTRALEN(*cc.sub(1) as u32) {
                    cc = cc.add(GET_EXTRALEN(*cc.sub(1) as u32) as usize);
                }
            }

            OP_TYPEEXACT => {
                branchlength += GET2(cc, 1) as c_int;
                cc = cc.add(2 + IMM2_SIZE
                    + if *cc.add(1 + IMM2_SIZE) == OP_PROP || *cc.add(1 + IMM2_SIZE) == OP_NOTPROP {
                        2
                    } else {
                        0
                    });
            }

            OP_PROP | OP_NOTPROP => {
                cc = cc.add(2);
                // Fall through
                branchlength += 1;
                cc = cc.add(1);
            }

            OP_NOT_DIGIT | OP_DIGIT | OP_NOT_WHITESPACE | OP_WHITESPACE
            | OP_NOT_WORDCHAR | OP_WORDCHAR | OP_ANY | OP_ALLANY | OP_EXTUNI
            | OP_HSPACE | OP_NOT_HSPACE | OP_VSPACE | OP_NOT_VSPACE => {
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

            OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEQUERY | OP_TYPEMINQUERY
            | OP_TYPEPOSSTAR | OP_TYPEPOSQUERY => {
                if *cc.add(1) == OP_PROP || *cc.add(1) == OP_NOTPROP {
                    cc = cc.add(2);
                }
                cc = cc.add(op_len(op));
            }

            OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO => {
                if *cc.add(1 + IMM2_SIZE) == OP_PROP || *cc.add(1 + IMM2_SIZE) == OP_NOTPROP {
                    cc = cc.add(2);
                }
                cc = cc.add(op_len(op));
            }

            OP_CLASS | OP_NCLASS | OP_XCLASS | OP_ECLASS => {
                if op == OP_XCLASS || op == OP_ECLASS {
                    cc = cc.add(GET(cc, 1) as usize);
                } else {
                    cc = cc.add(op_len(OP_CLASS));
                }

                match *cc {
                    OP_CRPLUS | OP_CRMINPLUS | OP_CRPOSPLUS => {
                        branchlength += 1;
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

            OP_DNREF | OP_DNREFI => {
                let mut d: c_int;
                if dupcapused == FALSE && ((*re).overall_options & PCRE2_MATCH_UNSET_BACKREF) == 0 {
                    let mut count = GET2(cc, 1 + IMM2_SIZE) as c_int;
                    let mut slot: PCRE2_SPTR = (re as *const u8)
                        .add(core::mem::size_of::<pcre2_real_code>())
                        .add((GET2(cc, 1) as usize) * ((*re).name_entry_size as usize));

                    d = c_int::MAX;

                    while count > 0 {
                        count -= 1;
                        let mut dd: c_int;
                        let recno = GET2(slot, 0) as c_int;

                        if recno <= *backref_cache.add(0) && *backref_cache.add(recno as usize) >= 0 {
                            dd = *backref_cache.add(recno as usize);
                        } else {
                            let cs = _pcre2_find_bracket_8(startcode, utf, recno);
                            let mut ce = cs;
                            if cs.is_null() {
                                return -2;
                            }
                            loop {
                                ce = ce.add(GET(ce, 1) as usize);
                                if *ce != OP_ALT { break; }
                            }

                            dd = 0;
                            if dupcapused == FALSE || _pcre2_find_bracket_8(ce, utf, recno).is_null() {
                                if cc > cs && cc < ce {
                                    had_recurse = TRUE;
                                } else {
                                    let mut r = recurses;
                                    while !r.is_null() {
                                        if (*r).group == cs { break; }
                                        r = (*r).prev;
                                    }
                                    if !r.is_null() {
                                        had_recurse = TRUE;
                                    } else {
                                        this_recurse.prev = recurses;
                                        this_recurse.group = cs;
                                        dd = find_minlength(re, cs, startcode, utf,
                                            &mut this_recurse as *mut recurse_check, countptr, backref_cache);
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
                cc = cc.add(op_len(*cc));
                // REPEAT_BACK_REFERENCE
                let min: c_int;
                match *cc {
                    OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY
                    | OP_CRPOSSTAR | OP_CRPOSQUERY => {
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

                if (d > 0 && (c_int::MAX / d) < min)
                    || (u16::MAX as c_int) - branchlength < min.wrapping_mul(d)
                {
                    branchlength = u16::MAX as c_int;
                } else {
                    branchlength += min.wrapping_mul(d);
                }
            }

            OP_REF | OP_REFI => {
                let recno = GET2(cc, 1) as c_int;
                let mut d: c_int;
                if recno <= *backref_cache.add(0) && *backref_cache.add(recno as usize) >= 0 {
                    d = *backref_cache.add(recno as usize);
                } else {
                    d = 0;

                    if ((*re).overall_options & PCRE2_MATCH_UNSET_BACKREF) == 0 {
                        let cs = _pcre2_find_bracket_8(startcode, utf, recno);
                        let mut ce = cs;
                        if cs.is_null() {
                            return -2;
                        }
                        loop {
                            ce = ce.add(GET(ce, 1) as usize);
                            if *ce != OP_ALT { break; }
                        }

                        if dupcapused == FALSE || _pcre2_find_bracket_8(ce, utf, recno).is_null() {
                            if cc > cs && cc < ce {
                                had_recurse = TRUE;
                            } else {
                                let mut r = recurses;
                                while !r.is_null() {
                                    if (*r).group == cs { break; }
                                    r = (*r).prev;
                                }
                                if !r.is_null() {
                                    had_recurse = TRUE;
                                } else {
                                    this_recurse.prev = recurses;
                                    this_recurse.group = cs;
                                    d = find_minlength(re, cs, startcode, utf,
                                        &mut this_recurse as *mut recurse_check, countptr, backref_cache);
                                    if d < 0 {
                                        return d;
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
                    } else {
                        *backref_cache.add(recno as usize) = d;
                        let mut i = *backref_cache.add(0) + 1;
                        while i < recno {
                            *backref_cache.add(i as usize) = -1;
                            i += 1;
                        }
                        *backref_cache.add(0) = recno;
                    }
                }

                cc = cc.add(op_len(*cc));

                // REPEAT_BACK_REFERENCE
                let min: c_int;
                match *cc {
                    OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY
                    | OP_CRPOSSTAR | OP_CRPOSQUERY => {
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

                if (d > 0 && (c_int::MAX / d) < min)
                    || (u16::MAX as c_int) - branchlength < min.wrapping_mul(d)
                {
                    branchlength = u16::MAX as c_int;
                } else {
                    branchlength += min.wrapping_mul(d);
                }
            }

            OP_RECURSE => {
                let cs = startcode.add(GET(cc, 1) as usize);
                let mut ce = cs;
                let recno = GET2(cs, 1 + LINK_SIZE) as c_int;
                if recno == prev_recurse_recno {
                    branchlength += prev_recurse_d;
                } else {
                    loop {
                        ce = ce.add(GET(ce, 1) as usize);
                        if *ce != OP_ALT { break; }
                    }
                    if cc > cs && cc < ce {
                        had_recurse = TRUE;
                    } else {
                        let mut r = recurses;
                        while !r.is_null() {
                            if (*r).group == cs { break; }
                            r = (*r).prev;
                        }
                        if !r.is_null() {
                            had_recurse = TRUE;
                        } else {
                            this_recurse.prev = recurses;
                            this_recurse.group = cs;
                            prev_recurse_d = find_minlength(re, cs, startcode, utf,
                                &mut this_recurse as *mut recurse_check, countptr, backref_cache);
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
            | OP_NOTMINUPTO | OP_NOTMINUPTOI | OP_POSUPTO | OP_POSUPTOI
            | OP_NOTPOSUPTO | OP_NOTPOSUPTOI | OP_STAR | OP_STARI | OP_NOTSTAR
            | OP_NOTSTARI | OP_MINSTAR | OP_MINSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI
            | OP_POSSTAR | OP_POSSTARI | OP_NOTPOSSTAR | OP_NOTPOSSTARI | OP_QUERY
            | OP_QUERYI | OP_NOTQUERY | OP_NOTQUERYI | OP_MINQUERY | OP_MINQUERYI
            | OP_NOTMINQUERY | OP_NOTMINQUERYI | OP_POSQUERY | OP_POSQUERYI
            | OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                cc = cc.add(op_len(op));
                if utf != FALSE && HAS_EXTRALEN(*cc.sub(1) as u32) {
                    cc = cc.add(GET_EXTRALEN(*cc.sub(1) as u32) as usize);
                }
            }

            OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                cc = cc.add(op_len(op) + *cc.add(1) as usize);
            }

            OP_CLOSE | OP_COMMIT | OP_FAIL | OP_PRUNE | OP_SET_SOM | OP_SKIP | OP_THEN => {
                cc = cc.add(op_len(op));
            }

            _ => {
                return -3;
            }
        }
    }
}

/*************************************************
*      Set a bit and maybe its alternate case    *
*************************************************/

unsafe fn set_table_bit(
    re: *mut pcre2_real_code,
    mut p: PCRE2_SPTR,
    caseless: BOOL,
    utf: BOOL,
    ucp: BOOL,
) -> PCRE2_SPTR {
    let mut c: u32 = *p as u32; // First code unit
    p = p.add(1);

    set_bit(re, c);

    // In UTF-8 mode, pick up the remaining code units to find the end of the
    // character, even when caseless.
    if utf != FALSE {
        if c >= 0xc0 {
            let extra = GET_EXTRALEN(c);
            c = getutf8(c, p.sub(1));
            p = p.add(extra as usize);
        }
    }

    // If caseless, handle the other case of the character.
    if caseless != FALSE {
        if utf != FALSE || ucp != FALSE {
            c = UCD_OTHERCASE(c);
            if utf != FALSE {
                let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
                _pcre2_ord2utf_8(c, buff.as_mut_ptr());
                set_bit(re, buff[0] as u32);
            } else if c < 256 {
                set_bit(re, c);
            }
        } else {
            // Not UTF or UCP; MAX_255 is always true in 8-bit mode.
            set_bit(re, *(*re).tables.add(fcc_offset + c as usize) as u32);
        }
    }

    p
}

/*************************************************
*     Set bits for a positive character type     *
*************************************************/

unsafe fn set_type_bits(re: *mut pcre2_real_code, cbit_type: c_int, table_limit: u32) {
    let mut c: u32 = 0;
    while c < table_limit {
        (*re).start_bitmap[c as usize] |=
            *(*re).tables.add(c as usize + cbits_offset + cbit_type as usize);
        c += 1;
    }
    if table_limit == 32 {
        return;
    }
    let mut c: u32 = 128;
    while c < 256 {
        if (*(*re).tables.add(cbits_offset + (c / 8) as usize) & (1u8 << (c & 7))) != 0 {
            let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
            _pcre2_ord2utf_8(c, buff.as_mut_ptr());
            set_bit(re, buff[0] as u32);
        }
        c += 1;
    }
}

/*************************************************
*     Set bits for a negative character type     *
*************************************************/

unsafe fn set_nottype_bits(re: *mut pcre2_real_code, cbit_type: c_int, table_limit: u32) {
    let mut c: u32 = 0;
    while c < table_limit {
        (*re).start_bitmap[c as usize] |=
            !(*(*re).tables.add(c as usize + cbits_offset + cbit_type as usize));
        c += 1;
    }
    if table_limit != 32 {
        let mut c: u32 = 24;
        while c < 32 {
            (*re).start_bitmap[c as usize] = 0xff;
            c += 1;
        }
    }
}

/*************************************************
*     Set starting bits for a character list.    *
*************************************************/

unsafe fn study_char_list(
    mut code: PCRE2_SPTR,
    start_bitmap: *mut u8,
    char_lists_end: *const u8,
) {
    let mut char_list_add: u32 = XCL_CHAR_LIST_LOW_16_ADD;
    let mut range_start: u32 = !0u32;
    let mut range_end: u32;
    let mut start_buffer: [PCRE2_UCHAR; 6] = [0; 6];
    let mut end_buffer: [PCRE2_UCHAR; 6] = [0; 6];
    let mut start: PCRE2_UCHAR;
    let mut end: PCRE2_UCHAR;

    let mut type_: u32 = ((*code.add(0) as u32) << 8) | *code.add(1) as u32;
    code = code.add(2);

    let mut next_char: *const u8 = char_lists_end.sub((GET(code, 0) as usize) << 1);
    type_ &= XCL_TYPE_MASK;
    let mut list_ind: u32 = 0;

    if (type_ & XCL_BEGIN_WITH_RANGE) != 0 {
        range_start = XCL_CHAR_LIST_LOW_16_START;
    }

    while type_ > 0 {
        let mut item_count: u32 = type_ & XCL_ITEM_COUNT_MASK;

        if item_count == XCL_ITEM_COUNT_MASK {
            if list_ind <= 1 {
                item_count = core::ptr::read_unaligned(next_char as *const u16) as u32;
                next_char = next_char.add(2);
            } else {
                item_count = core::ptr::read_unaligned(next_char as *const u32);
                next_char = next_char.add(4);
            }
        }

        while item_count > 0 {
            if list_ind <= 1 {
                range_end = core::ptr::read_unaligned(next_char as *const u16) as u32;
                next_char = next_char.add(2);
            } else {
                range_end = core::ptr::read_unaligned(next_char as *const u32);
                next_char = next_char.add(4);
            }

            if (range_end & XCL_CHAR_END) != 0 {
                range_end = char_list_add + (range_end >> XCL_CHAR_SHIFT);

                _pcre2_ord2utf_8(range_end, end_buffer.as_mut_ptr());
                end = end_buffer[0];

                if range_start < range_end {
                    _pcre2_ord2utf_8(range_start, start_buffer.as_mut_ptr());
                    start = start_buffer[0];
                    if start <= end {
                        loop {
                            *start_bitmap.add((start / 8) as usize) |= 1u8 << (start & 7);
                            if start == end { break; }
                            start += 1;
                        }
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
                if list_ind == 1 {
                    range_start = XCL_CHAR_LIST_HIGH_16_START;
                } else {
                    range_start = XCL_CHAR_LIST_LOW_32_START;
                }
            }
        } else if (type_ & XCL_BEGIN_WITH_RANGE) == 0 {
            _pcre2_ord2utf_8(range_start, start_buffer.as_mut_ptr());

            if list_ind == 1 {
                range_end = XCL_CHAR_LIST_LOW_16_END;
            } else {
                range_end = XCL_CHAR_LIST_HIGH_16_END;
            }

            _pcre2_ord2utf_8(range_end, end_buffer.as_mut_ptr());
            end = end_buffer[0];

            start = start_buffer[0];
            if start <= end {
                loop {
                    *start_bitmap.add((start / 8) as usize) |= 1u8 << (start & 7);
                    if start == end { break; }
                    start += 1;
                }
            }

            range_start = !0u32;
        }

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

unsafe fn set_start_bits(
    re: *mut pcre2_real_code,
    mut code: PCRE2_SPTR,
    utf: BOOL,
    ucp: BOOL,
    depthptr: *mut c_int,
) -> c_int {
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

        if *code == OP_CBRA || *code == OP_SCBRA || *code == OP_CBRAPOS || *code == OP_SCBRAPOS {
            tcode = tcode.add(IMM2_SIZE);
        }

        'try_next: while try_next != FALSE {
            match *tcode {
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

                OP_CIRC => {
                    tcode = tcode.add(op_len(OP_CIRC));
                }

                OP_PROP => {
                    if *tcode.add(1) as u32 != PT_CLIST {
                        return SSB_FAIL;
                    }
                    let mut p = _pcre2_ucd_caseless_sets_8.as_ptr().add(*tcode.add(2) as usize);
                    loop {
                        c = *p;
                        p = p.add(1);
                        if c >= NOTACHAR {
                            break;
                        }
                        if utf != FALSE {
                            let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
                            _pcre2_ord2utf_8(c, buff.as_mut_ptr());
                            c = buff[0] as u32;
                        }
                        if c > 0xff {
                            set_bit(re, 0xff);
                        } else {
                            set_bit(re, c);
                        }
                    }
                    try_next = FALSE;
                }

                OP_WORD_BOUNDARY | OP_NOT_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY
                | OP_NOT_UCP_WORD_BOUNDARY => {
                    tcode = tcode.add(1);
                }

                OP_ASSERT | OP_ASSERT_NA => {
                    let mut ncode = tcode.add(GET(tcode, 1) as usize);
                    while *ncode == OP_ALT {
                        ncode = ncode.add(GET(ncode, 1) as usize);
                    }
                    ncode = ncode.add(1 + LINK_SIZE);

                    // Skip irrelevant items
                    loop {
                        match *ncode {
                            OP_ASSERT | OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT
                            | OP_ASSERT_NA | OP_ASSERTBACK_NA | OP_ASSERT_SCS => {
                                ncode = ncode.add(GET(ncode, 1) as usize);
                                while *ncode == OP_ALT {
                                    ncode = ncode.add(GET(ncode, 1) as usize);
                                }
                                ncode = ncode.add(1 + LINK_SIZE);
                            }
                            OP_WORD_BOUNDARY | OP_NOT_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY
                            | OP_NOT_UCP_WORD_BOUNDARY => {
                                ncode = ncode.add(1);
                            }
                            OP_CALLOUT => {
                                ncode = ncode.add(op_len(OP_CALLOUT));
                            }
                            OP_CALLOUT_STR => {
                                ncode = ncode.add(GET(ncode, 1 + 2 * LINK_SIZE) as usize);
                            }
                            _ => {
                                break;
                            }
                        }
                    }

                    // Now check the next significant item.
                    let mut go_bracket = false;
                    match *ncode {
                        OP_PROP => {
                            if *ncode.add(1) as u32 != PT_CLIST {
                                go_bracket = true;
                            } else {
                                tcode = ncode;
                                continue 'try_next;
                            }
                        }
                        OP_ANYNL | OP_CHAR | OP_CHARI | OP_EXACT | OP_EXACTI | OP_HSPACE
                        | OP_MINPLUS | OP_MINPLUSI | OP_PLUS | OP_PLUSI | OP_POSPLUS
                        | OP_POSPLUSI | OP_VSPACE | OP_DIGIT | OP_NOT_DIGIT | OP_WORDCHAR
                        | OP_NOT_WORDCHAR | OP_WHITESPACE | OP_NOT_WHITESPACE => {
                            tcode = ncode;
                            continue 'try_next;
                        }
                        _ => {
                            go_bracket = true;
                        }
                    }
                    let _ = go_bracket;

                    // Fall through to bracket-group handling.
                    let rc = set_start_bits(re, tcode, utf, ucp, depthptr);
                    if rc == SSB_DONE {
                        try_next = FALSE;
                    } else if rc == SSB_CONTINUE {
                        loop {
                            tcode = tcode.add(GET(tcode, 1) as usize);
                            if *tcode != OP_ALT { break; }
                        }
                        tcode = tcode.add(1 + LINK_SIZE);
                    } else {
                        return rc;
                    }
                }

                OP_BRA | OP_SBRA | OP_CBRA | OP_SCBRA | OP_BRAPOS | OP_SBRAPOS
                | OP_CBRAPOS | OP_SCBRAPOS | OP_ONCE | OP_SCRIPT_RUN => {
                    let rc = set_start_bits(re, tcode, utf, ucp, depthptr);
                    if rc == SSB_DONE {
                        try_next = FALSE;
                    } else if rc == SSB_CONTINUE {
                        loop {
                            tcode = tcode.add(GET(tcode, 1) as usize);
                            if *tcode != OP_ALT { break; }
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
                    tcode = tcode.add(op_len(OP_CALLOUT));
                }

                OP_CALLOUT_STR => {
                    tcode = tcode.add(GET(tcode, 1 + 2 * LINK_SIZE) as usize);
                }

                OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT | OP_ASSERTBACK_NA
                | OP_ASSERT_SCS => {
                    loop {
                        tcode = tcode.add(GET(tcode, 1) as usize);
                        if *tcode != OP_ALT { break; }
                    }
                    tcode = tcode.add(1 + LINK_SIZE);
                }

                OP_BRAZERO | OP_BRAMINZERO | OP_BRAPOSZERO => {
                    tcode = tcode.add(1);
                    let rc = set_start_bits(re, tcode, utf, ucp, depthptr);
                    if rc == SSB_FAIL || rc == SSB_UNKNOWN || rc == SSB_TOODEEP {
                        return rc;
                    }
                    loop {
                        tcode = tcode.add(GET(tcode, 1) as usize);
                        if *tcode != OP_ALT { break; }
                    }
                    tcode = tcode.add(1 + LINK_SIZE);
                }

                OP_SKIPZERO => {
                    tcode = tcode.add(1);
                    loop {
                        tcode = tcode.add(GET(tcode, 1) as usize);
                        if *tcode != OP_ALT { break; }
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
                    set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                    try_next = FALSE;
                }
                OP_CHAR | OP_PLUS | OP_MINPLUS | OP_POSPLUS => {
                    set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                    try_next = FALSE;
                }

                OP_EXACTI => {
                    tcode = tcode.add(IMM2_SIZE);
                    set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                    try_next = FALSE;
                }
                OP_CHARI | OP_PLUSI | OP_MINPLUSI | OP_POSPLUSI => {
                    set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                    try_next = FALSE;
                }

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
                    try_next = FALSE;
                }

                OP_ANYNL | OP_VSPACE => {
                    set_bit(re, CHAR_LF);
                    set_bit(re, CHAR_VT);
                    set_bit(re, CHAR_FF);
                    set_bit(re, CHAR_CR);
                    if utf != FALSE {
                        set_bit(re, 0xC2);
                        set_bit(re, 0xE2);
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
                    if *tcode == OP_TYPEUPTO || *tcode == OP_TYPEMINUPTO || *tcode == OP_TYPEPOSUPTO {
                        tcode = tcode.add(IMM2_SIZE);
                    }
                    match *tcode.add(1) {
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
                            set_bit(re, CHAR_LF);
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
                        _ => {
                            // default, OP_ANY, OP_ALLANY
                            return SSB_FAIL;
                        }
                    }
                    tcode = tcode.add(2);
                }

                OP_ECLASS => {
                    return SSB_FAIL;
                }

                OP_XCLASS | OP_NCLASS | OP_CLASS => {
                    let entry_op = *tcode;
                    let mut classmap: *const u8 = core::ptr::null();
                    let mut go_handle_classmap = false;

                    if entry_op == OP_XCLASS {
                        let xclassflags = *tcode.add(1 + LINK_SIZE);
                        if (xclassflags & XCL_HASPROP) != 0
                            || (xclassflags & (XCL_MAP | XCL_NOT)) == XCL_NOT
                        {
                            return SSB_FAIL;
                        }

                        classmap = if (xclassflags & XCL_MAP) == 0 {
                            core::ptr::null()
                        } else {
                            tcode.add(1 + LINK_SIZE + 1)
                        };

                        if utf != FALSE && (xclassflags & XCL_NOT) == 0 {
                            let mut p = tcode.add(1 + LINK_SIZE + 1
                                + if classmap.is_null() { 0 } else { 32 });
                            tcode = tcode.add(GET(tcode, 1) as usize);

                            if (*p as u32) >= XCL_LIST {
                                study_char_list(
                                    p,
                                    (*re).start_bitmap.as_mut_ptr(),
                                    (re as *const u8).add((*re).code_start),
                                );
                                go_handle_classmap = true;
                            } else {
                                'xlist: loop {
                                    let op0 = *p;
                                    p = p.add(1);
                                    match op0 {
                                        XCL_SINGLE => {
                                            let b = *p;
                                            p = p.add(1);
                                            while (*p & 0xc0) == 0x80 {
                                                p = p.add(1);
                                            }
                                            (*re).start_bitmap[(b / 8) as usize] |= 1u8 << (b & 7);
                                        }
                                        XCL_RANGE => {
                                            let b0 = *p;
                                            p = p.add(1);
                                            while (*p & 0xc0) == 0x80 {
                                                p = p.add(1);
                                            }
                                            let e = *p;
                                            p = p.add(1);
                                            while (*p & 0xc0) == 0x80 {
                                                p = p.add(1);
                                            }
                                            let mut b = b0;
                                            if b <= e {
                                                loop {
                                                    (*re).start_bitmap[(b / 8) as usize] |=
                                                        1u8 << (b & 7);
                                                    if b == e { break; }
                                                    b += 1;
                                                }
                                            }
                                        }
                                        XCL_END => {
                                            go_handle_classmap = true;
                                            break 'xlist;
                                        }
                                        _ => {
                                            return SSB_UNKNOWN;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !go_handle_classmap {
                        // NCLASS body (reached directly or by fall-through from XCLASS).
                        if entry_op == OP_XCLASS || entry_op == OP_NCLASS {
                            if utf != FALSE {
                                (*re).start_bitmap[24] |= 0xf0;
                                for k in 25..32 {
                                    (*re).start_bitmap[k] = 0xff;
                                }
                            }
                        }

                        // CLASS body pointer setup.
                        if *tcode == OP_XCLASS {
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
                            for cc in 0..16 {
                                (*re).start_bitmap[cc] |= *classmap.add(cc);
                            }
                            let mut cv: u32 = 128;
                            while cv < 256 {
                                if (*classmap.add((cv / 8) as usize) & (1u8 << (cv & 7))) != 0 {
                                    let d = (cv >> 6) | 0xc0;
                                    (*re).start_bitmap[(d / 8) as usize] |= 1u8 << (d & 7);
                                    cv = (cv & 0xc0) + 0x40 - 1;
                                }
                                cv += 1;
                            }
                        } else {
                            for cc in 0..32 {
                                (*re).start_bitmap[cc] |= *classmap.add(cc);
                            }
                        }
                    }

                    // Act on what follows the class.
                    match *tcode {
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

                _ => {
                    return SSB_UNKNOWN;
                }
            }
        }

        code = code.add(GET(code, 1) as usize);
        if *code != OP_ALT {
            break;
        }
    }

    yield_
}

/*************************************************
*          Study a compiled expression           *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_study_8(re: *mut pcre2_real_code) -> c_int {
    let mut count: c_int = 0;
    let utf: BOOL = if ((*re).overall_options & PCRE2_UTF) != 0 { TRUE } else { FALSE };
    let ucp: BOOL = if ((*re).overall_options & PCRE2_UCP) != 0 { TRUE } else { FALSE };

    // Find start of compiled code
    let code: PCRE2_SPTR = (re as *const u8).add((*re).code_start);

    if ((*re).flags & (PCRE2_FIRSTSET | PCRE2_STARTLINE)) == 0 {
        let mut depth: c_int = 0;
        let rc = set_start_bits(re, code, utf, ucp, &mut depth);
        if rc == SSB_UNKNOWN {
            return 1;
        }

        if rc == SSB_DONE {
            let mut a: c_int = -1;
            let mut b: c_int = -1;
            let mut flags: u32 = PCRE2_FIRSTMAPSET;

            'done: {
                let mut i: c_int = 0;
                while i < 256 {
                    let x = (*re).start_bitmap[(i / 8) as usize];
                    if x != 0 {
                        let y = x & x.wrapping_neg();
                        if y != x {
                            break 'done;
                        }

                        let mut cv: c_int = i;
                        match x {
                            1 => {}
                            2 => cv += 1,
                            4 => cv += 2,
                            8 => cv += 3,
                            16 => cv += 4,
                            32 => cv += 5,
                            64 => cv += 6,
                            128 => cv += 7,
                            _ => {}
                        }

                        if utf != FALSE && cv > 127 {
                            break 'done;
                        }

                        if a < 0 {
                            a = cv;
                        } else if b < 0 {
                            let mut d: c_int = *(*re).tables.add(fcc_offset + cv as usize) as c_int;

                            if utf != FALSE || ucp != FALSE {
                                if UCD_CASESET(cv as u32) != 0 {
                                    break 'done;
                                }
                                if cv > 127 {
                                    d = UCD_OTHERCASE(cv as u32) as c_int;
                                }
                            }

                            if d != a {
                                break 'done;
                            }
                            b = cv;
                        } else {
                            break 'done;
                        }
                    }
                    i += 8;
                }

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

            (*re).flags |= flags;
        }
    }

    if ((*re).flags & (PCRE2_MATCH_EMPTY | PCRE2_HASACCEPT)) == 0
        && (*re).top_backref as usize <= MAX_CACHE_BACKREF
    {
        let mut backref_cache: [c_int; MAX_CACHE_BACKREF + 1] = [0; MAX_CACHE_BACKREF + 1];
        backref_cache[0] = 0;
        let min = find_minlength(
            re,
            code,
            code,
            utf,
            core::ptr::null_mut(),
            &mut count,
            backref_cache.as_mut_ptr(),
        );
        match min {
            -1 => {}
            -2 => {
                return 2;
            }
            -3 => {
                return 3;
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
