{
/* ===================================================================== */
/* Opcode arms chunk "h": C lines 5989-6569 of pcre2_match.c.
   Contains OP_CALLOUT/OP_CALLOUT_STR (C 5989), OP_COND/OP_SCOND (C 6008),
   OP_REVERSE (C 6190), OP_VREVERSE (C 6233), OP_ALT (C 6292) and
   OP_KET/OP_KETRMIN/OP_KETRMAX/OP_KETRPOS (C 6304), plus the RMATCH
   continuations RM5, RM35, RM37, RM39, RM6, RM7.                        */
/* ===================================================================== */

if lbl == LBL_SWITCH {
    match (*F).op as u32 {

    /* ===================================================================== */
    /* The callout item calls an external function, if one is provided, passing
    details of the match so far. This is mainly for debugging, though the
    function is able to force a failure. */

    /* case OP_CALLOUT: (C 5989) / case OP_CALLOUT_STR: (C 5990) */
    OP_CALLOUT | OP_CALLOUT_STR => {
        rrc = do_callout(F, mb, &mut length);
        if rrc > 0 {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        if rrc < 0 {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).ecode = (*F).ecode.add(length);
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    /* ===================================================================== */
    /* Conditional group: compilation checked that there are no more than two
    branches. If the condition is false, skipping the first branch takes us
    past the end of the item if there is only one branch, but that's exactly
    what we want. */

    /* Lstart_branch -> F->fields.op_cond.start_branch
       Llength       -> F->fields.op_cond.length
       Lpositive     -> F->byte1                                           */

    /* case OP_COND: (C 6008) / case OP_SCOND: (C 6009) */
    OP_COND | OP_SCOND => {
        /* The variable Llength will be added to Fecode when the condition is
        false, to get to the second branch. Setting it to the offset to the ALT or
        KET, then incrementing Fecode achieves this effect. However, if the second
        branch is non-existent, we must point to the KET so that the end of the
        group is correctly processed. We now have Fecode pointing to the condition
        or callout. */

        (*F).fields.op_cond.length = GET((*F).ecode, 1) as PCRE2_SIZE; /* Offset to the second branch */
        if *(*F).ecode.add((*F).fields.op_cond.length) as u32 != OP_ALT {
            (*F).fields.op_cond.length =
                (*F).fields.op_cond.length.wrapping_sub(1 + LINK_SIZE);
        }
        (*F).ecode = (*F).ecode.add(1 + LINK_SIZE); /* From this opcode */

        /* Because of the way auto-callout works during compile, a callout item is
        inserted between OP_COND and an assertion condition. Such a callout can
        also be inserted manually. */

        if *(*F).ecode as u32 == OP_CALLOUT || *(*F).ecode as u32 == OP_CALLOUT_STR {
            rrc = do_callout(F, mb, &mut length);
            if rrc > 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            if rrc < 0 {
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }

            /* Advance Fecode past the callout, so it now points to the condition. We
            must adjust Llength so that the value of Fecode+Llength is unchanged. */

            (*F).ecode = (*F).ecode.add(length);
            (*F).fields.op_cond.length = (*F).fields.op_cond.length.wrapping_sub(length);
        }

        /* Test the various possible conditions */

        condition = FALSE;
        match *(*F).ecode as u32 {
            /* case OP_RREF: Group recursion test */
            OP_RREF => {
                if (*F).current_recurse != RECURSE_UNSET {
                    number = GET2((*F).ecode, 1);
                    condition =
                        ((number == RREF_ANY || number == (*F).current_recurse) as BOOL);
                }
            }

            /* case OP_DNRREF: Duplicate named group recursion test */
            OP_DNRREF => {
                if (*F).current_recurse != RECURSE_UNSET {
                    let mut count: c_int = GET2((*F).ecode, 1 + IMM2_SIZE) as c_int;
                    let mut slot: PCRE2_SPTR = (*mb).name_table.add(
                        (GET2((*F).ecode, 1) * (*mb).name_entry_size as u32) as usize,
                    );
                    loop {
                        let c0: c_int = count;
                        count -= 1;
                        if !(c0 > 0) {
                            break;
                        }
                        number = GET2(slot, 0);
                        condition = ((number == (*F).current_recurse) as BOOL);
                        if condition != FALSE {
                            break;
                        }
                        slot = slot.add((*mb).name_entry_size as usize);
                    }
                }
            }

            /* case OP_CREF: Numbered group used test */
            OP_CREF => {
                offset = ((GET2((*F).ecode, 1) << 1).wrapping_sub(2)) as PCRE2_SIZE; /* Doubled ref number */
                condition = ((offset < (*F).offset_top
                    && *ovec(F).add(offset) != PCRE2_UNSET) as BOOL);
            }

            /* case OP_DNCREF: Duplicate named group used test */
            OP_DNCREF => {
                let mut count: c_int = GET2((*F).ecode, 1 + IMM2_SIZE) as c_int;
                let mut slot: PCRE2_SPTR = (*mb).name_table.add(
                    (GET2((*F).ecode, 1) * (*mb).name_entry_size as u32) as usize,
                );
                loop {
                    let c0: c_int = count;
                    count -= 1;
                    if !(c0 > 0) {
                        break;
                    }
                    offset = ((GET2(slot, 0) << 1).wrapping_sub(2)) as PCRE2_SIZE;
                    condition = ((offset < (*F).offset_top
                        && *ovec(F).add(offset) != PCRE2_UNSET) as BOOL);
                    if condition != FALSE {
                        break;
                    }
                    slot = slot.add((*mb).name_entry_size as usize);
                }
            }

            /* case OP_FALSE: / case OP_FAIL: The assertion (?!) becomes OP_FAIL */
            OP_FALSE | OP_FAIL => {}

            OP_TRUE => {
                condition = TRUE;
            }

            /* The condition is an assertion. Run code similar to the assertion code
            above. */

            _ => {
                (*F).byte1 = ((*(*F).ecode as u32 == OP_ASSERT
                    || *(*F).ecode as u32 == OP_ASSERTBACK) as u8);
                (*F).fields.op_cond.start_branch = (*F).ecode;

                /* for (;;) { ... RMATCH(..., RM5); ... } */
                group_frame_type = GF_CONDASSERT;
                start_ecode = (*F).fields.op_cond.start_branch.add(
                    _pcre2_OP_lengths_8[*(*F).fields.op_cond.start_branch as usize] as usize,
                );
                (*F).return_id = RM5;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            }
        }

        /* Choose branch according to the condition. */

        (*F).ecode = (*F).ecode.add(if condition != FALSE {
            _pcre2_OP_lengths_8[*(*F).ecode as usize] as usize
        } else {
            (*F).fields.op_cond.length
        });

        /* If the opcode is OP_SCOND it means we are at a repeated conditional
        group that might match an empty string. We must therefore descend a level
        so that the start is remembered for checking. For OP_COND we can just
        continue at this level. */

        if (*F).op as u32 == OP_SCOND {
            group_frame_type = GF_NOCAPTURE;
            start_ecode = (*F).ecode;
            (*F).return_id = RM35;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    /* ========================================================================= */
    /*                  End of start of parenthesis opcodes                      */
    /* ========================================================================= */

    /* ===================================================================== */
    /* Move the subject pointer back by one fixed amount. This occurs at the
    start of each branch that has a fixed length in a lookbehind assertion. If
    we are too close to the start to move back, fail. When working with UTF-8
    we move back a number of characters, not bytes. */

    /* case OP_REVERSE: (C 6190) */
    OP_REVERSE => {
        number = GET2((*F).ecode, 1);
        if utf != FALSE {
            /* We used to do a simpler `while (number-- > 0)` but that triggers
            clang's unsigned integer overflow sanitizer. */
            while number > 0 {
                number -= 1;
                if (*F).eptr <= (*mb).check_subject {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                (*F).eptr = (*F).eptr.sub(1);
                /* BACKCHAR(Feptr) */
                while (*(*F).eptr & 0xc0) == 0x80 {
                    (*F).eptr = (*F).eptr.sub(1);
                }
            }
        } else
        /* No UTF support, or not in UTF mode: count is code unit count */
        {
            if (number as isize) > (*F).eptr.offset_from((*mb).start_subject) {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).eptr = (*F).eptr.sub(number as usize);
        }

        /* Save the earliest consulted character, then skip to next opcode */

        if (*F).eptr < (*mb).start_used_ptr {
            (*mb).start_used_ptr = (*F).eptr;
        }
        (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    /* ===================================================================== */
    /* Move the subject pointer back by a variable amount. This occurs at the
    start of each branch of a lookbehind assertion when the branch has a
    variable, but limited, length. A loop is needed to try matching the branch
    after moving back different numbers of characters. If we are too close to
    the start to move back even the minimum amount, fail. When working with
    UTF-8 we move back a number of characters, not bytes. */

    /* Lmin -> F->fields.op_vreverse.min
       Lmax -> F->fields.op_vreverse.max                                    */

    /* case OP_VREVERSE: (C 6233) */
    OP_VREVERSE => {
        (*F).fields.op_vreverse.min = GET2((*F).ecode, 1);
        (*F).fields.op_vreverse.max = GET2((*F).ecode, 1 + IMM2_SIZE);

        /* Move back by the maximum branch length and then work forwards. This
        ensures that items such as \d{3,5} get the maximum length, which is
        relevant for captures, and makes for Perl compatibility. */

        if utf != FALSE {
            i = 0;
            while i < (*F).fields.op_vreverse.max {
                if (*F).eptr == (*mb).start_subject {
                    if i < (*F).fields.op_vreverse.min {
                        rrc = MATCH_NOMATCH;
                        lbl = LBL_RETURN_SWITCH;
                        continue 'sw;
                    }
                    (*F).fields.op_vreverse.max = i;
                    break;
                }
                (*F).eptr = (*F).eptr.sub(1);
                /* BACKCHAR(Feptr) */
                while (*(*F).eptr & 0xc0) == 0x80 {
                    (*F).eptr = (*F).eptr.sub(1);
                }
                i = i.wrapping_add(1);
            }
        } else
        /* No UTF support or not in UTF mode */
        {
            let diff: isize = (*F).eptr.offset_from((*mb).start_subject);
            let available: u32 = if diff > 65535 {
                65535
            } else if diff > 0 {
                diff as c_int as u32
            } else {
                0
            };
            if (*F).fields.op_vreverse.min > available {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            if (*F).fields.op_vreverse.max > available {
                (*F).fields.op_vreverse.max = available;
            }
            (*F).eptr = (*F).eptr.sub((*F).fields.op_vreverse.max as usize);
        }

        /* Now try matching, moving forward one character on failure, until we
        reach the minimum back length. */

        /* for (;;) { RMATCH(Fecode + 1 + 2 * IMM2_SIZE, RM37); ... } */
        start_ecode = (*F).ecode.add(1 + 2 * IMM2_SIZE);
        (*F).return_id = RM37;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }

    /* ===================================================================== */
    /* An alternation is the end of a branch; scan along to find the end of the
    bracketed group. */

    /* case OP_ALT: (C 6292) */
    OP_ALT => {
        branch_end = (*F).ecode;
        loop {
            (*F).ecode = (*F).ecode.add(GET((*F).ecode, 1) as usize);
            if *(*F).ecode as u32 != OP_ALT {
                break;
            }
        }
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    /* ===================================================================== */
    /* The end of a parenthesized group. For all but OP_BRA and OP_COND, the
    starting frame was added to the chained frames in order to remember the
    starting subject position for the group. (Not true for OP_BRA when it's a
    whole pattern recursion, but that is handled separately below.)*/

    /* case OP_KET: (C 6304) / case OP_KETRMIN: / case OP_KETRMAX:
       case OP_KETRPOS: */
    OP_KET | OP_KETRMIN | OP_KETRMAX | OP_KETRPOS => {
        bracode = (*F).ecode.sub(GET((*F).ecode, 1) as usize);

        if branch_end.is_null() {
            branch_end = (*F).ecode;
        }
        branch_start = bracode;
        while branch_start.add(GET(branch_start, 1) as usize) != branch_end {
            branch_start = branch_start.add(GET(branch_start, 1) as usize);
        }
        branch_end = core::ptr::null();

        /* Point N to the frame at the start of the most recent group, and P to its
        predecessor. Remember the subject pointer at the start of the group. */

        if *bracode as u32 != OP_BRA && *bracode as u32 != OP_COND {
            N = frame_at((*match_data).heapframes, (*F).last_group_offset);
            P = frame_sub(N, frame_size);
            (*F).last_group_offset = (*P).last_group_offset;

            /* If we are at the end of an assertion that is a condition, first check
            to see if we are at the end of a variable-length branch in a lookbehind.
            If this is the case and we have not landed on the current character,
            return no match. Compare code below for non-condition lookbehinds. In
            other cases, return a match, discarding any intermediate backtracking
            points. Copy back the mark setting and the captures into the frame before
            N so that they are set on return. Doing this for all assertions, both
            positive and negative, seems to match what Perl does. */

            if (*N).group_frame_type == GF_CONDASSERT {
                if (*bracode as u32 == OP_ASSERTBACK
                    || *bracode as u32 == OP_ASSERTBACK_NOT)
                    && *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                    && (*F).eptr != (*P).eptr
                {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                memcpy(
                    (P as *mut u8).add(core::mem::offset_of!(heapframe, ovector))
                        as *mut c_void,
                    ovec(F) as *const c_void,
                    (*F).offset_top * core::mem::size_of::<PCRE2_SIZE>(),
                );
                (*P).offset_top = (*F).offset_top;
                (*P).mark = (*F).mark;
                (*F).back_frame = (F as *mut u8).offset_from(P as *mut u8) as PCRE2_SIZE;
                rrc = MATCH_MATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        } else {
            P = core::ptr::null_mut(); /* Indicates starting frame not recorded */
        }

        /* The group was not a conditional assertion. */

        match *bracode as u32 {
            /* Whole pattern recursion is handled as a recursion into group 0, but
            the entire pattern is wrapped in OP_BRA/OP_KET rather than a capturing
            group - a design mistake: it should perhaps have been capture group 0.
            Anyway, that means the end of such recursion must be handled here. It is
            detected by checking for an immediately following OP_END when we are
            recursing in group 0. If this is not the end of a whole-pattern
            recursion, there is nothing to be done. */

            OP_BRA => {
                if !((*F).current_recurse != 0
                    || *(*F).ecode.add(1 + LINK_SIZE) as u32 != OP_END)
                {
                    /* It is the end of whole-pattern recursion. */

                    offset = (*F).last_group_offset;

                    /* Corrupted heapframes?. Trigger an assert and return an error */
                    /* PCRE2_ASSERT(offset != PCRE2_UNSET); */
                    if offset == PCRE2_UNSET {
                        return PCRE2_ERROR_INTERNAL;
                    }

                    N = frame_at((*match_data).heapframes, offset);
                    P = frame_sub(N, frame_size);
                    (*F).last_group_offset = (*P).last_group_offset;

                    /* Reinstate the previous set of captures and then carry on after the
                    recursion call. */

                    (*F).ecode = (*P).ecode.add(1 + LINK_SIZE);

                    if *(*F).ecode as u32 != OP_CREF {
                        memcpy(
                            ovec(F) as *mut c_void,
                            ovec(P) as *const c_void,
                            (*F).offset_top * core::mem::size_of::<PCRE2_SIZE>(),
                        );
                        (*F).offset_top = (*P).offset_top;
                    } else {
                        recurse_update_offsets(F, P);
                    }

                    (*F).capture_last = (*P).capture_last;
                    (*F).current_recurse = (*P).current_recurse;
                    lbl = LBL_TOP_OF_LOOP;
                    continue 'sw; /* With next opcode */
                }
            }

            /* case OP_COND: / case OP_SCOND: No need to do anything for these */
            OP_COND | OP_SCOND => {}

            /* Non-atomic positive assertions are like OP_BRA, except that the
            subject pointer must be put back to where it was at the start of the
            assertion. For a variable lookbehind, check its end point. */

            /* case OP_ASSERTBACK_NA: falls through into case OP_ASSERT_NA. */
            OP_ASSERTBACK_NA | OP_ASSERT_NA => {
                if *bracode as u32 == OP_ASSERTBACK_NA {
                    if *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                        && (*F).eptr != (*P).eptr
                    {
                        rrc = MATCH_NOMATCH;
                        lbl = LBL_RETURN_SWITCH;
                        continue 'sw;
                    }
                }
                if (*F).eptr > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = (*F).eptr;
                }
                (*F).eptr = (*P).eptr;
            }

            /* Atomic positive assertions are like OP_ONCE, except that in addition
            the subject pointer must be put back to where it was at the start of the
            assertion. For a variable lookbehind, check its end point. */

            /* case OP_ASSERTBACK: falls through into case OP_ASSERT:, which falls
            through into case OP_ONCE:. */
            OP_ASSERTBACK | OP_ASSERT | OP_ONCE => {
                if *bracode as u32 == OP_ASSERTBACK {
                    if *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                        && (*F).eptr != (*P).eptr
                    {
                        rrc = MATCH_NOMATCH;
                        lbl = LBL_RETURN_SWITCH;
                        continue 'sw;
                    }
                }
                if *bracode as u32 != OP_ONCE {
                    if (*F).eptr > (*mb).last_used_ptr {
                        (*mb).last_used_ptr = (*F).eptr;
                    }
                    (*F).eptr = (*P).eptr;
                }

                /* For an atomic group, discard internal backtracking points. We must
                also ensure that any remaining branches within the top-level of the group
                are not tried. Do this by adjusting the code pointer within the backtrack
                frame so that it points to the final branch. */

                (*F).back_frame = (F as *mut u8).offset_from(P as *mut u8) as PCRE2_SIZE;
                loop {
                    let y: u32 = GET((*P).ecode, 1);
                    if *(*P).ecode.add(y as usize) as u32 != OP_ALT {
                        break;
                    }
                    (*P).ecode = (*P).ecode.add(y as usize);
                }
            }

            /* A matching negative assertion returns MATCH, which is turned into
            NOMATCH at the assertion level. For a variable lookbehind, check its end
            point. */

            /* case OP_ASSERTBACK_NOT: falls through into case OP_ASSERT_NOT. */
            OP_ASSERTBACK_NOT | OP_ASSERT_NOT => {
                if *bracode as u32 == OP_ASSERTBACK_NOT {
                    if *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                        && (*F).eptr != (*P).eptr
                    {
                        rrc = MATCH_NOMATCH;
                        lbl = LBL_RETURN_SWITCH;
                        continue 'sw;
                    }
                }
                rrc = MATCH_MATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }

            /* A scan substring group must preserve the current end_subject,
            and restore it before the backtracking is performed into its sub
            pattern. */

            OP_ASSERT_SCS => {
                (*F).fields.op_assert_scs.saved_end_subject = (*mb).end_subject;
                (*mb).end_subject = (*P).fields.op_assert_scs.saved_end_subject;
                (*mb).true_end_subject = (*mb)
                    .end_subject
                    .add((*P).fields.op_assert_scs.true_end_extra);
                (*F).eptr = (*P).fields.op_assert_scs.saved_eptr;

                start_ecode = (*F).ecode.add(1 + LINK_SIZE);
                (*F).return_id = RM39;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            }

            /* At the end of a script run, apply the script-checking rules. This code
            will never by exercised if Unicode support it not compiled, because in
            that environment script runs cause an error at compile time. */

            OP_SCRIPT_RUN => {
                if crate::script_run::_pcre2_script_run_8((*P).eptr, (*F).eptr, utf) == FALSE
                {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            }

            /* Whole-pattern recursion is coded as a recurse into group 0, and is
            handled with OP_BRA above. Other recursion is handled here. */

            OP_CBRA | OP_CBRAPOS | OP_SCBRA | OP_SCBRAPOS => {
                number = GET2(bracode, 1 + LINK_SIZE);

                /* Handle a recursively called group. We reinstate the previous set of
                captures and then carry on after the recursion call. */

                if (*F).current_recurse == number {
                    P = frame_sub(N, frame_size);
                    (*F).ecode = (*P).ecode.add(1 + LINK_SIZE);

                    if *(*F).ecode as u32 != OP_CREF {
                        memcpy(
                            ovec(F) as *mut c_void,
                            ovec(P) as *const c_void,
                            (*F).offset_top * core::mem::size_of::<PCRE2_SIZE>(),
                        );
                        (*F).offset_top = (*P).offset_top;
                    } else {
                        recurse_update_offsets(F, P);
                    }

                    (*F).capture_last = (*P).capture_last;
                    (*F).current_recurse = (*P).current_recurse;
                    lbl = LBL_TOP_OF_LOOP;
                    continue 'sw; /* With next opcode */
                }

                /* Deal with actual capturing. */

                offset = ((number << 1).wrapping_sub(2)) as PCRE2_SIZE;
                (*F).capture_last = number;
                *ovec(F).add(offset) =
                    (*P).eptr.offset_from((*mb).start_subject) as PCRE2_SIZE;
                *ovec(F).add(offset + 1) =
                    (*F).eptr.offset_from((*mb).start_subject) as PCRE2_SIZE;
                if offset >= (*F).offset_top {
                    (*F).offset_top = offset + 2;
                }
            }

            _ => {}
        } /* End actions relating to the starting opcode */

        /* OP_KETRPOS is a possessive repeating ket. Remember the current position,
        and return the MATCH_KETRPOS. This makes it possible to do the repeats one
        at a time from the outer level. This must precede the empty string test -
        in this case that test is done at the outer level. */

        if *(*F).ecode as u32 == OP_KETRPOS {
            memcpy(
                (P as *mut u8).add(core::mem::offset_of!(heapframe, eptr)) as *mut c_void,
                (F as *mut u8).add(core::mem::offset_of!(heapframe, eptr)) as *const c_void,
                frame_copy_size,
            );
            rrc = MATCH_KETRPOS;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }

        /* Handle the different kinds of closing brackets. A non-repeating ket
        needs no special action, just continuing at this level. This also happens
        for the repeating kets if the group matched no characters, in order to
        forcibly break infinite loops. Otherwise, the repeating kets try the rest
        of the pattern or restart from the preceding bracket, in the appropriate
        order. */

        if (*F).op as u32 != OP_KET && (P.is_null() || (*F).eptr != (*P).eptr) {
            if (*F).op as u32 == OP_KETRMIN {
                start_ecode = (*F).ecode.add(1 + LINK_SIZE);
                (*F).return_id = RM6;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            }

            /* Repeat the maximum number of times (KETRMAX) */

            start_ecode = bracode;
            (*F).return_id = RM7;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        /* Carry on at this level for a non-repeating ket, or after matching an
        empty string, or after repeating for a maximum number of times. */

        (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    _ => {}
    }
}

/* ---- RMATCH continuations owned by this chunk ---- */

if lbl == LBL_RM_BASE + RM5 as u32 {
    /* After RMATCH in the assertion condition of case OP_COND/OP_SCOND (C 6105).
    We are inside the `for (;;)` branch loop. */

    match rrc {
        /* case MATCH_ACCEPT: Save captures */
        MATCH_ACCEPT => {
            memcpy(
                ovec(F) as *mut c_void,
                (assert_accept_frame as *mut u8)
                    .add(core::mem::offset_of!(heapframe, ovector)) as *const c_void,
                (*assert_accept_frame).offset_top * core::mem::size_of::<PCRE2_SIZE>(),
            );
            (*F).offset_top = (*assert_accept_frame).offset_top;

            /* Fall through into case MATCH_MATCH. In the case of a match, the
            captures have already been put into the current frame. */

            condition = (*F).byte1 as BOOL; /* TRUE for positive assertion */
        }

        MATCH_MATCH => {
            condition = (*F).byte1 as BOOL; /* TRUE for positive assertion */
        }

        /* PCRE doesn't allow the effect of (*THEN) to escape beyond an
        assertion; it is therefore always treated as NOMATCH. */

        MATCH_NOMATCH | MATCH_THEN => {
            (*F).fields.op_cond.start_branch = (*F)
                .fields
                .op_cond
                .start_branch
                .add(GET((*F).fields.op_cond.start_branch, 1) as usize);
            if *(*F).fields.op_cond.start_branch as u32 == OP_ALT {
                /* continue: try next branch */
                group_frame_type = GF_CONDASSERT;
                start_ecode = (*F).fields.op_cond.start_branch.add(
                    _pcre2_OP_lengths_8[*(*F).fields.op_cond.start_branch as usize] as usize,
                );
                (*F).return_id = RM5;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            }
            condition = ((*F).byte1 == 0) as BOOL; /* TRUE for negative assertion */
        }

        /* These force no match without checking other branches. */

        MATCH_COMMIT | MATCH_SKIP | MATCH_PRUNE => {
            condition = ((*F).byte1 == 0) as BOOL;
        }

        _ => {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
    }
    /* break: out of the branch loop */

    /* If the condition is true, find the end of the assertion so that
    advancing past it gets us to the start of the first branch. */

    if condition != FALSE {
        loop {
            (*F).ecode = (*F).ecode.add(GET((*F).ecode, 1) as usize);
            if *(*F).ecode as u32 != OP_ALT {
                break;
            }
        }
    }
    /* break: End of assertion condition */

    /* Choose branch according to the condition. */

    (*F).ecode = (*F).ecode.add(if condition != FALSE {
        _pcre2_OP_lengths_8[*(*F).ecode as usize] as usize
    } else {
        (*F).fields.op_cond.length
    });

    /* If the opcode is OP_SCOND it means we are at a repeated conditional
    group that might match an empty string. We must therefore descend a level
    so that the start is remembered for checking. For OP_COND we can just
    continue at this level. */

    if (*F).op as u32 == OP_SCOND {
        group_frame_type = GF_NOCAPTURE;
        start_ecode = (*F).ecode;
        (*F).return_id = RM35;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    lbl = LBL_TOP_OF_LOOP;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM35 as u32 {
    /* After RMATCH in case OP_SCOND (C 6169) */
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM37 as u32 {
    /* After RMATCH inside the `for (;;)` loop of case OP_VREVERSE (C 6274) */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    {
        /* if (Lmax-- <= Lmin) RRETURN(MATCH_NOMATCH); */
        let old_max: u32 = (*F).fields.op_vreverse.max;
        (*F).fields.op_vreverse.max = old_max.wrapping_sub(1);
        if old_max <= (*F).fields.op_vreverse.min {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
    }
    (*F).eptr = (*F).eptr.add(1);
    if utf != FALSE {
        /* FORWARDCHARTEST(Feptr, mb->end_subject) */
        while (*F).eptr < (*mb).end_subject && (*(*F).eptr & 0xc0) == 0x80 {
            (*F).eptr = (*F).eptr.add(1);
        }
    }
    /* Round the `for (;;)` loop again. */
    start_ecode = (*F).ecode.add(1 + 2 * IMM2_SIZE);
    (*F).return_id = RM37;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM39 as u32 {
    /* After RMATCH in case OP_ASSERT_SCS of the OP_KET group switch (C 6469) */
    (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
    (*mb).true_end_subject = (*mb).end_subject;
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM6 as u32 {
    /* After RMATCH for OP_KETRMIN (C 6548) */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    (*F).ecode = (*F).ecode.sub(GET((*F).ecode, 1) as usize);
    lbl = LBL_TOP_OF_LOOP;
    continue 'sw; /* End of ket processing */
}

if lbl == LBL_RM_BASE + RM7 as u32 {
    /* After RMATCH for OP_KETRMAX (C 6556) */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }

    /* Carry on at this level for a non-repeating ket, or after matching an
    empty string, or after repeating for a maximum number of times. */

    (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
    lbl = LBL_TOP_OF_LOOP;
    continue 'sw;
}
}
