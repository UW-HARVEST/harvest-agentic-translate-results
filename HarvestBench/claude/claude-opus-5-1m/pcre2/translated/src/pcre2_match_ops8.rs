/* Fragment 8: the "maximize" (else) branch of the repeated character type
group (OP_TYPESTAR etc.), i.e. c_src/src/pcre2_match.c lines 4369..5224.

  Lstart_eptr = F->fields.type_repeat.start_eptr
  Lmin        = F->fields.type_repeat.min
  Lmax        = F->fields.type_repeat.max
  Lctype      = F->fields.type_repeat.ctype
  Lpropvalue  = F->fields.type_repeat.propvalue
*/
{
    match state {
        /* If maximizing, it is worth using inline code for speed, doing the type
        test once at the start (i.e. keep it out of the loops). Once again,
        "notmatch" can be an ordinary local variable because the loops do not call
        RMATCH. */
        ST_C6_2 => {
            /* In C, `proptype` is an ordinary local that was set when the
            REPEATTYPE label was passed. The state machine reaches this code
            through the head of the 'sm loop, so the Rust compiler cannot see
            that assignment; recompute the identical value from Fecode, which
            has not been altered since: when Lctype is OP_PROP/OP_NOTPROP,
            Fecode[-2] holds the property type (and Fecode[-1] Lpropvalue),
            otherwise proptype is -1. */
            let proptype: c_int = if (*F).fields.type_repeat.ctype == OP_PROP
                || (*F).fields.type_repeat.ctype == OP_NOTPROP
            {
                *(*F).ecode.offset(-2) as c_int
            } else {
                -1
            };

            (*F).fields.type_repeat.start_eptr = (*F).eptr; /* Remember where we started */

            if proptype >= 0 {
                let notmatch: BOOL = ((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL;
                match proptype as u32 {
                    PT_LAMP => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let chartype: c_int;
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                            chartype = UCD_CHARTYPE(fc) as c_int;
                            if ((chartype == ucp_Lu as c_int
                                || chartype == ucp_Ll as c_int
                                || chartype == ucp_Lt as c_int) as BOOL)
                                == notmatch
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    PT_GC => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                            if ((UCD_CATEGORY(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                                == notmatch
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    PT_PC => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                            if ((UCD_CHARTYPE(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                                == notmatch
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    PT_SC => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                            if ((UCD_SCRIPT(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                                == notmatch
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    PT_SCX => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let ok: BOOL;
                            let prop: *const ucd_record;
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                            prop = GET_UCD(fc);
                            ok = (((*prop).script as u32 == (*F).fields.type_repeat.propvalue)
                                || MAPBIT!(
                                    _pcre2_ucd_script_sets_8
                                        .as_ptr()
                                        .add(UCD_SCRIPTX_PROP(prop) as usize),
                                    (*F).fields.type_repeat.propvalue
                                ) != 0) as BOOL;
                            if ok == notmatch {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    PT_ALNUM => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let category: c_int;
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                            category = UCD_CATEGORY(fc) as c_int;
                            if ((category == ucp_L as c_int || category == ucp_N as c_int) as BOOL)
                                == notmatch
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                    which means that Perl space and POSIX space are now identical. PCRE
                    was changed at release 8.34. */
                    PT_SPACE /* Perl space */ | PT_PXSPACE /* POSIX space */ => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                            match fc {
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
                                | 0x3000
                                /* VSPACE_CASES: */
                                | CHAR_LF
                                | CHAR_VT
                                | CHAR_FF
                                | CHAR_CR
                                | CHAR_NEL
                                | 0x2028
                                | 0x2029 => {
                                    if notmatch != 0 {
                                        state = ST_ENDLOOP99;
                                        continue 'sm;
                                    } /* Break the loop */
                                }

                                _ => {
                                    if ((UCD_CATEGORY(fc) == ucp_Z) as BOOL) == notmatch {
                                        state = ST_ENDLOOP99;
                                        continue 'sm;
                                    } /* Break the loop */
                                }
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                        state = ST_ENDLOOP99;
                        continue 'sm;
                    }

                    PT_WORD => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let chartype: c_int;
                            let category: c_int;
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                            chartype = UCD_CHARTYPE(fc) as c_int;
                            category =
                                *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) as c_int;
                            if ((category == ucp_L as c_int
                                || category == ucp_N as c_int
                                || chartype == ucp_Mn as c_int
                                || chartype == ucp_Pc as c_int) as BOOL)
                                == notmatch
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    PT_CLIST => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut cp: *const u32;
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                            {
                                cp = _pcre2_ucd_caseless_sets_8
                                    .as_ptr()
                                    .add((*F).fields.type_repeat.propvalue as usize);
                                loop {
                                    if fc < *cp {
                                        if notmatch != 0 {
                                            break;
                                        } else {
                                            state = ST_GOT_MAX;
                                            continue 'sm;
                                        }
                                    }
                                    let cpv: u32 = *cp;
                                    cp = cp.add(1);
                                    if fc == cpv {
                                        if notmatch != 0 {
                                            state = ST_GOT_MAX;
                                            continue 'sm;
                                        } else {
                                            break;
                                        }
                                    }
                                }
                            }

                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                        state = ST_GOT_MAX;
                        continue 'sm;
                    }

                    PT_UCNC => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                            if ((fc == CHAR_DOLLAR_SIGN
                                || fc == CHAR_COMMERCIAL_AT
                                || fc == CHAR_GRAVE_ACCENT
                                || (fc >= 0xa0 && fc <= 0xd7ff)
                                || fc >= 0xe000) as BOOL)
                                == notmatch
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    PT_BIDICL => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                            if ((UCD_BIDICLASS(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                                == notmatch
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    PT_BOOL => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let ok: BOOL;
                            let prop: *const ucd_record;
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                            prop = GET_UCD(fc);
                            ok = (MAPBIT!(
                                _pcre2_ucd_boolprop_sets_8
                                    .as_ptr()
                                    .add(UCD_BPROPS_PROP(prop) as usize),
                                (*F).fields.type_repeat.propvalue
                            ) != 0) as BOOL;
                            if ok == notmatch {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    /* LCOV_EXCL_START */
                    _ => {
                        return PCRE2_ERROR_INTERNAL;
                    }
                    /* LCOV_EXCL_STOP */
                }

                /* Feptr is now past the end of the maximum run */

                state = ST_C8_1;
                continue 'sm;
            }
            /* Match extended Unicode grapheme clusters. We will get here only if the
            support is in the binary; otherwise a compile-time error occurs. */
            else if (*F).fields.type_repeat.ctype == OP_EXTUNI {
                i = (*F).fields.type_repeat.min;
                while i < (*F).fields.type_repeat.max {
                    if (*F).eptr >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        break;
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
                    i = i.wrapping_add(1);
                }

                /* Feptr is now past the end of the maximum run */

                state = ST_C8_3;
                continue 'sm;
            } else if utf != 0 {
                match (*F).fields.type_repeat.ctype {
                    OP_ANY => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            if IS_NEWLINE!((*F).eptr, mb, (*mb).end_subject, utf) {
                                break;
                            }
                            if (*mb).partial != 0 && /* Take care with CRLF partial */
                                (*F).eptr.add(1) >= (*mb).end_subject &&
                                (*mb).nltype == NLTYPE_FIXED &&
                                (*mb).nllen == 2 &&
                                *(*F).eptr == (*mb).nl[0]
                            {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 {
                                    return PCRE2_ERROR_PARTIAL;
                                }
                            }
                            (*F).eptr = (*F).eptr.add(1);
                            ACROSSCHAR!(
                                (*F).eptr < (*mb).end_subject,
                                (*F).eptr,
                                (*F).eptr = (*F).eptr.add(1)
                            );
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_ALLANY => {
                        if (*F).fields.type_repeat.max < u32::MAX {
                            i = (*F).fields.type_repeat.min;
                            while i < (*F).fields.type_repeat.max {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                (*F).eptr = (*F).eptr.add(1);
                                ACROSSCHAR!(
                                    (*F).eptr < (*mb).end_subject,
                                    (*F).eptr,
                                    (*F).eptr = (*F).eptr.add(1)
                                );
                                i = i.wrapping_add(1);
                            }
                        } else {
                            (*F).eptr = (*mb).end_subject; /* Unlimited UTF-8 repeat */
                            SCHECK_PARTIAL!();
                        }
                    }

                    /* The "byte" (i.e. "code unit") case is the same as non-UTF */
                    OP_ANYBYTE => {
                        fc = (*F)
                            .fields
                            .type_repeat
                            .max
                            .wrapping_sub((*F).fields.type_repeat.min);
                        if fc > (*mb).end_subject.offset_from((*F).eptr) as u32 {
                            (*F).eptr = (*mb).end_subject;
                            SCHECK_PARTIAL!();
                        } else {
                            (*F).eptr = (*F).eptr.add(fc as usize);
                        }
                    }

                    OP_ANYNL => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLEN!(fc, (*F).eptr, len);
                            if fc == CHAR_CR {
                                (*F).eptr = (*F).eptr.add(1);
                                if (*F).eptr >= (*mb).end_subject {
                                    break;
                                }
                                if *(*F).eptr as u32 == CHAR_LF {
                                    (*F).eptr = (*F).eptr.add(1);
                                }
                            } else {
                                if fc != CHAR_LF
                                    && ((*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF
                                        || (fc != CHAR_VT
                                            && fc != CHAR_FF
                                            && fc != CHAR_NEL
                                            && fc != 0x2028
                                            && fc != 0x2029))
                                {
                                    break;
                                }
                                (*F).eptr = (*F).eptr.add(len as usize);
                            }
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_NOT_HSPACE | OP_HSPACE => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let gotspace: BOOL;
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLEN!(fc, (*F).eptr, len);
                            gotspace = match fc {
                                /* HSPACE_CASES: */
                                CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000
                                | 0x2001 | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007
                                | 0x2008 | 0x2009 | 0x200a | 0x202f | 0x205f | 0x3000 => TRUE,
                                _ => FALSE,
                            };
                            if gotspace
                                == (((*F).fields.type_repeat.ctype == OP_NOT_HSPACE) as BOOL)
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_NOT_VSPACE | OP_VSPACE => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let gotspace: BOOL;
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLEN!(fc, (*F).eptr, len);
                            gotspace = match fc {
                                /* VSPACE_CASES: */
                                CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028
                                | 0x2029 => TRUE,
                                _ => FALSE,
                            };
                            if gotspace
                                == (((*F).fields.type_repeat.ctype == OP_NOT_VSPACE) as BOOL)
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_NOT_DIGIT => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLEN!(fc, (*F).eptr, len);
                            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_DIGIT => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLEN!(fc, (*F).eptr, len);
                            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_NOT_WHITESPACE => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLEN!(fc, (*F).eptr, len);
                            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_WHITESPACE => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLEN!(fc, (*F).eptr, len);
                            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_NOT_WORDCHAR => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLEN!(fc, (*F).eptr, len);
                            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_WORDCHAR => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLEN!(fc, (*F).eptr, len);
                            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }

                    /* LCOV_EXCL_START */
                    _ => {
                        return PCRE2_ERROR_INTERNAL;
                    }
                    /* LCOV_EXCL_STOP */
                }

                state = ST_C8_5;
                continue 'sm;
            }
            /* Not UTF mode */
            else {
                match (*F).fields.type_repeat.ctype {
                    OP_ANY => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            if IS_NEWLINE!((*F).eptr, mb, (*mb).end_subject, utf) {
                                break;
                            }
                            if (*mb).partial != 0 && /* Take care with CRLF partial */
                                (*F).eptr.add(1) >= (*mb).end_subject &&
                                (*mb).nltype == NLTYPE_FIXED &&
                                (*mb).nllen == 2 &&
                                *(*F).eptr == (*mb).nl[0]
                            {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 {
                                    return PCRE2_ERROR_PARTIAL;
                                }
                            }
                            (*F).eptr = (*F).eptr.add(1);
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_ALLANY | OP_ANYBYTE => {
                        fc = (*F)
                            .fields
                            .type_repeat
                            .max
                            .wrapping_sub((*F).fields.type_repeat.min);
                        if fc > (*mb).end_subject.offset_from((*F).eptr) as u32 {
                            (*F).eptr = (*mb).end_subject;
                            SCHECK_PARTIAL!();
                        } else {
                            (*F).eptr = (*F).eptr.add(fc as usize);
                        }
                    }

                    OP_ANYNL => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            fc = *(*F).eptr as u32;
                            if fc == CHAR_CR {
                                (*F).eptr = (*F).eptr.add(1);
                                if (*F).eptr >= (*mb).end_subject {
                                    break;
                                }
                                if *(*F).eptr as u32 == CHAR_LF {
                                    (*F).eptr = (*F).eptr.add(1);
                                }
                            } else {
                                if fc != CHAR_LF
                                    && ((*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF
                                        || (fc != CHAR_VT && fc != CHAR_FF && fc != CHAR_NEL))
                                {
                                    break;
                                }
                                (*F).eptr = (*F).eptr.add(1);
                            }
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_NOT_HSPACE => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            match *(*F).eptr as u32 {
                                /* HSPACE_BYTE_CASES: */
                                CHAR_HT | CHAR_SPACE | CHAR_NBSP => {
                                    state = ST_ENDLOOP00;
                                    continue 'sm;
                                }
                                _ => {
                                    (*F).eptr = (*F).eptr.add(1);
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                        state = ST_ENDLOOP00;
                        continue 'sm;
                    }

                    OP_HSPACE => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            match *(*F).eptr as u32 {
                                /* HSPACE_BYTE_CASES: */
                                CHAR_HT | CHAR_SPACE | CHAR_NBSP => {
                                    (*F).eptr = (*F).eptr.add(1);
                                }
                                _ => {
                                    state = ST_ENDLOOP01;
                                    continue 'sm;
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                        state = ST_ENDLOOP01;
                        continue 'sm;
                    }

                    OP_NOT_VSPACE => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            match *(*F).eptr as u32 {
                                /* VSPACE_BYTE_CASES: */
                                CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL => {
                                    state = ST_ENDLOOP02;
                                    continue 'sm;
                                }
                                _ => {
                                    (*F).eptr = (*F).eptr.add(1);
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                        state = ST_ENDLOOP02;
                        continue 'sm;
                    }

                    OP_VSPACE => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            match *(*F).eptr as u32 {
                                /* VSPACE_BYTE_CASES: */
                                CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL => {
                                    (*F).eptr = (*F).eptr.add(1);
                                }
                                _ => {
                                    state = ST_ENDLOOP03;
                                    continue 'sm;
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                        state = ST_ENDLOOP03;
                        continue 'sm;
                    }

                    OP_NOT_DIGIT => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            if MAX_255!(*(*F).eptr) != 0
                                && (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_digit) != 0
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(1);
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_DIGIT => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            if MAX_255!(*(*F).eptr) == 0
                                || (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_digit) == 0
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(1);
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_NOT_WHITESPACE => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            if MAX_255!(*(*F).eptr) != 0
                                && (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_space) != 0
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(1);
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_WHITESPACE => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            if MAX_255!(*(*F).eptr) == 0
                                || (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_space) == 0
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(1);
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_NOT_WORDCHAR => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            if MAX_255!(*(*F).eptr) != 0
                                && (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_word) != 0
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(1);
                            i = i.wrapping_add(1);
                        }
                    }

                    OP_WORDCHAR => {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            if MAX_255!(*(*F).eptr) == 0
                                || (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_word) == 0
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(1);
                            i = i.wrapping_add(1);
                        }
                    }

                    /* LCOV_EXCL_START */
                    _ => {
                        return PCRE2_ERROR_INTERNAL;
                    }
                    /* LCOV_EXCL_STOP */
                }

                state = ST_C8_7;
                continue 'sm;
            }
        }

        /* GOT_MAX: and ENDLOOP99: leave the switch(proptype) of the property
        branch, so control continues with the code that follows it. */

        ST_GOT_MAX => {
            state = ST_C8_1;
            continue 'sm;
        }

        ST_ENDLOOP99 => {
            state = ST_C8_1;
            continue 'sm;
        }

        /* ENDLOOP00: .. ENDLOOP03: leave the switch(Lctype) of the non-UTF
        branch, so control continues with the code that follows it. */

        ST_ENDLOOP00 => {
            state = ST_C8_7;
            continue 'sm;
        }

        ST_ENDLOOP01 => {
            state = ST_C8_7;
            continue 'sm;
        }

        ST_ENDLOOP02 => {
            state = ST_C8_7;
            continue 'sm;
        }

        ST_ENDLOOP03 => {
            state = ST_C8_7;
            continue 'sm;
        }

        /* Tail of the Unicode property branch: Feptr is now past the end of the
        maximum run. */

        ST_C8_1 => {
            if reptype == REPTYPE_POS {
                state = ST_TOP;
                continue 'sm;
            } /* No backtracking */

            /* After \C in UTF mode, Lstart_eptr might be in the middle of a
            Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
            go too far. */

            state = ST_C8_2;
            continue 'sm;
        }

        ST_C8_2 => {
            if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
                state = ST_TOP;
                continue 'sm;
            }
            RMATCH!((*F).ecode, RM221);
        }

        ST_L_RM221 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).eptr = (*F).eptr.sub(1);
            if utf != 0 {
                BACKCHAR!((*F).eptr);
            }
            state = ST_C8_2;
            continue 'sm;
        }

        /* Tail of the OP_EXTUNI branch. */

        ST_C8_3 => {
            if reptype == REPTYPE_POS {
                state = ST_TOP;
                continue 'sm;
            } /* No backtracking */

            /* We use <= Lstart_eptr rather than == Lstart_eptr to detect the start
            of the run while backtracking because the use of \C in UTF mode can
            cause BACKCHAR to move back past Lstart_eptr. This is just palliative;
            the use of \C in UTF mode is fraught with danger. */

            state = ST_C8_4;
            continue 'sm;
        }

        ST_C8_4 => {
            if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
                state = ST_TOP;
                continue 'sm;
            } /* At start of char run */
            RMATCH!((*F).ecode, RM219);
        }

        ST_L_RM219 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }

            /* Backtracking over an extended grapheme cluster involves inspecting
            the previous two characters (if present) to see if a break is
            permitted between them. */

            let mut lgb: c_int;
            let mut rgb: c_int;
            let mut fptr: PCRE2_SPTR;

            (*F).eptr = (*F).eptr.sub(1);
            if utf == 0 {
                fc = *(*F).eptr as u32;
            } else {
                BACKCHAR!((*F).eptr);
                GETCHAR!(fc, (*F).eptr);
            }
            rgb = UCD_GRAPHBREAK(fc) as c_int;

            loop {
                if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
                    break;
                } /* At start of char run */
                fptr = (*F).eptr.sub(1);
                if utf == 0 {
                    fc = *fptr as u32;
                } else {
                    BACKCHAR!(fptr);
                    GETCHAR!(fc, fptr);
                }
                lgb = UCD_GRAPHBREAK(fc) as c_int;
                if (*_pcre2_ucp_gbtable_8.as_ptr().add(lgb as usize) & (1u32 << rgb)) == 0 {
                    break;
                }
                (*F).eptr = fptr;
                rgb = lgb;
            }

            state = ST_C8_4;
            continue 'sm;
        }

        /* Tail of the UTF branch. */

        ST_C8_5 => {
            if reptype == REPTYPE_POS {
                state = ST_TOP;
                continue 'sm;
            } /* No backtracking */

            /* After \C in UTF mode, Lstart_eptr might be in the middle of a
            Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't go
            too far. */

            state = ST_C8_6;
            continue 'sm;
        }

        ST_C8_6 => {
            if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
                state = ST_TOP;
                continue 'sm;
            }
            RMATCH!((*F).ecode, RM220);
        }

        ST_L_RM220 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).eptr = (*F).eptr.sub(1);
            BACKCHAR!((*F).eptr);
            if (*F).fields.type_repeat.ctype == OP_ANYNL
                && (*F).eptr > (*F).fields.type_repeat.start_eptr
                && *(*F).eptr as u32 == CHAR_NL
                && *(*F).eptr.offset(-1) as u32 == CHAR_CR
            {
                (*F).eptr = (*F).eptr.sub(1);
            }
            state = ST_C8_6;
            continue 'sm;
        }

        /* Tail of the non-UTF branch. */

        ST_C8_7 => {
            if reptype == REPTYPE_POS {
                state = ST_TOP;
                continue 'sm;
            } /* No backtracking */

            state = ST_C8_8;
            continue 'sm;
        }

        ST_C8_8 => {
            if (*F).eptr == (*F).fields.type_repeat.start_eptr {
                state = ST_TOP;
                continue 'sm;
            }
            RMATCH!((*F).ecode, RM34);
        }

        ST_L_RM34 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).eptr = (*F).eptr.sub(1);
            if (*F).fields.type_repeat.ctype == OP_ANYNL
                && (*F).eptr > (*F).fields.type_repeat.start_eptr
                && *(*F).eptr as u32 == CHAR_LF
                && *(*F).eptr.offset(-1) as u32 == CHAR_CR
            {
                (*F).eptr = (*F).eptr.sub(1);
            }
            state = ST_C8_8;
            continue 'sm;
        }

        _ => {}
    }
}
