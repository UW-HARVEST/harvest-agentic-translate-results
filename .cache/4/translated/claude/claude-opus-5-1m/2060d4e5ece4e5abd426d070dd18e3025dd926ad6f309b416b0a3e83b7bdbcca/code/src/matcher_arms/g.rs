{
if lbl == LBL_SWITCH {
    match (*F).op as u32 {
        /* ===================================================================== */
        /* BRAZERO, BRAMINZERO and SKIPZERO occur just before a non-possessive
        bracket group, indicating that it may occur zero times. It may repeat
        infinitely, or not at all - i.e. it could be ()* or ()? or even (){0} in
        the pattern. Brackets with fixed upper repeat limits are compiled as a
        number of copies, with the optional ones preceded by BRAZERO or BRAMINZERO.
        Possessive groups with possible zero repeats are preceded by BRAPOSZERO. */

        /* case OP_BRAZERO: (C 5489) */
        OP_BRAZERO => {
            /* PCRE2_SPTR next_ecode; -- used after the RMATCH, see RM9 */
            (*F).ecode = (*F).ecode.add(1);
            start_ecode = (*F).ecode;
            (*F).return_id = RM9;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        /* case OP_BRAMINZERO: (C 5502) */
        OP_BRAMINZERO => {
            let mut next_ecode: PCRE2_SPTR;

            (*F).ecode = (*F).ecode.add(1);
            next_ecode = (*F).ecode;
            loop {
                next_ecode = next_ecode.add(GET(next_ecode, 1) as usize);
                if !(*next_ecode as u32 == OP_ALT) {
                    break;
                }
            }
            start_ecode = next_ecode.add(1 + LINK_SIZE);
            (*F).return_id = RM10;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        /* case OP_SKIPZERO: (C 5514) */
        OP_SKIPZERO => {
            let mut next_ecode: PCRE2_SPTR = (*F).ecode.add(1);
            loop {
                next_ecode = next_ecode.add(GET(next_ecode, 1) as usize);
                if !(*next_ecode as u32 == OP_ALT) {
                    break;
                }
            }
            (*F).ecode = next_ecode.add(1 + LINK_SIZE);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Handle possessive brackets with an unlimited repeat. The end of these
        brackets will always be OP_KETRPOS, which returns MATCH_KETRPOS without
        going further in the pattern. */

        /* #define Lstart_eptr    F->fields.op_brapos.start_eptr
           #define Lstart_group   F->fields.op_brapos.start_group
           #define Lframe_type    F->fields.op_brapos.frame_type
           #define Lmatched_once  F->byte1
           #define Lzero_allowed  F->byte2                                    */

        /* case OP_BRAPOSZERO: (C 5534) */
        OP_BRAPOSZERO => {
            (*F).byte2 = TRUE as u8; /* Zero repeat is allowed */
            (*F).ecode = (*F).ecode.add(1);
            if *(*F).ecode as u32 == OP_CBRAPOS || *(*F).ecode as u32 == OP_SCBRAPOS {
                lbl = LBL_POSSESSIVE_CAPTURE;
                continue 'sw;
            }
            lbl = LBL_POSSESSIVE_NON_CAPTURE;
            continue 'sw;
        }

        /* case OP_BRAPOS: case OP_SBRAPOS: (C 5541) */
        OP_BRAPOS | OP_SBRAPOS => {
            (*F).byte2 = FALSE as u8; /* Zero repeat not allowed */
            lbl = LBL_POSSESSIVE_NON_CAPTURE;
            continue 'sw;
        }

        /* case OP_CBRAPOS: case OP_SCBRAPOS: (C 5549) */
        OP_CBRAPOS | OP_SCBRAPOS => {
            (*F).byte2 = FALSE as u8; /* Zero repeat not allowed */
            lbl = LBL_POSSESSIVE_CAPTURE;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Handle non-capturing brackets that cannot match an empty string. When we
        get to the final alternative within the brackets, as long as there are no
        THEN's in the pattern, we can optimize by not recording a new backtracking
        point. (Ideally we should test for a THEN within this group, but we don't
        have that information.) Don't do this if we are at the very top level,
        however, because that would make handling assertions and once-only brackets
        messier when there is nothing to go back to. */

        /* #define Lframe_type    F->fields.op_bra.frame_type */

        /* case OP_BRA: (C 5622) */
        OP_BRA => {
            if (*mb).hasthen != FALSE || (*F).rdepth == 0 {
                (*F).fields.op_bra.frame_type = 0;
                lbl = LBL_GROUPLOOP;
                continue 'sw;
            }

            loop {
                let current_branch: PCRE2_SPTR = (*F).ecode;
                let next_branch: PCRE2_SPTR = current_branch.add(GET(current_branch, 1) as usize);

                if *next_branch as u32 != OP_ALT {
                    break;
                }

                /* This is never the final branch. We do not need to test for MATCH_THEN
                here because this code is not used when there is a THEN in the pattern. */

                (*F).ecode = next_branch;

                start_ecode = current_branch.add(1 + LINK_SIZE);
                (*F).return_id = RM1;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            }

            /* Hit the start of the final branch. Continue at this level. */

            (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Handle a capturing bracket, other than those that are possessive with an
        unlimited repeat. */

        /* case OP_CBRA: case OP_SCBRA: (C 5661) */
        OP_CBRA | OP_SCBRA => {
            (*F).fields.op_bra.frame_type = GF_CAPTURE | GET2((*F).ecode, 1 + LINK_SIZE);
            lbl = LBL_GROUPLOOP;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Atomic groups and non-capturing brackets that can match an empty string
        must record a backtracking point and also set up a chained frame. */

        /* case OP_ONCE: case OP_SCRIPT_RUN: case OP_SBRA: (C 5671) */
        OP_ONCE | OP_SCRIPT_RUN | OP_SBRA => {
            (*F).fields.op_bra.frame_type = GF_NOCAPTURE;
            lbl = LBL_GROUPLOOP;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Pattern recursion either matches the current regex, or some
        subexpression. The offset data is the offset to the starting bracket from
        the start of the whole pattern. This is so that it works from duplicated
        subpatterns. For a whole-pattern recursion, we have to infer the number
        zero. */

        /* #define Lstart_branch  F->fields.op_recurse.start_branch
           #define Lframe_type    F->fields.op_recurse.frame_type            */

        /* case OP_RECURSE: (C 5707) */
        OP_RECURSE => {
            bracode = (*mb).start_code.add(GET((*F).ecode, 1) as usize);
            number = if bracode == (*mb).start_code {
                0
            } else {
                GET2(bracode, 1 + LINK_SIZE)
            };

            /* If we are already in a pattern recursion, check for repeating the same
            one without changing the subject pointer or the last referenced character
            in the subject. This should catch convoluted mutual recursions; some
            simple cases are caught at compile time. However, there are rare cases when
            this check needs to be turned off. In this case, actual recursion loops
            will be caught by the match or heap limits. */

            if (*F).current_recurse != RECURSE_UNSET {
                offset = (*F).last_group_offset;
                while offset != PCRE2_UNSET {
                    N = frame_at((*match_data).heapframes, offset);
                    P = frame_sub(N, frame_size);
                    if (*N).group_frame_type == (GF_RECURSE | number) {
                        if (*F).eptr == (*P).eptr
                            && (*mb).last_used_ptr == (*P).recurse_last_used
                            && ((*mb).moptions & PCRE2_DISABLE_RECURSELOOP_CHECK) == 0
                        {
                            return PCRE2_ERROR_RECURSELOOP;
                        }
                        break;
                    }
                    offset = (*P).last_group_offset;
                }
            }

            /* Remember the current last referenced character and then run the
            recursion branch by branch. */

            (*F).recurse_last_used = (*mb).last_used_ptr;
            (*F).fields.op_recurse.start_branch = bracode;
            (*F).fields.op_recurse.frame_type = GF_RECURSE | number;

            /* for (;;) -- loop head */
            group_frame_type = (*F).fields.op_recurse.frame_type;
            start_ecode = (*F).fields.op_recurse.start_branch.add(
                _pcre2_OP_lengths_8[*(*F).fields.op_recurse.start_branch as usize] as usize,
            );
            (*F).return_id = RM11;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Positive assertions are like other groups except that PCRE doesn't allow
        the effect of (*THEN) to escape beyond an assertion; it is therefore
        treated as NOMATCH. (*ACCEPT) is treated as successful assertion, with its
        captures and mark retained. Any other return is an error. */

        /* case OP_ASSERT: case OP_ASSERTBACK: case OP_ASSERT_NA:
           case OP_ASSERTBACK_NA: (C 5789) */
        OP_ASSERT | OP_ASSERTBACK | OP_ASSERT_NA | OP_ASSERTBACK_NA => {
            /* for (;;) -- loop head */
            group_frame_type = GF_NOCAPTURE;
            start_ecode = (*F)
                .ecode
                .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize);
            (*F).return_id = RM3;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Handle negative assertions. Loop for each non-matching branch as for
        positive assertions. */

        /* case OP_ASSERT_NOT: case OP_ASSERTBACK_NOT: (C 5820) */
        OP_ASSERT_NOT | OP_ASSERTBACK_NOT => {
            /* for (;;) -- loop head */
            group_frame_type = GF_NOCAPTURE;
            start_ecode = (*F)
                .ecode
                .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize);
            (*F).return_id = RM4;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Handle scan substring operation. */

        /* #define Lsaved_end_subject   F->fields.op_assert_scs.saved_end_subject
           #define Lsaved_eptr          F->fields.op_assert_scs.saved_eptr
           #define Ltrue_end_extra      F->fields.op_assert_scs.true_end_extra
           #define Lextra_size          F->fields.op_assert_scs.extra_size
           #define Lsaved_moptions      F->fields.op_assert_scs.saved_moptions */

        /* case OP_ASSERT_SCS: (C 5867) */
        OP_ASSERT_SCS => {
            length = 0;
            {
                let mut ecode: PCRE2_SPTR = (*F).ecode.add(1 + LINK_SIZE);
                let mut count: c_int;
                let mut slot: PCRE2_SPTR;

                /* Disable compiler warning. */
                offset = 0;

                'scs_offset_found: {
                    loop {
                        if *ecode as u32 == OP_CREF {
                            length += 1 + IMM2_SIZE;
                            offset = (GET2(ecode, 1) << 1).wrapping_sub(2) as PCRE2_SIZE;
                            ecode = ecode.add(1 + IMM2_SIZE);
                            if offset < (*F).offset_top && *ovec(F).add(offset) != PCRE2_UNSET {
                                break 'scs_offset_found;
                            }
                            continue;
                        }

                        if *ecode as u32 != OP_DNCREF {
                            rrc = MATCH_NOMATCH;
                            lbl = LBL_RETURN_SWITCH;
                            continue 'sw;
                        }

                        count = GET2(ecode, 1 + IMM2_SIZE) as c_int;
                        slot = (*mb).name_table.add(
                            GET2(ecode, 1).wrapping_mul((*mb).name_entry_size as u32) as usize,
                        );
                        length += 1 + 2 * IMM2_SIZE;
                        ecode = ecode.add(1 + 2 * IMM2_SIZE);

                        while count > 0 {
                            offset = (GET2(slot, 0) << 1).wrapping_sub(2) as PCRE2_SIZE;
                            if offset < (*F).offset_top && *ovec(F).add(offset) != PCRE2_UNSET {
                                break 'scs_offset_found;
                            }
                            slot = slot.add((*mb).name_entry_size as usize);
                            count -= 1;
                        }
                    }
                }

                /* SCS_OFFSET_FOUND: */

                /* Skip remaining options. */
                loop {
                    if *ecode as u32 == OP_CREF {
                        length += 1 + IMM2_SIZE;
                        ecode = ecode.add(1 + IMM2_SIZE);
                    } else if *ecode as u32 == OP_DNCREF {
                        length += 1 + 2 * IMM2_SIZE;
                        ecode = ecode.add(1 + 2 * IMM2_SIZE);
                    } else {
                        break;
                    }
                }
            }

            (*F).fields.op_assert_scs.saved_end_subject = (*mb).end_subject;
            (*F).fields.op_assert_scs.true_end_extra =
                (*mb).true_end_subject.offset_from((*mb).end_subject) as PCRE2_SIZE;
            (*F).fields.op_assert_scs.saved_eptr = (*F).eptr;
            (*F).fields.op_assert_scs.saved_moptions = (*mb).moptions;

            (*F).eptr = (*mb).start_subject.add(*ovec(F).add(offset));
            (*mb).end_subject = (*mb).start_subject.add(*ovec(F).add(offset + 1));
            (*mb).true_end_subject = (*mb).end_subject;
            (*mb).moptions &= !PCRE2_NOTEOL;

            /* for (;;) -- loop head */
            group_frame_type = GF_NOCAPTURE;
            start_ecode = (*F).ecode.add(1 + LINK_SIZE + length);
            (*F).return_id = RM38;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        _ => {}
    }
}

/* ---- intra-switch labels owned by this chunk ---- */

/* POSSESSIVE_NON_CAPTURE: (C 5545) */
if lbl == LBL_POSSESSIVE_NON_CAPTURE {
    (*F).fields.op_brapos.frame_type = GF_NOCAPTURE; /* Remembered frame type */
    lbl = LBL_POSSESSIVE_GROUP;
    continue 'sw;
}

/* POSSESSIVE_CAPTURE: (C 5553) */
if lbl == LBL_POSSESSIVE_CAPTURE {
    number = GET2((*F).ecode, 1 + LINK_SIZE);
    (*F).fields.op_brapos.frame_type = GF_CAPTURE | number; /* Remembered frame type */
    lbl = LBL_POSSESSIVE_GROUP;
    continue 'sw;
}

/* POSSESSIVE_GROUP: (C 5557) */
if lbl == LBL_POSSESSIVE_GROUP {
    (*F).byte1 = FALSE as u8; /* Never matched */
    (*F).fields.op_brapos.start_group = (*F).ecode; /* Start of this group */

    /* for (;;) -- loop head */
    (*F).fields.op_brapos.start_eptr = (*F).eptr; /* Position at group start */
    group_frame_type = (*F).fields.op_brapos.frame_type;
    start_ecode = (*F)
        .ecode
        .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize);
    (*F).return_id = RM8;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* GROUPLOOP: (C 5676) */
if lbl == LBL_GROUPLOOP {
    /* for (;;) -- loop head */
    group_frame_type = (*F).fields.op_bra.frame_type;
    start_ecode = (*F)
        .ecode
        .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize);
    (*F).return_id = RM2;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* ASSERT_NOT_FAILED: (C 5853) */
if lbl == LBL_ASSERT_NOT_FAILED {
    (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
    lbl = LBL_TOP_OF_LOOP;
    continue 'sw;
}

/* ---- RMATCH continuations owned by this chunk ---- */

if lbl == LBL_RM_BASE + RM9 as u32 {
    /* After RMATCH in case OP_BRAZERO (C 5494) */
    let mut next_ecode: PCRE2_SPTR;

    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    next_ecode = (*F).ecode;
    loop {
        next_ecode = next_ecode.add(GET(next_ecode, 1) as usize);
        if !(*next_ecode as u32 == OP_ALT) {
            break;
        }
    }
    (*F).ecode = next_ecode.add(1 + LINK_SIZE);
    lbl = LBL_TOP_OF_LOOP;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM10 as u32 {
    /* After RMATCH in case OP_BRAMINZERO (C 5509) */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    lbl = LBL_TOP_OF_LOOP;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM8 as u32 {
    /* After RMATCH in the possessive group loop (C 5565) */
    'brapos_loop_end: {
        if rrc == MATCH_KETRPOS {
            (*F).byte1 = TRUE as u8; /* Matched at least once */
            if (*F).eptr == (*F).fields.op_brapos.start_eptr
            /* Empty match; skip to end */
            {
                loop {
                    (*F).ecode = (*F).ecode.add(GET((*F).ecode, 1) as usize);
                    if !(*(*F).ecode as u32 == OP_ALT) {
                        break;
                    }
                }
                break 'brapos_loop_end;
            }

            (*F).ecode = (*F).fields.op_brapos.start_group;

            /* continue; -- for (;;) loop head */
            (*F).fields.op_brapos.start_eptr = (*F).eptr;
            group_frame_type = (*F).fields.op_brapos.frame_type;
            start_ecode = (*F)
                .ecode
                .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize);
            (*F).return_id = RM8;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        /* See comment above about handling THEN. */

        if rrc == MATCH_THEN {
            let next_ecode: PCRE2_SPTR = (*F).ecode.add(GET((*F).ecode, 1) as usize);
            if (*mb).verb_ecode_ptr < next_ecode
                && (*(*F).ecode as u32 == OP_ALT || *next_ecode as u32 == OP_ALT)
            {
                rrc = MATCH_NOMATCH;
            }
        }

        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).ecode = (*F).ecode.add(GET((*F).ecode, 1) as usize);
        if *(*F).ecode as u32 != OP_ALT {
            break 'brapos_loop_end;
        }

        /* for (;;) loop head */
        (*F).fields.op_brapos.start_eptr = (*F).eptr;
        group_frame_type = (*F).fields.op_brapos.frame_type;
        start_ecode = (*F)
            .ecode
            .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize);
        (*F).return_id = RM8;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }

    /* Success if matched something or zero repeat allowed */

    if (*F).byte1 != 0 || (*F).byte2 != 0 {
        (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    rrc = MATCH_NOMATCH;
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM1 as u32 {
    /* After RMATCH in case OP_BRA (C 5644) */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }

    loop {
        let current_branch: PCRE2_SPTR = (*F).ecode;
        let next_branch: PCRE2_SPTR = current_branch.add(GET(current_branch, 1) as usize);

        if *next_branch as u32 != OP_ALT {
            break;
        }

        (*F).ecode = next_branch;

        start_ecode = current_branch.add(1 + LINK_SIZE);
        (*F).return_id = RM1;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }

    /* Hit the start of the final branch. Continue at this level. */

    (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
    lbl = LBL_TOP_OF_LOOP;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM2 as u32 {
    /* After RMATCH in GROUPLOOP (C 5680) */
    if rrc == MATCH_THEN {
        let next_ecode: PCRE2_SPTR = (*F).ecode.add(GET((*F).ecode, 1) as usize);
        if (*mb).verb_ecode_ptr < next_ecode
            && (*(*F).ecode as u32 == OP_ALT || *next_ecode as u32 == OP_ALT)
        {
            rrc = MATCH_NOMATCH;
        }
    }
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    (*F).ecode = (*F).ecode.add(GET((*F).ecode, 1) as usize);
    if *(*F).ecode as u32 != OP_ALT {
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    lbl = LBL_GROUPLOOP;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM11 as u32 {
    /* After RMATCH in case OP_RECURSE (C 5748) */
    let next_ecode: PCRE2_SPTR = (*F)
        .fields
        .op_recurse
        .start_branch
        .add(GET((*F).fields.op_recurse.start_branch, 1) as usize);

    /* Handle backtracking verbs, which are defined in a range that can
    easily be tested for. PCRE does not allow THEN, SKIP, PRUNE or COMMIT to
    escape beyond a recursion; they cause a NOMATCH for the entire recursion.

    When one of these verbs triggers, the current recursion group number is
    recorded. If it matches the recursion we are processing, the verb
    happened within the recursion and we must deal with it. Otherwise it must
    have happened after the recursion completed, and so has to be passed
    back. See comment above about handling THEN. */

    if rrc >= MATCH_BACKTRACK_MIN
        && rrc <= MATCH_BACKTRACK_MAX
        && (*mb).verb_current_recurse == ((*F).fields.op_recurse.frame_type ^ GF_RECURSE)
    {
        if rrc == MATCH_THEN
            && (*mb).verb_ecode_ptr < next_ecode
            && (*(*F).fields.op_recurse.start_branch as u32 == OP_ALT
                || *next_ecode as u32 == OP_ALT)
        {
            rrc = MATCH_NOMATCH;
        } else {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
    }

    /* Note that carrying on after (*ACCEPT) in a recursion is handled in the
    OP_ACCEPT code. Nothing needs to be done here. */

    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    (*F).fields.op_recurse.start_branch = next_ecode;
    if *(*F).fields.op_recurse.start_branch as u32 != OP_ALT {
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }

    /* for (;;) loop head */
    group_frame_type = (*F).fields.op_recurse.frame_type;
    start_ecode = (*F).fields.op_recurse.start_branch.add(
        _pcre2_OP_lengths_8[*(*F).fields.op_recurse.start_branch as usize] as usize,
    );
    (*F).return_id = RM11;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM3 as u32 {
    /* After RMATCH in the positive assertion loop (C 5796) */
    'assert_loop_end: {
        if rrc == MATCH_ACCEPT {
            memcpy(
                ovec(F) as *mut c_void,
                (assert_accept_frame as *mut u8)
                    .add(core::mem::offset_of!(heapframe, ovector)) as *const c_void,
                (*assert_accept_frame).offset_top * core::mem::size_of::<PCRE2_SIZE>(),
            );
            (*F).offset_top = (*assert_accept_frame).offset_top;
            (*F).mark = (*assert_accept_frame).mark;
            break 'assert_loop_end;
        }
        if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).ecode = (*F).ecode.add(GET((*F).ecode, 1) as usize);
        if *(*F).ecode as u32 != OP_ALT {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }

        /* for (;;) loop head */
        group_frame_type = GF_NOCAPTURE;
        start_ecode = (*F)
            .ecode
            .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize);
        (*F).return_id = RM3;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }

    loop {
        (*F).ecode = (*F).ecode.add(GET((*F).ecode, 1) as usize);
        if !(*(*F).ecode as u32 == OP_ALT) {
            break;
        }
    }
    (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
    lbl = LBL_TOP_OF_LOOP;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM4 as u32 {
    /* After RMATCH in the negative assertion loop (C 5825) */
    match rrc {
        /* Assertion matched, therefore it fails. */
        MATCH_ACCEPT | MATCH_MATCH => {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }

        /* Branch failed, try next if present. */
        MATCH_NOMATCH | MATCH_THEN => {
            (*F).ecode = (*F).ecode.add(GET((*F).ecode, 1) as usize);
            if *(*F).ecode as u32 != OP_ALT {
                lbl = LBL_ASSERT_NOT_FAILED;
                continue 'sw;
            }

            /* for (;;) loop head */
            group_frame_type = GF_NOCAPTURE;
            start_ecode = (*F)
                .ecode
                .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize);
            (*F).return_id = RM4;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        /* Assertion forced to fail, therefore continue. */
        MATCH_COMMIT | MATCH_SKIP | MATCH_PRUNE => {
            loop {
                (*F).ecode = (*F).ecode.add(GET((*F).ecode, 1) as usize);
                if !(*(*F).ecode as u32 == OP_ALT) {
                    break;
                }
            }
            lbl = LBL_ASSERT_NOT_FAILED;
            continue 'sw;
        }

        /* Pass back any other return */
        _ => {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
    }
}

if lbl == LBL_RM_BASE + RM38 as u32 {
    /* After RMATCH in case OP_ASSERT_SCS (C 5939) */
    'scs_loop_end: {
        if rrc == MATCH_ACCEPT {
            memcpy(
                ovec(F) as *mut c_void,
                (assert_accept_frame as *mut u8)
                    .add(core::mem::offset_of!(heapframe, ovector)) as *const c_void,
                (*assert_accept_frame).offset_top * core::mem::size_of::<PCRE2_SIZE>(),
            );
            (*F).offset_top = (*assert_accept_frame).offset_top;
            (*F).mark = (*assert_accept_frame).mark;
            (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
            (*mb).true_end_subject = (*mb)
                .end_subject
                .add((*F).fields.op_assert_scs.true_end_extra);
            (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
            break 'scs_loop_end;
        }

        if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
            (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
            (*mb).true_end_subject = (*mb)
                .end_subject
                .add((*F).fields.op_assert_scs.true_end_extra);
            (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }

        (*F).ecode = (*F).ecode.add(GET((*F).ecode, 1) as usize);
        if *(*F).ecode as u32 != OP_ALT {
            (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
            (*mb).true_end_subject = (*mb)
                .end_subject
                .add((*F).fields.op_assert_scs.true_end_extra);
            (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        length = 0;

        /* for (;;) loop head */
        group_frame_type = GF_NOCAPTURE;
        start_ecode = (*F).ecode.add(1 + LINK_SIZE + length);
        (*F).return_id = RM38;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }

    loop {
        (*F).ecode = (*F).ecode.add(GET((*F).ecode, 1) as usize);
        if !(*(*F).ecode as u32 == OP_ALT) {
            break;
        }
    }
    (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
    (*F).eptr = (*F).fields.op_assert_scs.saved_eptr;
    lbl = LBL_TOP_OF_LOOP;
    continue 'sw;
}
}
