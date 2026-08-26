{
if lbl == LBL_SWITCH {
    match (*F).op as u32 {
    /* ===================================================================== */
    /* Match a single character type repeatedly. Note that the property type
    does not need to be in a stack frame as it is not used within an RMATCH()
    loop.

       #define Lstart_eptr  F->fields.type_repeat.start_eptr
       #define Lmin         F->fields.type_repeat.min
       #define Lmax         F->fields.type_repeat.max
       #define Lctype       F->fields.type_repeat.ctype
       #define Lpropvalue   F->fields.type_repeat.propvalue          */

    /* case OP_TYPEEXACT: (C 2919) */
    OP_TYPEEXACT => {
        (*F).fields.type_repeat.max = GET2((*F).ecode, 1);
        (*F).fields.type_repeat.min = (*F).fields.type_repeat.max;
        (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
        lbl = LBL_REPEATTYPE;
        continue 'sw;
    }

    /* case OP_TYPEUPTO: case OP_TYPEMINUPTO: (C 2924) */
    OP_TYPEUPTO | OP_TYPEMINUPTO => {
        (*F).fields.type_repeat.min = 0;
        (*F).fields.type_repeat.max = GET2((*F).ecode, 1);
        reptype = if *(*F).ecode as u32 == OP_TYPEMINUPTO {
            REPTYPE_MIN
        } else {
            REPTYPE_MAX
        };
        (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
        lbl = LBL_REPEATTYPE;
        continue 'sw;
    }

    /* case OP_TYPEPOSSTAR: (C 2932) */
    OP_TYPEPOSSTAR => {
        reptype = REPTYPE_POS;
        (*F).fields.type_repeat.min = 0;
        (*F).fields.type_repeat.max = u32::MAX;
        (*F).ecode = (*F).ecode.add(1);
        lbl = LBL_REPEATTYPE;
        continue 'sw;
    }

    /* case OP_TYPEPOSPLUS: (C 2939) */
    OP_TYPEPOSPLUS => {
        reptype = REPTYPE_POS;
        (*F).fields.type_repeat.min = 1;
        (*F).fields.type_repeat.max = u32::MAX;
        (*F).ecode = (*F).ecode.add(1);
        lbl = LBL_REPEATTYPE;
        continue 'sw;
    }

    /* case OP_TYPEPOSQUERY: (C 2946) */
    OP_TYPEPOSQUERY => {
        reptype = REPTYPE_POS;
        (*F).fields.type_repeat.min = 0;
        (*F).fields.type_repeat.max = 1;
        (*F).ecode = (*F).ecode.add(1);
        lbl = LBL_REPEATTYPE;
        continue 'sw;
    }

    /* case OP_TYPEPOSUPTO: (C 2953) */
    OP_TYPEPOSUPTO => {
        reptype = REPTYPE_POS;
        (*F).fields.type_repeat.min = 0;
        (*F).fields.type_repeat.max = GET2((*F).ecode, 1);
        (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
        lbl = LBL_REPEATTYPE;
        continue 'sw;
    }

    /* case OP_TYPESTAR: ... case OP_TYPEMINQUERY: (C 2960-2965) */
    OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
    | OP_TYPEMINQUERY => {
        fc = (*(*F).ecode as u32).wrapping_sub(OP_TYPESTAR);
        (*F).ecode = (*F).ecode.add(1);
        (*F).fields.type_repeat.min = rep_min[fc as usize];
        (*F).fields.type_repeat.max = rep_max[fc as usize];
        reptype = rep_typ[fc as usize];

        /* Common code for all repeated character type matches. */

        lbl = LBL_REPEATTYPE;
        continue 'sw;
    }

    _ => {}
    }
}

/* --------------------------------------------------------------------- */
/* REPEATTYPE: (C 2973) Common code for all repeated character type matches. */

if lbl == LBL_REPEATTYPE {
    (*F).fields.type_repeat.ctype = *(*F).ecode as u32; /* Code for the character type */
    (*F).ecode = (*F).ecode.add(1);

    if (*F).fields.type_repeat.ctype == OP_PROP || (*F).fields.type_repeat.ctype == OP_NOTPROP {
        proptype = *(*F).ecode as c_int;
        (*F).ecode = (*F).ecode.add(1);
        (*F).fields.type_repeat.propvalue = *(*F).ecode as u32;
        (*F).ecode = (*F).ecode.add(1);
    } else {
        proptype = -1;
    }

    /* First, ensure the minimum number of matches are present. Use inline
    code for maximizing the speed, and do the type test once at the start
    (i.e. keep it out of the loops). As there are no calls to RMATCH in the
    loops, we can use an ordinary variable for "notmatch". The code for UTF
    mode is separated out for tidiness, except for Unicode property tests. */

    if (*F).fields.type_repeat.min > 0 {
        if proptype >= 0 {
            /* Property tests in all modes */
            let notmatch: bool = (*F).fields.type_repeat.ctype == OP_NOTPROP;
            match proptype as u32 {
                /* case PT_LAMP: (C 2999) */
                PT_LAMP => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let chartype: c_int;
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
                        chartype = UCD_CHARTYPE(fc) as c_int;
                        if (chartype == ucp_Lu as c_int
                            || chartype == ucp_Ll as c_int
                            || chartype == ucp_Lt as c_int)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case PT_GC: (C 3017) */
                PT_GC => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                        if (UCD_CATEGORY(fc) == (*F).fields.type_repeat.propvalue) == notmatch {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case PT_PC: (C 3031) */
                PT_PC => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                        if (UCD_CHARTYPE(fc) == (*F).fields.type_repeat.propvalue) == notmatch {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case PT_SC: (C 3045) */
                PT_SC => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                        if (UCD_SCRIPT(fc) == (*F).fields.type_repeat.propvalue) == notmatch {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case PT_SCX: (C 3059) */
                PT_SCX => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let ok: bool;
                        let prop: &ucd_record;
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
                        prop = GET_UCD(fc);
                        ok = prop.script as u32 == (*F).fields.type_repeat.propvalue
                            || crate::internal::script_set_bit(
                                UCD_SCRIPTX_PROP(prop) as usize,
                                (*F).fields.type_repeat.propvalue,
                            );
                        if ok == notmatch {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case PT_ALNUM: (C 3078) */
                PT_ALNUM => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let category: c_int;
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
                        category = UCD_CATEGORY(fc) as c_int;
                        if (category == ucp_L as c_int || category == ucp_N as c_int) == notmatch {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                which means that Perl space and POSIX space are now identical. PCRE
                was changed at release 8.34. */

                /* case PT_SPACE: case PT_PXSPACE: (C 3098-3099) */
                PT_SPACE | PT_PXSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            /* HSPACE_CASES: VSPACE_CASES: */
                            0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002
                            | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                            | 0x200a | 0x202f | 0x205f | 0x3000 | 0x0a | 0x0b | 0x0c | 0x0d
                            | 0x85 | 0x2028 | 0x2029 => {
                                if notmatch {
                                    rrc = MATCH_NOMATCH;
                                    lbl = LBL_RETURN_SWITCH;
                                    continue 'sw;
                                }
                            }

                            _ => {
                                if (UCD_CATEGORY(fc) == ucp_Z) == notmatch {
                                    rrc = MATCH_NOMATCH;
                                    lbl = LBL_RETURN_SWITCH;
                                    continue 'sw;
                                }
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case PT_WORD: (C 3123) */
                PT_WORD => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let chartype: c_int;
                        let category: c_int;
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
                        chartype = UCD_CHARTYPE(fc) as c_int;
                        category = _pcre2_ucp_gentype_8[chartype as usize] as c_int;
                        if (category == ucp_L as c_int
                            || category == ucp_N as c_int
                            || chartype == ucp_Mn as c_int
                            || chartype == ucp_Pc as c_int)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case PT_CLIST: (C 3141) */
                PT_CLIST => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let mut cp: *const u32;
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
                        cp = _pcre2_ucd_caseless_sets_8
                            .as_ptr()
                            .add((*F).fields.type_repeat.propvalue as usize);
                        loop {
                            if fc < *cp {
                                if notmatch {
                                    break;
                                }
                                rrc = MATCH_NOMATCH;
                                lbl = LBL_RETURN_SWITCH;
                                continue 'sw;
                            }
                            let cpv = *cp;
                            cp = cp.add(1);
                            if fc == cpv {
                                if notmatch {
                                    rrc = MATCH_NOMATCH;
                                    lbl = LBL_RETURN_SWITCH;
                                    continue 'sw;
                                }
                                break;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case PT_UCNC: (C 3175) */
                PT_UCNC => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                        if (fc == CHAR_DOLLAR_SIGN
                            || fc == CHAR_COMMERCIAL_AT
                            || fc == CHAR_GRAVE_ACCENT
                            || (fc >= 0xa0 && fc <= 0xd7ff)
                            || fc >= 0xe000)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case PT_BIDICL: (C 3191) */
                PT_BIDICL => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                        if (UCD_BIDICLASS(fc) == (*F).fields.type_repeat.propvalue) == notmatch {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case PT_BOOL: (C 3205) */
                PT_BOOL => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let ok: bool;
                        let prop: &ucd_record;
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
                        prop = GET_UCD(fc);
                        ok = crate::internal::boolprop_set_bit(
                            UCD_BPROPS_PROP(prop) as usize,
                            (*F).fields.type_repeat.propvalue,
                        );
                        if ok == notmatch {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* This should not occur */

                /* LCOV_EXCL_START */
                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
                /* LCOV_EXCL_STOP */
            }
        }
        /* Match extended Unicode sequences. We will get here only if the
        support is in the binary; otherwise a compile-time error occurs. */
        else if (*F).fields.type_repeat.ctype == OP_EXTUNI {
            i = 1;
            while i <= (*F).fields.type_repeat.min {
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
                    if (*mb).partial != 0
                        && ((*F).eptr > (*mb).start_used_ptr
                            || (*mb).allowemptypartial != FALSE)
                    {
                        (*mb).hitend = TRUE;
                        if (*mb).partial > 1 {
                            return PCRE2_ERROR_PARTIAL;
                        }
                    }
                }
                i = i.wrapping_add(1);
            }
        }
        /* Handle all other cases in UTF mode */
        else if utf != FALSE {
            match (*F).fields.type_repeat.ctype {
                /* case OP_ANY: (C 3263) */
                OP_ANY => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        if {
                            /* IS_NEWLINE(Feptr) */
                            let p = (*F).eptr;
                            if (*mb).nltype != NLTYPE_FIXED {
                                p < (*mb).end_subject
                                    && crate::newline::_pcre2_is_newline_8(
                                        p,
                                        (*mb).nltype,
                                        (*mb).end_subject,
                                        &mut (*mb).nllen,
                                        utf,
                                    ) != FALSE
                            } else {
                                p <= (*mb).end_subject.sub((*mb).nllen as usize)
                                    && *p as u32 == (*mb).nl[0] as u32
                                    && ((*mb).nllen == 1
                                        || *p.add(1) as u32 == (*mb).nl[1] as u32)
                            }
                        } {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        if (*mb).partial != 0
                            && (*F).eptr.add(1) >= (*mb).end_subject
                            && (*mb).nltype == NLTYPE_FIXED
                            && (*mb).nllen == 2
                            && *(*F).eptr as u32 == (*mb).nl[0] as u32
                        {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        /* ACROSSCHAR(Feptr < mb->end_subject, Feptr, Feptr++) */
                        while (*F).eptr < (*mb).end_subject
                            && (*(*F).eptr as u32 & 0xc0) == 0x80
                        {
                            (*F).eptr = (*F).eptr.add(1);
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_ALLANY: (C 3286) */
                OP_ALLANY => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        /* ACROSSCHAR(Feptr < mb->end_subject, Feptr, Feptr++) */
                        while (*F).eptr < (*mb).end_subject
                            && (*(*F).eptr as u32 & 0xc0) == 0x80
                        {
                            (*F).eptr = (*F).eptr.add(1);
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_ANYBYTE: (C 3299) */
                OP_ANYBYTE => {
                    if (*F).eptr
                        > (*mb)
                            .end_subject
                            .wrapping_sub((*F).fields.type_repeat.min as usize)
                    {
                        rrc = MATCH_NOMATCH;
                        lbl = LBL_RETURN_SWITCH;
                        continue 'sw;
                    }
                    (*F).eptr = (*F).eptr.add((*F).fields.type_repeat.min as usize);
                }

                /* case OP_ANYNL: (C 3304) */
                OP_ANYNL => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        /* GETCHARINC(fc, Feptr) */
                        fc = *(*F).eptr as u32;
                        (*F).eptr = (*F).eptr.add(1);
                        if fc >= 0xc0 {
                            let r = getutf8inc(fc, (*F).eptr);
                            fc = r.0;
                            (*F).eptr = r.1;
                        }
                        match fc {
                            CHAR_CR => {
                                if (*F).eptr < (*mb).end_subject
                                    && *(*F).eptr as u32 == CHAR_LF
                                {
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
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_NOT_HSPACE: (C 3337) */
                OP_NOT_HSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        /* GETCHARINC(fc, Feptr) */
                        fc = *(*F).eptr as u32;
                        (*F).eptr = (*F).eptr.add(1);
                        if fc >= 0xc0 {
                            let r = getutf8inc(fc, (*F).eptr);
                            fc = r.0;
                            (*F).eptr = r.1;
                        }
                        match fc {
                            /* HSPACE_CASES: */
                            0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002
                            | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                            | 0x200a | 0x202f | 0x205f | 0x3000 => {
                                rrc = MATCH_NOMATCH;
                                lbl = LBL_RETURN_SWITCH;
                                continue 'sw;
                            }
                            _ => {}
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_HSPACE: (C 3354) */
                OP_HSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        /* GETCHARINC(fc, Feptr) */
                        fc = *(*F).eptr as u32;
                        (*F).eptr = (*F).eptr.add(1);
                        if fc >= 0xc0 {
                            let r = getutf8inc(fc, (*F).eptr);
                            fc = r.0;
                            (*F).eptr = r.1;
                        }
                        match fc {
                            /* HSPACE_CASES: */
                            0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002
                            | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                            | 0x200a | 0x202f | 0x205f | 0x3000 => {}
                            _ => {
                                rrc = MATCH_NOMATCH;
                                lbl = LBL_RETURN_SWITCH;
                                continue 'sw;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_NOT_VSPACE: (C 3371) */
                OP_NOT_VSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        /* GETCHARINC(fc, Feptr) */
                        fc = *(*F).eptr as u32;
                        (*F).eptr = (*F).eptr.add(1);
                        if fc >= 0xc0 {
                            let r = getutf8inc(fc, (*F).eptr);
                            fc = r.0;
                            (*F).eptr = r.1;
                        }
                        match fc {
                            /* VSPACE_CASES: */
                            0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => {
                                rrc = MATCH_NOMATCH;
                                lbl = LBL_RETURN_SWITCH;
                                continue 'sw;
                            }
                            _ => {}
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_VSPACE: (C 3388) */
                OP_VSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        /* GETCHARINC(fc, Feptr) */
                        fc = *(*F).eptr as u32;
                        (*F).eptr = (*F).eptr.add(1);
                        if fc >= 0xc0 {
                            let r = getutf8inc(fc, (*F).eptr);
                            fc = r.0;
                            (*F).eptr = r.1;
                        }
                        match fc {
                            /* VSPACE_CASES: */
                            0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => {}
                            _ => {
                                rrc = MATCH_NOMATCH;
                                lbl = LBL_RETURN_SWITCH;
                                continue 'sw;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_NOT_DIGIT: (C 3405) */
                OP_NOT_DIGIT => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        /* GETCHARINC(fc, Feptr) */
                        fc = *(*F).eptr as u32;
                        (*F).eptr = (*F).eptr.add(1);
                        if fc >= 0xc0 {
                            let r = getutf8inc(fc, (*F).eptr);
                            fc = r.0;
                            (*F).eptr = r.1;
                        }
                        if fc < 128 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_DIGIT: (C 3419) */
                OP_DIGIT => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        cc = *(*F).eptr as u32;
                        if cc >= 128 || (*(*mb).ctypes.add(cc as usize) & ctype_digit) == 0 {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        /* No need to skip more code units - we know it has only one. */
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_NOT_WHITESPACE: (C 3436) */
                OP_NOT_WHITESPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        cc = *(*F).eptr as u32;
                        if cc < 128 && (*(*mb).ctypes.add(cc as usize) & ctype_space) != 0 {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        /* ACROSSCHAR(Feptr < mb->end_subject, Feptr, Feptr++) */
                        while (*F).eptr < (*mb).end_subject
                            && (*(*F).eptr as u32 & 0xc0) == 0x80
                        {
                            (*F).eptr = (*F).eptr.add(1);
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_WHITESPACE: (C 3453) */
                OP_WHITESPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        cc = *(*F).eptr as u32;
                        if cc >= 128 || (*(*mb).ctypes.add(cc as usize) & ctype_space) == 0 {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        /* No need to skip more code units - we know it has only one. */
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_NOT_WORDCHAR: (C 3470) */
                OP_NOT_WORDCHAR => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        cc = *(*F).eptr as u32;
                        if cc < 128 && (*(*mb).ctypes.add(cc as usize) & ctype_word) != 0 {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        /* ACROSSCHAR(Feptr < mb->end_subject, Feptr, Feptr++) */
                        while (*F).eptr < (*mb).end_subject
                            && (*(*F).eptr as u32 & 0xc0) == 0x80
                        {
                            (*F).eptr = (*F).eptr.add(1);
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_WORDCHAR: (C 3487) */
                OP_WORDCHAR => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        cc = *(*F).eptr as u32;
                        if cc >= 128 || (*(*mb).ctypes.add(cc as usize) & ctype_word) == 0 {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        /* No need to skip more code units - we know it has only one. */
                        i = i.wrapping_add(1);
                    }
                }

                /* LCOV_EXCL_START */
                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
                /* LCOV_EXCL_STOP */
            } /* End switch(Lctype) */
        }
        /* Code for the non-UTF case for minimum matching of operators other
        than OP_PROP and OP_NOTPROP. */
        else {
            match (*F).fields.type_repeat.ctype {
                /* case OP_ANY: (C 3519) */
                OP_ANY => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        if {
                            /* IS_NEWLINE(Feptr) */
                            let p = (*F).eptr;
                            if (*mb).nltype != NLTYPE_FIXED {
                                p < (*mb).end_subject
                                    && crate::newline::_pcre2_is_newline_8(
                                        p,
                                        (*mb).nltype,
                                        (*mb).end_subject,
                                        &mut (*mb).nllen,
                                        utf,
                                    ) != FALSE
                            } else {
                                p <= (*mb).end_subject.sub((*mb).nllen as usize)
                                    && *p as u32 == (*mb).nl[0] as u32
                                    && ((*mb).nllen == 1
                                        || *p.add(1) as u32 == (*mb).nl[1] as u32)
                            }
                        } {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        if (*mb).partial != 0
                            && (*F).eptr.add(1) >= (*mb).end_subject
                            && (*mb).nltype == NLTYPE_FIXED
                            && (*mb).nllen == 2
                            && *(*F).eptr as u32 == (*mb).nl[0] as u32
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

                /* case OP_ALLANY: (C 3541) */
                OP_ALLANY => {
                    if (*F).eptr
                        > (*mb)
                            .end_subject
                            .wrapping_sub((*F).fields.type_repeat.min as usize)
                    {
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
                        rrc = MATCH_NOMATCH;
                        lbl = LBL_RETURN_SWITCH;
                        continue 'sw;
                    }
                    (*F).eptr = (*F).eptr.add((*F).fields.type_repeat.min as usize);
                }

                /* This OP_ANYBYTE case will never be reached because \C gets turned
                into OP_ALLANY in non-UTF mode. Cut out the code so that coverage
                reports don't complain about it's never being used. (C 3550-3562) */

                /* case OP_ANYNL: (C 3563) */
                OP_ANYNL => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        let cu = *(*F).eptr as u32;
                        (*F).eptr = (*F).eptr.add(1);
                        match cu {
                            CHAR_CR => {
                                if (*F).eptr < (*mb).end_subject
                                    && *(*F).eptr as u32 == CHAR_LF
                                {
                                    (*F).eptr = (*F).eptr.add(1);
                                }
                            }

                            CHAR_LF => {}

                            CHAR_VT | CHAR_FF | CHAR_NEL => {
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
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_NOT_HSPACE: (C 3595) */
                OP_NOT_HSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        let cu = *(*F).eptr as u32;
                        (*F).eptr = (*F).eptr.add(1);
                        match cu {
                            /* HSPACE_BYTE_CASES: */
                            0x09 | 0x20 | 0xa0 => {
                                rrc = MATCH_NOMATCH;
                                lbl = LBL_RETURN_SWITCH;
                                continue 'sw;
                            }
                            _ => {}
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_HSPACE: (C 3615) */
                OP_HSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        let cu = *(*F).eptr as u32;
                        (*F).eptr = (*F).eptr.add(1);
                        match cu {
                            /* HSPACE_BYTE_CASES: */
                            0x09 | 0x20 | 0xa0 => {}
                            _ => {
                                rrc = MATCH_NOMATCH;
                                lbl = LBL_RETURN_SWITCH;
                                continue 'sw;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_NOT_VSPACE: (C 3635) */
                OP_NOT_VSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        let cu = *(*F).eptr as u32;
                        (*F).eptr = (*F).eptr.add(1);
                        match cu {
                            /* VSPACE_BYTE_CASES: */
                            0x0a | 0x0b | 0x0c | 0x0d | 0x85 => {
                                rrc = MATCH_NOMATCH;
                                lbl = LBL_RETURN_SWITCH;
                                continue 'sw;
                            }
                            _ => {}
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_VSPACE: (C 3655) */
                OP_VSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        let cu = *(*F).eptr as u32;
                        (*F).eptr = (*F).eptr.add(1);
                        match cu {
                            /* VSPACE_BYTE_CASES: */
                            0x0a | 0x0b | 0x0c | 0x0d | 0x85 => {}
                            _ => {
                                rrc = MATCH_NOMATCH;
                                lbl = LBL_RETURN_SWITCH;
                                continue 'sw;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_NOT_DIGIT: (C 3675) */
                OP_NOT_DIGIT => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        if MAX_255(*(*F).eptr as u32)
                            && (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_digit) != 0
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_DIGIT: (C 3689) */
                OP_DIGIT => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        if !MAX_255(*(*F).eptr as u32)
                            || (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_digit) == 0
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_NOT_WHITESPACE: (C 3703) */
                OP_NOT_WHITESPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        if MAX_255(*(*F).eptr as u32)
                            && (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_space) != 0
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_WHITESPACE: (C 3717) */
                OP_WHITESPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        if !MAX_255(*(*F).eptr as u32)
                            || (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_space) == 0
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_NOT_WORDCHAR: (C 3731) */
                OP_NOT_WORDCHAR => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        if MAX_255(*(*F).eptr as u32)
                            && (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_word) != 0
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        i = i.wrapping_add(1);
                    }
                }

                /* case OP_WORDCHAR: (C 3745) */
                OP_WORDCHAR => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
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
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }
                        if !MAX_255(*(*F).eptr as u32)
                            || (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_word) == 0
                        {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
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
        }
    }

    /* If Lmin = Lmax we are done. Continue with the main loop. (C 3769) */

    if (*F).fields.type_repeat.min == (*F).fields.type_repeat.max {
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    /* If minimizing, we have to test the rest of the pattern before each
    subsequent match. This means we cannot use a local "notmatch" variable as
    in the other cases. As all 4 temporary 32-bit values in the frame are
    already in use, just test the type each time.

    C line 3776 onwards is translated in matcher_arms/e2.rs. */

    lbl = LBL_REPEATTYPE_2;
    continue 'sw;
}
}
