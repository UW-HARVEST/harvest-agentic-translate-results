/* Fragment 4 of the switch(codevalue) in internal_dfa_match():
c_src/src/pcre2_dfa_match.c lines 1985-2410.

The "EXTRA" EXACT/UPTO variants for PROP, EXTUNI, ANYNL, VSPACE and HSPACE,
followed by OP_CHAR, OP_CHARI, OP_EXTUNI, OP_ANYNL, OP_NOT_VSPACE, OP_VSPACE,
OP_NOT_HSPACE and OP_HSPACE. The C `goto ANYNL03` in the ANYNL EXACT/UPTO case
is reproduced with a labelled block. */
{
    match codevalue {
        /*-----------------------------------------------------------------*/
        OPX_PROP_TYPEEXACT | OPX_PROP_TYPEUPTO | OPX_PROP_TYPEMINUPTO
        | OPX_PROP_TYPEPOSUPTO => {
            if codevalue != OPX_PROP_TYPEEXACT {
                ADD_ACTIVE!(state_offset + 1 + IMM2_SIZE as c_int + 3, 0);
            }
            count = (*current_state).count; /* Number already matched */
            if clen > 0 {
                let mut OK: BOOL;
                let prop: *const ucd_record = GET_UCD(c);
                match *code.add(1 + IMM2_SIZE + 1) as u32 {
                    PT_LAMP => {
                        let chartype: c_int = (*prop).chartype as c_int;
                        OK = (chartype == ucp_Lu as c_int
                            || chartype == ucp_Ll as c_int
                            || chartype == ucp_Lt as c_int) as BOOL;
                    }

                    PT_GC => {
                        OK = (*_pcre2_ucp_gentype_8
                            .as_ptr()
                            .add((*prop).chartype as usize)
                            == *code.add(1 + IMM2_SIZE + 2) as u32) as BOOL;
                    }

                    PT_PC => {
                        OK = ((*prop).chartype == *code.add(1 + IMM2_SIZE + 2)) as BOOL;
                    }

                    PT_SC => {
                        OK = ((*prop).script == *code.add(1 + IMM2_SIZE + 2)) as BOOL;
                    }

                    PT_SCX => {
                        OK = ((*prop).script == *code.add(1 + IMM2_SIZE + 2)
                            || MAPBIT!(
                                _pcre2_ucd_script_sets_8
                                    .as_ptr()
                                    .add(UCD_SCRIPTX_PROP(prop) as usize),
                                *code.add(1 + IMM2_SIZE + 2) as u32
                            ) != 0) as BOOL;
                    }

                    /* These are specials for combination cases. */
                    PT_ALNUM => {
                        let chartype: c_int = (*prop).chartype as c_int;
                        OK = (*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_L
                            || *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_N)
                            as BOOL;
                    }

                    /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                    which means that Perl space and POSIX space are now identical. PCRE
                    was changed at release 8.34. */
                    PT_SPACE /* Perl space */ | PT_PXSPACE /* POSIX space */ => {
                        match c {
                            /* HSPACE_CASES */
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
                            /* VSPACE_CASES */
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
                        let chartype: c_int = (*prop).chartype as c_int;
                        OK = (*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_L
                            || *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_N
                            || chartype == ucp_Mn as c_int
                            || chartype == ucp_Pc as c_int) as BOOL;
                    }

                    PT_CLIST => {
                        let mut cp: *const u32 = _pcre2_ucd_caseless_sets_8
                            .as_ptr()
                            .add(*code.add(1 + IMM2_SIZE + 2) as usize);
                        loop {
                            if c < *cp {
                                OK = FALSE;
                                break;
                            }
                            let cpv = *cp;
                            cp = cp.add(1);
                            if c == cpv {
                                OK = TRUE;
                                break;
                            }
                        }
                    }

                    PT_UCNC => {
                        OK = (c == CHAR_DOLLAR_SIGN
                            || c == CHAR_COMMERCIAL_AT
                            || c == CHAR_GRAVE_ACCENT
                            || (c >= 0xa0 && c <= 0xd7ff)
                            || c >= 0xe000) as BOOL;
                    }

                    PT_BIDICL => {
                        OK = (UCD_BIDICLASS(c) == *code.add(1 + IMM2_SIZE + 2) as u32) as BOOL;
                    }

                    PT_BOOL => {
                        OK = (MAPBIT!(
                            _pcre2_ucd_boolprop_sets_8
                                .as_ptr()
                                .add(UCD_BPROPS_PROP(prop) as usize),
                            *code.add(1 + IMM2_SIZE + 2) as u32
                        ) != 0) as BOOL;
                    }

                    /* Should never occur, but keep compilers from grumbling. */
                    _ => {
                        OK = (codevalue != OP_PROP) as BOOL;
                    }
                }

                if OK == ((d == OP_PROP) as BOOL) {
                    if codevalue == OPX_PROP_TYPEPOSUPTO {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    count += 1;
                    if count >= GET2!(code, 1) as c_int {
                        ADD_NEW!(state_offset + 1 + IMM2_SIZE as c_int + 3, 0);
                    } else {
                        ADD_NEW!(state_offset, count);
                    }
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OPX_EXTUNI_TYPEEXACT | OPX_EXTUNI_TYPEUPTO | OPX_EXTUNI_TYPEMINUPTO
        | OPX_EXTUNI_TYPEPOSUPTO => {
            if codevalue != OPX_EXTUNI_TYPEEXACT {
                ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
            }
            count = (*current_state).count; /* Number already matched */
            if clen > 0 {
                let nptr: PCRE2_SPTR;
                let mut ncount: c_int = 0;
                if codevalue == OPX_EXTUNI_TYPEPOSUPTO {
                    active_count -= 1; /* Remove non-match possibility */
                    next_active_state = next_active_state.sub(1);
                }
                nptr = _pcre2_extuni_8(
                    c,
                    ptr.add(clen as usize),
                    (*mb).start_subject,
                    end_subject,
                    utf,
                    &mut ncount,
                );
                if nptr >= end_subject && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                    reset_could_continue = TRUE;
                }
                count += 1;
                if count >= GET2!(code, 1) as c_int {
                    ADD_NEW_DATA!(-(state_offset + 2 + IMM2_SIZE as c_int), 0, ncount);
                } else {
                    ADD_NEW_DATA!(-state_offset, count, ncount);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OPX_ANYNL_TYPEEXACT | OPX_ANYNL_TYPEUPTO | OPX_ANYNL_TYPEMINUPTO
        | OPX_ANYNL_TYPEPOSUPTO => {
            if codevalue != OPX_ANYNL_TYPEEXACT {
                ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
            }
            count = (*current_state).count; /* Number already matched */
            if clen > 0 {
                let mut ncount: c_int = 0;
                /* The C code jumps into the CHAR_LF case with "goto ANYNL03"; the
                common tail is the body of this labelled block. */
                'ANYNL03: {
                    match c {
                        CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                            if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                                break 'ANYNL03;
                            }
                            /* goto ANYNL03 */
                        }

                        CHAR_CR => {
                            if ptr.add(1) < end_subject && *ptr.add(1) as u32 == CHAR_LF {
                                ncount = 1;
                            }
                            /* Fall through */
                        }

                        /* ANYNL03: */
                        CHAR_LF => {}

                        _ => {
                            break 'ANYNL03;
                        }
                    }

                    if codevalue == OPX_ANYNL_TYPEPOSUPTO {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    count += 1;
                    if count >= GET2!(code, 1) as c_int {
                        ADD_NEW_DATA!(-(state_offset + 2 + IMM2_SIZE as c_int), 0, ncount);
                    } else {
                        ADD_NEW_DATA!(-state_offset, count, ncount);
                    }
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OPX_VSPACE_TYPEEXACT | OPX_VSPACE_TYPEUPTO | OPX_VSPACE_TYPEMINUPTO
        | OPX_VSPACE_TYPEPOSUPTO => {
            if codevalue != OPX_VSPACE_TYPEEXACT {
                ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
            }
            count = (*current_state).count; /* Number already matched */
            if clen > 0 {
                let OK: BOOL;
                match c {
                    /* VSPACE_CASES */
                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {
                        OK = TRUE;
                    }

                    _ => {
                        OK = FALSE;
                    }
                }

                if OK == ((d == OP_VSPACE) as BOOL) {
                    if codevalue == OPX_VSPACE_TYPEPOSUPTO {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    count += 1;
                    if count >= GET2!(code, 1) as c_int {
                        ADD_NEW_DATA!(-(state_offset + 2 + IMM2_SIZE as c_int), 0, 0);
                    } else {
                        ADD_NEW_DATA!(-state_offset, count, 0);
                    }
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OPX_HSPACE_TYPEEXACT | OPX_HSPACE_TYPEUPTO | OPX_HSPACE_TYPEMINUPTO
        | OPX_HSPACE_TYPEPOSUPTO => {
            if codevalue != OPX_HSPACE_TYPEEXACT {
                ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
            }
            count = (*current_state).count; /* Number already matched */
            if clen > 0 {
                let OK: BOOL;
                match c {
                    /* HSPACE_CASES */
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
                    | 0x3000 => {
                        OK = TRUE;
                    }

                    _ => {
                        OK = FALSE;
                    }
                }

                if OK == ((d == OP_HSPACE) as BOOL) {
                    if codevalue == OPX_HSPACE_TYPEPOSUPTO {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    count += 1;
                    if count >= GET2!(code, 1) as c_int {
                        ADD_NEW_DATA!(-(state_offset + 2 + IMM2_SIZE as c_int), 0, 0);
                    } else {
                        ADD_NEW_DATA!(-state_offset, count, 0);
                    }
                }
            }
            break 'next_active_state;
        }

        /* ================================================================== */
        /* These opcodes are followed by a character that is usually compared
        to the current subject character; it is loaded into d. We still get
        here even if there is no subject character, because in some cases zero
        repetitions are permitted. */

        /*-----------------------------------------------------------------*/
        OP_CHAR => {
            if clen > 0 && c == d {
                ADD_NEW!(state_offset + dlen + 1, 0);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_CHARI => {
            if clen == 0 {
                break 'next_active_state;
            }

            if utf_or_ucp != 0 {
                if c == d {
                    ADD_NEW!(state_offset + dlen + 1, 0);
                } else {
                    let othercase: c_uint;
                    if c < 128 {
                        othercase = *fcc.add(c as usize) as c_uint;
                    } else {
                        othercase = UCD_OTHERCASE(c);
                    }
                    if d == othercase {
                        ADD_NEW!(state_offset + dlen + 1, 0);
                    }
                }
            }
            /* Not UTF or UCP mode */
            else {
                if TABLE_GET!(c, lcc, c) == TABLE_GET!(d, lcc, d) {
                    ADD_NEW!(state_offset + 2, 0);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        /* This is a tricky one because it can match more than one character.
        Find out how many characters to skip, and then set up a negative state
        to wait for them to pass before continuing. */
        OP_EXTUNI => {
            if clen > 0 {
                let mut ncount: c_int = 0;
                let nptr: PCRE2_SPTR = _pcre2_extuni_8(
                    c,
                    ptr.add(clen as usize),
                    (*mb).start_subject,
                    end_subject,
                    utf,
                    &mut ncount,
                );
                if nptr >= end_subject && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                    reset_could_continue = TRUE;
                }
                ADD_NEW_DATA!(-(state_offset + 1), 0, ncount);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        /* This is a tricky like EXTUNI because it too can match more than one
        character (when CR is followed by LF). In this case, set up a negative
        state to wait for one character to pass before continuing. */
        OP_ANYNL => {
            if clen > 0 {
                match c {
                    CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                        if (*mb).bsr_convention as u32 != PCRE2_BSR_ANYCRLF {
                            /* Fall through into the CHAR_LF case */
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    CHAR_LF => {
                        ADD_NEW!(state_offset + 1, 0);
                    }

                    CHAR_CR => {
                        if ptr.add(1) >= end_subject {
                            ADD_NEW!(state_offset + 1, 0);
                            if ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                                reset_could_continue = TRUE;
                            }
                        } else if *ptr.add(1) as u32 == CHAR_LF {
                            ADD_NEW_DATA!(-(state_offset + 1), 0, 1);
                        } else {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    _ => {}
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_NOT_VSPACE => {
            if clen > 0 {
                match c {
                    /* VSPACE_CASES */
                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {}

                    _ => {
                        ADD_NEW!(state_offset + 1, 0);
                    }
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_VSPACE => {
            if clen > 0 {
                match c {
                    /* VSPACE_CASES */
                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {
                        ADD_NEW!(state_offset + 1, 0);
                    }

                    _ => {}
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_NOT_HSPACE => {
            if clen > 0 {
                match c {
                    /* HSPACE_CASES */
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
                    | 0x3000 => {}

                    _ => {
                        ADD_NEW!(state_offset + 1, 0);
                    }
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_HSPACE => {
            if clen > 0 {
                match c {
                    /* HSPACE_CASES */
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
                    | 0x3000 => {
                        ADD_NEW!(state_offset + 1, 0);
                    }

                    _ => {}
                }
            }
            break 'next_active_state;
        }

        _ => {}
    }
}
