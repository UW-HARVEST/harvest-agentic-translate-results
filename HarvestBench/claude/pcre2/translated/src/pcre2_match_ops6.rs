{
    /* ===================================================================== */
    /* Match a single character type repeatedly. Note that the property type
    does not need to be in a stack frame as it is not used within an RMATCH()
    loop.

    #define Lstart_eptr  F->fields.type_repeat.start_eptr
    #define Lmin         F->fields.type_repeat.min
    #define Lmax         F->fields.type_repeat.max
    #define Lctype       F->fields.type_repeat.ctype
    #define Lpropvalue   F->fields.type_repeat.propvalue          */

    macro_rules! Lmin {
        () => {
            (*F).fields.type_repeat.min
        };
    }
    macro_rules! Lmax {
        () => {
            (*F).fields.type_repeat.max
        };
    }
    macro_rules! Lctype {
        () => {
            (*F).fields.type_repeat.ctype
        };
    }
    macro_rules! Lpropvalue {
        () => {
            (*F).fields.type_repeat.propvalue
        };
    }

    match state {
        OP_TYPEEXACT => {
            Lmax!() = GET2!((*F).ecode, 1);
            Lmin!() = Lmax!();
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            state = ST_REPEATTYPE;
            continue 'sm;
        }

        OP_TYPEUPTO | OP_TYPEMINUPTO => {
            Lmin!() = 0;
            Lmax!() = GET2!((*F).ecode, 1);
            reptype = if *(*F).ecode as u32 == OP_TYPEMINUPTO {
                REPTYPE_MIN
            } else {
                REPTYPE_MAX
            };
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            state = ST_REPEATTYPE;
            continue 'sm;
        }

        OP_TYPEPOSSTAR => {
            reptype = REPTYPE_POS;
            Lmin!() = 0;
            Lmax!() = u32::MAX;
            (*F).ecode = (*F).ecode.add(1);
            state = ST_REPEATTYPE;
            continue 'sm;
        }

        OP_TYPEPOSPLUS => {
            reptype = REPTYPE_POS;
            Lmin!() = 1;
            Lmax!() = u32::MAX;
            (*F).ecode = (*F).ecode.add(1);
            state = ST_REPEATTYPE;
            continue 'sm;
        }

        OP_TYPEPOSQUERY => {
            reptype = REPTYPE_POS;
            Lmin!() = 0;
            Lmax!() = 1;
            (*F).ecode = (*F).ecode.add(1);
            state = ST_REPEATTYPE;
            continue 'sm;
        }

        OP_TYPEPOSUPTO => {
            reptype = REPTYPE_POS;
            Lmin!() = 0;
            Lmax!() = GET2!((*F).ecode, 1);
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            state = ST_REPEATTYPE;
            continue 'sm;
        }

        OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
        | OP_TYPEMINQUERY => {
            fc = ({
                let t__ = *(*F).ecode;
                (*F).ecode = (*F).ecode.add(1);
                t__
            } as u32)
                .wrapping_sub(OP_TYPESTAR);
            Lmin!() = *rep_min.as_ptr().add(fc as usize);
            Lmax!() = *rep_max.as_ptr().add(fc as usize);
            reptype = *rep_typ.as_ptr().add(fc as usize);

            /* Common code for all repeated character type matches. */

            state = ST_REPEATTYPE;
            continue 'sm;
        }

        ST_REPEATTYPE => {
            Lctype!() = {
                let t__ = *(*F).ecode;
                (*F).ecode = (*F).ecode.add(1);
                t__
            } as u32; /* Code for the character type */

            if Lctype!() == OP_PROP || Lctype!() == OP_NOTPROP {
                proptype = {
                    let t__ = *(*F).ecode;
                    (*F).ecode = (*F).ecode.add(1);
                    t__
                } as c_int;
                Lpropvalue!() = {
                    let t__ = *(*F).ecode;
                    (*F).ecode = (*F).ecode.add(1);
                    t__
                } as u32;
            } else {
                proptype = -1;
            }

            /* First, ensure the minimum number of matches are present. Use inline
            code for maximizing the speed, and do the type test once at the start
            (i.e. keep it out of the loops). As there are no calls to RMATCH in the
            loops, we can use an ordinary variable for "notmatch". The code for UTF
            mode is separated out for tidiness, except for Unicode property tests. */

            if Lmin!() > 0 {
                if proptype >= 0
                /* Property tests in all modes */
                {
                    let notmatch: BOOL = (Lctype!() == OP_NOTPROP) as BOOL;
                    match proptype as u32 {
                        PT_LAMP => {
                            i = 1;
                            while i <= Lmin!() {
                                let chartype: u32;
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINCTEST!(fc, (*F).eptr, utf);
                                chartype = UCD_CHARTYPE(fc);
                                if ((chartype == ucp_Lu
                                    || chartype == ucp_Ll
                                    || chartype == ucp_Lt) as BOOL)
                                    == notmatch
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        PT_GC => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINCTEST!(fc, (*F).eptr, utf);
                                if ((UCD_CATEGORY(fc) == Lpropvalue!()) as BOOL) == notmatch {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        PT_PC => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINCTEST!(fc, (*F).eptr, utf);
                                if ((UCD_CHARTYPE(fc) == Lpropvalue!()) as BOOL) == notmatch {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        PT_SC => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINCTEST!(fc, (*F).eptr, utf);
                                if ((UCD_SCRIPT(fc) == Lpropvalue!()) as BOOL) == notmatch {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        PT_SCX => {
                            i = 1;
                            while i <= Lmin!() {
                                let ok: BOOL;
                                let prop: *const ucd_record;
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINCTEST!(fc, (*F).eptr, utf);
                                prop = GET_UCD(fc);
                                ok = (((*prop).script as u32 == Lpropvalue!())
                                    || MAPBIT!(
                                        _pcre2_ucd_script_sets_8
                                            .as_ptr()
                                            .add(UCD_SCRIPTX_PROP(prop) as usize),
                                        Lpropvalue!()
                                    ) != 0) as BOOL;
                                if ok == notmatch {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        PT_ALNUM => {
                            i = 1;
                            while i <= Lmin!() {
                                let category: u32;
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINCTEST!(fc, (*F).eptr, utf);
                                category = UCD_CATEGORY(fc);
                                if (((category == ucp_L || category == ucp_N) as BOOL)) == notmatch
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                        which means that Perl space and POSIX space are now identical. PCRE
                        was changed at release 8.34. */

                        PT_SPACE | PT_PXSPACE => {
                            /* Perl space / POSIX space */
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINCTEST!(fc, (*F).eptr, utf);
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
                                            RRETURN!(MATCH_NOMATCH);
                                        }
                                    }

                                    _ => {
                                        if ((UCD_CATEGORY(fc) == ucp_Z) as BOOL) == notmatch {
                                            RRETURN!(MATCH_NOMATCH);
                                        }
                                    }
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        PT_WORD => {
                            i = 1;
                            while i <= Lmin!() {
                                let chartype: u32;
                                let category: u32;
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINCTEST!(fc, (*F).eptr, utf);
                                chartype = UCD_CHARTYPE(fc);
                                category = *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize);
                                if ((category == ucp_L
                                    || category == ucp_N
                                    || chartype == ucp_Mn
                                    || chartype == ucp_Pc) as BOOL)
                                    == notmatch
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        PT_CLIST => {
                            i = 1;
                            while i <= Lmin!() {
                                let mut cp: *const u32;
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINCTEST!(fc, (*F).eptr, utf);
                                cp = _pcre2_ucd_caseless_sets_8
                                    .as_ptr()
                                    .add(Lpropvalue!() as usize);
                                loop {
                                    if fc < *cp {
                                        if notmatch != 0 {
                                            break;
                                        }
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    let t__ = *cp;
                                    cp = cp.add(1);
                                    if fc == t__ {
                                        if notmatch != 0 {
                                            RRETURN!(MATCH_NOMATCH);
                                        }
                                        break;
                                    }
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        PT_UCNC => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINCTEST!(fc, (*F).eptr, utf);
                                if ((fc == CHAR_DOLLAR_SIGN
                                    || fc == CHAR_COMMERCIAL_AT
                                    || fc == CHAR_GRAVE_ACCENT
                                    || (fc >= 0xa0 && fc <= 0xd7ff)
                                    || fc >= 0xe000) as BOOL)
                                    == notmatch
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        PT_BIDICL => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINCTEST!(fc, (*F).eptr, utf);
                                if ((UCD_BIDICLASS(fc) == Lpropvalue!()) as BOOL) == notmatch {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        PT_BOOL => {
                            i = 1;
                            while i <= Lmin!() {
                                let ok: BOOL;
                                let prop: *const ucd_record;
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINCTEST!(fc, (*F).eptr, utf);
                                prop = GET_UCD(fc);
                                ok = (MAPBIT!(
                                    _pcre2_ucd_boolprop_sets_8
                                        .as_ptr()
                                        .add(UCD_BPROPS_PROP(prop) as usize),
                                    Lpropvalue!()
                                ) != 0) as BOOL;
                                if ok == notmatch {
                                    RRETURN!(MATCH_NOMATCH);
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
                else if Lctype!() == OP_EXTUNI {
                    i = 1;
                    while i <= Lmin!() {
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
                        i = i.wrapping_add(1);
                    }
                }
                /* Handle all other cases in UTF mode */
                else if utf != 0 {
                    match Lctype!() {
                        OP_ANY => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                if IS_NEWLINE!((*F).eptr, mb, (*mb).end_subject, utf) {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                if (*mb).partial != 0
                                    && (*F).eptr.add(1) >= (*mb).end_subject
                                    && (*mb).nltype == NLTYPE_FIXED
                                    && (*mb).nllen == 2
                                    && *(*F).eptr == (*mb).nl[0]
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
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
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

                        OP_ANYBYTE => {
                            if (*F).eptr > (*mb).end_subject.wrapping_sub(Lmin!() as usize) {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            (*F).eptr = (*F).eptr.add(Lmin!() as usize);
                        }

                        OP_ANYNL => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINC!(fc, (*F).eptr);
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
                                            RRETURN!(MATCH_NOMATCH);
                                        }
                                    }

                                    _ => {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_NOT_HSPACE => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINC!(fc, (*F).eptr);
                                match fc {
                                    /* HSPACE_CASES: */
                                    CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000
                                    | 0x2001 | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006
                                    | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f | 0x205f
                                    | 0x3000 => {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    _ => {}
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_HSPACE => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINC!(fc, (*F).eptr);
                                match fc {
                                    /* HSPACE_CASES: */
                                    CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000
                                    | 0x2001 | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006
                                    | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f | 0x205f
                                    | 0x3000 => {}
                                    _ => {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_NOT_VSPACE => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINC!(fc, (*F).eptr);
                                match fc {
                                    /* VSPACE_CASES: */
                                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028
                                    | 0x2029 => {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    _ => {}
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_VSPACE => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINC!(fc, (*F).eptr);
                                match fc {
                                    /* VSPACE_CASES: */
                                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028
                                    | 0x2029 => {}
                                    _ => {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_NOT_DIGIT => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                GETCHARINC!(fc, (*F).eptr);
                                if fc < 128
                                    && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_DIGIT => {
                            i = 1;
                            while i <= Lmin!() {
                                let cc: u32;
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                cc = *(*F).eptr as u32;
                                if cc >= 128
                                    || (*(*mb).ctypes.add(cc as usize) & ctype_digit) == 0
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                (*F).eptr = (*F).eptr.add(1);
                                /* No need to skip more code units - we know it has only one. */
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_NOT_WHITESPACE => {
                            i = 1;
                            while i <= Lmin!() {
                                let cc: u32;
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                cc = *(*F).eptr as u32;
                                if cc < 128
                                    && (*(*mb).ctypes.add(cc as usize) & ctype_space) != 0
                                {
                                    RRETURN!(MATCH_NOMATCH);
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

                        OP_WHITESPACE => {
                            i = 1;
                            while i <= Lmin!() {
                                let cc: u32;
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                cc = *(*F).eptr as u32;
                                if cc >= 128
                                    || (*(*mb).ctypes.add(cc as usize) & ctype_space) == 0
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                (*F).eptr = (*F).eptr.add(1);
                                /* No need to skip more code units - we know it has only one. */
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_NOT_WORDCHAR => {
                            i = 1;
                            while i <= Lmin!() {
                                let cc: u32;
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                cc = *(*F).eptr as u32;
                                if cc < 128 && (*(*mb).ctypes.add(cc as usize) & ctype_word) != 0
                                {
                                    RRETURN!(MATCH_NOMATCH);
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

                        OP_WORDCHAR => {
                            i = 1;
                            while i <= Lmin!() {
                                let cc: u32;
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                cc = *(*F).eptr as u32;
                                if cc >= 128 || (*(*mb).ctypes.add(cc as usize) & ctype_word) == 0
                                {
                                    RRETURN!(MATCH_NOMATCH);
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
                    match Lctype!() {
                        OP_ANY => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                if IS_NEWLINE!((*F).eptr, mb, (*mb).end_subject, utf) {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                if (*mb).partial != 0
                                    && (*F).eptr.add(1) >= (*mb).end_subject
                                    && (*mb).nltype == NLTYPE_FIXED
                                    && (*mb).nllen == 2
                                    && *(*F).eptr == (*mb).nl[0]
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

                        OP_ALLANY => {
                            if (*F).eptr > (*mb).end_subject.wrapping_sub(Lmin!() as usize) {
                                SCHECK_PARTIAL!();
                                RRETURN!(MATCH_NOMATCH);
                            }
                            (*F).eptr = (*F).eptr.add(Lmin!() as usize);
                        }

                        /* This OP_ANYBYTE case will never be reached because \C gets turned
                        into OP_ALLANY in non-UTF mode. Cut out the code so that coverage
                        reports don't complain about it's never being used. */

                        /*        case OP_ANYBYTE:
                         *        if (Feptr > mb->end_subject - Lmin)
                         *          {
                         *          SCHECK_PARTIAL();
                         *          RRETURN(MATCH_NOMATCH);
                         *          }
                         *        Feptr += Lmin;
                         *        break;
                         */
                        OP_ANYNL => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                match {
                                    let t__ = *(*F).eptr;
                                    (*F).eptr = (*F).eptr.add(1);
                                    t__
                                } as u32
                                {
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
                                            RRETURN!(MATCH_NOMATCH);
                                        }
                                    }

                                    _ => {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_NOT_HSPACE => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                match {
                                    let t__ = *(*F).eptr;
                                    (*F).eptr = (*F).eptr.add(1);
                                    t__
                                } as u32
                                {
                                    /* HSPACE_BYTE_CASES: */
                                    CHAR_HT | CHAR_SPACE | CHAR_NBSP => {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    _ => {}
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_HSPACE => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                match {
                                    let t__ = *(*F).eptr;
                                    (*F).eptr = (*F).eptr.add(1);
                                    t__
                                } as u32
                                {
                                    /* HSPACE_BYTE_CASES: */
                                    CHAR_HT | CHAR_SPACE | CHAR_NBSP => {}
                                    _ => {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_NOT_VSPACE => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                match {
                                    let t__ = *(*F).eptr;
                                    (*F).eptr = (*F).eptr.add(1);
                                    t__
                                } as u32
                                {
                                    /* VSPACE_BYTE_CASES: */
                                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL => {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    _ => {}
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_VSPACE => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                match {
                                    let t__ = *(*F).eptr;
                                    (*F).eptr = (*F).eptr.add(1);
                                    t__
                                } as u32
                                {
                                    /* VSPACE_BYTE_CASES: */
                                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL => {}
                                    _ => {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_NOT_DIGIT => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                if MAX_255!(*(*F).eptr) != 0
                                    && (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_digit) != 0
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                (*F).eptr = (*F).eptr.add(1);
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_DIGIT => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                if MAX_255!(*(*F).eptr) == 0
                                    || (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_digit) == 0
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                (*F).eptr = (*F).eptr.add(1);
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_NOT_WHITESPACE => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                if MAX_255!(*(*F).eptr) != 0
                                    && (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_space) != 0
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                (*F).eptr = (*F).eptr.add(1);
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_WHITESPACE => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                if MAX_255!(*(*F).eptr) == 0
                                    || (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_space) == 0
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                (*F).eptr = (*F).eptr.add(1);
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_NOT_WORDCHAR => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                if MAX_255!(*(*F).eptr) != 0
                                    && (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_word) != 0
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                (*F).eptr = (*F).eptr.add(1);
                                i = i.wrapping_add(1);
                            }
                        }

                        OP_WORDCHAR => {
                            i = 1;
                            while i <= Lmin!() {
                                if (*F).eptr >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                if MAX_255!(*(*F).eptr) == 0
                                    || (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_word) == 0
                                {
                                    RRETURN!(MATCH_NOMATCH);
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

            /* If Lmin = Lmax we are done. Continue with the main loop. */

            if Lmin!() == Lmax!() {
                state = ST_TOP;
                continue 'sm;
            }

            /* If minimizing, we have to test the rest of the pattern before each
            subsequent match. This means we cannot use a local "notmatch" variable as
            in the other cases. As all 4 temporary 32-bit values in the frame are
            already in use, just test the type each time. */

            if reptype == REPTYPE_MIN {
                state = ST_C6_1;
                continue 'sm;
            } else {
                state = ST_C6_2;
                continue 'sm;
            }
        }

        _ => {}
    }
}
