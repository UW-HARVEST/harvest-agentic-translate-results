/* Fragment 1 of the switch(codevalue) in internal_dfa_match(), translated from
c_src/src/pcre2_dfa_match.c lines 861-1281. Included by pcre2_dfa_match.rs. */
{
    match codevalue {
        /* ================================================================== */
        /* Reached a closing bracket. If not at the end of the pattern, carry
        on with the next opcode. For repeating opcodes, also add the repeat
        state. Note that KETRPOS will always be encountered at the end of the
        subpattern, because the possessive subpattern repeats are always handled
        using recursive calls. Thus, it never adds any new states.

        At the end of the (sub)pattern, unless we have an empty string and
        PCRE2_NOTEMPTY is set, or PCRE2_NOTEMPTY_ATSTART is set and we are at the
        start of the subject, save the match data, shifting up all previous
        matches so we always have the longest first. */
        OP_KET | OP_KETRMIN | OP_KETRMAX | OP_KETRPOS => {
            if code != end_code {
                ADD_ACTIVE!(state_offset + 1 + LINK_SIZE as c_int, 0);
                if codevalue != OP_KET {
                    ADD_ACTIVE!(state_offset - GET!(code, 1) as c_int, 0);
                }
            } else {
                if ptr > current_subject
                    || (((*mb).moptions & PCRE2_NOTEMPTY) == 0
                        && (((*mb).moptions & PCRE2_NOTEMPTY_ATSTART) == 0
                            || current_subject > start_subject.add((*mb).start_offset)))
                {
                    if match_count < 0 {
                        match_count = if offsetcount >= 2 { 1 } else { 0 };
                    } else if match_count > 0
                        && {
                            match_count += 1;
                            match_count * 2 > offsetcount as c_int
                        }
                    {
                        match_count = 0;
                    }
                    count = (if match_count == 0 {
                        offsetcount as c_int
                    } else {
                        match_count * 2
                    }) - 2;
                    if count > 0 {
                        memmove(
                            offsets.add(2) as *mut c_void,
                            offsets as *const c_void,
                            count as usize * size_of::<PCRE2_SIZE>(),
                        );
                    }
                    if offsetcount >= 2 {
                        *offsets = current_subject.offset_from(start_subject) as PCRE2_SIZE;
                        *offsets.add(1) = ptr.offset_from(start_subject) as PCRE2_SIZE;
                    }
                    if ((*mb).moptions & PCRE2_DFA_SHORTEST) != 0 {
                        return match_count;
                    }
                }
            }
            break 'next_active_state;
        }

        /* ================================================================== */
        /* These opcodes add to the current list of states without looking
        at the current character. */

        /*-----------------------------------------------------------------*/
        OP_ALT => {
            loop {
                code = code.add(GET!(code, 1) as usize);
                if *code as u32 != OP_ALT {
                    break;
                }
            }
            ADD_ACTIVE!(code.offset_from(start_code) as c_int, 0);
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_BRA | OP_SBRA => {
            loop {
                ADD_ACTIVE!(
                    code.offset_from(start_code) as c_int + 1 + LINK_SIZE as c_int,
                    0
                );
                code = code.add(GET!(code, 1) as usize);
                if *code as u32 != OP_ALT {
                    break;
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_CBRA | OP_SCBRA => {
            ADD_ACTIVE!(
                code.offset_from(start_code) as c_int + 1 + LINK_SIZE as c_int
                    + IMM2_SIZE as c_int,
                0
            );
            code = code.add(GET!(code, 1) as usize);
            while *code as u32 == OP_ALT {
                ADD_ACTIVE!(
                    code.offset_from(start_code) as c_int + 1 + LINK_SIZE as c_int,
                    0
                );
                code = code.add(GET!(code, 1) as usize);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_BRAZERO | OP_BRAMINZERO => {
            ADD_ACTIVE!(state_offset + 1, 0);
            code = code.add(1 + GET!(code, 2) as usize);
            while *code as u32 == OP_ALT {
                code = code.add(GET!(code, 1) as usize);
            }
            ADD_ACTIVE!(
                code.offset_from(start_code) as c_int + 1 + LINK_SIZE as c_int,
                0
            );
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_SKIPZERO => {
            code = code.add(1 + GET!(code, 2) as usize);
            while *code as u32 == OP_ALT {
                code = code.add(GET!(code, 1) as usize);
            }
            ADD_ACTIVE!(
                code.offset_from(start_code) as c_int + 1 + LINK_SIZE as c_int,
                0
            );
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_CIRC => {
            if ptr == start_subject && ((*mb).moptions & PCRE2_NOTBOL) == 0 {
                ADD_ACTIVE!(state_offset + 1, 0);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_CIRCM => {
            if (ptr == start_subject && ((*mb).moptions & PCRE2_NOTBOL) == 0)
                || ((ptr != end_subject || ((*mb).poptions & PCRE2_ALT_CIRCUMFLEX) != 0)
                    && (WAS_NEWLINE!(ptr, mb, (*mb).start_subject, utf)))
            {
                ADD_ACTIVE!(state_offset + 1, 0);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_EOD => {
            if ptr >= end_subject {
                if ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                    return PCRE2_ERROR_PARTIAL;
                } else {
                    ADD_ACTIVE!(state_offset + 1, 0);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_SOD => {
            if ptr == start_subject {
                ADD_ACTIVE!(state_offset + 1, 0);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_SOM => {
            if ptr == start_subject.add(start_offset) {
                ADD_ACTIVE!(state_offset + 1, 0);
            }
            break 'next_active_state;
        }

        /* ================================================================== */
        /* These opcodes inspect the next subject character, and sometimes
        the previous one as well, but do not have an argument. The variable
        clen contains the length of the current character and is zero if we are
        at the end of the subject. */

        /*-----------------------------------------------------------------*/
        OP_ANY => {
            if clen > 0 && !(IS_NEWLINE!(ptr, mb, (*mb).end_subject, utf)) {
                if ptr.add(1) >= (*mb).end_subject
                    && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                    && (*mb).nltype == NLTYPE_FIXED
                    && (*mb).nllen == 2
                    && c == (*mb).nl[0] as u32
                {
                    partial_newline = TRUE;
                    could_continue = TRUE;
                } else {
                    ADD_NEW!(state_offset + 1, 0);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_ALLANY => {
            if clen > 0 {
                ADD_NEW!(state_offset + 1, 0);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_EODN => {
            if clen == 0
                || ((IS_NEWLINE!(ptr, mb, (*mb).end_subject, utf))
                    && ptr == end_subject.sub((*mb).nllen as usize))
            {
                if ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                    return PCRE2_ERROR_PARTIAL;
                }
                ADD_ACTIVE!(state_offset + 1, 0);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_DOLL => {
            if ((*mb).moptions & PCRE2_NOTEOL) == 0 {
                if clen == 0 && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                    could_continue = TRUE;
                } else if clen == 0
                    || (((*mb).poptions & PCRE2_DOLLAR_ENDONLY) == 0
                        && (IS_NEWLINE!(ptr, mb, (*mb).end_subject, utf))
                        && (ptr == end_subject.sub((*mb).nllen as usize)))
                {
                    ADD_ACTIVE!(state_offset + 1, 0);
                } else if ptr.add(1) >= (*mb).end_subject
                    && ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) != 0
                    && (*mb).nltype == NLTYPE_FIXED
                    && (*mb).nllen == 2
                    && c == (*mb).nl[0] as u32
                {
                    if ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                        reset_could_continue = TRUE;
                        ADD_NEW_DATA!(-(state_offset + 1), 0, 1);
                    } else {
                        partial_newline = TRUE;
                        could_continue = TRUE;
                    }
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_DOLLM => {
            if ((*mb).moptions & PCRE2_NOTEOL) == 0 {
                if clen == 0 && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                    could_continue = TRUE;
                } else if clen == 0
                    || (((*mb).poptions & PCRE2_DOLLAR_ENDONLY) == 0
                        && (IS_NEWLINE!(ptr, mb, (*mb).end_subject, utf)))
                {
                    ADD_ACTIVE!(state_offset + 1, 0);
                } else if ptr.add(1) >= (*mb).end_subject
                    && ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) != 0
                    && (*mb).nltype == NLTYPE_FIXED
                    && (*mb).nllen == 2
                    && c == (*mb).nl[0] as u32
                {
                    if ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                        reset_could_continue = TRUE;
                        ADD_NEW_DATA!(-(state_offset + 1), 0, 1);
                    } else {
                        partial_newline = TRUE;
                        could_continue = TRUE;
                    }
                }
            } else if (IS_NEWLINE!(ptr, mb, (*mb).end_subject, utf)) {
                ADD_ACTIVE!(state_offset + 1, 0);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_DIGIT | OP_WHITESPACE | OP_WORDCHAR => {
            if clen > 0
                && c < 256
                && ((*ctypes.add(c as usize) & *toptable1.as_ptr().add(codevalue as usize))
                    ^ *toptable2.as_ptr().add(codevalue as usize))
                    != 0
            {
                ADD_NEW!(state_offset + 1, 0);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_NOT_DIGIT | OP_NOT_WHITESPACE | OP_NOT_WORDCHAR => {
            if clen > 0
                && (c >= 256
                    || ((*ctypes.add(c as usize) & *toptable1.as_ptr().add(codevalue as usize))
                        ^ *toptable2.as_ptr().add(codevalue as usize))
                        != 0)
            {
                ADD_NEW!(state_offset + 1, 0);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_WORD_BOUNDARY
        | OP_NOT_WORD_BOUNDARY
        | OP_NOT_UCP_WORD_BOUNDARY
        | OP_UCP_WORD_BOUNDARY => {
            let left_word: c_int;
            let right_word: c_int;

            if ptr > start_subject {
                let mut temp: PCRE2_SPTR = ptr.sub(1);
                if temp < (*mb).start_used_ptr {
                    (*mb).start_used_ptr = temp;
                }
                if utf != 0 {
                    BACKCHAR!(temp);
                }
                GETCHARTEST!(d, temp, utf);
                if codevalue == OP_UCP_WORD_BOUNDARY || codevalue == OP_NOT_UCP_WORD_BOUNDARY {
                    let chartype: u32 = UCD_CHARTYPE(d);
                    let category: u32 = *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize);
                    left_word = (category == ucp_L
                        || category == ucp_N
                        || chartype == ucp_Mn
                        || chartype == ucp_Pc) as c_int;
                } else {
                    left_word =
                        (d < 256 && (*ctypes.add(d as usize) & ctype_word) != 0) as c_int;
                }
            } else {
                left_word = FALSE;
            }

            if clen > 0 {
                if ptr >= (*mb).last_used_ptr {
                    let mut temp: PCRE2_SPTR = ptr.add(1);
                    if utf != 0 {
                        FORWARDCHARTEST!(temp, (*mb).end_subject);
                    }
                    (*mb).last_used_ptr = temp;
                }
                if codevalue == OP_UCP_WORD_BOUNDARY || codevalue == OP_NOT_UCP_WORD_BOUNDARY {
                    let chartype: u32 = UCD_CHARTYPE(c);
                    let category: u32 = *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize);
                    right_word = (category == ucp_L
                        || category == ucp_N
                        || chartype == ucp_Mn
                        || chartype == ucp_Pc) as c_int;
                } else {
                    right_word =
                        (c < 256 && (*ctypes.add(c as usize) & ctype_word) != 0) as c_int;
                }
            } else {
                right_word = FALSE;
            }

            if (left_word == right_word)
                == (codevalue == OP_NOT_WORD_BOUNDARY
                    || codevalue == OP_NOT_UCP_WORD_BOUNDARY)
            {
                ADD_ACTIVE!(state_offset + 1, 0);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        /* Check the next character by Unicode property. We will get here only
        if the support is in the binary; otherwise a compile-time error occurs.
        */
        OP_PROP | OP_NOTPROP => {
            if clen > 0 {
                let OK: BOOL;
                let prop: *const ucd_record = GET_UCD(c);
                match *code.add(1) as u32 {
                    PT_LAMP => {
                        let chartype: u32 = (*prop).chartype as u32;
                        OK = (chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt)
                            as BOOL;
                    }

                    PT_GC => {
                        OK = (*_pcre2_ucp_gentype_8
                            .as_ptr()
                            .add((*prop).chartype as usize)
                            == *code.add(2) as u32) as BOOL;
                    }

                    PT_PC => {
                        OK = ((*prop).chartype == *code.add(2)) as BOOL;
                    }

                    PT_SC => {
                        OK = ((*prop).script == *code.add(2)) as BOOL;
                    }

                    PT_SCX => {
                        OK = ((*prop).script == *code.add(2)
                            || MAPBIT!(
                                _pcre2_ucd_script_sets_8
                                    .as_ptr()
                                    .add(UCD_SCRIPTX_PROP(prop) as usize),
                                *code.add(2) as u32
                            ) != 0) as BOOL;
                    }

                    /* These are specials for combination cases. */
                    PT_ALNUM => {
                        let chartype: u32 = (*prop).chartype as u32;
                        OK = (*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_L
                            || *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_N)
                            as BOOL;
                    }

                    /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                    which means that Perl space and POSIX space are now identical. PCRE
                    was changed at release 8.34. */
                    PT_SPACE /* Perl space */ | PT_PXSPACE /* POSIX space */ => {
                        match c {
                            /* HSPACE_CASES: VSPACE_CASES: */
                            CHAR_HT
                            | CHAR_SPACE
                            | CHAR_NBSP
                            | 0x1680
                            | 0x180e
                            | 0x2000
                            | 0x2001
                            | 0x2002
                            | 0x2003
                            | 0x2004
                            | 0x2005
                            | 0x2006
                            | 0x2007
                            | 0x2008
                            | 0x2009
                            | 0x200a
                            | 0x202f
                            | 0x205f
                            | 0x3000
                            | CHAR_LF
                            | CHAR_VT
                            | CHAR_FF
                            | CHAR_CR
                            | CHAR_NEL
                            | 0x2028
                            | 0x2029 => {
                                OK = TRUE;
                            }

                            _ => {
                                OK = (*_pcre2_ucp_gentype_8
                                    .as_ptr()
                                    .add((*prop).chartype as usize)
                                    == ucp_Z) as BOOL;
                            }
                        }
                    }

                    PT_WORD => {
                        let chartype: u32 = (*prop).chartype as u32;
                        OK = (*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_L
                            || *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_N
                            || chartype == ucp_Mn
                            || chartype == ucp_Pc) as BOOL;
                    }

                    PT_CLIST => {
                        let mut cp: *const u32 = _pcre2_ucd_caseless_sets_8
                            .as_ptr()
                            .add(*code.add(2) as usize);
                        let ok__: BOOL;
                        loop {
                            if c < *cp {
                                ok__ = FALSE;
                                break;
                            }
                            let t__ = *cp;
                            cp = cp.add(1);
                            if c == t__ {
                                ok__ = TRUE;
                                break;
                            }
                        }
                        OK = ok__;
                    }

                    PT_UCNC => {
                        OK = (c == CHAR_DOLLAR_SIGN
                            || c == CHAR_COMMERCIAL_AT
                            || c == CHAR_GRAVE_ACCENT
                            || (c >= 0xa0 && c <= 0xd7ff)
                            || c >= 0xe000) as BOOL;
                    }

                    PT_BIDICL => {
                        OK = (UCD_BIDICLASS(c) == *code.add(2) as u32) as BOOL;
                    }

                    PT_BOOL => {
                        OK = (MAPBIT!(
                            _pcre2_ucd_boolprop_sets_8
                                .as_ptr()
                                .add(UCD_BPROPS_PROP(prop) as usize),
                            *code.add(2) as u32
                        ) != 0) as BOOL;
                    }

                    /* Should never occur, but keep compilers from grumbling. */
                    _ => {
                        OK = (codevalue != OP_PROP) as BOOL;
                    }
                }

                if OK == ((codevalue == OP_PROP) as BOOL) {
                    ADD_NEW!(state_offset + 3, 0);
                }
            }
            break 'next_active_state;
        }

        _ => {}
    }
}
