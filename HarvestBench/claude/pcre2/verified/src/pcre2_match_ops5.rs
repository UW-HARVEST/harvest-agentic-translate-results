{
    match state {
        /* ===================================================================== */
        /* Match various character types when PCRE2_UCP is not set. These opcodes
        are not generated when PCRE2_UCP is set - instead appropriate property
        tests are compiled. */

        OP_NOT_DIGIT => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            if CHMAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        OP_DIGIT => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            if CHMAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        OP_NOT_WHITESPACE => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            if CHMAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        OP_WHITESPACE => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            if CHMAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        OP_NOT_WORDCHAR => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            if CHMAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        OP_WORDCHAR => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            if CHMAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        OP_ANYNL => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            match fc {
                CHAR_CR => {
                    if (*F).eptr >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                    } else if *(*F).eptr as u32 == CHAR_LF {
                        (*F).eptr = (*F).eptr.add(1);
                    }
                }

                CHAR_LF => {}

                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                    if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                _ => {
                    RRETURN!(MATCH_NOMATCH);
                }
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        OP_NOT_HSPACE => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            match fc {
                /* HSPACE_CASES: byte and multibyte cases */
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
                    RRETURN!(MATCH_NOMATCH);
                }
                _ => {}
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        OP_HSPACE => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            match fc {
                /* HSPACE_CASES: byte and multibyte cases */
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
                    RRETURN!(MATCH_NOMATCH);
                }
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        OP_NOT_VSPACE => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            match fc {
                /* VSPACE_CASES */
                CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {
                    RRETURN!(MATCH_NOMATCH);
                }
                _ => {}
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        OP_VSPACE => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            match fc {
                /* VSPACE_CASES */
                CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {}
                _ => {
                    RRETURN!(MATCH_NOMATCH);
                }
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Check the next character by Unicode property. We will get here only
        if the support is in the binary; otherwise a compile-time error occurs. */

        OP_PROP | OP_NOTPROP => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            {
                let chartype: u32;
                let prop: *const ucd_record = GET_UCD(fc);
                let notmatch: BOOL = ((*F).op as u32 == OP_NOTPROP) as BOOL;

                match *(*F).ecode.add(1) as u32 {
                    PT_LAMP => {
                        chartype = (*prop).chartype as u32;
                        if ((chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt)
                            as BOOL)
                            == notmatch
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }

                    PT_GC => {
                        if ((*(*F).ecode.add(2) as u32
                            == *_pcre2_ucp_gentype_8
                                .as_ptr()
                                .add((*prop).chartype as usize))
                            as BOOL)
                            == notmatch
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }

                    PT_PC => {
                        if ((*(*F).ecode.add(2) == (*prop).chartype) as BOOL) == notmatch {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }

                    PT_SC => {
                        if ((*(*F).ecode.add(2) == (*prop).script) as BOOL) == notmatch {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }

                    PT_SCX => {
                        let ok: BOOL = ((*(*F).ecode.add(2) == (*prop).script)
                            || MAPBIT!(
                                _pcre2_ucd_script_sets_8
                                    .as_ptr()
                                    .add(UCD_SCRIPTX_PROP(prop) as usize),
                                *(*F).ecode.add(2) as u32
                            ) != 0) as BOOL;
                        if ok == notmatch {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }

                    /* These are specials */
                    PT_ALNUM => {
                        chartype = (*prop).chartype as u32;
                        if ((*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_L
                            || *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_N)
                            as BOOL)
                            == notmatch
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }

                    /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                    which means that Perl space and POSIX space are now identical. PCRE
                    was changed at release 8.34. */
                    PT_SPACE | PT_PXSPACE => {
                        match fc {
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
                                if notmatch != 0 {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                            }

                            _ => {
                                if ((*_pcre2_ucp_gentype_8
                                    .as_ptr()
                                    .add((*prop).chartype as usize)
                                    == ucp_Z) as BOOL)
                                    == notmatch
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                            }
                        }
                    }

                    PT_WORD => {
                        chartype = (*prop).chartype as u32;
                        if ((*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_L
                            || *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_N
                            || chartype == ucp_Mn
                            || chartype == ucp_Pc) as BOOL)
                            == notmatch
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }

                    PT_CLIST => {
                        let mut cp: *const u32 = _pcre2_ucd_caseless_sets_8
                            .as_ptr()
                            .add(*(*F).ecode.add(2) as usize);
                        loop {
                            if fc < *cp {
                                if notmatch != 0 {
                                    break;
                                } else {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                            }
                            if fc == {
                                let t = *cp;
                                cp = cp.add(1);
                                t
                            } {
                                if notmatch != 0 {
                                    RRETURN!(MATCH_NOMATCH);
                                } else {
                                    break;
                                }
                            }
                        }
                    }

                    PT_UCNC => {
                        if ((fc == CHAR_DOLLAR_SIGN
                            || fc == CHAR_COMMERCIAL_AT
                            || fc == CHAR_GRAVE_ACCENT
                            || (fc >= 0xa0 && fc <= 0xd7ff)
                            || fc >= 0xe000) as BOOL)
                            == notmatch
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }

                    PT_BIDICL => {
                        if ((UCD_BIDICLASS_PROP(prop) == *(*F).ecode.add(2) as u32) as BOOL)
                            == notmatch
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }

                    PT_BOOL => {
                        let ok: BOOL = (MAPBIT!(
                            _pcre2_ucd_boolprop_sets_8
                                .as_ptr()
                                .add(UCD_BPROPS_PROP(prop) as usize),
                            *(*F).ecode.add(2) as u32
                        ) != 0) as BOOL;
                        if ok == notmatch {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }

                    /* This should never occur */
                    _ => {
                        return PCRE2_ERROR_INTERNAL;
                    }
                }

                (*F).ecode = (*F).ecode.add(3);
            }
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Match an extended Unicode sequence. We will get here only if the support
        is in the binary; otherwise a compile-time error occurs. */

        OP_EXTUNI => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            } else {
                GETCHARINCTEST!(fc, (*F).eptr, utf);
                (*F).eptr = _pcre2_extuni_8(
                    fc,
                    (*F).eptr,
                    (*mb).start_subject,
                    (*mb).end_subject,
                    utf,
                    core::ptr::null_mut(),
                );
            }
            CHECK_PARTIAL!();
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        _ => {}
    }
}
