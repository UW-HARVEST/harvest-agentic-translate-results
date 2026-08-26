/* Fragment 10 of the match() switch: OP_RECURSE, the positive assertions, the
negative assertions and OP_ASSERT_SCS.
Translated from c_src/src/pcre2_match.c lines 5707..5988. */
{
    match state {
        /* ===================================================================== */
        /* Pattern recursion either matches the current regex, or some
        subexpression. The offset data is the offset to the starting bracket from
        the start of the whole pattern. This is so that it works from duplicated
        subpatterns. For a whole-pattern recursion, we have to infer the number
        zero. */

        /* Lstart_branch = (*F).fields.op_recurse.start_branch
           Lframe_type   = (*F).fields.op_recurse.frame_type */

        OP_RECURSE => {
            bracode = (*mb).start_code.add(GET!((*F).ecode, 1) as usize);
            number = if bracode == (*mb).start_code {
                0
            } else {
                GET2!(bracode, 1 + LINK_SIZE)
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
                    N = ((*match_data).heapframes as *mut u8).add(offset) as *mut heapframe;
                    P = (N as *mut u8).sub(frame_size) as *mut heapframe;
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

            state = ST_C10_1;
            continue 'sm;
        }

        /* Top of the C for (;;) loop of OP_RECURSE. */
        ST_C10_1 => {
            group_frame_type = (*F).fields.op_recurse.frame_type;
            let sb: PCRE2_SPTR = (*F).fields.op_recurse.start_branch;
            RMATCH!(
                sb.add(*_pcre2_OP_lengths_8.as_ptr().add(*sb as usize) as usize),
                RM11
            );
        }

        ST_L_RM11 => {
            let sb: PCRE2_SPTR = (*F).fields.op_recurse.start_branch;
            let next_ecode: PCRE2_SPTR = sb.add(GET!(sb, 1) as usize);

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
                    && (*sb as u32 == OP_ALT || *next_ecode as u32 == OP_ALT)
                {
                    rrc = MATCH_NOMATCH;
                } else {
                    RRETURN!(MATCH_NOMATCH);
                }
            }

            /* Note that carrying on after (*ACCEPT) in a recursion is handled in the
            OP_ACCEPT code. Nothing needs to be done here. */

            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).fields.op_recurse.start_branch = next_ecode;
            if *(*F).fields.op_recurse.start_branch as u32 != OP_ALT {
                RRETURN!(MATCH_NOMATCH);
            }
            state = ST_C10_1;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Positive assertions are like other groups except that PCRE doesn't allow
        the effect of (*THEN) to escape beyond an assertion; it is therefore
        treated as NOMATCH. (*ACCEPT) is treated as successful assertion, with its
        captures and mark retained. Any other return is an error. */

        OP_ASSERT | OP_ASSERTBACK | OP_ASSERT_NA | OP_ASSERTBACK_NA => {
            state = ST_C10_2;
            continue 'sm;
        }

        /* Top of the C for (;;) loop of the positive assertions. */
        ST_C10_2 => {
            group_frame_type = GF_NOCAPTURE;
            RMATCH!(
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize),
                RM3
            );
        }

        ST_L_RM3 => {
            if rrc == MATCH_ACCEPT {
                memcpy(
                    (*F).ovector.as_mut_ptr() as *mut c_void,
                    (assert_accept_frame as *mut u8).add(offset_of!(heapframe, ovector))
                        as *const c_void,
                    (*assert_accept_frame).offset_top * size_of::<PCRE2_SIZE>(),
                );
                (*F).offset_top = (*assert_accept_frame).offset_top;
                (*F).mark = (*assert_accept_frame).mark;

                /* The C `break` leaves the for (;;) loop and falls into the tail. */

                loop {
                    (*F).ecode = (*F).ecode.add(GET!((*F).ecode, 1) as usize);
                    if *(*F).ecode as u32 != OP_ALT {
                        break;
                    }
                }
                (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
                state = ST_TOP;
                continue 'sm;
            }
            if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
                RRETURN!(rrc);
            }
            (*F).ecode = (*F).ecode.add(GET!((*F).ecode, 1) as usize);
            if *(*F).ecode as u32 != OP_ALT {
                RRETURN!(MATCH_NOMATCH);
            }
            state = ST_C10_2;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Handle negative assertions. Loop for each non-matching branch as for
        positive assertions. */

        OP_ASSERT_NOT | OP_ASSERTBACK_NOT => {
            state = ST_C10_3;
            continue 'sm;
        }

        /* Top of the C for (;;) loop of the negative assertions. */
        ST_C10_3 => {
            group_frame_type = GF_NOCAPTURE;
            RMATCH!(
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize),
                RM4
            );
        }

        ST_L_RM4 => {
            if rrc == MATCH_ACCEPT || rrc == MATCH_MATCH {
                /* Assertion matched, therefore it fails. */
                RRETURN!(MATCH_NOMATCH);
            } else if rrc == MATCH_NOMATCH || rrc == MATCH_THEN {
                /* Branch failed, try next if present. */
                (*F).ecode = (*F).ecode.add(GET!((*F).ecode, 1) as usize);
                if *(*F).ecode as u32 != OP_ALT {
                    state = ST_ASSERT_NOT_FAILED;
                    continue 'sm;
                }
                /* The C `break` leaves the inner switch: repeat the for (;;) loop. */
                state = ST_C10_3;
                continue 'sm;
            } else if rrc == MATCH_COMMIT || rrc == MATCH_SKIP || rrc == MATCH_PRUNE {
                /* Assertion forced to fail, therefore continue. */
                loop {
                    (*F).ecode = (*F).ecode.add(GET!((*F).ecode, 1) as usize);
                    if *(*F).ecode as u32 != OP_ALT {
                        break;
                    }
                }
                state = ST_ASSERT_NOT_FAILED;
                continue 'sm;
            } else {
                /* Pass back any other return */
                RRETURN!(rrc);
            }
        }

        /* None of the branches have matched or there was a backtrack to (*COMMIT),
        (*SKIP), (*PRUNE), or (*THEN) in the last branch. This is success for a
        negative assertion, so carry on. */

        ST_ASSERT_NOT_FAILED => {
            (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Handle scan substring operation. */

        /* Lsaved_end_subject = (*F).fields.op_assert_scs.saved_end_subject
           Lsaved_eptr        = (*F).fields.op_assert_scs.saved_eptr
           Ltrue_end_extra    = (*F).fields.op_assert_scs.true_end_extra
           Lsaved_moptions    = (*F).fields.op_assert_scs.saved_moptions */

        OP_ASSERT_SCS => {
            length = 0;
            {
                let mut ecode: PCRE2_SPTR = (*F).ecode.add(1 + LINK_SIZE);

                /* Disable compiler warning. */
                offset = 0;

                /* The labelled loop is the C for (;;); `break 'scs_offset_found`
                is the C `goto SCS_OFFSET_FOUND` (the label sits immediately after
                this loop, which the C code never leaves any other way). */

                'scs_offset_found: loop {
                    if *ecode as u32 == OP_CREF {
                        length += 1 + IMM2_SIZE;
                        offset = (GET2!(ecode, 1) << 1).wrapping_sub(2) as PCRE2_SIZE;
                        ecode = ecode.add(1 + IMM2_SIZE);
                        if offset < (*F).offset_top && Fov!(offset) != PCRE2_UNSET {
                            break 'scs_offset_found;
                        }
                        continue 'scs_offset_found;
                    }

                    if *ecode as u32 != OP_DNCREF {
                        RRETURN!(MATCH_NOMATCH);
                    }

                    let mut count: c_int = GET2!(ecode, 1 + IMM2_SIZE) as c_int;
                    let mut slot: PCRE2_SPTR = (*mb).name_table.add(
                        GET2!(ecode, 1).wrapping_mul((*mb).name_entry_size as u32) as usize,
                    );
                    length += 1 + 2 * IMM2_SIZE;
                    ecode = ecode.add(1 + 2 * IMM2_SIZE);

                    while count > 0 {
                        offset = (GET2!(slot, 0) << 1).wrapping_sub(2) as PCRE2_SIZE;
                        if offset < (*F).offset_top && Fov!(offset) != PCRE2_UNSET {
                            break 'scs_offset_found;
                        }
                        slot = slot.add((*mb).name_entry_size as usize);
                        count -= 1;
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

            (*F).eptr = (*mb).start_subject.add(Fov!(offset));
            (*mb).end_subject = (*mb).start_subject.add(Fov!(offset + 1));
            (*mb).true_end_subject = (*mb).end_subject;
            (*mb).moptions &= !PCRE2_NOTEOL;

            /* Top of the C for (;;) loop. */
            group_frame_type = GF_NOCAPTURE;
            RMATCH!((*F).ecode.add(1 + LINK_SIZE + length), RM38);
        }

        ST_L_RM38 => {
            if rrc == MATCH_ACCEPT {
                memcpy(
                    (*F).ovector.as_mut_ptr() as *mut c_void,
                    (assert_accept_frame as *mut u8).add(offset_of!(heapframe, ovector))
                        as *const c_void,
                    (*assert_accept_frame).offset_top * size_of::<PCRE2_SIZE>(),
                );
                (*F).offset_top = (*assert_accept_frame).offset_top;
                (*F).mark = (*assert_accept_frame).mark;
                (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
                (*mb).true_end_subject = (*mb)
                    .end_subject
                    .add((*F).fields.op_assert_scs.true_end_extra);
                (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;

                /* The C `break` leaves the for (;;) loop and falls into the tail. */

                loop {
                    (*F).ecode = (*F).ecode.add(GET!((*F).ecode, 1) as usize);
                    if *(*F).ecode as u32 != OP_ALT {
                        break;
                    }
                }
                (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
                (*F).eptr = (*F).fields.op_assert_scs.saved_eptr;
                state = ST_TOP;
                continue 'sm;
            }

            if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
                (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
                (*mb).true_end_subject = (*mb)
                    .end_subject
                    .add((*F).fields.op_assert_scs.true_end_extra);
                (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
                RRETURN!(rrc);
            }

            (*F).ecode = (*F).ecode.add(GET!((*F).ecode, 1) as usize);
            if *(*F).ecode as u32 != OP_ALT {
                (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
                (*mb).true_end_subject = (*mb)
                    .end_subject
                    .add((*F).fields.op_assert_scs.true_end_extra);
                (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
                RRETURN!(MATCH_NOMATCH);
            }
            length = 0;

            /* Top of the C for (;;) loop. */
            group_frame_type = GF_NOCAPTURE;
            RMATCH!((*F).ecode.add(1 + LINK_SIZE + length), RM38);
        }

        _ => {}
    }
}
