{
/* ===================================================================== */
/* Opcode arms chunk "e3": C lines 4369-5248 of pcre2_match.c.
   This is the `else` (maximizing/possessive) arm of the REPEATTYPE
   processing, reached from chunk e2 via LBL_REPEATTYPE_3, plus the RMATCH
   continuations RM221, RM219, RM220 and RM34.

   #define Lstart_eptr  F->fields.type_repeat.start_eptr
   #define Lmin         F->fields.type_repeat.min
   #define Lmax         F->fields.type_repeat.max
   #define Lctype       F->fields.type_repeat.ctype
   #define Lpropvalue   F->fields.type_repeat.propvalue                   */
/* ===================================================================== */

if lbl == LBL_REPEATTYPE_3 {
    /* C 4369: else { (i.e. not REPTYPE_MIN) */

    (*F).fields.type_repeat.start_eptr = (*F).eptr; /* Remember where we started */

    if proptype >= 0 {
        let notmatch: BOOL = ((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL;
        match proptype as u32 {
            PT_LAMP => {
                i = (*F).fields.type_repeat.min;
                while i < (*F).fields.type_repeat.max {
                    let chartype: u32;
                    let mut len: c_int = 1;
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
                        break;
                    }
                    /* GETCHARLENTEST(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if utf != FALSE && fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
                    chartype = UCD_CHARTYPE(fc);
                    if ((chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt)
                        as BOOL)
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
                        break;
                    }
                    /* GETCHARLENTEST(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if utf != FALSE && fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
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
                        break;
                    }
                    /* GETCHARLENTEST(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if utf != FALSE && fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
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
                        break;
                    }
                    /* GETCHARLENTEST(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if utf != FALSE && fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
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
                    let prop: &'static ucd_record;
                    let mut len: c_int = 1;
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
                        break;
                    }
                    /* GETCHARLENTEST(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if utf != FALSE && fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
                    prop = GET_UCD(fc);
                    ok = ((prop.script as u32 == (*F).fields.type_repeat.propvalue)
                        || crate::internal::script_set_bit(
                            UCD_SCRIPTX_PROP(prop) as usize,
                            (*F).fields.type_repeat.propvalue,
                        )) as BOOL;
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
                    let category: u32;
                    let mut len: c_int = 1;
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
                        break;
                    }
                    /* GETCHARLENTEST(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if utf != FALSE && fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
                    category = UCD_CATEGORY(fc);
                    if (((category == ucp_L) || (category == ucp_N)) as BOOL) == notmatch {
                        break;
                    }
                    (*F).eptr = (*F).eptr.add(len as usize);
                    i = i.wrapping_add(1);
                }
            }

            /* Perl space used to exclude VT, but from Perl 5.18 it is included,
            which means that Perl space and POSIX space are now identical. PCRE
            was changed at release 8.34. */

            /* PT_SPACE: Perl space; PT_PXSPACE: POSIX space */
            PT_SPACE | PT_PXSPACE => {
                'endloop99: {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: c_int = 1;
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
                            break;
                        }
                        /* GETCHARLENTEST(fc, Feptr, len) */
                        fc = *(*F).eptr as u32;
                        if utf != FALSE && fc >= 0xc0 {
                            len += utf8_extra(fc) as c_int;
                            fc = getutf8(fc, (*F).eptr);
                        }
                        match fc {
                            /* HSPACE_CASES: */
                            0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002
                            | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                            | 0x200a | 0x202f | 0x205f | 0x3000
                            /* VSPACE_CASES: */
                            | 0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => {
                                if notmatch != FALSE {
                                    break 'endloop99; /* Break the loop */
                                }
                            }

                            _ => {
                                if ((UCD_CATEGORY(fc) == ucp_Z) as BOOL) == notmatch {
                                    break 'endloop99; /* Break the loop */
                                }
                            }
                        }
                        (*F).eptr = (*F).eptr.add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }
                /* ENDLOOP99: (C 4511) */
            }

            PT_WORD => {
                i = (*F).fields.type_repeat.min;
                while i < (*F).fields.type_repeat.max {
                    let chartype: u32;
                    let category: u32;
                    let mut len: c_int = 1;
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
                        break;
                    }
                    /* GETCHARLENTEST(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if utf != FALSE && fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
                    chartype = UCD_CHARTYPE(fc);
                    category = _pcre2_ucp_gentype_8[chartype as usize];
                    if ((category == ucp_L
                        || category == ucp_N
                        || chartype == ucp_Mn
                        || chartype == ucp_Pc) as BOOL)
                        == notmatch
                    {
                        break;
                    }
                    (*F).eptr = (*F).eptr.add(len as usize);
                    i = i.wrapping_add(1);
                }
            }

            PT_CLIST => {
                'got_max: {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: c_int = 1;
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
                            break;
                        }
                        /* GETCHARLENTEST(fc, Feptr, len) */
                        fc = *(*F).eptr as u32;
                        if utf != FALSE && fc >= 0xc0 {
                            len += utf8_extra(fc) as c_int;
                            fc = getutf8(fc, (*F).eptr);
                        }
                        {
                            let mut cp: *const u32 = _pcre2_ucd_caseless_sets_8
                                .as_ptr()
                                .add((*F).fields.type_repeat.propvalue as usize);
                            loop {
                                if fc < *cp {
                                    if notmatch != FALSE {
                                        break;
                                    } else {
                                        break 'got_max;
                                    }
                                }
                                let v = *cp;
                                cp = cp.add(1);
                                if fc == v {
                                    if notmatch != FALSE {
                                        break 'got_max;
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }

                        (*F).eptr = (*F).eptr.add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }
                /* GOT_MAX: (C 4567) */
            }

            PT_UCNC => {
                i = (*F).fields.type_repeat.min;
                while i < (*F).fields.type_repeat.max {
                    let mut len: c_int = 1;
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
                        break;
                    }
                    /* GETCHARLENTEST(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if utf != FALSE && fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
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
                        break;
                    }
                    /* GETCHARLENTEST(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if utf != FALSE && fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
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
                    let prop: &'static ucd_record;
                    let mut len: c_int = 1;
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
                        break;
                    }
                    /* GETCHARLENTEST(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if utf != FALSE && fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
                    prop = GET_UCD(fc);
                    ok = crate::internal::boolprop_set_bit(
                        UCD_BPROPS_PROP(prop) as usize,
                        (*F).fields.type_repeat.propvalue,
                    ) as BOOL;
                    if ok == notmatch {
                        break;
                    }
                    (*F).eptr = (*F).eptr.add(len as usize);
                    i = i.wrapping_add(1);
                }
            }

            _ => {
                return PCRE2_ERROR_INTERNAL;
            }
        }

        /* Feptr is now past the end of the maximum run */

        if reptype == REPTYPE_POS {
            lbl = LBL_TOP_OF_LOOP; /* No backtracking */
            continue 'sw;
        }

        /* After \C in UTF mode, Lstart_eptr might be in the middle of a
        Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
        go too far. */

        loop {
            if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
                break;
            }
            start_ecode = (*F).ecode;
            (*F).return_id = RM221;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }
    }
    /* Match extended Unicode grapheme clusters. We will get here only if the
    support is in the binary; otherwise a compile-time error occurs. */
    else if (*F).fields.type_repeat.ctype == OP_EXTUNI {
        i = (*F).fields.type_repeat.min;
        while i < (*F).fields.type_repeat.max {
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
                break;
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
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
            }
            i = i.wrapping_add(1);
        }

        /* Feptr is now past the end of the maximum run */

        if reptype == REPTYPE_POS {
            lbl = LBL_TOP_OF_LOOP; /* No backtracking */
            continue 'sw;
        }

        /* We use <= Lstart_eptr rather than == Lstart_eptr to detect the start
        of the run while backtracking because the use of \C in UTF mode can
        cause BACKCHAR to move back past Lstart_eptr. This is just palliative;
        the use of \C in UTF mode is fraught with danger. */

        loop {
            if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
                break; /* At start of char run */
            }
            start_ecode = (*F).ecode;
            (*F).return_id = RM219;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }
    } else if utf != FALSE {
        match (*F).fields.type_repeat.ctype {
            OP_ANY => {
                i = (*F).fields.type_repeat.min;
                while i < (*F).fields.type_repeat.max {
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
                        break;
                    }
                    /* IS_NEWLINE(Feptr) */
                    let is_nl: bool = if (*mb).nltype != NLTYPE_FIXED {
                        (*F).eptr < (*mb).end_subject
                            && crate::newline::_pcre2_is_newline_8(
                                (*F).eptr,
                                (*mb).nltype,
                                (*mb).end_subject,
                                &mut (*mb).nllen,
                                utf,
                            ) != FALSE
                    } else {
                        (*F).eptr <= (*mb).end_subject.wrapping_sub((*mb).nllen as usize)
                            && *(*F).eptr as u32 == (*mb).nl[0] as u32
                            && ((*mb).nllen == 1
                                || *(*F).eptr.add(1) as u32 == (*mb).nl[1] as u32)
                    };
                    if is_nl {
                        break;
                    }
                    if (*mb).partial != 0 /* Take care with CRLF partial */
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
                    while (*F).eptr < (*mb).end_subject && (*(*F).eptr as u32 & 0xc0) == 0x80
                    {
                        (*F).eptr = (*F).eptr.add(1);
                    }
                    i = i.wrapping_add(1);
                }
            }

            OP_ALLANY => {
                if (*F).fields.type_repeat.max < u32::MAX {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
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
                            break;
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
                } else {
                    (*F).eptr = (*mb).end_subject; /* Unlimited UTF-8 repeat */
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
                }
            }

            /* The "byte" (i.e. "code unit") case is the same as non-UTF */
            OP_ANYBYTE => {
                fc = (*F)
                    .fields
                    .type_repeat
                    .max
                    .wrapping_sub((*F).fields.type_repeat.min);
                if fc > ((*mb).end_subject.offset_from((*F).eptr) as u32) {
                    (*F).eptr = (*mb).end_subject;
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
                } else {
                    (*F).eptr = (*F).eptr.add(fc as usize);
                }
            }

            OP_ANYNL => {
                i = (*F).fields.type_repeat.min;
                while i < (*F).fields.type_repeat.max {
                    let mut len: c_int = 1;
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
                        break;
                    }
                    /* GETCHARLEN(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
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
                        break;
                    }
                    /* GETCHARLEN(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
                    match fc {
                        /* HSPACE_CASES: */
                        0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002
                        | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                        | 0x200a | 0x202f | 0x205f | 0x3000 => {
                            gotspace = TRUE;
                        }
                        _ => {
                            gotspace = FALSE;
                        }
                    }
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
                        break;
                    }
                    /* GETCHARLEN(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
                    match fc {
                        /* VSPACE_CASES: */
                        0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => {
                            gotspace = TRUE;
                        }
                        _ => {
                            gotspace = FALSE;
                        }
                    }
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
                        break;
                    }
                    /* GETCHARLEN(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
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
                        break;
                    }
                    /* GETCHARLEN(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
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
                        break;
                    }
                    /* GETCHARLEN(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
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
                        break;
                    }
                    /* GETCHARLEN(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
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
                        break;
                    }
                    /* GETCHARLEN(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
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
                        break;
                    }
                    /* GETCHARLEN(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
                    if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                        break;
                    }
                    (*F).eptr = (*F).eptr.add(len as usize);
                    i = i.wrapping_add(1);
                }
            }

            _ => {
                return PCRE2_ERROR_INTERNAL;
            }
        }

        if reptype == REPTYPE_POS {
            lbl = LBL_TOP_OF_LOOP; /* No backtracking */
            continue 'sw;
        }

        /* After \C in UTF mode, Lstart_eptr might be in the middle of a
        Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't go
        too far. */

        loop {
            if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
                break;
            }
            start_ecode = (*F).ecode;
            (*F).return_id = RM220;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }
    }
    /* Not UTF mode */
    else {
        match (*F).fields.type_repeat.ctype {
            OP_ANY => {
                i = (*F).fields.type_repeat.min;
                while i < (*F).fields.type_repeat.max {
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
                        break;
                    }
                    /* IS_NEWLINE(Feptr) */
                    let is_nl: bool = if (*mb).nltype != NLTYPE_FIXED {
                        (*F).eptr < (*mb).end_subject
                            && crate::newline::_pcre2_is_newline_8(
                                (*F).eptr,
                                (*mb).nltype,
                                (*mb).end_subject,
                                &mut (*mb).nllen,
                                utf,
                            ) != FALSE
                    } else {
                        (*F).eptr <= (*mb).end_subject.wrapping_sub((*mb).nllen as usize)
                            && *(*F).eptr as u32 == (*mb).nl[0] as u32
                            && ((*mb).nllen == 1
                                || *(*F).eptr.add(1) as u32 == (*mb).nl[1] as u32)
                    };
                    if is_nl {
                        break;
                    }
                    if (*mb).partial != 0 /* Take care with CRLF partial */
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

            OP_ALLANY | OP_ANYBYTE => {
                fc = (*F)
                    .fields
                    .type_repeat
                    .max
                    .wrapping_sub((*F).fields.type_repeat.min);
                if fc > ((*mb).end_subject.offset_from((*F).eptr) as u32) {
                    (*F).eptr = (*mb).end_subject;
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
                } else {
                    (*F).eptr = (*F).eptr.add(fc as usize);
                }
            }

            OP_ANYNL => {
                i = (*F).fields.type_repeat.min;
                while i < (*F).fields.type_repeat.max {
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
                'endloop00: {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
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
                            break;
                        }
                        match *(*F).eptr as u32 {
                            /* HSPACE_BYTE_CASES: */
                            0x09 | 0x20 | 0xa0 => {
                                break 'endloop00;
                            }
                            _ => {
                                (*F).eptr = (*F).eptr.add(1);
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }
                /* ENDLOOP00: (C 5054) */
            }

            OP_HSPACE => {
                'endloop01: {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
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
                            break;
                        }
                        match *(*F).eptr as u32 {
                            /* HSPACE_BYTE_CASES: */
                            0x09 | 0x20 | 0xa0 => {
                                (*F).eptr = (*F).eptr.add(1);
                            }
                            _ => {
                                break 'endloop01;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }
                /* ENDLOOP01: (C 5075) */
            }

            OP_NOT_VSPACE => {
                'endloop02: {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
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
                            break;
                        }
                        match *(*F).eptr as u32 {
                            /* VSPACE_BYTE_CASES: */
                            0x0a | 0x0b | 0x0c | 0x0d | 0x85 => {
                                break 'endloop02;
                            }
                            _ => {
                                (*F).eptr = (*F).eptr.add(1);
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }
                /* ENDLOOP02: (C 5096) */
            }

            OP_VSPACE => {
                'endloop03: {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
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
                            break;
                        }
                        match *(*F).eptr as u32 {
                            /* VSPACE_BYTE_CASES: */
                            0x0a | 0x0b | 0x0c | 0x0d | 0x85 => {
                                (*F).eptr = (*F).eptr.add(1);
                            }
                            _ => {
                                break 'endloop03;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }
                /* ENDLOOP03: (C 5117) */
            }

            OP_NOT_DIGIT => {
                i = (*F).fields.type_repeat.min;
                while i < (*F).fields.type_repeat.max {
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
                        break;
                    }
                    if MAX_255(*(*F).eptr as u32)
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
                        break;
                    }
                    if !MAX_255(*(*F).eptr as u32)
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
                        break;
                    }
                    if MAX_255(*(*F).eptr as u32)
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
                        break;
                    }
                    if !MAX_255(*(*F).eptr as u32)
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
                        break;
                    }
                    if MAX_255(*(*F).eptr as u32)
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
                        break;
                    }
                    if !MAX_255(*(*F).eptr as u32)
                        || (*(*mb).ctypes.add(*(*F).eptr as usize) & ctype_word) == 0
                    {
                        break;
                    }
                    (*F).eptr = (*F).eptr.add(1);
                    i = i.wrapping_add(1);
                }
            }

            _ => {
                return PCRE2_ERROR_INTERNAL;
            }
        }

        if reptype == REPTYPE_POS {
            lbl = LBL_TOP_OF_LOOP; /* No backtracking */
            continue 'sw;
        }

        loop {
            if (*F).eptr == (*F).fields.type_repeat.start_eptr {
                break;
            }
            start_ecode = (*F).ecode;
            (*F).return_id = RM34;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }
    }

    lbl = LBL_TOP_OF_LOOP; /* C 5224: break -- End of repeat character type processing */
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM221) at C 4641: backtracking for the
maximizing property-test repeat. */

