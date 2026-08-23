{
    /* ===================================================================== */
    /* Match a single character repeatedly. */

    /* The frame fields used by this group (the C #defines) are:
         Llength     = F->byte1
         Loclength   = F->byte2
         Lstart_eptr = F->fields.char_repeat.start_eptr
         Lcharptr    = F->fields.char_repeat.charptr
         Lmin        = F->fields.char_repeat.min
         Lmax        = F->fields.char_repeat.max
         Lc          = F->fields.char_repeat.c
         Loc         = F->fields.char_repeat.oc.oc
         Loccu       = F->fields.char_repeat.oc.occu                        */

    match state {
        OP_EXACT | OP_EXACTI => {
            (*F).fields.char_repeat.max = GET2!((*F).ecode, 1);
            (*F).fields.char_repeat.min = (*F).fields.char_repeat.max;
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            state = ST_REPEATCHAR;
            continue 'sm;
        }

        OP_POSUPTO | OP_POSUPTOI => {
            reptype = REPTYPE_POS;
            (*F).fields.char_repeat.min = 0;
            (*F).fields.char_repeat.max = GET2!((*F).ecode, 1);
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            state = ST_REPEATCHAR;
            continue 'sm;
        }

        OP_UPTO | OP_UPTOI => {
            reptype = REPTYPE_MAX;
            (*F).fields.char_repeat.min = 0;
            (*F).fields.char_repeat.max = GET2!((*F).ecode, 1);
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            state = ST_REPEATCHAR;
            continue 'sm;
        }

        OP_MINUPTO | OP_MINUPTOI => {
            reptype = REPTYPE_MIN;
            (*F).fields.char_repeat.min = 0;
            (*F).fields.char_repeat.max = GET2!((*F).ecode, 1);
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            state = ST_REPEATCHAR;
            continue 'sm;
        }

        OP_POSSTAR | OP_POSSTARI => {
            reptype = REPTYPE_POS;
            (*F).fields.char_repeat.min = 0;
            (*F).fields.char_repeat.max = u32::MAX;
            (*F).ecode = (*F).ecode.add(1);
            state = ST_REPEATCHAR;
            continue 'sm;
        }

        OP_POSPLUS | OP_POSPLUSI => {
            reptype = REPTYPE_POS;
            (*F).fields.char_repeat.min = 1;
            (*F).fields.char_repeat.max = u32::MAX;
            (*F).ecode = (*F).ecode.add(1);
            state = ST_REPEATCHAR;
            continue 'sm;
        }

        OP_POSQUERY | OP_POSQUERYI => {
            reptype = REPTYPE_POS;
            (*F).fields.char_repeat.min = 0;
            (*F).fields.char_repeat.max = 1;
            (*F).ecode = (*F).ecode.add(1);
            state = ST_REPEATCHAR;
            continue 'sm;
        }

        OP_STAR | OP_STARI | OP_MINSTAR | OP_MINSTARI | OP_PLUS | OP_PLUSI | OP_MINPLUS
        | OP_MINPLUSI | OP_QUERY | OP_QUERYI | OP_MINQUERY | OP_MINQUERYI => {
            fc = ({
                let t = *(*F).ecode;
                (*F).ecode = (*F).ecode.add(1);
                t as u32
            })
            .wrapping_sub(if ((*F).op as u32) < OP_STARI {
                OP_STAR
            } else {
                OP_STARI
            });
            (*F).fields.char_repeat.min = *rep_min.as_ptr().add(fc as usize);
            (*F).fields.char_repeat.max = *rep_max.as_ptr().add(fc as usize);
            reptype = *rep_typ.as_ptr().add(fc as usize);

            /* Fall through into REPEATCHAR. */
            state = ST_REPEATCHAR;
            continue 'sm;
        }

        /* Common code for all repeated single-character matches. We first check
        for the minimum number of characters. If the minimum equals the maximum, we
        are done. Otherwise, if minimizing, check the rest of the pattern for a
        match; if there isn't one, advance up to the maximum, one character at a
        time.

        If maximizing, advance up to the maximum number of matching characters,
        until Feptr is past the end of the maximum run. If possessive, we are
        then done (no backing up). Otherwise, match at this position; anything
        other than no match is immediately returned. For nomatch, back up one
        character, unless we are matching \R and the last thing matched was
        \r\n, in which case, back up two code units until we reach the first
        optional character position.

        The various UTF/non-UTF and caseful/caseless cases are handled separately,
        for speed. */

        ST_REPEATCHAR => {
            if utf != 0 {
                length = 1;
                (*F).fields.char_repeat.charptr = (*F).ecode;
                GETCHARLEN!(fc, (*F).ecode, length);
                (*F).ecode = (*F).ecode.add(length);
                (*F).byte1 = length as u8;

                /* Handle multi-code-unit character matching, caseful and caseless. */

                if length > 1 {
                    let mut othercase: u32 = 0;

                    if (*F).op as u32 >= OP_STARI /* Caseless */
                        && {
                            othercase = UCD_OTHERCASE(fc);
                            othercase != fc
                        }
                    {
                        (*F).byte2 = _pcre2_ord2utf_8(
                            othercase,
                            (*F).fields.char_repeat.oc.occu.as_mut_ptr(),
                        ) as u8;
                    } else {
                        (*F).byte2 = 0;
                    }

                    i = 1;
                    while i <= (*F).fields.char_repeat.min {
                        if (*F).eptr <= (*mb).end_subject.wrapping_sub(length)
                            && memcmp(
                                (*F).eptr as *const c_void,
                                (*F).fields.char_repeat.charptr as *const c_void,
                                CU2BYTES!(length),
                            ) == 0
                        {
                            (*F).eptr = (*F).eptr.add(length);
                        } else if (*F).byte2 > 0
                            && (*F).eptr
                                <= (*mb).end_subject.wrapping_sub((*F).byte2 as usize)
                            && memcmp(
                                (*F).eptr as *const c_void,
                                (*F).fields.char_repeat.oc.occu.as_mut_ptr() as *const c_void,
                                CU2BYTES!((*F).byte2 as usize),
                            ) == 0
                        {
                            (*F).eptr = (*F).eptr.add((*F).byte2 as usize);
                        } else {
                            CHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        i = i.wrapping_add(1);
                    }

                    if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
                        state = ST_TOP;
                        continue 'sm;
                    }

                    if reptype == REPTYPE_MIN {
                        state = ST_C2_1;
                        continue 'sm;
                    } else
                    /* Maximize */
                    {
                        (*F).fields.char_repeat.start_eptr = (*F).eptr;
                        i = (*F).fields.char_repeat.min;
                        while i < (*F).fields.char_repeat.max {
                            if (*F).eptr
                                <= (*mb).end_subject.wrapping_sub((*F).byte1 as usize)
                                && memcmp(
                                    (*F).eptr as *const c_void,
                                    (*F).fields.char_repeat.charptr as *const c_void,
                                    CU2BYTES!((*F).byte1 as usize),
                                ) == 0
                            {
                                (*F).eptr = (*F).eptr.add((*F).byte1 as usize);
                            } else if (*F).byte2 > 0
                                && (*F).eptr
                                    <= (*mb).end_subject.wrapping_sub((*F).byte2 as usize)
                                && memcmp(
                                    (*F).eptr as *const c_void,
                                    (*F).fields.char_repeat.oc.occu.as_mut_ptr()
                                        as *const c_void,
                                    CU2BYTES!((*F).byte2 as usize),
                                ) == 0
                            {
                                (*F).eptr = (*F).eptr.add((*F).byte2 as usize);
                            } else {
                                CHECK_PARTIAL!();
                                break;
                            }
                            i = i.wrapping_add(1);
                        }

                        /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                        Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
                        go too far. */

                        if reptype != REPTYPE_POS {
                            state = ST_C2_2;
                            continue 'sm;
                        }
                    }

                    state = ST_TOP; /* End of repeated wide character handling */
                    continue 'sm;
                }

                /* Length of UTF character is 1. Put it into the preserved variable and
                fall through to the non-UTF code. */

                (*F).fields.char_repeat.c = fc;
            } else {
                /* When not in UTF mode, load a single-code-unit character. Then proceed as
                above, using Unicode casing if either UTF or UCP is set. */

                (*F).fields.char_repeat.c = {
                    let t = *(*F).ecode;
                    (*F).ecode = (*F).ecode.add(1);
                    t as u32
                };
            }

            /* Caseless comparison */

            if (*F).op as u32 >= OP_STARI {
                if ucp != 0 && utf == 0 && (*F).fields.char_repeat.c > 127 {
                    (*F).fields.char_repeat.oc.oc =
                        UCD_OTHERCASE((*F).fields.char_repeat.c);
                } else {
                    /* Lc will be < 128 in UTF-8 mode. */
                    (*F).fields.char_repeat.oc.oc =
                        *(*mb).fcc.offset((*F).fields.char_repeat.c as isize) as u32;
                }

                i = 1;
                while i <= (*F).fields.char_repeat.min {
                    let cc: u32; /* Faster than PCRE2_UCHAR */
                    if (*F).eptr >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        RRETURN!(MATCH_NOMATCH);
                    }
                    cc = *(*F).eptr as u32;
                    if (*F).fields.char_repeat.c != cc && (*F).fields.char_repeat.oc.oc != cc
                    {
                        RRETURN!(MATCH_NOMATCH);
                    }
                    (*F).eptr = (*F).eptr.add(1);
                    i = i.wrapping_add(1);
                }
                if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
                    state = ST_TOP;
                    continue 'sm;
                }

                if reptype == REPTYPE_MIN {
                    state = ST_C2_3;
                    continue 'sm;
                } else
                /* Maximize */
                {
                    (*F).fields.char_repeat.start_eptr = (*F).eptr;
                    i = (*F).fields.char_repeat.min;
                    while i < (*F).fields.char_repeat.max {
                        let cc: u32; /* Faster than PCRE2_UCHAR */
                        if (*F).eptr >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        cc = *(*F).eptr as u32;
                        if (*F).fields.char_repeat.c != cc
                            && (*F).fields.char_repeat.oc.oc != cc
                        {
                            break;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        i = i.wrapping_add(1);
                    }
                    if reptype != REPTYPE_POS {
                        state = ST_C2_4;
                        continue 'sm;
                    }
                }
            }
            /* Caseful comparisons (includes all multi-byte characters) */
            else {
                i = 1;
                while i <= (*F).fields.char_repeat.min {
                    if (*F).eptr >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        RRETURN!(MATCH_NOMATCH);
                    }
                    if (*F).fields.char_repeat.c != ({
                        let t = *(*F).eptr;
                        (*F).eptr = (*F).eptr.add(1);
                        t as u32
                    }) {
                        RRETURN!(MATCH_NOMATCH);
                    }
                    i = i.wrapping_add(1);
                }

                if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
                    state = ST_TOP;
                    continue 'sm;
                }

                if reptype == REPTYPE_MIN {
                    state = ST_C2_5;
                    continue 'sm;
                } else
                /* Maximize */
                {
                    (*F).fields.char_repeat.start_eptr = (*F).eptr;
                    i = (*F).fields.char_repeat.min;
                    while i < (*F).fields.char_repeat.max {
                        if (*F).eptr >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }

                        if (*F).fields.char_repeat.c != *(*F).eptr as u32 {
                            break;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        i = i.wrapping_add(1);
                    }

                    if reptype != REPTYPE_POS {
                        state = ST_C2_6;
                        continue 'sm;
                    }
                }
            }

            state = ST_TOP;
            continue 'sm;
        }

        /* Top of the minimizing for(;;) loop for a repeated wide UTF character. */

        ST_C2_1 => {
            RMATCH!((*F).ecode, RM202);
        }

        ST_L_RM202 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            if {
                let t = (*F).fields.char_repeat.min;
                (*F).fields.char_repeat.min = t.wrapping_add(1);
                t >= (*F).fields.char_repeat.max
            } {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr <= (*mb).end_subject.wrapping_sub((*F).byte1 as usize)
                && memcmp(
                    (*F).eptr as *const c_void,
                    (*F).fields.char_repeat.charptr as *const c_void,
                    CU2BYTES!((*F).byte1 as usize),
                ) == 0
            {
                (*F).eptr = (*F).eptr.add((*F).byte1 as usize);
            } else if (*F).byte2 > 0
                && (*F).eptr <= (*mb).end_subject.wrapping_sub((*F).byte2 as usize)
                && memcmp(
                    (*F).eptr as *const c_void,
                    (*F).fields.char_repeat.oc.occu.as_mut_ptr() as *const c_void,
                    CU2BYTES!((*F).byte2 as usize),
                ) == 0
            {
                (*F).eptr = (*F).eptr.add((*F).byte2 as usize);
            } else {
                CHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            state = ST_C2_1;
            continue 'sm;
        }

        /* Top of the maximizing backtracking for(;;) loop for a repeated wide UTF
        character. */

        ST_C2_2 => {
            if (*F).eptr <= (*F).fields.char_repeat.start_eptr {
                state = ST_TOP;
                continue 'sm;
            }
            RMATCH!((*F).ecode, RM203);
        }

        ST_L_RM203 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).eptr = (*F).eptr.sub(1);
            BACKCHAR!((*F).eptr);
            state = ST_C2_2;
            continue 'sm;
        }

        /* Top of the minimizing for(;;) loop, caseless single-code-unit. */

        ST_C2_3 => {
            RMATCH!((*F).ecode, RM25);
        }

        ST_L_RM25 => {
            let cc: u32; /* Faster than PCRE2_UCHAR */
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            if {
                let t = (*F).fields.char_repeat.min;
                (*F).fields.char_repeat.min = t.wrapping_add(1);
                t >= (*F).fields.char_repeat.max
            } {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            cc = *(*F).eptr as u32;
            if (*F).fields.char_repeat.c != cc && (*F).fields.char_repeat.oc.oc != cc {
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).eptr = (*F).eptr.add(1);
            state = ST_C2_3;
            continue 'sm;
        }

        /* Top of the maximizing backtracking for(;;) loop, caseless. */

        ST_C2_4 => {
            if (*F).eptr == (*F).fields.char_repeat.start_eptr {
                state = ST_TOP;
                continue 'sm;
            }
            RMATCH!((*F).ecode, RM26);
        }

        ST_L_RM26 => {
            (*F).eptr = (*F).eptr.sub(1);
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            state = ST_C2_4;
            continue 'sm;
        }

        /* Top of the minimizing for(;;) loop, caseful. */

        ST_C2_5 => {
            RMATCH!((*F).ecode, RM27);
        }

        ST_L_RM27 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            if {
                let t = (*F).fields.char_repeat.min;
                (*F).fields.char_repeat.min = t.wrapping_add(1);
                t >= (*F).fields.char_repeat.max
            } {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).fields.char_repeat.c != ({
                let t = *(*F).eptr;
                (*F).eptr = (*F).eptr.add(1);
                t as u32
            }) {
                RRETURN!(MATCH_NOMATCH);
            }
            state = ST_C2_5;
            continue 'sm;
        }

        /* Top of the maximizing backtracking for(;;) loop, caseful. */

        ST_C2_6 => {
            if (*F).eptr <= (*F).fields.char_repeat.start_eptr {
                state = ST_TOP;
                continue 'sm;
            }
            RMATCH!((*F).ecode, RM28);
        }

        ST_L_RM28 => {
            (*F).eptr = (*F).eptr.sub(1);
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            state = ST_C2_6;
            continue 'sm;
        }

        _ => {}
    }
}
