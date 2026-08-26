{
    /* ===================================================================== */
    /* Match a negated single one-byte character repeatedly. This is almost a
    repeat of the code for a repeated single character, but I haven't found a
    nice way of commoning these up that doesn't require a test of the
    positive/negative option for each character match. Maybe that wouldn't add
    very much to the time taken, but character matching *is* what this is all
    about... */

    /* The C macros that are in force in this region:

         Lstart_eptr  ->  (*F).fields.charnot_repeat.start_eptr
         Lmin         ->  (*F).fields.charnot_repeat.min
         Lmax         ->  (*F).fields.charnot_repeat.max
         Lc           ->  (*F).fields.charnot_repeat.c
         Loc          ->  (*F).fields.charnot_repeat.oc                    */

    match state {
        OP_NOTEXACT | OP_NOTEXACTI => {
            let v: u32 = GET2!((*F).ecode, 1);
            (*F).fields.charnot_repeat.max = v; /* Lmin = Lmax = GET2(Fecode, 1) */
            (*F).fields.charnot_repeat.min = v;
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            state = ST_REPEATNOTCHAR;
            continue 'sm;
        }

        OP_NOTUPTO | OP_NOTUPTOI => {
            (*F).fields.charnot_repeat.min = 0;
            (*F).fields.charnot_repeat.max = GET2!((*F).ecode, 1);
            reptype = REPTYPE_MAX;
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            state = ST_REPEATNOTCHAR;
            continue 'sm;
        }

        OP_NOTMINUPTO | OP_NOTMINUPTOI => {
            (*F).fields.charnot_repeat.min = 0;
            (*F).fields.charnot_repeat.max = GET2!((*F).ecode, 1);
            reptype = REPTYPE_MIN;
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            state = ST_REPEATNOTCHAR;
            continue 'sm;
        }

        OP_NOTPOSSTAR | OP_NOTPOSSTARI => {
            reptype = REPTYPE_POS;
            (*F).fields.charnot_repeat.min = 0;
            (*F).fields.charnot_repeat.max = u32::MAX;
            (*F).ecode = (*F).ecode.add(1);
            state = ST_REPEATNOTCHAR;
            continue 'sm;
        }

        OP_NOTPOSPLUS | OP_NOTPOSPLUSI => {
            reptype = REPTYPE_POS;
            (*F).fields.charnot_repeat.min = 1;
            (*F).fields.charnot_repeat.max = u32::MAX;
            (*F).ecode = (*F).ecode.add(1);
            state = ST_REPEATNOTCHAR;
            continue 'sm;
        }

        OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
            reptype = REPTYPE_POS;
            (*F).fields.charnot_repeat.min = 0;
            (*F).fields.charnot_repeat.max = 1;
            (*F).ecode = (*F).ecode.add(1);
            state = ST_REPEATNOTCHAR;
            continue 'sm;
        }

        OP_NOTPOSUPTO | OP_NOTPOSUPTOI => {
            reptype = REPTYPE_POS;
            (*F).fields.charnot_repeat.min = 0;
            (*F).fields.charnot_repeat.max = GET2!((*F).ecode, 1);
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            state = ST_REPEATNOTCHAR;
            continue 'sm;
        }

        OP_NOTSTAR | OP_NOTSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI | OP_NOTPLUS | OP_NOTPLUSI
        | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_NOTQUERY | OP_NOTQUERYI | OP_NOTMINQUERY
        | OP_NOTMINQUERYI => {
            let t: u32 = *(*F).ecode as u32; /* fc = *Fecode++ - ... */
            (*F).ecode = (*F).ecode.add(1);
            fc = t - (if (*F).op as u32 >= OP_NOTSTARI {
                OP_NOTSTARI
            } else {
                OP_NOTSTAR
            });
            (*F).fields.charnot_repeat.min = *rep_min.as_ptr().add(fc as usize);
            (*F).fields.charnot_repeat.max = *rep_max.as_ptr().add(fc as usize);
            reptype = *rep_typ.as_ptr().add(fc as usize);

            /* Common code for all repeated single-character non-matches. */

            state = ST_REPEATNOTCHAR;
            continue 'sm;
        }

        ST_REPEATNOTCHAR => {
            GETCHARINCTEST!((*F).fields.charnot_repeat.c, (*F).ecode, utf);

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
                if (utf != 0 || ucp != 0) && (*F).fields.charnot_repeat.c > 127 {
                    (*F).fields.charnot_repeat.oc = UCD_OTHERCASE((*F).fields.charnot_repeat.c);
                } else {
                    /* Other case from table */
                    (*F).fields.charnot_repeat.oc = TABLE_GET!(
                        (*F).fields.charnot_repeat.c,
                        (*mb).fcc,
                        (*F).fields.charnot_repeat.c
                    ) as u32;
                }

                if utf != 0 {
                    let mut d: u32;
                    i = 1;
                    while i <= (*F).fields.charnot_repeat.min {
                        if (*F).eptr >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        GETCHARINC!(d, (*F).eptr);
                        if (*F).fields.charnot_repeat.c == d || (*F).fields.charnot_repeat.oc == d {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        i += 1;
                    }
                }
                /* Not UTF mode */
                else {
                    i = 1;
                    while i <= (*F).fields.charnot_repeat.min {
                        if (*F).eptr >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        if (*F).fields.charnot_repeat.c == *(*F).eptr as u32
                            || (*F).fields.charnot_repeat.oc == *(*F).eptr as u32
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        i += 1;
                    }
                }

                if (*F).fields.charnot_repeat.min == (*F).fields.charnot_repeat.max {
                    state = ST_TOP;
                    continue 'sm;
                } /* Finished for exact count */

                if reptype == REPTYPE_MIN {
                    if utf != 0 {
                        RMATCH!((*F).ecode, RM204);
                    }
                    /* Not UTF mode */
                    else {
                        RMATCH!((*F).ecode, RM29);
                    }
                }
                /* Maximize case */
                else {
                    (*F).fields.charnot_repeat.start_eptr = (*F).eptr;

                    if utf != 0 {
                        let mut d: u32;
                        i = (*F).fields.charnot_repeat.min;
                        while i < (*F).fields.charnot_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLEN!(d, (*F).eptr, len);
                            if (*F).fields.charnot_repeat.c == d
                                || (*F).fields.charnot_repeat.oc == d
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i += 1;
                        }

                        /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                        Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
                        go too far. */

                        if reptype != REPTYPE_POS {
                            state = ST_C3_1;
                            continue 'sm;
                        }
                    }
                    /* Not UTF mode */
                    else {
                        i = (*F).fields.charnot_repeat.min;
                        while i < (*F).fields.charnot_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            if (*F).fields.charnot_repeat.c == *(*F).eptr as u32
                                || (*F).fields.charnot_repeat.oc == *(*F).eptr as u32
                            {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(1);
                            i += 1;
                        }
                        if reptype != REPTYPE_POS {
                            state = ST_C3_2;
                            continue 'sm;
                        }
                    }
                }
            }
            /* Caseful comparisons */
            else {
                if utf != 0 {
                    let mut d: u32;
                    i = 1;
                    while i <= (*F).fields.charnot_repeat.min {
                        if (*F).eptr >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        GETCHARINC!(d, (*F).eptr);
                        if (*F).fields.charnot_repeat.c == d {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        i += 1;
                    }
                }
                /* Not UTF mode */
                else {
                    i = 1;
                    while i <= (*F).fields.charnot_repeat.min {
                        if (*F).eptr >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        let cu: u32 = *(*F).eptr as u32; /* Lc == *Feptr++ */
                        (*F).eptr = (*F).eptr.add(1);
                        if (*F).fields.charnot_repeat.c == cu {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        i += 1;
                    }
                }

                if (*F).fields.charnot_repeat.min == (*F).fields.charnot_repeat.max {
                    state = ST_TOP;
                    continue 'sm;
                }

                if reptype == REPTYPE_MIN {
                    if utf != 0 {
                        RMATCH!((*F).ecode, RM206);
                    }
                    /* Not UTF mode */
                    else {
                        RMATCH!((*F).ecode, RM31);
                    }
                }
                /* Maximize case */
                else {
                    (*F).fields.charnot_repeat.start_eptr = (*F).eptr;

                    if utf != 0 {
                        let mut d: u32;
                        i = (*F).fields.charnot_repeat.min;
                        while i < (*F).fields.charnot_repeat.max {
                            let mut len: c_int = 1;
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLEN!(d, (*F).eptr, len);
                            if (*F).fields.charnot_repeat.c == d {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(len as usize);
                            i += 1;
                        }

                        /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                        Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
                        go too far. */

                        if reptype != REPTYPE_POS {
                            state = ST_C3_3;
                            continue 'sm;
                        }
                    }
                    /* Not UTF mode */
                    else {
                        i = (*F).fields.charnot_repeat.min;
                        while i < (*F).fields.charnot_repeat.max {
                            if (*F).eptr >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            if (*F).fields.charnot_repeat.c == *(*F).eptr as u32 {
                                break;
                            }
                            (*F).eptr = (*F).eptr.add(1);
                            i += 1;
                        }
                        if reptype != REPTYPE_POS {
                            state = ST_C3_4;
                            continue 'sm;
                        }
                    }
                }
            }

            state = ST_TOP; /* break */
            continue 'sm;
        }

        /* Caseless, minimizing, UTF mode: the top of the C for(;;) loop is the
        RMATCH itself, so resuming here and re-issuing it repeats the loop. */

        ST_L_RM204 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            {
                let old: u32 = (*F).fields.charnot_repeat.min; /* Lmin++ >= Lmax */
                (*F).fields.charnot_repeat.min = old.wrapping_add(1);
                if old >= (*F).fields.charnot_repeat.max {
                    RRETURN!(MATCH_NOMATCH);
                }
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            let mut d: u32;
            GETCHARINC!(d, (*F).eptr);
            if (*F).fields.charnot_repeat.c == d || (*F).fields.charnot_repeat.oc == d {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM204); /* Repeat the for(;;) loop */
        }

        /* Caseless, minimizing, not UTF mode */

        ST_L_RM29 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            {
                let old: u32 = (*F).fields.charnot_repeat.min; /* Lmin++ >= Lmax */
                (*F).fields.charnot_repeat.min = old.wrapping_add(1);
                if old >= (*F).fields.charnot_repeat.max {
                    RRETURN!(MATCH_NOMATCH);
                }
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).fields.charnot_repeat.c == *(*F).eptr as u32
                || (*F).fields.charnot_repeat.oc == *(*F).eptr as u32
            {
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).eptr = (*F).eptr.add(1);
            RMATCH!((*F).ecode, RM29); /* Repeat the for(;;) loop */
        }

        /* Caseless, maximizing, UTF mode: backtracking loop */

        ST_C3_1 => {
            if (*F).eptr <= (*F).fields.charnot_repeat.start_eptr {
                state = ST_TOP; /* break out of the for(;;), then out of the switch */
                continue 'sm;
            }
            RMATCH!((*F).ecode, RM205);
        }

        ST_L_RM205 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).eptr = (*F).eptr.sub(1);
            BACKCHAR!((*F).eptr);
            state = ST_C3_1;
            continue 'sm;
        }

        /* Caseless, maximizing, not UTF mode: backtracking loop */

        ST_C3_2 => {
            if (*F).eptr == (*F).fields.charnot_repeat.start_eptr {
                state = ST_TOP;
                continue 'sm;
            }
            RMATCH!((*F).ecode, RM30);
        }

        ST_L_RM30 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).eptr = (*F).eptr.sub(1);
            state = ST_C3_2;
            continue 'sm;
        }

        /* Caseful, minimizing, UTF mode */

        ST_L_RM206 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            {
                let old: u32 = (*F).fields.charnot_repeat.min; /* Lmin++ >= Lmax */
                (*F).fields.charnot_repeat.min = old.wrapping_add(1);
                if old >= (*F).fields.charnot_repeat.max {
                    RRETURN!(MATCH_NOMATCH);
                }
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            let mut d: u32;
            GETCHARINC!(d, (*F).eptr);
            if (*F).fields.charnot_repeat.c == d {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM206); /* Repeat the for(;;) loop */
        }

        /* Caseful, minimizing, not UTF mode */

        ST_L_RM31 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            {
                let old: u32 = (*F).fields.charnot_repeat.min; /* Lmin++ >= Lmax */
                (*F).fields.charnot_repeat.min = old.wrapping_add(1);
                if old >= (*F).fields.charnot_repeat.max {
                    RRETURN!(MATCH_NOMATCH);
                }
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            let cu: u32 = *(*F).eptr as u32; /* Lc == *Feptr++ */
            (*F).eptr = (*F).eptr.add(1);
            if (*F).fields.charnot_repeat.c == cu {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM31); /* Repeat the for(;;) loop */
        }

        /* Caseful, maximizing, UTF mode: backtracking loop */

        ST_C3_3 => {
            if (*F).eptr <= (*F).fields.charnot_repeat.start_eptr {
                state = ST_TOP;
                continue 'sm;
            }
            RMATCH!((*F).ecode, RM207);
        }

        ST_L_RM207 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).eptr = (*F).eptr.sub(1);
            BACKCHAR!((*F).eptr);
            state = ST_C3_3;
            continue 'sm;
        }

        /* Caseful, maximizing, not UTF mode: backtracking loop */

        ST_C3_4 => {
            if (*F).eptr == (*F).fields.charnot_repeat.start_eptr {
                state = ST_TOP;
                continue 'sm;
            }
            RMATCH!((*F).ecode, RM32);
        }

        ST_L_RM32 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).eptr = (*F).eptr.sub(1);
            state = ST_C3_4;
            continue 'sm;
        }

        _ => {}
    }
}