if lbl == LBL_RM_BASE + RM221 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).eptr = (*F).eptr.wrapping_sub(1);
        if utf != FALSE {
            /* BACKCHAR(Feptr) */
            while (*(*F).eptr as u32 & 0xc0) == 0x80 {
                (*F).eptr = (*F).eptr.wrapping_sub(1);
            }
        }
        /* Top of the for(;;) loop */
        if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
            break;
        }
        start_ecode = (*F).ecode;
        (*F).return_id = RM221;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    lbl = LBL_TOP_OF_LOOP; /* C 5224: break */
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM219) at C 4684: backtracking over an
extended grapheme cluster. */

if lbl == LBL_RM_BASE + RM219 as u32 {
    loop {
        let mut lgb: u32;
        let mut rgb: u32;
        let mut fptr: PCRE2_SPTR;

        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }

        /* Backtracking over an extended grapheme cluster involves inspecting
        the previous two characters (if present) to see if a break is
        permitted between them. */

        (*F).eptr = (*F).eptr.wrapping_sub(1);
        if utf == FALSE {
            fc = *(*F).eptr as u32;
        } else {
            /* BACKCHAR(Feptr) */
            while (*(*F).eptr as u32 & 0xc0) == 0x80 {
                (*F).eptr = (*F).eptr.wrapping_sub(1);
            }
            /* GETCHAR(fc, Feptr) */
            fc = *(*F).eptr as u32;
            if fc >= 0xc0 {
                fc = getutf8(fc, (*F).eptr);
            }
        }
        rgb = UCD_GRAPHBREAK(fc);

        loop {
            if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
                break; /* At start of char run */
            }
            fptr = (*F).eptr.wrapping_sub(1);
            if utf == FALSE {
                fc = *fptr as u32;
            } else {
                /* BACKCHAR(fptr) */
                while (*fptr as u32 & 0xc0) == 0x80 {
                    fptr = fptr.wrapping_sub(1);
                }
                /* GETCHAR(fc, fptr) */
                fc = *fptr as u32;
                if fc >= 0xc0 {
                    fc = getutf8(fc, fptr);
                }
            }
            lgb = UCD_GRAPHBREAK(fc);
            if (_pcre2_ucp_gbtable_8[lgb as usize] & (1u32 << rgb)) == 0 {
                break;
            }
            (*F).eptr = fptr;
            rgb = lgb;
        }

        /* Top of the outer for(;;) loop */
        if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
            break; /* At start of char run */
        }
        start_ecode = (*F).ecode;
        (*F).return_id = RM219;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    lbl = LBL_TOP_OF_LOOP; /* C 5224: break */
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM220) at C 4960: backtracking for the
maximizing character-type repeat in UTF mode. */

