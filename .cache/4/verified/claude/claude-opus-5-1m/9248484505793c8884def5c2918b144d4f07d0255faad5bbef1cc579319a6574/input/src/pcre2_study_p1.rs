/* Translated from c_src/src/pcre2_study.c lines 45-787 */

/* This module contains functions for scanning a compiled pattern and
collecting data (e.g. minimum matching length). */

/* The maximum remembered capturing brackets minimum. */

const MAX_CACHE_BACKREF: usize = 128;

/* Set a bit in the starting code unit bit map. The C macro implicitly used the
local variable `re`, so here it is passed as an extra first argument:
  SET_BIT(c)  =>  SET_BIT!(re, c)  */

macro_rules! SET_BIT {
    ($re:expr, $c:expr) => {{
        let c__: u32 = ($c) as u32;
        *(*$re).start_bitmap.as_mut_ptr().add((c__ / 8) as usize) |= (1u32 << (c__ & 7)) as u8;
    }};
}

/* Returns from set_start_bits() */

const SSB_FAIL: c_int = 0;
const SSB_DONE: c_int = 1;
const SSB_CONTINUE: c_int = 2;
const SSB_UNKNOWN: c_int = 3;
const SSB_TOODEEP: c_int = 4;

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
    let mut nextbranch: PCRE2_SPTR = code.add(GET!(code, 1) as usize);
    let mut cc: PCRE2_SPTR = code.add(1 + LINK_SIZE);
    let mut this_recurse: recurse_check = recurse_check {
        prev: std::ptr::null_mut(),
        group: std::ptr::null(),
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
        let count__ = *countptr;
        *countptr = count__ + 1;
        if count__ > 1000 {
            return -1;
        }
    }

    /* Scan along the opcodes for this branch. If we get to the end of the branch,
    check the length against that of the other branches. If the accumulated length
    passes 16-bits, reset to that value and skip the rest of the branch. */

    'outer: loop {
        let mut d: c_int = 0;
        let min: c_int;
        let mut recno: c_int = 0;
        let op: PCRE2_UCHAR;
        let mut cs: PCRE2_SPTR = std::ptr::null();
        let mut ce: PCRE2_SPTR = std::ptr::null();

        if branchlength >= u16::MAX as c_int {
            branchlength = u16::MAX as c_int;
            cc = nextbranch;
        }

        op = *cc;

        'repeat_back_reference: {
            'process_non_capture: {
                match op as u32 {
                    OP_COND | OP_SCOND => {
                        /* If there is only one branch in a condition, the implied branch has zero
                        length, so we don't add anything. This covers the DEFINE "condition"
                        automatically. If there are two branches we can treat it the same as any
                        other non-capturing subpattern. */

                        cs = cc.add(GET!(cc, 1) as usize);
                        if *cs as u32 != OP_ALT {
                            cc = cs.add(1 + LINK_SIZE);
                            continue 'outer;
                        }
                        break 'process_non_capture; /* goto PROCESS_NON_CAPTURE */
                    }

                    OP_BRA => {
                        /* There's a special case of OP_BRA, when it is wrapped round a repeated
                        OP_RECURSE. We'd like to process the latter at this level so that
                        remembering the value works for repeated cases. So we do nothing, but
                        set a fudge value to skip over the OP_KET after the recurse. */

                        if *cc.add(1 + LINK_SIZE) as u32 == OP_RECURSE
                            && *cc.add(2 * (1 + LINK_SIZE)) as u32 == OP_KET
                        {
                            once_fudge = (1 + LINK_SIZE) as u32;
                            cc = cc.add(1 + LINK_SIZE);
                            continue 'outer;
                        }
                        break 'process_non_capture; /* Fall through */
                    }

                    OP_ONCE | OP_SCRIPT_RUN | OP_SBRA | OP_BRAPOS | OP_SBRAPOS => {
                        break 'process_non_capture; /* PROCESS_NON_CAPTURE */
                    }

                    /* To save time for repeated capturing subpatterns, we remember the
                    length of the previous one. Unfortunately we can't do the same for
                    the unnumbered ones above. Nor can we do this if (?| is present in the
                    pattern because captures with the same number are not then identical. */
                    OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS => {
                        recno = GET2!(cc, 1 + LINK_SIZE) as c_int;
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
                            if *cc as u32 != OP_ALT {
                                break;
                            }
                        }
                        cc = cc.add(1 + LINK_SIZE);
                        continue 'outer;
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
                        if op as u32 != OP_ALT || length == 0 {
                            return length;
                        }
                        nextbranch = cc.add(GET!(cc, 1) as usize);
                        cc = cc.add(1 + LINK_SIZE);
                        branchlength = 0;
                        had_recurse = FALSE;
                        continue 'outer;
                    }

                    /* Skip over assertive subpatterns */
                    OP_ASSERT | OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT
                    | OP_ASSERT_NA | OP_ASSERT_SCS | OP_ASSERTBACK_NA => {
                        loop {
                            cc = cc.add(GET!(cc, 1) as usize);
                            if *cc as u32 != OP_ALT {
                                break;
                            }
                        }
                        /* Fall through into the "don't match chars" group below */
                        cc = cc.add(*_pcre2_OP_lengths_8.as_ptr().add(*cc as usize) as usize);
                        continue 'outer;
                    }

                    /* Skip over things that don't match chars */
                    OP_REVERSE | OP_VREVERSE | OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF
                    | OP_FALSE | OP_TRUE | OP_CALLOUT | OP_SOD | OP_SOM | OP_EOD | OP_EODN
                    | OP_CIRC | OP_CIRCM | OP_DOLL | OP_DOLLM | OP_NOT_WORD_BOUNDARY
                    | OP_WORD_BOUNDARY | OP_NOT_UCP_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY => {
                        cc = cc.add(*_pcre2_OP_lengths_8.as_ptr().add(*cc as usize) as usize);
                        continue 'outer;
                    }

                    OP_CALLOUT_STR => {
                        cc = cc.add(GET!(cc, 1 + 2 * LINK_SIZE) as usize);
                        continue 'outer;
                    }

                    /* Skip over a subpattern that has a {0} or {0,x} quantifier */
                    OP_BRAZERO | OP_BRAMINZERO | OP_BRAPOSZERO | OP_SKIPZERO => {
                        cc = cc.add(*_pcre2_OP_lengths_8.as_ptr().add(*cc as usize) as usize);
                        loop {
                            cc = cc.add(GET!(cc, 1) as usize);
                            if *cc as u32 != OP_ALT {
                                break;
                            }
                        }
                        cc = cc.add(1 + LINK_SIZE);
                        continue 'outer;
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
                        continue 'outer;
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
                        continue 'outer;
                    }

                    /* Handle exact repetitions. The count is already in characters, but we
                    may need to skip over a multibyte character in UTF mode.  */
                    OP_EXACT | OP_EXACTI | OP_NOTEXACT | OP_NOTEXACTI => {
                        branchlength += GET2!(cc, 1) as c_int;
                        cc = cc.add(2 + IMM2_SIZE);
                        if utf != 0 && HAS_EXTRALEN!(*cc.offset(-1)) {
                            cc = cc.add(GET_EXTRALEN!(*cc.offset(-1)) as usize);
                        }
                        continue 'outer;
                    }

                    OP_TYPEEXACT => {
                        branchlength += GET2!(cc, 1) as c_int;
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
                        continue 'outer;
                    }

                    /* Handle single-char non-literal matchers */
                    OP_PROP | OP_NOTPROP => {
                        cc = cc.add(2);
                        /* Fall through */
                        branchlength += 1;
                        cc = cc.add(1);
                        continue 'outer;
                    }

                    OP_NOT_DIGIT | OP_DIGIT | OP_NOT_WHITESPACE | OP_WHITESPACE
                    | OP_NOT_WORDCHAR | OP_WORDCHAR | OP_ANY | OP_ALLANY | OP_EXTUNI
                    | OP_HSPACE | OP_NOT_HSPACE | OP_VSPACE | OP_NOT_VSPACE => {
                        branchlength += 1;
                        cc = cc.add(1);
                        continue 'outer;
                    }

                    /* "Any newline" might match two characters, but it also might match just
                    one. */
                    OP_ANYNL => {
                        branchlength += 1;
                        cc = cc.add(1);
                        continue 'outer;
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
                        continue 'outer;
                    }

                    /* For repeated character types, we have to test for \p and \P, which have
                    an extra two bytes of parameters. */
                    OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEQUERY | OP_TYPEMINQUERY
                    | OP_TYPEPOSSTAR | OP_TYPEPOSQUERY => {
                        if *cc.add(1) as u32 == OP_PROP || *cc.add(1) as u32 == OP_NOTPROP {
                            cc = cc.add(2);
                        }
                        cc = cc.add(*_pcre2_OP_lengths_8.as_ptr().add(op as usize) as usize);
                        continue 'outer;
                    }

                    OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO => {
                        if *cc.add(1 + IMM2_SIZE) as u32 == OP_PROP
                            || *cc.add(1 + IMM2_SIZE) as u32 == OP_NOTPROP
                        {
                            cc = cc.add(2);
                        }
                        cc = cc.add(*_pcre2_OP_lengths_8.as_ptr().add(op as usize) as usize);
                        continue 'outer;
                    }

                    /* Check a class for variable quantification */
                    OP_CLASS | OP_NCLASS | OP_XCLASS | OP_ECLASS => {
                        /* The original code caused an unsigned overflow in 64 bit systems,
                        so now we use a conditional statement. */
                        if op as u32 == OP_XCLASS || op as u32 == OP_ECLASS {
                            cc = cc.add(GET!(cc, 1) as usize);
                        } else {
                            cc = cc
                                .add(*_pcre2_OP_lengths_8.as_ptr().add(OP_CLASS as usize) as usize);
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
                                branchlength += GET2!(cc, 1) as c_int;
                                cc = cc.add(1 + 2 * IMM2_SIZE);
                            }

                            _ => {
                                branchlength += 1;
                            }
                        }
                        continue 'outer;
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
                            let mut count: c_int = GET2!(cc, 1 + IMM2_SIZE) as c_int;
                            let mut slot: PCRE2_SPTR = (re as *const u8)
                                .add(size_of::<pcre2_real_code>())
                                .add((GET2!(cc, 1) * (*re).name_entry_size as u32) as usize);

                            d = c_int::MAX;

                            /* Scan all groups with the same name; find the shortest. */

                            while count > 0 {
                                count -= 1;
                                let mut dd: c_int;
                                recno = GET2!(slot, 0) as c_int;

                                if recno <= *backref_cache
                                    && *backref_cache.offset(recno as isize) >= 0
                                {
                                    dd = *backref_cache.offset(recno as isize);
                                } else {
                                    cs = _pcre2_find_bracket_8(startcode, utf, recno);
                                    ce = cs;
                                    if cs.is_null() {
                                        return -2;
                                    }
                                    loop {
                                        ce = ce.add(GET!(ce, 1) as usize);
                                        if *ce as u32 != OP_ALT {
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
                                    let mut i: c_int = *backref_cache + 1;
                                    while i < recno {
                                        *backref_cache.offset(i as isize) = -1;
                                        i += 1;
                                    }
                                    *backref_cache = recno;
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
                        cc = cc.add(*_pcre2_OP_lengths_8.as_ptr().add(*cc as usize) as usize);
                        break 'repeat_back_reference; /* goto REPEAT_BACK_REFERENCE */
                    }

                    /* Single back reference by number. References by name are converted to by
                    number when there is no duplication. */
                    OP_REF | OP_REFI => {
                        recno = GET2!(cc, 1) as c_int;
                        if recno <= *backref_cache && *backref_cache.offset(recno as isize) >= 0 {
                            d = *backref_cache.offset(recno as isize);
                        } else {
                            d = 0;

                            if ((*re).overall_options & PCRE2_MATCH_UNSET_BACKREF) == 0 {
                                cs = _pcre2_find_bracket_8(startcode, utf, recno);
                                ce = cs;
                                if cs.is_null() {
                                    return -2;
                                }
                                loop {
                                    ce = ce.add(GET!(ce, 1) as usize);
                                    if *ce as u32 != OP_ALT {
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
                            let mut i: c_int = *backref_cache + 1;
                            while i < recno {
                                *backref_cache.offset(i as isize) = -1;
                                i += 1;
                            }
                            *backref_cache = recno;
                        }

                        cc = cc.add(*_pcre2_OP_lengths_8.as_ptr().add(*cc as usize) as usize);

                        /* Fall through into REPEAT_BACK_REFERENCE */
                        break 'repeat_back_reference;
                    }

                    /* Recursion always refers to the first occurrence of a subpattern with a
                    given number. Therefore, we can always make use of caching, even when the
                    pattern contains multiple subpatterns with the same number. */
                    OP_RECURSE => {
                        cs = startcode.add(GET!(cc, 1) as usize);
                        ce = cs;
                        recno = GET2!(cs, 1 + LINK_SIZE) as c_int;
                        if recno == prev_recurse_recno {
                            branchlength += prev_recurse_d;
                        } else {
                            loop {
                                ce = ce.add(GET!(ce, 1) as usize);
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
                        continue 'outer;
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
                    | OP_NOTSTARI | OP_MINSTAR | OP_MINSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI
                    | OP_POSSTAR | OP_POSSTARI | OP_NOTPOSSTAR | OP_NOTPOSSTARI | OP_QUERY
                    | OP_QUERYI | OP_NOTQUERY | OP_NOTQUERYI | OP_MINQUERY | OP_MINQUERYI
                    | OP_NOTMINQUERY | OP_NOTMINQUERYI | OP_POSQUERY | OP_POSQUERYI
                    | OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                        cc = cc.add(*_pcre2_OP_lengths_8.as_ptr().add(op as usize) as usize);
                        if utf != 0 && HAS_EXTRALEN!(*cc.offset(-1)) {
                            cc = cc.add(GET_EXTRALEN!(*cc.offset(-1)) as usize);
                        }
                        continue 'outer;
                    }

                    /* Skip these, but we need to add in the name length. */
                    OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                        cc = cc.add(
                            *_pcre2_OP_lengths_8.as_ptr().add(op as usize) as usize
                                + *cc.add(1) as usize,
                        );
                        continue 'outer;
                    }

                    /* The remaining opcodes are just skipped over. */
                    OP_CLOSE | OP_COMMIT | OP_FAIL | OP_PRUNE | OP_SET_SOM | OP_SKIP | OP_THEN => {
                        cc = cc.add(*_pcre2_OP_lengths_8.as_ptr().add(op as usize) as usize);
                        continue 'outer;
                    }

                    /* This should not occur: we list all opcodes explicitly so that when
                    new ones get added they are properly considered. */
                    _ => {
                        /* PCRE2_DEBUG_UNREACHABLE(); */
                        return -3;
                    }
                }
            }

            /* PROCESS_NON_CAPTURE: */

            d = find_minlength(re, cc, startcode, utf, recurses, countptr, backref_cache);
            if d < 0 {
                return d;
            }
            branchlength += d;
            loop {
                cc = cc.add(GET!(cc, 1) as usize);
                if *cc as u32 != OP_ALT {
                    break;
                }
            }
            cc = cc.add(1 + LINK_SIZE);
            continue 'outer;
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
                min = GET2!(cc, 1) as c_int;
                cc = cc.add(1 + 2 * IMM2_SIZE);
            }

            _ => {
                min = 1;
            }
        }

        /* Take care not to overflow: (1) min and d are ints, so check that their
        product is not greater than INT_MAX. (2) branchlength is limited to
        UINT16_MAX (checked at the top of the loop). */

        if (d > 0 && (c_int::MAX / d) < min)
            || u16::MAX as c_int - branchlength < min.wrapping_mul(d)
        {
            branchlength = u16::MAX as c_int;
        } else {
            branchlength += min * d;
        }

        /* End of switch: continue with the next opcode */
    }

    /* Control should never reach here; the C code has an unreachable
    "return -3;" after the loop to avoid compiler warnings. */
}
