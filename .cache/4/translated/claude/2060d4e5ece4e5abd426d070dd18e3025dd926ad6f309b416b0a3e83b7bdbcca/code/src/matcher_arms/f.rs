{
/* C 5249-5488: OP_DNREF/OP_DNREFI, OP_REF/OP_REFI and the REF_REPEAT block.

#define Lstart    F->fields.ref_repeat.start
#define Loffset   F->fields.ref_repeat.offset
#define Llength   F->fields.ref_repeat.length
#define Lmin      F->fields.ref_repeat.min
#define Lmax      F->fields.ref_repeat.max
#define Lcaseless F->byte1
#define Lcaseopts F->byte2
*/

if lbl == LBL_SWITCH {
    match (*F).op as u32 {
        /* ===================================================================== */
        /* Match a back reference, possibly repeatedly. Look past the end of the
        item to see if there is repeat information following. */

        OP_DNREF | OP_DNREFI => {
            (*F).byte1 = ((*F).op as u32 == OP_DNREFI) as u8;
            (*F).byte2 = if (*F).op as u32 == OP_DNREFI {
                *(*F).ecode.add(1 + 2 * IMM2_SIZE)
            } else {
                0
            };
            {
                let mut count: c_int = GET2((*F).ecode, 1 + IMM2_SIZE) as c_int;
                let mut slot: PCRE2_SPTR = (*mb)
                    .name_table
                    .add(GET2((*F).ecode, 1) as usize * (*mb).name_entry_size as usize);
                (*F).ecode = (*F).ecode.add(
                    1 + 2 * IMM2_SIZE + (if (*F).op as u32 == OP_DNREFI { 1 } else { 0 }),
                );

                loop {
                    let c = count;
                    count = count.wrapping_sub(1);
                    if !(c > 0) {
                        break;
                    }
                    (*F).fields.ref_repeat.offset =
                        (GET2(slot, 0) << 1).wrapping_sub(2) as PCRE2_SIZE;
                    if (*F).fields.ref_repeat.offset < (*F).offset_top
                        && *ovec(F).add((*F).fields.ref_repeat.offset) != PCRE2_UNSET
                    {
                        break;
                    }
                    slot = slot.add((*mb).name_entry_size as usize);
                }
            }
            lbl = LBL_REF_REPEAT;
            continue 'sw;
        }

        OP_REF | OP_REFI => {
            (*F).byte1 = ((*F).op as u32 == OP_REFI) as u8;
            (*F).byte2 = if (*F).op as u32 == OP_REFI {
                *(*F).ecode.add(1 + IMM2_SIZE)
            } else {
                0
            };
            (*F).fields.ref_repeat.offset =
                (GET2((*F).ecode, 1) << 1).wrapping_sub(2) as PCRE2_SIZE;
            (*F).ecode = (*F)
                .ecode
                .add(1 + IMM2_SIZE + (if (*F).op as u32 == OP_REFI { 1 } else { 0 }));

            /* Set up for repetition, or handle the non-repeated case. The maximum and
            minimum must be in the heap frame, but as they are short-term values, we
            use temporary fields. */

            lbl = LBL_REF_REPEAT;
            continue 'sw;
        }

        _ => {}
    }
}

/* REF_REPEAT: */
if lbl == LBL_REF_REPEAT {
    match *(*F).ecode as u32 {
        OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY | OP_CRMINQUERY
        | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
            fc = ({
                let v = *(*F).ecode as u32;
                (*F).ecode = (*F).ecode.add(1);
                v
            })
            .wrapping_sub(OP_CRSTAR);
            (*F).fields.ref_repeat.min = rep_min[fc as usize];
            (*F).fields.ref_repeat.max = rep_max[fc as usize];
            reptype = rep_typ[fc as usize];
        }

        OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
            (*F).fields.ref_repeat.min = GET2((*F).ecode, 1);
            (*F).fields.ref_repeat.max = GET2((*F).ecode, 1 + IMM2_SIZE);
            reptype = rep_typ[(*(*F).ecode as u32).wrapping_sub(OP_CRSTAR) as usize];
            if (*F).fields.ref_repeat.max == 0 {
                (*F).fields.ref_repeat.max = u32::MAX; /* Max 0 => infinity */
            }
            (*F).ecode = (*F).ecode.add(1 + 2 * IMM2_SIZE);
        }

        _ => {
            /* No repeat follows */
            {
                rrc = match_ref(
                    (*F).fields.ref_repeat.offset,
                    (*F).byte1 as BOOL,
                    (*F).byte2 as c_int,
                    F,
                    mb,
                    &mut length,
                );
                if rrc != 0 {
                    if rrc > 0 {
                        (*F).eptr = (*mb).end_subject; /* Partial match */
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
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            }
            (*F).eptr = (*F).eptr.add(length);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw; /* With the main loop */
        }
    }

    /* Handle repeated back references. If a set group has length zero, just
    continue with the main loop, because it matches however many times. For an
    unset reference, if the minimum is zero, we can also just continue. We can
    also continue if PCRE2_MATCH_UNSET_BACKREF is set, because this makes unset
    group behave as a zero-length group. For any other unset cases, carrying
    on will result in NOMATCH. */

    if (*F).fields.ref_repeat.offset < (*F).offset_top
        && *ovec(F).add((*F).fields.ref_repeat.offset) != PCRE2_UNSET
    {
        if *ovec(F).add((*F).fields.ref_repeat.offset)
            == *ovec(F).add((*F).fields.ref_repeat.offset + 1)
        {
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }
    } else
    /* Group is not set */
    {
        if (*F).fields.ref_repeat.min == 0
            || ((*mb).poptions & PCRE2_MATCH_UNSET_BACKREF) != 0
        {
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }
    }

    /* First, ensure the minimum number of matches are present. */

    i = 1;
    while i <= (*F).fields.ref_repeat.min {
        let mut slength: PCRE2_SIZE = 0;
        rrc = match_ref(
            (*F).fields.ref_repeat.offset,
            (*F).byte1 as BOOL,
            (*F).byte2 as c_int,
            F,
            mb,
            &mut slength,
        );
        if rrc != 0 {
            if rrc > 0 {
                (*F).eptr = (*mb).end_subject; /* Partial match */
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
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).eptr = (*F).eptr.add(slength);
        i = i.wrapping_add(1);
    }

    /* If min = max, we are done. They are not both allowed to be zero. */

    if (*F).fields.ref_repeat.min == (*F).fields.ref_repeat.max {
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    /* If minimizing, keep trying and advancing the pointer. */

    if reptype == REPTYPE_MIN {
        /* for (;;) { RMATCH(Fecode, RM20); ... } */
        start_ecode = (*F).ecode;
        (*F).return_id = RM20;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    /* If maximizing, find the longest string and work backwards, as long as
    the matched lengths for each iteration are the same. */
    else {
        let mut samelengths: BOOL = TRUE;
        (*F).fields.ref_repeat.start = (*F).eptr; /* Starting position */
        (*F).fields.ref_repeat.length = (*ovec(F).add((*F).fields.ref_repeat.offset + 1))
            .wrapping_sub(*ovec(F).add((*F).fields.ref_repeat.offset));

        i = (*F).fields.ref_repeat.min;
        while i < (*F).fields.ref_repeat.max {
            let mut slength: PCRE2_SIZE = 0;
            rrc = match_ref(
                (*F).fields.ref_repeat.offset,
                (*F).byte1 as BOOL,
                (*F).byte2 as c_int,
                F,
                mb,
                &mut slength,
            );
            if rrc != 0 {
                /* Can't use CHECK_PARTIAL because we don't want to update Feptr in
                the soft partial matching case. */

                if rrc > 0 && (*mb).partial != 0 && (*mb).end_subject > (*mb).start_used_ptr {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                break;
            }

            if slength != (*F).fields.ref_repeat.length {
                samelengths = FALSE;
            }
            (*F).eptr = (*F).eptr.add(slength);
            i = i.wrapping_add(1);
        }

        /* No recursion if the repeat type is possessive. */
        if reptype == REPTYPE_POS {
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* If the length matched for each repetition is the same as the length of
        the captured group, we can easily work backwards. This is the normal
        case. However, in caseless UTF-8 mode there are pairs of case-equivalent
        characters whose lengths (in terms of code units) differ. However, this
        is very rare, so we handle it by re-matching fewer and fewer times. */

        if samelengths != FALSE {
            while (*F).eptr >= (*F).fields.ref_repeat.start {
                /* RMATCH(Fecode, RM21); */
                start_ecode = (*F).ecode;
                (*F).return_id = RM21;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            }
        }
        /* The rare case of non-matching lengths. Re-scan the repetition for each
        iteration. We know that match_ref() will succeed every time. */
        else {
            (*F).fields.ref_repeat.max = i;
            /* for (;;) { RMATCH(Fecode, RM22); ... } */
            start_ecode = (*F).ecode;
            (*F).return_id = RM22;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
}

/* ---- RMATCH continuations owned by this chunk ---- */

if lbl == LBL_RM_BASE + RM20 as u32 {
    /* After RMATCH in the minimizing loop of REF_REPEAT (C 5363). */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    {
        let old = (*F).fields.ref_repeat.min;
        (*F).fields.ref_repeat.min = old.wrapping_add(1);
        if old >= (*F).fields.ref_repeat.max {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
    }
    {
        let mut slength: PCRE2_SIZE = 0;
        rrc = match_ref(
            (*F).fields.ref_repeat.offset,
            (*F).byte1 as BOOL,
            (*F).byte2 as c_int,
            F,
            mb,
            &mut slength,
        );
        if rrc != 0 {
            if rrc > 0 {
                (*F).eptr = (*mb).end_subject; /* Partial match */
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
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).eptr = (*F).eptr.add(slength);
    }
    /* Loop back to RMATCH(Fecode, RM20). */
    start_ecode = (*F).ecode;
    (*F).return_id = RM20;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM21 as u32 {
    /* After RMATCH in the "samelengths" backwards loop of REF_REPEAT (C 5423). */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    (*F).eptr = (*F).eptr.wrapping_sub((*F).fields.ref_repeat.length);

    while (*F).eptr >= (*F).fields.ref_repeat.start {
        /* RMATCH(Fecode, RM21); */
        start_ecode = (*F).ecode;
        (*F).return_id = RM21;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }

    rrc = MATCH_NOMATCH;
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM22 as u32 {
    /* After RMATCH in the non-matching-lengths loop of REF_REPEAT (C 5437). */
    'rm22: {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        if (*F).eptr == (*F).fields.ref_repeat.start {
            break 'rm22; /* Failed after minimal repetition */
        }
        (*F).eptr = (*F).fields.ref_repeat.start;
        (*F).fields.ref_repeat.max = (*F).fields.ref_repeat.max.wrapping_sub(1);
        i = (*F).fields.ref_repeat.min;
        while i < (*F).fields.ref_repeat.max {
            let mut slength: PCRE2_SIZE = 0;
            match_ref(
                (*F).fields.ref_repeat.offset,
                (*F).byte1 as BOOL,
                (*F).byte2 as c_int,
                F,
                mb,
                &mut slength,
            );
            (*F).eptr = (*F).eptr.add(slength);
            i = i.wrapping_add(1);
        }
        /* Loop back to RMATCH(Fecode, RM22). */
        start_ecode = (*F).ecode;
        (*F).return_id = RM22;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }

    rrc = MATCH_NOMATCH;
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}
}
