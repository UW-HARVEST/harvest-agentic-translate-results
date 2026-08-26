{
    match codevalue {
        /* ========================================================================== */
        /* These opcodes likewise inspect the subject character, but have an
        argument that is not a data character. It is one of these opcodes:
        OP_ANY, OP_ALLANY, OP_DIGIT, OP_NOT_DIGIT, OP_WHITESPACE, OP_NOT_SPACE,
        OP_WORDCHAR, OP_NOT_WORDCHAR. The value is loaded into d. */
        OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEPOSPLUS => {
            count = (*current_state).count; /* Already matched */
            if count > 0 {
                ADD_ACTIVE!(state_offset + 2, 0);
            }
            if clen > 0 {
                if d == OP_ANY
                    && ptr.add(1) >= (*mb).end_subject
                    && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                    && (*mb).nltype == NLTYPE_FIXED
                    && (*mb).nllen == 2
                    && c == (*mb).nl[0] as u32
                {
                    partial_newline = TRUE;
                    could_continue = TRUE;
                } else if (c >= 256 && d != OP_DIGIT && d != OP_WHITESPACE && d != OP_WORDCHAR)
                    || (c < 256
                        && (d != OP_ANY || !(IS_NEWLINE!(ptr, mb, (*mb).end_subject, utf)))
                        && ((*ctypes.add(c as usize) & *toptable1.as_ptr().add(d as usize))
                            ^ *toptable2.as_ptr().add(d as usize))
                            != 0)
                {
                    if count > 0 && codevalue == OP_TYPEPOSPLUS {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    count += 1;
                    ADD_NEW!(state_offset, count);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_TYPEQUERY | OP_TYPEMINQUERY | OP_TYPEPOSQUERY => {
            ADD_ACTIVE!(state_offset + 2, 0);
            if clen > 0 {
                if d == OP_ANY
                    && ptr.add(1) >= (*mb).end_subject
                    && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                    && (*mb).nltype == NLTYPE_FIXED
                    && (*mb).nllen == 2
                    && c == (*mb).nl[0] as u32
                {
                    partial_newline = TRUE;
                    could_continue = TRUE;
                } else if (c >= 256 && d != OP_DIGIT && d != OP_WHITESPACE && d != OP_WORDCHAR)
                    || (c < 256
                        && (d != OP_ANY || !(IS_NEWLINE!(ptr, mb, (*mb).end_subject, utf)))
                        && ((*ctypes.add(c as usize) & *toptable1.as_ptr().add(d as usize))
                            ^ *toptable2.as_ptr().add(d as usize))
                            != 0)
                {
                    if codevalue == OP_TYPEPOSQUERY {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    ADD_NEW!(state_offset + 2, 0);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPOSSTAR => {
            ADD_ACTIVE!(state_offset + 2, 0);
            if clen > 0 {
                if d == OP_ANY
                    && ptr.add(1) >= (*mb).end_subject
                    && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                    && (*mb).nltype == NLTYPE_FIXED
                    && (*mb).nllen == 2
                    && c == (*mb).nl[0] as u32
                {
                    partial_newline = TRUE;
                    could_continue = TRUE;
                } else if (c >= 256 && d != OP_DIGIT && d != OP_WHITESPACE && d != OP_WORDCHAR)
                    || (c < 256
                        && (d != OP_ANY || !(IS_NEWLINE!(ptr, mb, (*mb).end_subject, utf)))
                        && ((*ctypes.add(c as usize) & *toptable1.as_ptr().add(d as usize))
                            ^ *toptable2.as_ptr().add(d as usize))
                            != 0)
                {
                    if codevalue == OP_TYPEPOSSTAR {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    ADD_NEW!(state_offset, 0);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_TYPEEXACT => {
            count = (*current_state).count; /* Number already matched */
            if clen > 0 {
                if d == OP_ANY
                    && ptr.add(1) >= (*mb).end_subject
                    && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                    && (*mb).nltype == NLTYPE_FIXED
                    && (*mb).nllen == 2
                    && c == (*mb).nl[0] as u32
                {
                    partial_newline = TRUE;
                    could_continue = TRUE;
                } else if (c >= 256 && d != OP_DIGIT && d != OP_WHITESPACE && d != OP_WORDCHAR)
                    || (c < 256
                        && (d != OP_ANY || !(IS_NEWLINE!(ptr, mb, (*mb).end_subject, utf)))
                        && ((*ctypes.add(c as usize) & *toptable1.as_ptr().add(d as usize))
                            ^ *toptable2.as_ptr().add(d as usize))
                            != 0)
                {
                    count += 1;
                    if count >= GET2!(code, 1) as c_int {
                        ADD_NEW!(state_offset + 1 + IMM2_SIZE as c_int + 1, 0);
                    } else {
                        ADD_NEW!(state_offset, count);
                    }
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO => {
            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
            count = (*current_state).count; /* Number already matched */
            if clen > 0 {
                if d == OP_ANY
                    && ptr.add(1) >= (*mb).end_subject
                    && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                    && (*mb).nltype == NLTYPE_FIXED
                    && (*mb).nllen == 2
                    && c == (*mb).nl[0] as u32
                {
                    partial_newline = TRUE;
                    could_continue = TRUE;
                } else if (c >= 256 && d != OP_DIGIT && d != OP_WHITESPACE && d != OP_WORDCHAR)
                    || (c < 256
                        && (d != OP_ANY || !(IS_NEWLINE!(ptr, mb, (*mb).end_subject, utf)))
                        && ((*ctypes.add(c as usize) & *toptable1.as_ptr().add(d as usize))
                            ^ *toptable2.as_ptr().add(d as usize))
                            != 0)
                {
                    if codevalue == OP_TYPEPOSUPTO {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    count += 1;
                    if count >= GET2!(code, 1) as c_int {
                        ADD_NEW!(state_offset + 2 + IMM2_SIZE as c_int, 0);
                    } else {
                        ADD_NEW!(state_offset, count);
                    }
                }
            }
            break 'next_active_state;
        }

        /* ========================================================================== */
        /* These are virtual opcodes that are used when something like
        OP_TYPEPLUS has OP_PROP, OP_NOTPROP, OP_ANYNL, or OP_EXTUNI as its
        argument. It keeps the code above fast for the other cases. The argument
        is in the d variable. */
        OPX_PROP_TYPEPLUS | OPX_PROP_TYPEMINPLUS | OPX_PROP_TYPEPOSPLUS => {
            count = (*current_state).count; /* Already matched */
            if count > 0 {
                ADD_ACTIVE!(state_offset + 4, 0);
            }
            if clen > 0 {
                let mut OK: BOOL;
                let mut chartype: c_int;
                let mut cp: *const u32;
                let prop: *const ucd_record = GET_UCD(c);
                match *code.add(2) as u32 {
                    PT_LAMP => {
                        chartype = (*prop).chartype as c_int;
                        OK = (chartype == ucp_Lu as c_int
                            || chartype == ucp_Ll as c_int
                            || chartype == ucp_Lt as c_int) as BOOL;
                    }

                    PT_GC => {
                        OK = (*_pcre2_ucp_gentype_8
                            .as_ptr()
                            .add((*prop).chartype as usize)
                            == *code.add(3) as u32) as BOOL;
                    }

                    PT_PC => {
                        OK = ((*prop).chartype == *code.add(3)) as BOOL;
                    }

                    PT_SC => {
                        OK = ((*prop).script == *code.add(3)) as BOOL;
                    }

                    PT_SCX => {
                        OK = ((*prop).script == *code.add(3)
                            || MAPBIT!(
                                _pcre2_ucd_script_sets_8
                                    .as_ptr()
                                    .add(UCD_SCRIPTX_PROP(prop) as usize),
                                *code.add(3) as u32
                            ) != 0) as BOOL;
                    }

                    /* These are specials for combination cases. */
                    PT_ALNUM => {
                        chartype = (*prop).chartype as c_int;
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
                        chartype = (*prop).chartype as c_int;
                        OK = (*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_L
                            || *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_N
                            || chartype == ucp_Mn as c_int
                            || chartype == ucp_Pc as c_int) as BOOL;
                    }

                    PT_CLIST => {
                        cp = _pcre2_ucd_caseless_sets_8
                            .as_ptr()
                            .add(*code.add(3) as usize);
                        loop {
                            if c < *cp {
                                OK = FALSE;
                                break;
                            }
                            let t__ = *cp;
                            cp = cp.add(1);
                            if c == t__ {
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
                        OK = (UCD_BIDICLASS(c) == *code.add(3) as u32) as BOOL;
                    }

                    PT_BOOL => {
                        OK = (MAPBIT!(
                            _pcre2_ucd_boolprop_sets_8
                                .as_ptr()
                                .add(UCD_BPROPS_PROP(prop) as usize),
                            *code.add(3) as u32
                        ) != 0) as BOOL;
                    }

                    /* Should never occur, but keep compilers from grumbling. */
                    _ => {
                        OK = (codevalue != OP_PROP) as BOOL;
                    }
                }

                if OK == (d == OP_PROP) as BOOL {
                    if count > 0 && codevalue == OPX_PROP_TYPEPOSPLUS {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    count += 1;
                    ADD_NEW!(state_offset, count);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OPX_EXTUNI_TYPEPLUS | OPX_EXTUNI_TYPEMINPLUS | OPX_EXTUNI_TYPEPOSPLUS => {
            count = (*current_state).count; /* Already matched */
            if count > 0 {
                ADD_ACTIVE!(state_offset + 2, 0);
            }
            if clen > 0 {
                let mut ncount: c_int = 0;
                if count > 0 && codevalue == OPX_EXTUNI_TYPEPOSPLUS {
                    active_count -= 1; /* Remove non-match possibility */
                    next_active_state = next_active_state.sub(1);
                }
                let _ = _pcre2_extuni_8(
                    c,
                    ptr.add(clen as usize),
                    (*mb).start_subject,
                    end_subject,
                    utf,
                    &mut ncount,
                );
                count += 1;
                ADD_NEW_DATA!(-state_offset, count, ncount);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OPX_ANYNL_TYPEPLUS | OPX_ANYNL_TYPEMINPLUS | OPX_ANYNL_TYPEPOSPLUS => {
            count = (*current_state).count; /* Already matched */
            if count > 0 {
                ADD_ACTIVE!(state_offset + 2, 0);
            }
            if clen > 0 {
                let mut ncount: c_int = 0;
                /* The C code uses `goto ANYNL01` to jump into the middle of the
                switch; the labelled block below plays the role of the switch, and
                falling out of the `match` is the ANYNL01 label. */
                'anynl01: {
                    match c {
                        CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                            if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                                break 'anynl01;
                            }
                            /* goto ANYNL01; */
                        }

                        CHAR_CR => {
                            if ptr.add(1) < end_subject && *ptr.add(1) as u32 == CHAR_LF {
                                ncount = 1;
                            }
                            /* Fall through */
                        }

                        /* ANYNL01: */
                        CHAR_LF => {}

                        _ => {
                            break 'anynl01;
                        }
                    }

                    /* ANYNL01: */
                    if count > 0 && codevalue == OPX_ANYNL_TYPEPOSPLUS {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    count += 1;
                    ADD_NEW_DATA!(-state_offset, count, ncount);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OPX_VSPACE_TYPEPLUS | OPX_VSPACE_TYPEMINPLUS | OPX_VSPACE_TYPEPOSPLUS => {
            count = (*current_state).count; /* Already matched */
            if count > 0 {
                ADD_ACTIVE!(state_offset + 2, 0);
            }
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

                if OK == (d == OP_VSPACE) as BOOL {
                    if count > 0 && codevalue == OPX_VSPACE_TYPEPOSPLUS {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    count += 1;
                    ADD_NEW_DATA!(-state_offset, count, 0);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OPX_HSPACE_TYPEPLUS | OPX_HSPACE_TYPEMINPLUS | OPX_HSPACE_TYPEPOSPLUS => {
            count = (*current_state).count; /* Already matched */
            if count > 0 {
                ADD_ACTIVE!(state_offset + 2, 0);
            }
            if clen > 0 {
                let OK: BOOL;
                match c {
                    /* HSPACE_CASES */
                    CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000 | 0x2001
                    | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                    | 0x200a | 0x202f | 0x205f | 0x3000 => {
                        OK = TRUE;
                    }

                    _ => {
                        OK = FALSE;
                    }
                }

                if OK == (d == OP_HSPACE) as BOOL {
                    if count > 0 && codevalue == OPX_HSPACE_TYPEPOSPLUS {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    count += 1;
                    ADD_NEW_DATA!(-state_offset, count, 0);
                }
            }
            break 'next_active_state;
        }

        _ => {}
    }
}
