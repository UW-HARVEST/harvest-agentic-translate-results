{
/* ===================================================================== */
/* Match a negated single one-byte character repeatedly. This is almost a
repeat of the code for a repeated single character, but I haven't found a
nice way of commoning these up that doesn't require a test of the
positive/negative option for each character match. Maybe that wouldn't add
very much to the time taken, but character matching *is* what this is all
about... */

/* #define Lstart_eptr  F->fields.charnot_repeat.start_eptr
   #define Lmin         F->fields.charnot_repeat.min
   #define Lmax         F->fields.charnot_repeat.max
   #define Lc           F->fields.charnot_repeat.c
   #define Loc          F->fields.charnot_repeat.oc                      */

if lbl == LBL_SWITCH {
    match (*F).op as u32 {
        /* case OP_NOTEXACT: case OP_NOTEXACTI: (C 1660) */
        OP_NOTEXACT | OP_NOTEXACTI => {
            (*F).fields.charnot_repeat.max = GET2((*F).ecode, 1);
            (*F).fields.charnot_repeat.min = (*F).fields.charnot_repeat.max;
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            lbl = LBL_REPEATNOTCHAR;
            continue 'sw;
        }

        /* case OP_NOTUPTO: case OP_NOTUPTOI: (C 1666) */
        OP_NOTUPTO | OP_NOTUPTOI => {
            (*F).fields.charnot_repeat.min = 0;
            (*F).fields.charnot_repeat.max = GET2((*F).ecode, 1);
            reptype = REPTYPE_MAX;
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            lbl = LBL_REPEATNOTCHAR;
            continue 'sw;
        }

        /* case OP_NOTMINUPTO: case OP_NOTMINUPTOI: (C 1674) */
        OP_NOTMINUPTO | OP_NOTMINUPTOI => {
            (*F).fields.charnot_repeat.min = 0;
            (*F).fields.charnot_repeat.max = GET2((*F).ecode, 1);
            reptype = REPTYPE_MIN;
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            lbl = LBL_REPEATNOTCHAR;
            continue 'sw;
        }

        /* case OP_NOTPOSSTAR: case OP_NOTPOSSTARI: (C 1682) */
        OP_NOTPOSSTAR | OP_NOTPOSSTARI => {
            reptype = REPTYPE_POS;
            (*F).fields.charnot_repeat.min = 0;
            (*F).fields.charnot_repeat.max = u32::MAX;
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_REPEATNOTCHAR;
            continue 'sw;
        }

        /* case OP_NOTPOSPLUS: case OP_NOTPOSPLUSI: (C 1690) */
        OP_NOTPOSPLUS | OP_NOTPOSPLUSI => {
            reptype = REPTYPE_POS;
            (*F).fields.charnot_repeat.min = 1;
            (*F).fields.charnot_repeat.max = u32::MAX;
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_REPEATNOTCHAR;
            continue 'sw;
        }

        /* case OP_NOTPOSQUERY: case OP_NOTPOSQUERYI: (C 1698) */
        OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
            reptype = REPTYPE_POS;
            (*F).fields.charnot_repeat.min = 0;
            (*F).fields.charnot_repeat.max = 1;
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_REPEATNOTCHAR;
            continue 'sw;
        }

        /* case OP_NOTPOSUPTO: case OP_NOTPOSUPTOI: (C 1706) */
        OP_NOTPOSUPTO | OP_NOTPOSUPTOI => {
            reptype = REPTYPE_POS;
            (*F).fields.charnot_repeat.min = 0;
            (*F).fields.charnot_repeat.max = GET2((*F).ecode, 1);
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            lbl = LBL_REPEATNOTCHAR;
            continue 'sw;
        }

        /* case OP_NOTSTAR: ... case OP_NOTMINQUERYI: (C 1714-1725) */
        OP_NOTSTAR | OP_NOTSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI | OP_NOTPLUS
        | OP_NOTPLUSI | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_NOTQUERY | OP_NOTQUERYI
        | OP_NOTMINQUERY | OP_NOTMINQUERYI => {
            let opcu = *(*F).ecode as u32;
            (*F).ecode = (*F).ecode.add(1);
            fc = opcu.wrapping_sub(if (*F).op as u32 >= OP_NOTSTARI {
                OP_NOTSTARI
            } else {
                OP_NOTSTAR
            });
            (*F).fields.charnot_repeat.min = rep_min[fc as usize];
            (*F).fields.charnot_repeat.max = rep_max[fc as usize];
            reptype = rep_typ[fc as usize];

            lbl = LBL_REPEATNOTCHAR;
            continue 'sw;
        }

        _ => {}
    }
}

