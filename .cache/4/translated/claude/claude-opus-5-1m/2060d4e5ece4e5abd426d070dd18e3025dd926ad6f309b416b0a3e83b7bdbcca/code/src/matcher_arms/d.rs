{
if lbl == LBL_SWITCH {
    match (*F).op as u32 {
        /* ===================================================================== */
        /* Match various character types when PCRE2_UCP is not set. These opcodes
        are not generated when PCRE2_UCP is set - instead appropriate property
        tests are compiled. */

        OP_NOT_DIGIT => {
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            if CHMAX_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        OP_DIGIT => {
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            if !CHMAX_255(fc) || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        OP_NOT_WHITESPACE => {
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            if CHMAX_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        OP_WHITESPACE => {
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            if !CHMAX_255(fc) || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        OP_NOT_WORDCHAR => {
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            if CHMAX_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        OP_WORDCHAR => {
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            if !CHMAX_255(fc) || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        OP_ANYNL => {
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            match fc {
                CHAR_CR => {
                    if (*F).eptr >= (*mb).end_subject {
                        /* SCHECK_PARTIAL() */
                        if (*mb).partial != 0
                            && ((*F).eptr > (*mb).start_used_ptr
                                || (*mb).allowemptypartial != FALSE)
                        {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                    } else if *(*F).eptr as u32 == CHAR_LF {
                        (*F).eptr = (*F).eptr.add(1);
                    }
                }

                CHAR_LF => {}

                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                    if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                        rrc = MATCH_NOMATCH;
                        lbl = LBL_RETURN_SWITCH;
                        continue 'sw;
                    }
                }

                _ => {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        OP_NOT_HSPACE => {
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            match fc {
                /* HSPACE_CASES: byte and multibyte cases */
                0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002 | 0x2003
                | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f
                | 0x205f | 0x3000 => {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                _ => {}
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        OP_HSPACE => {
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            match fc {
                /* HSPACE_CASES: byte and multibyte cases */
                0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002 | 0x2003
                | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f
                | 0x205f | 0x3000 => {}
                _ => {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        OP_NOT_VSPACE => {
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            match fc {
                /* VSPACE_CASES */
                0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                _ => {}
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        OP_VSPACE => {
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            match fc {
                /* VSPACE_CASES */
                0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => {}
                _ => {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Check the next character by Unicode property. We will get here only
        if the support is in the binary; otherwise a compile-time error occurs. */

        OP_PROP | OP_NOTPROP => {
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            {
                /* `cp` and `chartype` are declared here in the C; they are
                introduced per-arm below since each arm uses at most one. */
                let prop: &'static ucd_record = GET_UCD(fc);
                let notmatch: BOOL = ((*F).op as u32 == OP_NOTPROP) as BOOL;

                match *(*F).ecode.add(1) as u32 {
                    PT_LAMP => {
                        let chartype: u32 = prop.chartype as u32;
                        if ((chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt)
                            as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                    }

                    PT_GC => {
                        if ((*(*F).ecode.add(2) as u32
                            == _pcre2_ucp_gentype_8[prop.chartype as usize])
                            as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                    }

                    PT_PC => {
                        if ((*(*F).ecode.add(2) as u32 == prop.chartype as u32) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                    }

                    PT_SC => {
                        if ((*(*F).ecode.add(2) as u32 == prop.script as u32) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                    }

                    PT_SCX => {
                        let ok: BOOL = ((*(*F).ecode.add(2) as u32 == prop.script as u32)
                            || crate::internal::script_set_bit(
                                UCD_SCRIPTX_PROP(prop) as usize,
                                *(*F).ecode.add(2) as u32,
                            )) as BOOL;
                        if ok == notmatch {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                    }

                    /* These are specials */

                    PT_ALNUM => {
                        let chartype: u32 = prop.chartype as u32;
                        if ((_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                            || _pcre2_ucp_gentype_8[chartype as usize] == ucp_N)
                            as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                    }

                    /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                    which means that Perl space and POSIX space are now identical. PCRE
                    was changed at release 8.34. */

                    /* PT_SPACE: Perl space; PT_PXSPACE: POSIX space */
                    PT_SPACE | PT_PXSPACE => {
                        match fc {
                            /* HSPACE_CASES: */
                            0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002
                            | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                            | 0x200a | 0x202f | 0x205f | 0x3000
                            /* VSPACE_CASES: */
                            | 0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => {
                                if notmatch != FALSE {
                                    rrc = MATCH_NOMATCH;
                                    lbl = LBL_RETURN_SWITCH;
                                    continue 'sw;
                                }
                            }

                            _ => {
                                if ((_pcre2_ucp_gentype_8[prop.chartype as usize] == ucp_Z)
                                    as BOOL)
                                    == notmatch
                                {
                                    rrc = MATCH_NOMATCH;
                                    lbl = LBL_RETURN_SWITCH;
                                    continue 'sw;
                                }
                            }
                        }
                    }

                    PT_WORD => {
                        let chartype: u32 = prop.chartype as u32;
                        if ((_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                            || _pcre2_ucp_gentype_8[chartype as usize] == ucp_N
                            || chartype == ucp_Mn
                            || chartype == ucp_Pc) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                    }

                    PT_CLIST => {
                        let mut cp: *const u32 = _pcre2_ucd_caseless_sets_8
                            .as_ptr()
                            .add(*(*F).ecode.add(2) as usize);
                        loop {
                            if fc < *cp {
                                if notmatch != FALSE {
                                    break;
                                } else {
                                    rrc = MATCH_NOMATCH;
                                    lbl = LBL_RETURN_SWITCH;
                                    continue 'sw;
                                }
                            }
                            let v = *cp;
                            cp = cp.add(1);
                            if fc == v {
                                if notmatch != FALSE {
                                    rrc = MATCH_NOMATCH;
                                    lbl = LBL_RETURN_SWITCH;
                                    continue 'sw;
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                    }

                    PT_BIDICL => {
                        if ((UCD_BIDICLASS_PROP(prop) == *(*F).ecode.add(2) as u32) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                    }

                    PT_BOOL => {
                        let ok: BOOL = crate::internal::boolprop_set_bit(
                            UCD_BPROPS_PROP(prop) as usize,
                            *(*F).ecode.add(2) as u32,
                        ) as BOOL;
                        if ok == notmatch {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                    }

                    /* This should never occur */

                    _ => {
                        return PCRE2_ERROR_INTERNAL;
                    }
                }

                (*F).ecode = (*F).ecode.add(3);
            }
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Match an extended Unicode sequence. We will get here only if the support
        is in the binary; otherwise a compile-time error occurs. */

        OP_EXTUNI => {
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            } else {
                /* GETCHARINCTEST(fc, Feptr) */
                fc = *(*F).eptr as u32;
                (*F).eptr = (*F).eptr.add(1);
                if utf != FALSE && fc >= 0xc0 {
                    let r = getutf8inc(fc, (*F).eptr);
                    fc = r.0;
                    (*F).eptr = r.1;
                }
                (*F).eptr = crate::extuni::_pcre2_extuni_8(
                    fc,
                    (*F).eptr,
                    (*mb).start_subject,
                    (*mb).end_subject,
                    utf,
                    core::ptr::null_mut(),
                );
            }
            /* CHECK_PARTIAL() */
            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        _ => {}
    }
}
}
