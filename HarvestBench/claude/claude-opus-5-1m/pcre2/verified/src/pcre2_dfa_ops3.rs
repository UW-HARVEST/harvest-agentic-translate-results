/* Fragment 3 of the switch(codevalue) in internal_dfa_match():
c_src/src/pcre2_dfa_match.c lines 1692-1984 - the "EXTRA" QUERY/STAR variants.
This file is textually included inside the 'next_active_state block. */
{
    match codevalue {
        /*-----------------------------------------------------------------*/
        /* case OP_PROP_EXTRA + OP_TYPEQUERY:
           case OP_PROP_EXTRA + OP_TYPEMINQUERY:
           case OP_PROP_EXTRA + OP_TYPEPOSQUERY:  count = 4; goto QS1;

           case OP_PROP_EXTRA + OP_TYPESTAR:
           case OP_PROP_EXTRA + OP_TYPEMINSTAR:
           case OP_PROP_EXTRA + OP_TYPEPOSSTAR:   count = 0;  then QS1: */
        OPX_PROP_TYPEQUERY
        | OPX_PROP_TYPEMINQUERY
        | OPX_PROP_TYPEPOSQUERY
        | OPX_PROP_TYPESTAR
        | OPX_PROP_TYPEMINSTAR
        | OPX_PROP_TYPEPOSSTAR => {
            count = if codevalue == OPX_PROP_TYPEQUERY
                || codevalue == OPX_PROP_TYPEMINQUERY
                || codevalue == OPX_PROP_TYPEPOSQUERY
            {
                4
            } else {
                0
            };

            /* QS1: */

            ADD_ACTIVE!(state_offset + 4, 0);
            if clen > 0 {
                let mut ok: BOOL;
                let chartype: u32;
                let mut cp: *const u32;
                let prop: *const ucd_record = GET_UCD(c);
                match *code.add(2) as u32 {
                    PT_LAMP => {
                        chartype = (*prop).chartype as u32;
                        ok = (chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt)
                            as BOOL;
                    }

                    PT_GC => {
                        ok = (*_pcre2_ucp_gentype_8
                            .as_ptr()
                            .add((*prop).chartype as usize)
                            == *code.add(3) as u32) as BOOL;
                    }

                    PT_PC => {
                        ok = ((*prop).chartype == *code.add(3)) as BOOL;
                    }

                    PT_SC => {
                        ok = ((*prop).script == *code.add(3)) as BOOL;
                    }

                    PT_SCX => {
                        ok = ((*prop).script == *code.add(3)
                            || MAPBIT!(
                                _pcre2_ucd_script_sets_8
                                    .as_ptr()
                                    .add(UCD_SCRIPTX_PROP(prop) as usize),
                                *code.add(3) as u32
                            ) != 0) as BOOL;
                    }

                    /* These are specials for combination cases. */
                    PT_ALNUM => {
                        chartype = (*prop).chartype as u32;
                        ok = (*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_L
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
                                ok = TRUE;
                            }

                            _ => {
                                ok = (*_pcre2_ucp_gentype_8
                                    .as_ptr()
                                    .add((*prop).chartype as usize)
                                    == ucp_Z) as BOOL;
                            }
                        }
                    }

                    PT_WORD => {
                        chartype = (*prop).chartype as u32;
                        ok = (*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_L
                            || *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_N
                            || chartype == ucp_Mn
                            || chartype == ucp_Pc) as BOOL;
                    }

                    PT_CLIST => {
                        cp = _pcre2_ucd_caseless_sets_8
                            .as_ptr()
                            .add(*code.add(3) as usize);
                        loop {
                            if c < *cp {
                                ok = FALSE;
                                break;
                            }
                            if c == {
                                let t = *cp;
                                cp = cp.add(1);
                                t
                            } {
                                ok = TRUE;
                                break;
                            }
                        }
                    }

                    PT_UCNC => {
                        ok = (c == CHAR_DOLLAR_SIGN
                            || c == CHAR_COMMERCIAL_AT
                            || c == CHAR_GRAVE_ACCENT
                            || (c >= 0xa0 && c <= 0xd7ff)
                            || c >= 0xe000) as BOOL;
                    }

                    PT_BIDICL => {
                        ok = (UCD_BIDICLASS(c) == *code.add(3) as u32) as BOOL;
                    }

                    PT_BOOL => {
                        ok = (MAPBIT!(
                            _pcre2_ucd_boolprop_sets_8
                                .as_ptr()
                                .add(UCD_BPROPS_PROP(prop) as usize),
                            *code.add(3) as u32
                        ) != 0) as BOOL;
                    }

                    /* Should never occur, but keep compilers from grumbling. */
                    _ => {
                        ok = (codevalue != OP_PROP) as BOOL;
                    }
                }

                if ok == ((d == OP_PROP) as BOOL) {
                    if codevalue == OPX_PROP_TYPEPOSSTAR || codevalue == OPX_PROP_TYPEPOSQUERY {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    ADD_NEW!(state_offset + count, 0);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        /* case OP_EXTUNI_EXTRA + OP_TYPEQUERY/MINQUERY/POSQUERY: count = 2; goto QS2;
           case OP_EXTUNI_EXTRA + OP_TYPESTAR/MINSTAR/POSSTAR:    count = 0;  then QS2: */
        OPX_EXTUNI_TYPEQUERY
        | OPX_EXTUNI_TYPEMINQUERY
        | OPX_EXTUNI_TYPEPOSQUERY
        | OPX_EXTUNI_TYPESTAR
        | OPX_EXTUNI_TYPEMINSTAR
        | OPX_EXTUNI_TYPEPOSSTAR => {
            count = if codevalue == OPX_EXTUNI_TYPEQUERY
                || codevalue == OPX_EXTUNI_TYPEMINQUERY
                || codevalue == OPX_EXTUNI_TYPEPOSQUERY
            {
                2
            } else {
                0
            };

            /* QS2: */

            ADD_ACTIVE!(state_offset + 2, 0);
            if clen > 0 {
                let mut ncount: c_int = 0;
                if codevalue == OPX_EXTUNI_TYPEPOSSTAR || codevalue == OPX_EXTUNI_TYPEPOSQUERY {
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
                ADD_NEW_DATA!(-(state_offset + count), 0, ncount);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        /* case OP_ANYNL_EXTRA + OP_TYPEQUERY/MINQUERY/POSQUERY: count = 2; goto QS3;
           case OP_ANYNL_EXTRA + OP_TYPESTAR/MINSTAR/POSSTAR:    count = 0;  then QS3: */
        OPX_ANYNL_TYPEQUERY
        | OPX_ANYNL_TYPEMINQUERY
        | OPX_ANYNL_TYPEPOSQUERY
        | OPX_ANYNL_TYPESTAR
        | OPX_ANYNL_TYPEMINSTAR
        | OPX_ANYNL_TYPEPOSSTAR => {
            count = if codevalue == OPX_ANYNL_TYPEQUERY
                || codevalue == OPX_ANYNL_TYPEMINQUERY
                || codevalue == OPX_ANYNL_TYPEPOSQUERY
            {
                2
            } else {
                0
            };

            /* QS3: */
            ADD_ACTIVE!(state_offset + 2, 0);
            if clen > 0 {
                let mut ncount: c_int = 0;
                /* The labelled block is the C `switch (c)`; `break 'anynl_switch`
                is the C `break` out of that switch, and the code after the inner
                match is the ANYNL02: label that CHAR_CR falls through into. */
                'anynl_switch: {
                    match c {
                        CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                            if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                                break 'anynl_switch;
                            }
                            /* goto ANYNL02 */
                        }

                        CHAR_CR => {
                            if ptr.add(1) < end_subject && *ptr.add(1) as u32 == CHAR_LF {
                                ncount = 1;
                            }
                            /* Fall through */
                        }

                        /* ANYNL02: */
                        CHAR_LF => {}

                        _ => {
                            break 'anynl_switch;
                        }
                    }

                    /* ANYNL02: */
                    if codevalue == OPX_ANYNL_TYPEPOSSTAR || codevalue == OPX_ANYNL_TYPEPOSQUERY {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    ADD_NEW_DATA!(-(state_offset + count), 0, ncount);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        /* case OP_VSPACE_EXTRA + OP_TYPEQUERY/MINQUERY/POSQUERY: count = 2; goto QS4;
           case OP_VSPACE_EXTRA + OP_TYPESTAR/MINSTAR/POSSTAR:    count = 0;  then QS4: */
        OPX_VSPACE_TYPEQUERY
        | OPX_VSPACE_TYPEMINQUERY
        | OPX_VSPACE_TYPEPOSQUERY
        | OPX_VSPACE_TYPESTAR
        | OPX_VSPACE_TYPEMINSTAR
        | OPX_VSPACE_TYPEPOSSTAR => {
            count = if codevalue == OPX_VSPACE_TYPEQUERY
                || codevalue == OPX_VSPACE_TYPEMINQUERY
                || codevalue == OPX_VSPACE_TYPEPOSQUERY
            {
                2
            } else {
                0
            };

            /* QS4: */
            ADD_ACTIVE!(state_offset + 2, 0);
            if clen > 0 {
                let ok: BOOL;
                match c {
                    /* VSPACE_CASES: */
                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {
                        ok = TRUE;
                    }

                    _ => {
                        ok = FALSE;
                    }
                }
                if ok == ((d == OP_VSPACE) as BOOL) {
                    if codevalue == OPX_VSPACE_TYPEPOSSTAR || codevalue == OPX_VSPACE_TYPEPOSQUERY
                    {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    ADD_NEW_DATA!(-(state_offset + count), 0, 0);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        /* case OP_HSPACE_EXTRA + OP_TYPEQUERY/MINQUERY/POSQUERY: count = 2; goto QS5;
           case OP_HSPACE_EXTRA + OP_TYPESTAR/MINSTAR/POSSTAR:    count = 0;  then QS5: */
        OPX_HSPACE_TYPEQUERY
        | OPX_HSPACE_TYPEMINQUERY
        | OPX_HSPACE_TYPEPOSQUERY
        | OPX_HSPACE_TYPESTAR
        | OPX_HSPACE_TYPEMINSTAR
        | OPX_HSPACE_TYPEPOSSTAR => {
            count = if codevalue == OPX_HSPACE_TYPEQUERY
                || codevalue == OPX_HSPACE_TYPEMINQUERY
                || codevalue == OPX_HSPACE_TYPEPOSQUERY
            {
                2
            } else {
                0
            };

            /* QS5: */
            ADD_ACTIVE!(state_offset + 2, 0);
            if clen > 0 {
                let ok: BOOL;
                match c {
                    /* HSPACE_CASES: */
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
                        ok = TRUE;
                    }

                    _ => {
                        ok = FALSE;
                    }
                }

                if ok == ((d == OP_HSPACE) as BOOL) {
                    if codevalue == OPX_HSPACE_TYPEPOSSTAR || codevalue == OPX_HSPACE_TYPEPOSQUERY
                    {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    ADD_NEW_DATA!(-(state_offset + count), 0, 0);
                }
            }
            break 'next_active_state;
        }

        _ => {}
    }
}
