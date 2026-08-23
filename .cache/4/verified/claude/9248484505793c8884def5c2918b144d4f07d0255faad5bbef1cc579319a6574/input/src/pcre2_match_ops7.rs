{
    match state {
        /* ===================================================================== */
        /* If minimizing, we have to test the rest of the pattern before each
        subsequent match. This means we cannot use a local "notmatch" variable as
        in the other cases. As all 4 temporary 32-bit values in the frame are
        already in use, just test the type each time.

        This is the body of "if (reptype == REPTYPE_MIN)" in the character type
        repeat group. Every exit path falls out of the enclosing if/else and then
        hits the "break" that ends the repeat character type processing, i.e.
        state = ST_TOP. */

        ST_C6_1 => {
            if proptype >= 0 {
                match proptype as u32 {
                    PT_LAMP => {
                        RMATCH!((*F).ecode, RM208);
                    }

                    PT_GC => {
                        RMATCH!((*F).ecode, RM209);
                    }

                    PT_PC => {
                        RMATCH!((*F).ecode, RM210);
                    }

                    PT_SC => {
                        RMATCH!((*F).ecode, RM211);
                    }

                    PT_SCX => {
                        RMATCH!((*F).ecode, RM224);
                    }

                    PT_ALNUM => {
                        RMATCH!((*F).ecode, RM212);
                    }

                    /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                    which means that Perl space and POSIX space are now identical. PCRE
                    was changed at release 8.34. */
                    PT_SPACE | PT_PXSPACE => {
                        RMATCH!((*F).ecode, RM213);
                    }

                    PT_WORD => {
                        RMATCH!((*F).ecode, RM214);
                    }

                    PT_CLIST => {
                        RMATCH!((*F).ecode, RM215);
                    }

                    PT_UCNC => {
                        RMATCH!((*F).ecode, RM216);
                    }

                    PT_BIDICL => {
                        RMATCH!((*F).ecode, RM223);
                    }

                    PT_BOOL => {
                        RMATCH!((*F).ecode, RM222);
                    }

                    /* This should never occur */
                    _ => {
                        return PCRE2_ERROR_INTERNAL;
                    }
                }
            }
            /* Match extended Unicode sequences. We will get here only if the
            support is in the binary; otherwise a compile-time error occurs. */
            else if (*F).fields.type_repeat.ctype == OP_EXTUNI {
                RMATCH!((*F).ecode, RM217);
            }
            /* UTF mode for non-property testing character types. */
            else if utf != 0 {
                RMATCH!((*F).ecode, RM218);
            }
            /* Not UTF mode */
            else {
                RMATCH!((*F).ecode, RM33);
            }
        }

        /* ---- case PT_LAMP: for (;;) { RMATCH(Fecode, RM208); ... } ---- */

        ST_L_RM208 => {
            let chartype: u32;
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            chartype = UCD_CHARTYPE(fc);
            if (chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt)
                == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
            {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM208);
        }

        /* ---- case PT_GC: ---- */

        ST_L_RM209 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            if (UCD_CATEGORY(fc) == (*F).fields.type_repeat.propvalue)
                == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
            {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM209);
        }

        /* ---- case PT_PC: ---- */

        ST_L_RM210 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            if (UCD_CHARTYPE(fc) == (*F).fields.type_repeat.propvalue)
                == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
            {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM210);
        }

        /* ---- case PT_SC: ---- */

        ST_L_RM211 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            if (UCD_SCRIPT(fc) == (*F).fields.type_repeat.propvalue)
                == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
            {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM211);
        }

        /* ---- case PT_SCX: ---- */

        ST_L_RM224 => {
            let ok: bool;
            let prop: *const ucd_record;
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            prop = GET_UCD(fc);
            ok = (*prop).script as u32 == (*F).fields.type_repeat.propvalue
                || MAPBIT!(
                    _pcre2_ucd_script_sets_8
                        .as_ptr()
                        .add(UCD_SCRIPTX_PROP(prop) as usize),
                    (*F).fields.type_repeat.propvalue
                ) != 0;
            if ok == ((*F).fields.type_repeat.ctype == OP_NOTPROP) {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM224);
        }

        /* ---- case PT_ALNUM: ---- */

        ST_L_RM212 => {
            let category: u32;
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            category = UCD_CATEGORY(fc);
            if (category == ucp_L || category == ucp_N)
                == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
            {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM212);
        }

        /* ---- case PT_SPACE / case PT_PXSPACE: ---- */

        ST_L_RM213 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
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
                    if (*F).fields.type_repeat.ctype == OP_NOTPROP {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                _ => {
                    if (UCD_CATEGORY(fc) == ucp_Z)
                        == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
                    {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }
            }
            RMATCH!((*F).ecode, RM213);
        }

        /* ---- case PT_WORD: ---- */

        ST_L_RM214 => {
            let chartype: u32;
            let category: u32;
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            chartype = UCD_CHARTYPE(fc);
            category = *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize);
            if (category == ucp_L || category == ucp_N || chartype == ucp_Mn || chartype == ucp_Pc)
                == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
            {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM214);
        }

        /* ---- case PT_CLIST: ---- */

        ST_L_RM215 => {
            let mut cp: *const u32;
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            cp = _pcre2_ucd_caseless_sets_8
                .as_ptr()
                .add((*F).fields.type_repeat.propvalue as usize);
            loop {
                if fc < *cp {
                    if (*F).fields.type_repeat.ctype == OP_NOTPROP {
                        break;
                    }
                    RRETURN!(MATCH_NOMATCH);
                }
                let t = *cp;
                cp = cp.add(1);
                if fc == t {
                    if (*F).fields.type_repeat.ctype == OP_NOTPROP {
                        RRETURN!(MATCH_NOMATCH);
                    }
                    break;
                }
            }
            RMATCH!((*F).ecode, RM215);
        }

        /* ---- case PT_UCNC: ---- */

        ST_L_RM216 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            if (fc == CHAR_DOLLAR_SIGN
                || fc == CHAR_COMMERCIAL_AT
                || fc == CHAR_GRAVE_ACCENT
                || (fc >= 0xa0 && fc <= 0xd7ff)
                || fc >= 0xe000)
                == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
            {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM216);
        }

        /* ---- case PT_BIDICL: ---- */

        ST_L_RM223 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            if (UCD_BIDICLASS(fc) == (*F).fields.type_repeat.propvalue)
                == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
            {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM223);
        }

        /* ---- case PT_BOOL: ---- */

        ST_L_RM222 => {
            let ok: bool;
            let prop: *const ucd_record;
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            prop = GET_UCD(fc);
            ok = MAPBIT!(
                _pcre2_ucd_boolprop_sets_8
                    .as_ptr()
                    .add(UCD_BPROPS_PROP(prop) as usize),
                (*F).fields.type_repeat.propvalue
            ) != 0;
            if ok == ((*F).fields.type_repeat.ctype == OP_NOTPROP) {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM222);
        }

        /* ---- Lctype == OP_EXTUNI: match extended Unicode sequences ---- */

        ST_L_RM217 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
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
            RMATCH!((*F).ecode, RM217);
        }

        /* ---- UTF mode for non-property testing character types ---- */

        ST_L_RM218 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).fields.type_repeat.ctype == OP_ANY
                && (IS_NEWLINE!((*F).eptr, mb, (*mb).end_subject, utf))
            {
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINC!(fc, (*F).eptr);
            match (*F).fields.type_repeat.ctype {
                OP_ANY => {
                    /* This is the non-NL case */
                    if (*mb).partial != 0 /* Take care with CRLF partial */
                        && (*F).eptr >= (*mb).end_subject
                        && (*mb).nltype == NLTYPE_FIXED
                        && (*mb).nllen == 2
                        && fc == (*mb).nl[0] as u32
                    {
                        (*mb).hitend = TRUE;
                        if (*mb).partial > 1 {
                            return PCRE2_ERROR_PARTIAL;
                        }
                    }
                }

                OP_ALLANY | OP_ANYBYTE => {}

                OP_ANYNL => match fc {
                    CHAR_CR => {
                        if (*F).eptr < (*mb).end_subject && *(*F).eptr as u32 == CHAR_LF {
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
                },

                OP_NOT_HSPACE => match fc {
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
                        RRETURN!(MATCH_NOMATCH);
                    }
                    _ => {}
                },

                OP_HSPACE => match fc {
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
                    | 0x3000 => {}
                    _ => {
                        RRETURN!(MATCH_NOMATCH);
                    }
                },

                OP_NOT_VSPACE => match fc {
                    /* VSPACE_CASES: */
                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {
                        RRETURN!(MATCH_NOMATCH);
                    }
                    _ => {}
                },

                OP_VSPACE => match fc {
                    /* VSPACE_CASES: */
                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {}
                    _ => {
                        RRETURN!(MATCH_NOMATCH);
                    }
                },

                OP_NOT_DIGIT => {
                    if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                OP_DIGIT => {
                    if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                OP_NOT_WHITESPACE => {
                    if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                OP_WHITESPACE => {
                    if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                OP_NOT_WORDCHAR => {
                    if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                OP_WORDCHAR => {
                    if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }
            RMATCH!((*F).ecode, RM218);
        }

        /* ---- Not UTF mode ---- */

        ST_L_RM33 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin = (*F).fields.type_repeat.min;
            (*F).fields.type_repeat.min = lmin.wrapping_add(1);
            if lmin >= (*F).fields.type_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).fields.type_repeat.ctype == OP_ANY
                && (IS_NEWLINE!((*F).eptr, mb, (*mb).end_subject, utf))
            {
                RRETURN!(MATCH_NOMATCH);
            }
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            match (*F).fields.type_repeat.ctype {
                OP_ANY => {
                    /* This is the non-NL case */
                    if (*mb).partial != 0 /* Take care with CRLF partial */
                        && (*F).eptr >= (*mb).end_subject
                        && (*mb).nltype == NLTYPE_FIXED
                        && (*mb).nllen == 2
                        && fc == (*mb).nl[0] as u32
                    {
                        (*mb).hitend = TRUE;
                        if (*mb).partial > 1 {
                            return PCRE2_ERROR_PARTIAL;
                        }
                    }
                }

                OP_ALLANY | OP_ANYBYTE => {}

                OP_ANYNL => match fc {
                    CHAR_CR => {
                        if (*F).eptr < (*mb).end_subject && *(*F).eptr as u32 == CHAR_LF {
                            (*F).eptr = (*F).eptr.add(1);
                        }
                    }

                    CHAR_LF => {}

                    CHAR_VT | CHAR_FF | CHAR_NEL => {
                        if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }

                    _ => {
                        RRETURN!(MATCH_NOMATCH);
                    }
                },

                OP_NOT_HSPACE => match fc {
                    /* HSPACE_BYTE_CASES: */
                    CHAR_HT | CHAR_SPACE | CHAR_NBSP => {
                        RRETURN!(MATCH_NOMATCH);
                    }
                    _ => {}
                },

                OP_HSPACE => match fc {
                    /* HSPACE_BYTE_CASES: */
                    CHAR_HT | CHAR_SPACE | CHAR_NBSP => {}
                    _ => {
                        RRETURN!(MATCH_NOMATCH);
                    }
                },

                OP_NOT_VSPACE => match fc {
                    /* VSPACE_BYTE_CASES: */
                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL => {
                        RRETURN!(MATCH_NOMATCH);
                    }
                    _ => {}
                },

                OP_VSPACE => match fc {
                    /* VSPACE_BYTE_CASES: */
                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL => {}
                    _ => {
                        RRETURN!(MATCH_NOMATCH);
                    }
                },

                OP_NOT_DIGIT => {
                    if MAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                OP_DIGIT => {
                    if MAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                OP_NOT_WHITESPACE => {
                    if MAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                OP_WHITESPACE => {
                    if MAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                OP_NOT_WORDCHAR => {
                    if MAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                OP_WORDCHAR => {
                    if MAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }
            RMATCH!((*F).ecode, RM33);
        }

        _ => {}
    }
}
