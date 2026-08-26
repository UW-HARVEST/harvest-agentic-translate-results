{
    match state {
        /* ===================================================================== */
        /* Match a back reference, possibly repeatedly. Look past the end of the
        item to see if there is repeat information following. The OP_REF and
        OP_REFI opcodes are used for a reference to a numbered group or to a
        non-duplicated named group. For a duplicated named group, OP_DNREF and
        OP_DNREFI are used. In this case we must scan the list of groups to which
        the name refers, and use the first one that is set. */

        /* Lstart    = (*F).fields.ref_repeat.start
           Loffset   = (*F).fields.ref_repeat.offset
           Llength   = (*F).fields.ref_repeat.length
           Lmin      = (*F).fields.ref_repeat.min
           Lmax      = (*F).fields.ref_repeat.max
           Lcaseless = (*F).byte1
           Lcaseopts = (*F).byte2 */

        OP_DNREF | OP_DNREFI => {
            (*F).byte1 = ((*F).op as u32 == OP_DNREFI) as u8;
            (*F).byte2 = if (*F).op as u32 == OP_DNREFI {
                *(*F).ecode.add(1 + 2 * IMM2_SIZE)
            } else {
                0
            };
            {
                let mut count: c_int = GET2!((*F).ecode, 1 + IMM2_SIZE) as c_int;
                let mut slot: PCRE2_SPTR = (*mb)
                    .name_table
                    .add((GET2!((*F).ecode, 1) * (*mb).name_entry_size as u32) as usize);
                (*F).ecode = (*F).ecode.add(
                    1 + 2 * IMM2_SIZE + (if (*F).op as u32 == OP_DNREFI { 1 } else { 0 }),
                );

                loop {
                    let c = count;
                    count -= 1;
                    if !(c > 0) {
                        break;
                    }
                    (*F).fields.ref_repeat.offset =
                        (GET2!(slot, 0) << 1).wrapping_sub(2) as PCRE2_SIZE;
                    if (*F).fields.ref_repeat.offset < (*F).offset_top
                        && Fov!((*F).fields.ref_repeat.offset) != PCRE2_UNSET
                    {
                        break;
                    }
                    slot = slot.add((*mb).name_entry_size as usize);
                }
            }
            state = ST_REF_REPEAT;
            continue 'sm;
        }

        OP_REF | OP_REFI => {
            (*F).byte1 = ((*F).op as u32 == OP_REFI) as u8;
            (*F).byte2 = if (*F).op as u32 == OP_REFI {
                *(*F).ecode.add(1 + IMM2_SIZE)
            } else {
                0
            };
            (*F).fields.ref_repeat.offset =
                (GET2!((*F).ecode, 1) << 1).wrapping_sub(2) as PCRE2_SIZE;
            (*F).ecode = (*F)
                .ecode
                .add(1 + IMM2_SIZE + (if (*F).op as u32 == OP_REFI { 1 } else { 0 }));

            /* Set up for repetition, or handle the non-repeated case. The maximum and
            minimum must be in the heap frame, but as they are short-term values, we
            use temporary fields. */

            state = ST_REF_REPEAT;
            continue 'sm;
        }

        ST_REF_REPEAT => {
            match *(*F).ecode as u32 {
                OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
                | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                    fc = {
                        let t = *(*F).ecode;
                        (*F).ecode = (*F).ecode.add(1);
                        t as u32
                    }
                    .wrapping_sub(OP_CRSTAR);
                    (*F).fields.ref_repeat.min = *rep_min.as_ptr().add(fc as usize);
                    (*F).fields.ref_repeat.max = *rep_max.as_ptr().add(fc as usize);
                    reptype = *rep_typ.as_ptr().add(fc as usize);
                }

                OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                    (*F).fields.ref_repeat.min = GET2!((*F).ecode, 1);
                    (*F).fields.ref_repeat.max = GET2!((*F).ecode, 1 + IMM2_SIZE);
                    reptype = *rep_typ
                        .as_ptr()
                        .add((*(*F).ecode as u32).wrapping_sub(OP_CRSTAR) as usize);
                    if (*F).fields.ref_repeat.max == 0 {
                        (*F).fields.ref_repeat.max = u32::MAX; /* Max 0 => infinity */
                    }
                    (*F).ecode = (*F).ecode.add(1 + 2 * IMM2_SIZE);
                }

                _ => {
                    /* No repeat follows */
                    {
                        length = 0;
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
                            CHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    (*F).eptr = (*F).eptr.add(length);
                    state = ST_TOP;
                    continue 'sm; /* With the main loop */
                }
            }

            /* Handle repeated back references. If a set group has length zero, just
            continue with the main loop, because it matches however many times. For an
            unset reference, if the minimum is zero, we can also just continue. We can
            also continue if PCRE2_MATCH_UNSET_BACKREF is set, because this makes unset
            group behave as a zero-length group. For any other unset cases, carrying
            on will result in NOMATCH. */

            if (*F).fields.ref_repeat.offset < (*F).offset_top
                && Fov!((*F).fields.ref_repeat.offset) != PCRE2_UNSET
            {
                if Fov!((*F).fields.ref_repeat.offset)
                    == Fov!((*F).fields.ref_repeat.offset + 1)
                {
                    state = ST_TOP;
                    continue 'sm;
                }
            } else
            /* Group is not set */
            {
                if (*F).fields.ref_repeat.min == 0
                    || ((*mb).poptions & PCRE2_MATCH_UNSET_BACKREF) != 0
                {
                    state = ST_TOP;
                    continue 'sm;
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
                    CHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                (*F).eptr = (*F).eptr.add(slength);
                i = i.wrapping_add(1);
            }

            /* If min = max, we are done. They are not both allowed to be zero. */

            if (*F).fields.ref_repeat.min == (*F).fields.ref_repeat.max {
                state = ST_TOP;
                continue 'sm;
            }

            /* If minimizing, keep trying and advancing the pointer. */

            if reptype == REPTYPE_MIN {
                RMATCH!((*F).ecode, RM20);
            }
            /* If maximizing, find the longest string and work backwards, as long as
            the matched lengths for each iteration are the same. */
            else {
                let mut samelengths: BOOL = TRUE;
                (*F).fields.ref_repeat.start = (*F).eptr; /* Starting position */
                (*F).fields.ref_repeat.length = Fov!((*F).fields.ref_repeat.offset + 1)
                    - Fov!((*F).fields.ref_repeat.offset);

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

                        if rrc > 0
                            && (*mb).partial != 0
                            && (*mb).end_subject > (*mb).start_used_ptr
                        {
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
                    state = ST_TOP;
                    continue 'sm;
                }

                /* If the length matched for each repetition is the same as the length of
                the captured group, we can easily work backwards. This is the normal
                case. However, in caseless UTF-8 mode there are pairs of case-equivalent
                characters whose lengths (in terms of code units) differ. However, this
                is very rare, so we handle it by re-matching fewer and fewer times. */

                if samelengths != 0 {
                    state = ST_C9_1; /* while (Feptr >= Lstart) */
                    continue 'sm;
                }
                /* The rare case of non-matching lengths. Re-scan the repetition for each
                iteration. We know that match_ref() will succeed every time. */
                else {
                    (*F).fields.ref_repeat.max = i;
                    RMATCH!((*F).ecode, RM22);
                }
            }
        }

        ST_L_RM20 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            {
                let old_min = (*F).fields.ref_repeat.min;
                (*F).fields.ref_repeat.min = old_min.wrapping_add(1);
                if old_min >= (*F).fields.ref_repeat.max {
                    RRETURN!(MATCH_NOMATCH);
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
                    CHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                (*F).eptr = (*F).eptr.add(slength);
            }
            RMATCH!((*F).ecode, RM20); /* Back to the top of the C for(;;) loop */
        }

        /* Top of the C "while (Feptr >= Lstart)" loop. */
        ST_C9_1 => {
            if (*F).eptr >= (*F).fields.ref_repeat.start {
                RMATCH!((*F).ecode, RM21);
            }
            RRETURN!(MATCH_NOMATCH);
        }

        ST_L_RM21 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).eptr = (*F).eptr.sub((*F).fields.ref_repeat.length);
            state = ST_C9_1;
            continue 'sm;
        }

        ST_L_RM22 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            if (*F).eptr == (*F).fields.ref_repeat.start {
                /* Failed after minimal repetition */
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).eptr = (*F).fields.ref_repeat.start;
            (*F).fields.ref_repeat.max = (*F).fields.ref_repeat.max.wrapping_sub(1);
            i = (*F).fields.ref_repeat.min;
            while i < (*F).fields.ref_repeat.max {
                let mut slength: PCRE2_SIZE = 0;
                let _ = match_ref(
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
            RMATCH!((*F).ecode, RM22);
        }

        /* ========================================================================= */
        /*           Opcodes for the start of various parenthesized items            */
        /* ========================================================================= */

        /* In all cases, if the result of RMATCH() is MATCH_THEN, check whether the
        (*THEN) is within the current branch by comparing the address of OP_THEN
        that is passed back with the end of the branch. If (*THEN) is within the
        current branch, and the branch is one of two or more alternatives (it
        either starts or ends with OP_ALT), we have reached the limit of THEN's
        action, so convert the return code to NOMATCH, which will cause normal
        backtracking to happen from now on. Otherwise, THEN is passed back to an
        outer alternative. This implements Perl's treatment of parenthesized
        groups, where a group not containing | does not affect the current
        alternative, that is, (X) is NOT the same as (X|(*F)). */

        /* ===================================================================== */
        /* BRAZERO, BRAMINZERO and SKIPZERO occur just before a non-possessive
        bracket group, indicating that it may occur zero times. It may repeat
        infinitely, or not at all - i.e. it could be ()* or ()? or even (){0} in
        the pattern. Brackets with fixed upper repeat limits are compiled as a
        number of copies, with the optional ones preceded by BRAZERO or BRAMINZERO.
        Possessive groups with possible zero repeats are preceded by BRAPOSZERO. */

        OP_BRAZERO => {
            (*F).ecode = (*F).ecode.add(1);
            RMATCH!((*F).ecode, RM9);
        }

        ST_L_RM9 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            {
                let mut next_ecode: PCRE2_SPTR = (*F).ecode;
                loop {
                    next_ecode = next_ecode.add(GET!(next_ecode, 1) as usize);
                    if *next_ecode as u32 != OP_ALT {
                        break;
                    }
                }
                (*F).ecode = next_ecode.add(1 + LINK_SIZE);
            }
            state = ST_TOP;
            continue 'sm;
        }

        OP_BRAMINZERO => {
            let mut next_ecode: PCRE2_SPTR;

            (*F).ecode = (*F).ecode.add(1);
            next_ecode = (*F).ecode;
            loop {
                next_ecode = next_ecode.add(GET!(next_ecode, 1) as usize);
                if *next_ecode as u32 != OP_ALT {
                    break;
                }
            }
            RMATCH!(next_ecode.add(1 + LINK_SIZE), RM10);
        }

        ST_L_RM10 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            state = ST_TOP;
            continue 'sm;
        }

        OP_SKIPZERO => {
            let mut next_ecode: PCRE2_SPTR = (*F).ecode.add(1);
            loop {
                next_ecode = next_ecode.add(GET!(next_ecode, 1) as usize);
                if *next_ecode as u32 != OP_ALT {
                    break;
                }
            }
            (*F).ecode = next_ecode.add(1 + LINK_SIZE);
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Handle possessive brackets with an unlimited repeat. The end of these
        brackets will always be OP_KETRPOS, which returns MATCH_KETRPOS without
        going further in the pattern. */

        /* Lstart_eptr   = (*F).fields.op_brapos.start_eptr
           Lstart_group  = (*F).fields.op_brapos.start_group
           Lframe_type   = (*F).fields.op_brapos.frame_type
           Lmatched_once = (*F).byte1
           Lzero_allowed = (*F).byte2 */

        OP_BRAPOSZERO => {
            (*F).byte2 = TRUE as u8; /* Zero repeat is allowed */
            (*F).ecode = (*F).ecode.add(1);
            if *(*F).ecode as u32 == OP_CBRAPOS || *(*F).ecode as u32 == OP_SCBRAPOS {
                state = ST_POSSESSIVE_CAPTURE;
                continue 'sm;
            }
            state = ST_POSSESSIVE_NON_CAPTURE;
            continue 'sm;
        }

        OP_BRAPOS | OP_SBRAPOS => {
            (*F).byte2 = FALSE as u8; /* Zero repeat not allowed */
            state = ST_POSSESSIVE_NON_CAPTURE;
            continue 'sm;
        }

        ST_POSSESSIVE_NON_CAPTURE => {
            (*F).fields.op_brapos.frame_type = GF_NOCAPTURE; /* Remembered frame type */
            state = ST_POSSESSIVE_GROUP;
            continue 'sm;
        }

        OP_CBRAPOS | OP_SCBRAPOS => {
            (*F).byte2 = FALSE as u8; /* Zero repeat not allowed */
            state = ST_POSSESSIVE_CAPTURE;
            continue 'sm;
        }

        ST_POSSESSIVE_CAPTURE => {
            number = GET2!((*F).ecode, 1 + LINK_SIZE);
            (*F).fields.op_brapos.frame_type = GF_CAPTURE | number; /* Remembered frame type */
            state = ST_POSSESSIVE_GROUP;
            continue 'sm;
        }

        ST_POSSESSIVE_GROUP => {
            (*F).byte1 = FALSE as u8; /* Never matched */
            (*F).fields.op_brapos.start_group = (*F).ecode; /* Start of this group */
            state = ST_C9_2;
            continue 'sm;
        }

        /* Top of the C for(;;) loop of the possessive group. */
        ST_C9_2 => {
            (*F).fields.op_brapos.start_eptr = (*F).eptr; /* Position at group start */
            group_frame_type = (*F).fields.op_brapos.frame_type;
            RMATCH!(
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize),
                RM8
            );
        }

        ST_L_RM8 => {
            if rrc == MATCH_KETRPOS {
                (*F).byte1 = TRUE as u8; /* Matched at least once */
                if (*F).eptr == (*F).fields.op_brapos.start_eptr {
                    /* Empty match; skip to end */
                    loop {
                        (*F).ecode = (*F).ecode.add(GET!((*F).ecode, 1) as usize);
                        if *(*F).ecode as u32 != OP_ALT {
                            break;
                        }
                    }
                    state = ST_C9_3;
                    continue 'sm;
                }

                (*F).ecode = (*F).fields.op_brapos.start_group;
                state = ST_C9_2;
                continue 'sm;
            }

            /* See comment above about handling THEN. */

            if rrc == MATCH_THEN {
                let next_ecode: PCRE2_SPTR = (*F).ecode.add(GET!((*F).ecode, 1) as usize);
                if (*mb).verb_ecode_ptr < next_ecode
                    && (*(*F).ecode as u32 == OP_ALT || *next_ecode as u32 == OP_ALT)
                {
                    rrc = MATCH_NOMATCH;
                }
            }

            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).ecode = (*F).ecode.add(GET!((*F).ecode, 1) as usize);
            if *(*F).ecode as u32 != OP_ALT {
                state = ST_C9_3;
                continue 'sm;
            }
            state = ST_C9_2;
            continue 'sm;
        }

        /* After the possessive group loop. */
        ST_C9_3 => {
            /* Success if matched something or zero repeat allowed */

            if (*F).byte1 != 0 || (*F).byte2 != 0 {
                (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
                state = ST_TOP;
                continue 'sm;
            }

            RRETURN!(MATCH_NOMATCH);
        }

        /* ===================================================================== */
        /* Handle non-capturing brackets that cannot match an empty string. When we
        get to the final alternative within the brackets, as long as there are no
        THEN's in the pattern, we can optimize by not recording a new backtracking
        point. (Ideally we should test for a THEN within this group, but we don't
        have that information.) Don't do this if we are at the very top level,
        however, because that would make handling assertions and once-only brackets
        messier when there is nothing to go back to. */

        /* Lframe_type = (*F).fields.op_bra.frame_type */

        OP_BRA => {
            if (*mb).hasthen != 0 || (*F).rdepth == 0 {
                (*F).fields.op_bra.frame_type = 0;
                state = ST_GROUPLOOP;
                continue 'sm;
            }

            state = ST_C9_4;
            continue 'sm;
        }

        /* Top of the C for(;;) loop of OP_BRA. */
        ST_C9_4 => {
            let current_branch: PCRE2_SPTR = (*F).ecode;
            let next_branch: PCRE2_SPTR = current_branch.add(GET!(current_branch, 1) as usize);

            if *next_branch as u32 != OP_ALT {
                /* Hit the start of the final branch. Continue at this level. */

                (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
                state = ST_TOP;
                continue 'sm;
            }

            /* This is never the final branch. We do not need to test for MATCH_THEN
            here because this code is not used when there is a THEN in the pattern. */

            (*F).ecode = next_branch;

            RMATCH!(current_branch.add(1 + LINK_SIZE), RM1);
        }

        ST_L_RM1 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            state = ST_C9_4;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Handle a capturing bracket, other than those that are possessive with an
        unlimited repeat. */

        OP_CBRA | OP_SCBRA => {
            (*F).fields.op_bra.frame_type = GF_CAPTURE | GET2!((*F).ecode, 1 + LINK_SIZE);
            state = ST_GROUPLOOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Atomic groups and non-capturing brackets that can match an empty string
        must record a backtracking point and also set up a chained frame. */

        OP_ONCE | OP_SCRIPT_RUN | OP_SBRA => {
            (*F).fields.op_bra.frame_type = GF_NOCAPTURE;
            state = ST_GROUPLOOP;
            continue 'sm;
        }

        ST_GROUPLOOP => {
            group_frame_type = (*F).fields.op_bra.frame_type;
            RMATCH!(
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize),
                RM2
            );
        }

        ST_L_RM2 => {
            if rrc == MATCH_THEN {
                let next_ecode: PCRE2_SPTR = (*F).ecode.add(GET!((*F).ecode, 1) as usize);
                if (*mb).verb_ecode_ptr < next_ecode
                    && (*(*F).ecode as u32 == OP_ALT || *next_ecode as u32 == OP_ALT)
                {
                    rrc = MATCH_NOMATCH;
                }
            }
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).ecode = (*F).ecode.add(GET!((*F).ecode, 1) as usize);
            if *(*F).ecode as u32 != OP_ALT {
                RRETURN!(MATCH_NOMATCH);
            }
            state = ST_GROUPLOOP;
            continue 'sm;
        }

        _ => {}
    }
}