/* --------------------------------------------------------------------- */
/* REPEATNOTCHAR: (C 1733)

Common code for all repeated single-character non-matches. */

if lbl == LBL_REPEATNOTCHAR {
    /* GETCHARINCTEST(Lc, Fecode) */
    (*F).fields.charnot_repeat.c = *(*F).ecode as u32;
    (*F).ecode = (*F).ecode.add(1);
    if utf != FALSE && (*F).fields.charnot_repeat.c >= 0xc0 {
        let r = getutf8inc((*F).fields.charnot_repeat.c, (*F).ecode);
        (*F).fields.charnot_repeat.c = r.0;
        (*F).ecode = r.1;
    }

    /* The code is duplicated for the caseless and caseful cases, for speed,
    since matching characters is likely to be quite common. First, ensure the
    minimum number of matches are present. If Lmin = Lmax, we are done.
    Otherwise, if minimizing, keep trying the rest of the expression and
    advancing one matching character if failing, up to the maximum.
    Alternatively, if maximizing, find the maximum number of characters and
    work backwards. */

    if (*F).op as u32 >= OP_NOTSTARI
    /* Caseless */
    {
        if (utf != FALSE || ucp != FALSE) && (*F).fields.charnot_repeat.c > 127 {
            (*F).fields.charnot_repeat.oc = UCD_OTHERCASE((*F).fields.charnot_repeat.c);
        } else {
            /* Other case from table */
            (*F).fields.charnot_repeat.oc = TABLE_GET(
                (*F).fields.charnot_repeat.c,
                (*mb).fcc,
                (*F).fields.charnot_repeat.c,
            );
        }

        if utf != FALSE {
            i = 1;
            while i <= (*F).fields.charnot_repeat.min {
                let d: u32;
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
                /* GETCHARINC(d, Feptr) */
                {
                    let mut dd = *(*F).eptr as u32;
                    (*F).eptr = (*F).eptr.add(1);
                    if dd >= 0xc0 {
                        let r = getutf8inc(dd, (*F).eptr);
                        dd = r.0;
                        (*F).eptr = r.1;
                    }
                    d = dd;
                }
                if (*F).fields.charnot_repeat.c == d || (*F).fields.charnot_repeat.oc == d {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                i = i.wrapping_add(1);
            }
        }
        /* Not UTF mode */
        else {
            i = 1;
            while i <= (*F).fields.charnot_repeat.min {
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
                if (*F).fields.charnot_repeat.c == *(*F).eptr as u32
                    || (*F).fields.charnot_repeat.oc == *(*F).eptr as u32
                {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                (*F).eptr = (*F).eptr.add(1);
                i = i.wrapping_add(1);
            }
        }

        if (*F).fields.charnot_repeat.min == (*F).fields.charnot_repeat.max {
            /* Finished for exact count */
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        if reptype == REPTYPE_MIN {
            if utf != FALSE {
                /* for (;;) { RMATCH(Fecode, RM204); ... } */
                start_ecode = (*F).ecode;
                (*F).return_id = RM204;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            }
            /* Not UTF mode */
            else {
                /* for (;;) { RMATCH(Fecode, RM29); ... } */
                start_ecode = (*F).ecode;
                (*F).return_id = RM29;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            }
        }
        /* Maximize case */
        else {
            (*F).fields.charnot_repeat.start_eptr = (*F).eptr;

            if utf != FALSE {
                i = (*F).fields.charnot_repeat.min;
                while i < (*F).fields.charnot_repeat.max {
                    let mut len: c_int = 1;
                    let d: u32;
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
                    /* GETCHARLEN(d, Feptr, len) */
                    {
                        let mut dd = *(*F).eptr as u32;
                        if dd >= 0xc0 {
                            len += utf8_extra(dd) as c_int;
                            dd = getutf8(dd, (*F).eptr);
                        }
                        d = dd;
                    }
                    if (*F).fields.charnot_repeat.c == d || (*F).fields.charnot_repeat.oc == d
                    {
                        break;
                    }
                    (*F).eptr = (*F).eptr.add(len as usize);
                    i = i.wrapping_add(1);
                }

                /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
                go too far. */

                if reptype != REPTYPE_POS {
                    loop {
                        if (*F).eptr <= (*F).fields.charnot_repeat.start_eptr {
                            break;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM205;
                        lbl = LBL_MATCH_RECURSE;
                        continue 'sw;
                    }
                }
            }
            /* Not UTF mode */
            else {
                i = (*F).fields.charnot_repeat.min;
                while i < (*F).fields.charnot_repeat.max {
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
                    if (*F).fields.charnot_repeat.c == *(*F).eptr as u32
                        || (*F).fields.charnot_repeat.oc == *(*F).eptr as u32
                    {
                        break;
                    }
                    (*F).eptr = (*F).eptr.add(1);
                    i = i.wrapping_add(1);
                }
                if reptype != REPTYPE_POS {
                    loop {
                        if (*F).eptr == (*F).fields.charnot_repeat.start_eptr {
                            break;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM30;
                        lbl = LBL_MATCH_RECURSE;
                        continue 'sw;
                    }
                }
            }
        }
    }
    /* Caseful comparisons */
    else {
        if utf != FALSE {
            i = 1;
            while i <= (*F).fields.charnot_repeat.min {
                let d: u32;
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
                /* GETCHARINC(d, Feptr) */
                {
                    let mut dd = *(*F).eptr as u32;
                    (*F).eptr = (*F).eptr.add(1);
                    if dd >= 0xc0 {
                        let r = getutf8inc(dd, (*F).eptr);
                        dd = r.0;
                        (*F).eptr = r.1;
                    }
                    d = dd;
                }
                if (*F).fields.charnot_repeat.c == d {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                i = i.wrapping_add(1);
            }
        }
        /* Not UTF mode */
        else {
            i = 1;
            while i <= (*F).fields.charnot_repeat.min {
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
                let sc = *(*F).eptr as u32;
                (*F).eptr = (*F).eptr.add(1);
                if (*F).fields.charnot_repeat.c == sc {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                i = i.wrapping_add(1);
            }
        }

        if (*F).fields.charnot_repeat.min == (*F).fields.charnot_repeat.max {
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        if reptype == REPTYPE_MIN {
            if utf != FALSE {
                /* for (;;) { RMATCH(Fecode, RM206); ... } */
                start_ecode = (*F).ecode;
                (*F).return_id = RM206;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            }
            /* Not UTF mode */
            else {
                /* for (;;) { RMATCH(Fecode, RM31); ... } */
                start_ecode = (*F).ecode;
                (*F).return_id = RM31;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            }
        }
        /* Maximize case */
        else {
            (*F).fields.charnot_repeat.start_eptr = (*F).eptr;

            if utf != FALSE {
                i = (*F).fields.charnot_repeat.min;
                while i < (*F).fields.charnot_repeat.max {
                    let mut len: c_int = 1;
                    let d: u32;
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
                    /* GETCHARLEN(d, Feptr, len) */
                    {
                        let mut dd = *(*F).eptr as u32;
                        if dd >= 0xc0 {
                            len += utf8_extra(dd) as c_int;
                            dd = getutf8(dd, (*F).eptr);
                        }
                        d = dd;
                    }
                    if (*F).fields.charnot_repeat.c == d {
                        break;
                    }
                    (*F).eptr = (*F).eptr.add(len as usize);
                    i = i.wrapping_add(1);
                }

                /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
                go too far. */

                if reptype != REPTYPE_POS {
                    loop {
                        if (*F).eptr <= (*F).fields.charnot_repeat.start_eptr {
                            break;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM207;
                        lbl = LBL_MATCH_RECURSE;
                        continue 'sw;
                    }
                }
            }
            /* Not UTF mode */
            else {
                i = (*F).fields.charnot_repeat.min;
                while i < (*F).fields.charnot_repeat.max {
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
                    if (*F).fields.charnot_repeat.c == *(*F).eptr as u32 {
                        break;
                    }
                    (*F).eptr = (*F).eptr.add(1);
                    i = i.wrapping_add(1);
                }
                if reptype != REPTYPE_POS {
                    loop {
                        if (*F).eptr == (*F).fields.charnot_repeat.start_eptr {
                            break;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM32;
                        lbl = LBL_MATCH_RECURSE;
                        continue 'sw;
                    }
                }
            }
        }
    }
    lbl = LBL_TOP_OF_LOOP; /* C 2028: break */
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM204) at C 1796: minimizing caseless
negated character repeat, UTF mode. */

if lbl == LBL_RM_BASE + RM204 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let oldmin = (*F).fields.charnot_repeat.min;
        (*F).fields.charnot_repeat.min = oldmin.wrapping_add(1);
        if oldmin >= (*F).fields.charnot_repeat.max {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
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
        /* GETCHARINC(d, Feptr) */
        let d: u32;
        {
            let mut dd = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if dd >= 0xc0 {
                let r = getutf8inc(dd, (*F).eptr);
                dd = r.0;
                (*F).eptr = r.1;
            }
            d = dd;
        }
        if (*F).fields.charnot_repeat.c == d || (*F).fields.charnot_repeat.oc == d {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        /* Loop back to RMATCH(Fecode, RM204) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM204;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM29) at C 1815: minimizing caseless
negated character repeat, non-UTF mode. */

if lbl == LBL_RM_BASE + RM29 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let oldmin = (*F).fields.charnot_repeat.min;
        (*F).fields.charnot_repeat.min = oldmin.wrapping_add(1);
        if oldmin >= (*F).fields.charnot_repeat.max {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
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
        if (*F).fields.charnot_repeat.c == *(*F).eptr as u32
            || (*F).fields.charnot_repeat.oc == *(*F).eptr as u32
        {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).eptr = (*F).eptr.add(1);
        /* Loop back to RMATCH(Fecode, RM29) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM29;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM205) at C 1860: backtracking for the
maximizing caseless negated character repeat, UTF mode. */

if lbl == LBL_RM_BASE + RM205 as u32 {
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
        /* Top of the for(;;) loop */
        if (*F).eptr <= (*F).fields.charnot_repeat.start_eptr {
            break;
        }
        start_ecode = (*F).ecode;
        (*F).return_id = RM205;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    lbl = LBL_TOP_OF_LOOP; /* C 2028: break */
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM30) at C 1884: backtracking for the
maximizing caseless negated character repeat, non-UTF mode. */

if lbl == LBL_RM_BASE + RM30 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).eptr = (*F).eptr.wrapping_sub(1);
        /* Top of the for(;;) loop */
        if (*F).eptr == (*F).fields.charnot_repeat.start_eptr {
            break;
        }
        start_ecode = (*F).ecode;
        (*F).return_id = RM30;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    lbl = LBL_TOP_OF_LOOP; /* C 2028: break */
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM206) at C 1936: minimizing caseful
negated character repeat, UTF mode. */

if lbl == LBL_RM_BASE + RM206 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let oldmin = (*F).fields.charnot_repeat.min;
        (*F).fields.charnot_repeat.min = oldmin.wrapping_add(1);
        if oldmin >= (*F).fields.charnot_repeat.max {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
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
        /* GETCHARINC(d, Feptr) */
        let d: u32;
        {
            let mut dd = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if dd >= 0xc0 {
                let r = getutf8inc(dd, (*F).eptr);
                dd = r.0;
                (*F).eptr = r.1;
            }
            d = dd;
        }
        if (*F).fields.charnot_repeat.c == d {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        /* Loop back to RMATCH(Fecode, RM206) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM206;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM31) at C 1954: minimizing caseful
negated character repeat, non-UTF mode. */

if lbl == LBL_RM_BASE + RM31 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let oldmin = (*F).fields.charnot_repeat.min;
        (*F).fields.charnot_repeat.min = oldmin.wrapping_add(1);
        if oldmin >= (*F).fields.charnot_repeat.max {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
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
        let sc = *(*F).eptr as u32;
        (*F).eptr = (*F).eptr.add(1);
        if (*F).fields.charnot_repeat.c == sc {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        /* Loop back to RMATCH(Fecode, RM31) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM31;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM207) at C 1998: backtracking for the
maximizing caseful negated character repeat, UTF mode. */

if lbl == LBL_RM_BASE + RM207 as u32 {
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
        /* Top of the for(;;) loop */
        if (*F).eptr <= (*F).fields.charnot_repeat.start_eptr {
            break;
        }
        start_ecode = (*F).ecode;
        (*F).return_id = RM207;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    lbl = LBL_TOP_OF_LOOP; /* C 2028: break */
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM32) at C 2021: backtracking for the
maximizing caseful negated character repeat, non-UTF mode. */

if lbl == LBL_RM_BASE + RM32 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).eptr = (*F).eptr.wrapping_sub(1);
        /* Top of the for(;;) loop */
        if (*F).eptr == (*F).fields.charnot_repeat.start_eptr {
            break;
        }
        start_ecode = (*F).ecode;
        (*F).return_id = RM32;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    lbl = LBL_TOP_OF_LOOP; /* C 2028: break */
    continue 'sw;
}
}
