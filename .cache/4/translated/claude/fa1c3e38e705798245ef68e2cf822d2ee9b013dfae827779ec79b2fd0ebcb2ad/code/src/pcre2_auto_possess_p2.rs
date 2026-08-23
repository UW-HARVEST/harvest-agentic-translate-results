/* Translated from c_src/src/pcre2_auto_possess.c lines 547-1177 */

/*************************************************
*    Scan further character sets for match       *
*************************************************/

/* Checks whether the base and the current opcode have a common character, in
which case the base cannot be possessified.

Arguments:
  code        points to the byte code
  utf         TRUE in UTF mode
  ucp         TRUE in UCP mode
  cb          compile data block
  base_list   the data list of the base opcode
  base_end    the end of the base opcode
  rec_limit   points to recursion depth counter

Returns:      TRUE if the auto-possessification is possible
*/

unsafe fn compare_opcodes(
    mut code: PCRE2_SPTR,
    utf: BOOL,
    ucp: BOOL,
    cb: *const compile_block,
    base_list: *const u32,
    base_end: PCRE2_SPTR,
    rec_limit: *mut c_int,
) -> BOOL {
    let mut c: PCRE2_UCHAR;
    let mut list: [u32; MAX_LIST] = [0; MAX_LIST];
    let mut chr_ptr: *const u32 = std::ptr::null();
    let mut ochr_ptr: *const u32;
    let mut list_ptr: *const u32 = std::ptr::null();
    let mut next_code: PCRE2_SPTR;
    let mut xclass_flags: PCRE2_SPTR;
    let mut class_bitset: *const u8;
    let mut set1: *const u8;
    let mut set2: *const u8;
    let mut set_end: *const u8;
    let mut chr: u32;
    let mut accepted: BOOL;
    let mut invert_bits: BOOL;
    let mut entered_a_group: BOOL = FALSE;

    *rec_limit -= 1;
    if *rec_limit <= 0 {
        return FALSE; /* Recursion has gone too deep */
    }

    /* Note: the base_list[1] contains whether the current opcode has a greedy
    (represented by a non-zero value) quantifier. This is a different from
    other character type lists, which store here that the character iterator
    matches to an empty string (also represented by a non-zero value). */

    loop {
        let mut bracode: PCRE2_SPTR;

        /* All operations move the code pointer forward.
        Therefore infinite recursions are not possible. */

        c = *code;

        /* Skip over callouts */

        if c as u32 == OP_CALLOUT {
            code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(c as usize) as usize);
            continue;
        }

        if c as u32 == OP_CALLOUT_STR {
            code = code.add(GET!(code, 1 + 2 * LINK_SIZE) as usize);
            continue;
        }

        /* At the end of a branch, skip to the end of the group and process it. */

        if c as u32 == OP_ALT {
            loop {
                code = code.add(GET!(code, 1) as usize);
                if *code as u32 != OP_ALT {
                    break;
                }
            }
            c = *code;
        }

        /* Inspect the next opcode. */

        /* We can always possessify a greedy iterator at the end of the pattern,
        which is reached after skipping over the final OP_KET. A non-greedy
        iterator must never be possessified. */

        if c as u32 == OP_END {
            return (*base_list.add(1) != 0) as BOOL;
        }
        /* When an iterator is at the end of certain kinds of group we can inspect
        what follows the group by skipping over the closing ket. Note that this
        does not apply to OP_KETRMAX or OP_KETRMIN because what follows any given
        iteration is variable (could be another iteration or could be the next
        item). As these two opcodes are not listed in the next switch, they will
        end up as the next code to inspect, and return FALSE by virtue of being
        unsupported. */
        else if c as u32 == OP_KET || c as u32 == OP_KETRPOS {
            /* The non-greedy case cannot be converted to a possessive form. */

            if *base_list.add(1) == 0 {
                return FALSE;
            }

            /* If the bracket is capturing it might be referenced by an OP_RECURSE
            so its last iterator can never be possessified if the pattern contains
            recursions. (This could be improved by keeping a list of group numbers that
            are called by recursion.) */

            bracode = code.sub(GET!(code, 1) as usize);
            let bc = *bracode as u32;
            if bc == OP_CBRA || bc == OP_SCBRA || bc == OP_CBRAPOS || bc == OP_SCBRAPOS {
                if (*cb).had_recurse != 0 {
                    return FALSE;
                }
            }
            /* A script run might have to backtrack if the iterated item can match
            characters from more than one script. So give up unless repeating an
            explicit character. */
            else if bc == OP_SCRIPT_RUN {
                if *base_list.add(0) != OP_CHAR && *base_list.add(0) != OP_CHARI {
                    return FALSE;
                }
            }
            /* Atomic sub-patterns and forward assertions can always auto-possessify
            their last iterator. However, if the group was entered as a result of
            checking a previous iterator, this is not possible. */
            else if bc == OP_ASSERT || bc == OP_ASSERT_NOT || bc == OP_ONCE {
                return (entered_a_group == 0) as BOOL;
            }
            /* Fixed-length lookbehinds can be treated the same way, but variable
            length lookbehinds must not auto-possessify their last iterator. Note
            that in order to identify a variable length lookbehind we must check
            through all branches, because some may be of fixed length. */
            else if bc == OP_ASSERTBACK || bc == OP_ASSERTBACK_NOT {
                loop {
                    if *bracode.add(1 + LINK_SIZE) as u32 == OP_VREVERSE {
                        return FALSE; /* Variable */
                    }
                    bracode = bracode.add(GET!(bracode, 1) as usize);
                    if *bracode as u32 != OP_ALT {
                        break;
                    }
                }
                return (entered_a_group == 0) as BOOL; /* Not variable length */
            }
            /* Non-atomic assertions - don't possessify last iterator. This needs
            more thought. */
            else if bc == OP_ASSERT_NA || bc == OP_ASSERTBACK_NA {
                return FALSE;
            }

            /* Skip over the bracket and inspect what comes next. */

            code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(c as usize) as usize);
            continue;
        }
        /* Handle cases where the next item is a group. */
        else if c as u32 == OP_ONCE || c as u32 == OP_BRA || c as u32 == OP_CBRA {
            next_code = code.add(GET!(code, 1) as usize);
            code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(c as usize) as usize);

            /* Check each branch. We have to recurse a level for all but the last
            branch. */

            while *next_code as u32 == OP_ALT {
                if compare_opcodes(code, utf, ucp, cb, base_list, base_end, rec_limit) == 0 {
                    return FALSE;
                }
                code = next_code.add(1 + LINK_SIZE);
                next_code = next_code.add(GET!(next_code, 1) as usize);
            }

            entered_a_group = TRUE;
            continue;
        } else if c as u32 == OP_BRAZERO || c as u32 == OP_BRAMINZERO {
            next_code = code.add(1);
            if *next_code as u32 != OP_BRA
                && *next_code as u32 != OP_CBRA
                && *next_code as u32 != OP_ONCE
            {
                return FALSE;
            }

            loop {
                next_code = next_code.add(GET!(next_code, 1) as usize);
                if *next_code as u32 != OP_ALT {
                    break;
                }
            }

            /* The bracket content will be checked by the OP_BRA/OP_CBRA case above. */

            next_code = next_code.add(1 + LINK_SIZE);
            if compare_opcodes(next_code, utf, ucp, cb, base_list, base_end, rec_limit) == 0 {
                return FALSE;
            }

            code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(c as usize) as usize);
            continue;
        }

        /* The next opcode does not need special handling; fall through and use it
        to see if the base can be possessified. */

        /* We now have the next appropriate opcode to compare with the base. Check
        for a supported opcode, and load its properties. */

        code = get_chr_property_list(code, utf, ucp, (*cb).fcc, list.as_mut_ptr());
        if code.is_null() {
            return FALSE; /* Unsupported */
        }

        /* If either opcode is a small character list, set pointers for comparing
        characters from that list with another list, or with a property. */

        if *base_list.add(0) == OP_CHAR {
            chr_ptr = base_list.add(2);
            list_ptr = list.as_ptr();
        } else if list[0] == OP_CHAR {
            chr_ptr = list.as_ptr().add(2);
            list_ptr = base_list;
        }
        /* Character bitsets can also be compared to certain opcodes. */
        else if *base_list.add(0) == OP_CLASS
            || list[0] == OP_CLASS
            /* In 8 bit, non-UTF mode, OP_CLASS and OP_NCLASS are the same. */
            || (utf == 0 && (*base_list.add(0) == OP_NCLASS || list[0] == OP_NCLASS))
        {
            if *base_list.add(0) == OP_CLASS || (utf == 0 && *base_list.add(0) == OP_NCLASS) {
                set1 = base_end.sub(*base_list.add(2) as usize);
                list_ptr = list.as_ptr();
            } else {
                set1 = code.sub(list[2] as usize);
                list_ptr = base_list;
            }

            invert_bits = FALSE;
            let lp0 = *list_ptr.add(0);
            if lp0 == OP_CLASS || lp0 == OP_NCLASS {
                set2 = (if list_ptr == list.as_ptr() { code } else { base_end })
                    .sub(*list_ptr.add(2) as usize);
            } else if lp0 == OP_XCLASS {
                xclass_flags = (if list_ptr == list.as_ptr() { code } else { base_end })
                    .sub(*list_ptr.add(2) as usize)
                    .add(LINK_SIZE);
                if (*xclass_flags as u32 & XCL_HASPROP) != 0 {
                    return FALSE;
                }
                if (*xclass_flags as u32 & XCL_MAP) == 0 {
                    /* No bits are set for characters < 256. */
                    if list[1] == 0 {
                        return ((*xclass_flags as u32 & XCL_NOT) == 0) as BOOL;
                    }
                    /* Might be an empty repeat. */
                    continue;
                }
                set2 = xclass_flags.add(1);
            } else if lp0 == OP_NOT_DIGIT || lp0 == OP_DIGIT {
                if lp0 == OP_NOT_DIGIT {
                    invert_bits = TRUE;
                }
                set2 = (*cb).cbits.add(cbit_digit);
            } else if lp0 == OP_NOT_WHITESPACE || lp0 == OP_WHITESPACE {
                if lp0 == OP_NOT_WHITESPACE {
                    invert_bits = TRUE;
                }
                set2 = (*cb).cbits.add(cbit_space);
            } else if lp0 == OP_NOT_WORDCHAR || lp0 == OP_WORDCHAR {
                if lp0 == OP_NOT_WORDCHAR {
                    invert_bits = TRUE;
                }
                set2 = (*cb).cbits.add(cbit_word);
            } else {
                return FALSE;
            }

            /* Because the bit sets are unaligned bytes, we need to perform byte
            comparison here. */

            set_end = set1.add(32);
            if invert_bits != 0 {
                loop {
                    let s1 = *set1;
                    set1 = set1.add(1);
                    let s2 = *set2;
                    set2 = set2.add(1);
                    if (s1 & !s2) != 0 {
                        return FALSE;
                    }
                    if !(set1 < set_end) {
                        break;
                    }
                }
            } else {
                loop {
                    let s1 = *set1;
                    set1 = set1.add(1);
                    let s2 = *set2;
                    set2 = set2.add(1);
                    if (s1 & s2) != 0 {
                        return FALSE;
                    }
                    if !(set1 < set_end) {
                        break;
                    }
                }
            }

            if list[1] == 0 {
                return TRUE;
            }
            /* Might be an empty repeat. */
            continue;
        }
        /* Some property combinations also acceptable. Unicode property opcodes are
        processed specially; the rest can be handled with a lookup table. */
        else {
            let leftop: u32;
            let rightop: u32;

            leftop = *base_list.add(0);
            rightop = list[0];

            accepted = FALSE; /* Always set in non-unicode case. */
            if leftop == OP_PROP || leftop == OP_NOTPROP {
                if rightop == OP_EOD {
                    accepted = TRUE;
                } else if rightop == OP_PROP || rightop == OP_NOTPROP {
                    let n: c_int;
                    let same: BOOL = (leftop == rightop) as BOOL;
                    let lisprop: BOOL = (leftop == OP_PROP) as BOOL;
                    let risprop: BOOL = (rightop == OP_PROP) as BOOL;
                    let bothprop: BOOL = (lisprop != 0 && risprop != 0) as BOOL;

                    /* There's a table that specifies how each combination is to be
                    processed:
                      0   Always return FALSE (never auto-possessify)
                      1   Character groups are distinct (possessify if both are OP_PROP)
                      2   Check character categories in the same group (general or particular)
                      3   Return TRUE if the two opcodes are not the same
                      ... see comments below
                    */

                    n = *(propposstab.as_ptr() as *const u8)
                        .add(*base_list.add(2) as usize * PT_TABSIZE + list[2] as usize)
                        as c_int;
                    match n {
                        0 => {}
                        1 => accepted = bothprop,
                        2 => {
                            accepted =
                                (((*base_list.add(3) == list[3]) as BOOL) != same) as BOOL;
                        }
                        3 => accepted = (same == 0) as BOOL,

                        /* Left general category, right particular category */
                        4 => {
                            accepted = (risprop != 0
                                && *(catposstab.as_ptr() as *const u8)
                                    .add(*base_list.add(3) as usize * 30 + list[3] as usize)
                                    as c_int
                                    == same) as BOOL;
                        }

                        /* Right general category, left particular category */
                        5 => {
                            accepted = (lisprop != 0
                                && *(catposstab.as_ptr() as *const u8)
                                    .add(list[3] as usize * 30 + *base_list.add(3) as usize)
                                    as c_int
                                    == same) as BOOL;
                        }

                        /* This code is logically tricky. Think hard before fiddling with it.
                        The posspropstab table has four entries per row. Each row relates to
                        one of PCRE's special properties such as ALNUM or SPACE or WORD.
                        Only WORD actually needs all four entries, but using repeats for the
                        others means they can all use the same code below.

                        The first two entries in each row are Unicode general categories, and
                        apply always, because all the characters they include are part of the
                        PCRE character set. The third and fourth entries are a general and a
                        particular category, respectively, that include one or more relevant
                        characters. One or the other is used, depending on whether the check
                        is for a general or a particular category. However, in both cases the
                        category contains more characters than the specials that are defined
                        for the property being tested against. Therefore, it cannot be used
                        in a NOTPROP case.

                        Example: the row for WORD contains ucp_L, ucp_N, ucp_P, ucp_Po.
                        Underscore is covered by ucp_P or ucp_Po. */

                        /* 6: Left alphanum vs right general category */
                        /* 7: Left space vs right general category */
                        /* 8: Left word vs right general category */
                        6 | 7 | 8 => {
                            let p: *const u8 =
                                (posspropstab.as_ptr() as *const u8).add((n - 6) as usize * 4);
                            accepted = (risprop != 0
                                && lisprop
                                    == ((list[3] != *p.add(0) as u32
                                        && list[3] != *p.add(1) as u32
                                        && (list[3] != *p.add(2) as u32 || lisprop == 0))
                                        as BOOL)) as BOOL;
                        }

                        /* 9:  Right alphanum vs left general category */
                        /* 10: Right space vs left general category */
                        /* 11: Right word vs left general category */
                        9 | 10 | 11 => {
                            let p: *const u8 =
                                (posspropstab.as_ptr() as *const u8).add((n - 9) as usize * 4);
                            accepted = (lisprop != 0
                                && risprop
                                    == ((*base_list.add(3) != *p.add(0) as u32
                                        && *base_list.add(3) != *p.add(1) as u32
                                        && (*base_list.add(3) != *p.add(2) as u32
                                            || risprop == 0)) as BOOL)) as BOOL;
                        }

                        /* 12: Left alphanum vs right particular category */
                        /* 13: Left space vs right particular category */
                        /* 14: Left word vs right particular category */
                        12 | 13 | 14 => {
                            let p: *const u8 =
                                (posspropstab.as_ptr() as *const u8).add((n - 12) as usize * 4);
                            accepted = (risprop != 0
                                && lisprop
                                    == ((*(catposstab.as_ptr() as *const u8)
                                        .add(*p.add(0) as usize * 30 + list[3] as usize)
                                        != 0
                                        && *(catposstab.as_ptr() as *const u8)
                                            .add(*p.add(1) as usize * 30 + list[3] as usize)
                                            != 0
                                        && (list[3] != *p.add(3) as u32 || lisprop == 0))
                                        as BOOL)) as BOOL;
                        }

                        /* 15: Right alphanum vs left particular category */
                        /* 16: Right space vs left particular category */
                        /* 17: Right word vs left particular category */
                        15 | 16 | 17 => {
                            let p: *const u8 =
                                (posspropstab.as_ptr() as *const u8).add((n - 15) as usize * 4);
                            accepted = (lisprop != 0
                                && risprop
                                    == ((*(catposstab.as_ptr() as *const u8)
                                        .add(*p.add(0) as usize * 30
                                            + *base_list.add(3) as usize)
                                        != 0
                                        && *(catposstab.as_ptr() as *const u8)
                                            .add(*p.add(1) as usize * 30
                                                + *base_list.add(3) as usize)
                                            != 0
                                        && (*base_list.add(3) != *p.add(3) as u32
                                            || risprop == 0)) as BOOL)) as BOOL;
                        }
                        _ => {}
                    }
                }
            } else {
                accepted = (leftop >= FIRST_AUTOTAB_OP
                    && leftop <= LAST_AUTOTAB_LEFT_OP
                    && rightop >= FIRST_AUTOTAB_OP
                    && rightop <= LAST_AUTOTAB_RIGHT_OP
                    && *(autoposstab.as_ptr() as *const u8).add(
                        (leftop - FIRST_AUTOTAB_OP) as usize * APTCOLS
                            + (rightop - FIRST_AUTOTAB_OP) as usize,
                    ) != 0) as BOOL;
            }

            if accepted == 0 {
                return FALSE;
            }

            if list[1] == 0 {
                return TRUE;
            }
            /* Might be an empty repeat. */
            continue;
        }

        /* Control reaches here only if one of the items is a small character list.
        All characters are checked against the other side. */

        loop {
            chr = *chr_ptr;

            let lp0 = *list_ptr.add(0);

            if lp0 == OP_CHAR {
                ochr_ptr = list_ptr.add(2);
                loop {
                    if chr == *ochr_ptr {
                        return FALSE;
                    }
                    ochr_ptr = ochr_ptr.add(1);
                    if *ochr_ptr == NOTACHAR {
                        break;
                    }
                }
            } else if lp0 == OP_NOT {
                ochr_ptr = list_ptr.add(2);
                loop {
                    if chr == *ochr_ptr {
                        break;
                    }
                    ochr_ptr = ochr_ptr.add(1);
                    if *ochr_ptr == NOTACHAR {
                        break;
                    }
                }
                if *ochr_ptr == NOTACHAR {
                    return FALSE; /* Not found */
                }
            }
            /* Note that OP_DIGIT etc. are generated only when PCRE2_UCP is *not*
            set. When it is set, \d etc. are converted into OP_(NOT_)PROP codes. */
            else if lp0 == OP_DIGIT {
                if chr < 256 && (*(*cb).ctypes.add(chr as usize) & ctype_digit) != 0 {
                    return FALSE;
                }
            } else if lp0 == OP_NOT_DIGIT {
                if chr > 255 || (*(*cb).ctypes.add(chr as usize) & ctype_digit) == 0 {
                    return FALSE;
                }
            } else if lp0 == OP_WHITESPACE {
                if chr < 256 && (*(*cb).ctypes.add(chr as usize) & ctype_space) != 0 {
                    return FALSE;
                }
            } else if lp0 == OP_NOT_WHITESPACE {
                if chr > 255 || (*(*cb).ctypes.add(chr as usize) & ctype_space) == 0 {
                    return FALSE;
                }
            } else if lp0 == OP_WORDCHAR {
                if chr < 255 && (*(*cb).ctypes.add(chr as usize) & ctype_word) != 0 {
                    return FALSE;
                }
            } else if lp0 == OP_NOT_WORDCHAR {
                if chr > 255 || (*(*cb).ctypes.add(chr as usize) & ctype_word) == 0 {
                    return FALSE;
                }
            } else if lp0 == OP_HSPACE {
                match chr {
                    /* HSPACE_CASES */
                    CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000 | 0x2001
                    | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                    | 0x200a | 0x202f | 0x205f | 0x3000 => return FALSE,
                    _ => {}
                }
            } else if lp0 == OP_NOT_HSPACE {
                match chr {
                    /* HSPACE_CASES */
                    CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000 | 0x2001
                    | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                    | 0x200a | 0x202f | 0x205f | 0x3000 => {}
                    _ => return FALSE,
                }
            } else if lp0 == OP_ANYNL || lp0 == OP_VSPACE {
                match chr {
                    /* VSPACE_CASES */
                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {
                        return FALSE
                    }
                    _ => {}
                }
            } else if lp0 == OP_NOT_VSPACE {
                match chr {
                    /* VSPACE_CASES */
                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {}
                    _ => return FALSE,
                }
            } else if lp0 == OP_DOLL || lp0 == OP_EODN {
                match chr {
                    CHAR_CR | CHAR_LF | CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                        return FALSE
                    }
                    _ => {}
                }
            } else if lp0 == OP_EOD {
                /* Can always possessify before \z */
            } else if lp0 == OP_PROP || lp0 == OP_NOTPROP {
                if check_char_prop(
                    chr,
                    *list_ptr.add(2),
                    *list_ptr.add(3),
                    (*list_ptr.add(0) == OP_NOTPROP) as BOOL,
                ) == 0
                {
                    return FALSE;
                }
            } else if lp0 == OP_NCLASS || lp0 == OP_CLASS {
                if lp0 == OP_NCLASS && chr > 255 {
                    return FALSE;
                }
                /* Fall through */
                if chr <= 255 {
                    class_bitset = (if list_ptr == list.as_ptr() { code } else { base_end })
                        .sub(*list_ptr.add(2) as usize);
                    if (*class_bitset.add((chr >> 3) as usize) as u32 & (1u32 << (chr & 7))) != 0
                    {
                        return FALSE;
                    }
                }
            } else if lp0 == OP_XCLASS {
                if _pcre2_xclass_8(
                    chr,
                    (if list_ptr == list.as_ptr() { code } else { base_end })
                        .sub(*list_ptr.add(2) as usize)
                        .add(LINK_SIZE),
                    (*cb).start_code as *const u8,
                    utf,
                ) != 0
                {
                    return FALSE;
                }
            } else if lp0 == OP_ECLASS {
                if _pcre2_eclass_8(
                    chr,
                    (if list_ptr == list.as_ptr() { code } else { base_end })
                        .sub(*list_ptr.add(2) as usize)
                        .add(LINK_SIZE),
                    (if list_ptr == list.as_ptr() { code } else { base_end })
                        .sub(*list_ptr.add(3) as usize),
                    (*cb).start_code as *const u8,
                    utf,
                ) != 0
                {
                    return FALSE;
                }
            } else {
                return FALSE;
            }

            chr_ptr = chr_ptr.add(1);
            if *chr_ptr == NOTACHAR {
                break;
            }
        }

        /* At least one character must be matched from this opcode. */

        if list[1] == 0 {
            return TRUE;
        }
    }

    /* Control should never reach here */
}