if lbl == LBL_RM_BASE + RM220 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).eptr = (*F).eptr.wrapping_sub(1);
        /* BACKCHAR(Feptr) */
        while (*(*F).eptr as u32 & 0xc0) == 0x80 {
            (*F).eptr = (*F).eptr.wrapping_sub(1);
        }
        if (*F).fields.type_repeat.ctype == OP_ANYNL
            && (*F).eptr > (*F).fields.type_repeat.start_eptr
            && *(*F).eptr as u32 == CHAR_NL
            && *(*F).eptr.wrapping_sub(1) as u32 == CHAR_CR
        {
            (*F).eptr = (*F).eptr.wrapping_sub(1);
        }
        /* Top of the for(;;) loop */
        if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
            break;
        }
        start_ecode = (*F).ecode;
        (*F).return_id = RM220;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    lbl = LBL_TOP_OF_LOOP; /* C 5224: break */
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM34) at C 5216: backtracking for the
maximizing character-type repeat, not UTF mode. */

if lbl == LBL_RM_BASE + RM34 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).eptr = (*F).eptr.wrapping_sub(1);
        if (*F).fields.type_repeat.ctype == OP_ANYNL
            && (*F).eptr > (*F).fields.type_repeat.start_eptr
            && *(*F).eptr as u32 == CHAR_LF
            && *(*F).eptr.wrapping_sub(1) as u32 == CHAR_CR
        {
            (*F).eptr = (*F).eptr.wrapping_sub(1);
        }
        /* Top of the for(;;) loop */
        if (*F).eptr == (*F).fields.type_repeat.start_eptr {
            break;
        }
        start_ecode = (*F).ecode;
        (*F).return_id = RM34;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    lbl = LBL_TOP_OF_LOOP; /* C 5224: break */
    continue 'sw;
}
}
