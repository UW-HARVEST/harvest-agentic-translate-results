/* Fragment 11 of the match() opcode switch: c_src/src/pcre2_match.c lines
5989..6291 - OP_CALLOUT / OP_CALLOUT_STR, OP_COND / OP_SCOND, OP_REVERSE and
OP_VREVERSE. */
{
    match state {
        /* ===================================================================== */
        /* The callout item calls an external function, if one is provided, passing
        details of the match so far. This is mainly for debugging, though the
        function is able to force a failure. */
        OP_CALLOUT | OP_CALLOUT_STR => {
            /* `length` is an uninitialized local in C; Rust requires it to be
            initialized before a mutable reference can be taken. do_callout()
            unconditionally writes through the pointer before anything else, so
            this extra store is not observable. */
            length = 0;
            rrc = do_callout(F, mb, &mut length);
            if rrc > 0 {
                RRETURN!(MATCH_NOMATCH);
            }
            if rrc < 0 {
                RRETURN!(rrc);
            }
            (*F).ecode = (*F).ecode.add(length);
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Conditional group: compilation checked that there are no more than two
        branches. If the condition is false, skipping the first branch takes us
        past the end of the item if there is only one branch, but that's exactly
        what we want. */

        /* Lstart_branch = F->fields.op_cond.start_branch
           Llength       = F->fields.op_cond.length
           Lpositive     = F->byte1 */
        OP_COND | OP_SCOND => {
            /* The variable Llength will be added to Fecode when the condition is
            false, to get to the second branch. Setting it to the offset to the ALT
            or KET, then incrementing Fecode achieves this effect. However, if the
            second branch is non-existent, we must point to the KET so that the end
            of the group is correctly processed. We now have Fecode pointing to the
            condition or callout. */

            (*F).fields.op_cond.length = GET!((*F).ecode, 1) as PCRE2_SIZE; /* Offset to the second branch */
            if *(*F).ecode.add((*F).fields.op_cond.length) as u32 != OP_ALT {
                (*F).fields.op_cond.length =
                    (*F).fields.op_cond.length.wrapping_sub(1 + LINK_SIZE);
            }
            (*F).ecode = (*F).ecode.add(1 + LINK_SIZE); /* From this opcode */

            /* Because of the way auto-callout works during compile, a callout item
            is inserted between OP_COND and an assertion condition. Such a callout
            can also be inserted manually. */

            if *(*F).ecode as u32 == OP_CALLOUT || *(*F).ecode as u32 == OP_CALLOUT_STR {
                length = 0; /* see the note in OP_CALLOUT above */
                rrc = do_callout(F, mb, &mut length);
                if rrc > 0 {
                    RRETURN!(MATCH_NOMATCH);
                }
                if rrc < 0 {
                    RRETURN!(rrc);
                }

                /* Advance Fecode past the callout, so it now points to the
                condition. We must adjust Llength so that the value of
                Fecode+Llength is unchanged. */

                (*F).ecode = (*F).ecode.add(length);
                (*F).fields.op_cond.length = (*F).fields.op_cond.length.wrapping_sub(length);
            }

            /* Test the various possible conditions */

            condition = FALSE;
            match *(*F).ecode as u32 {
                OP_RREF => {
                    /* Group recursion test */
                    if (*F).current_recurse != RECURSE_UNSET {
                        number = GET2!((*F).ecode, 1);
                        condition =
                            (number == RREF_ANY || number == (*F).current_recurse) as BOOL;
                    }
                }

                OP_DNRREF => {
                    /* Duplicate named group recursion test */
                    if (*F).current_recurse != RECURSE_UNSET {
                        let mut count: c_int = GET2!((*F).ecode, 1 + IMM2_SIZE) as c_int;
                        let mut slot: PCRE2_SPTR = (*mb).name_table.add(
                            GET2!((*F).ecode, 1)
                                .wrapping_mul((*mb).name_entry_size as u32)
                                as usize,
                        );
                        loop {
                            let count_old = count;
                            count = count.wrapping_sub(1);
                            if !(count_old > 0) {
                                break;
                            }
                            number = GET2!(slot, 0);
                            condition = (number == (*F).current_recurse) as BOOL;
                            if condition != 0 {
                                break;
                            }
                            slot = slot.add((*mb).name_entry_size as usize);
                        }
                    }
                }

                OP_CREF => {
                    /* Numbered group used test */
                    offset = (GET2!((*F).ecode, 1) << 1).wrapping_sub(2) as PCRE2_SIZE; /* Doubled ref number */
                    condition =
                        (offset < (*F).offset_top && Fov!(offset) != PCRE2_UNSET) as BOOL;
                }

                OP_DNCREF => {
                    /* Duplicate named group used test */
                    let mut count: c_int = GET2!((*F).ecode, 1 + IMM2_SIZE) as c_int;
                    let mut slot: PCRE2_SPTR = (*mb).name_table.add(
                        GET2!((*F).ecode, 1)
                            .wrapping_mul((*mb).name_entry_size as u32) as usize,
                    );
                    loop {
                        let count_old = count;
                        count = count.wrapping_sub(1);
                        if !(count_old > 0) {
                            break;
                        }
                        offset = (GET2!(slot, 0) << 1).wrapping_sub(2) as PCRE2_SIZE;
                        condition =
                            (offset < (*F).offset_top && Fov!(offset) != PCRE2_UNSET) as BOOL;
                        if condition != 0 {
                            break;
                        }
                        slot = slot.add((*mb).name_entry_size as usize);
                    }
                }

                OP_FALSE | OP_FAIL => {
                    /* The assertion (?!) becomes OP_FAIL */
                }

                OP_TRUE => {
                    condition = TRUE;
                }

                /* The condition is an assertion. Run code similar to the assertion
                code above. */
                _ => {
                    (*F).byte1 = (*(*F).ecode as u32 == OP_ASSERT
                        || *(*F).ecode as u32 == OP_ASSERTBACK) as u8;
                    (*F).fields.op_cond.start_branch = (*F).ecode;

                    /* for (;;) - the branch loop; its body contains an RMATCH so it
                    becomes an explicit state. */
                    state = ST_C11_1;
                    continue 'sm;
                }
            }

            /* Choose branch according to the condition. */

            (*F).ecode = if condition != 0 {
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize)
            } else {
                (*F).ecode.add((*F).fields.op_cond.length)
            };

            /* If the opcode is OP_SCOND it means we are at a repeated conditional
            group that might match an empty string. We must therefore descend a
            level so that the start is remembered for checking. For OP_COND we can
            just continue at this level. */

            if (*F).op as u32 == OP_SCOND {
                group_frame_type = GF_NOCAPTURE;
                RMATCH!((*F).ecode, RM35);
            }
            state = ST_TOP;
            continue 'sm;
        }

        /* Top of the assertion-condition branch loop: for (;;) { ... } */
        ST_C11_1 => {
            group_frame_type = GF_CONDASSERT;
            RMATCH!(
                (*F).fields.op_cond.start_branch.add(
                    *_pcre2_OP_lengths_8
                        .as_ptr()
                        .add(*(*F).fields.op_cond.start_branch as usize) as usize
                ),
                RM5
            );
        }

        ST_L_RM5 => {
            if rrc == MATCH_ACCEPT {
                /* Save captures */
                memcpy(
                    (*F).ovector.as_mut_ptr() as *mut c_void,
                    (assert_accept_frame as *mut u8).add(offset_of!(heapframe, ovector))
                        as *const c_void,
                    (*assert_accept_frame).offset_top * size_of::<PCRE2_SIZE>(),
                );
                (*F).offset_top = (*assert_accept_frame).offset_top;

                /* Fall through */
                /* In the case of a match, the captures have already been put into
                the current frame. */

                condition = (*F).byte1 as BOOL; /* TRUE for positive assertion */
            } else if rrc == MATCH_MATCH {
                condition = (*F).byte1 as BOOL; /* TRUE for positive assertion */
            }
            /* PCRE doesn't allow the effect of (*THEN) to escape beyond an
            assertion; it is therefore always treated as NOMATCH. */
            else if rrc == MATCH_NOMATCH || rrc == MATCH_THEN {
                (*F).fields.op_cond.start_branch = (*F)
                    .fields
                    .op_cond
                    .start_branch
                    .add(GET!((*F).fields.op_cond.start_branch, 1) as usize);
                if *(*F).fields.op_cond.start_branch as u32 == OP_ALT {
                    state = ST_C11_1;
                    continue 'sm; /* Try next branch */
                }
                condition = ((*F).byte1 == 0) as BOOL; /* TRUE for negative assertion */
            }
            /* These force no match without checking other branches. */
            else if rrc == MATCH_COMMIT || rrc == MATCH_SKIP || rrc == MATCH_PRUNE {
                condition = ((*F).byte1 == 0) as BOOL;
            } else {
                RRETURN!(rrc);
            }

            /* Out of the branch loop */

            /* If the condition is true, find the end of the assertion so that
            advancing past it gets us to the start of the first branch. */

            if condition != 0 {
                loop {
                    (*F).ecode = (*F).ecode.add(GET!((*F).ecode, 1) as usize);
                    if *(*F).ecode as u32 != OP_ALT {
                        break;
                    }
                }
            }

            /* End of assertion condition; this is the code that follows the
            switch on the condition opcode (duplicated here because `condition`
            is a plain local that cannot be carried across a state transition in
            Rust). */

            /* Choose branch according to the condition. */

            (*F).ecode = if condition != 0 {
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize)
            } else {
                (*F).ecode.add((*F).fields.op_cond.length)
            };

            /* If the opcode is OP_SCOND it means we are at a repeated conditional
            group that might match an empty string. We must therefore descend a
            level so that the start is remembered for checking. For OP_COND we can
            just continue at this level. */

            if (*F).op as u32 == OP_SCOND {
                group_frame_type = GF_NOCAPTURE;
                RMATCH!((*F).ecode, RM35);
            }
            state = ST_TOP;
            continue 'sm;
        }

        ST_L_RM35 => {
            RRETURN!(rrc);
        }

        /* ========================================================================= */
        /*                  End of start of parenthesis opcodes                      */
        /* ========================================================================= */

        /* ===================================================================== */
        /* Move the subject pointer back by one fixed amount. This occurs at the
        start of each branch that has a fixed length in a lookbehind assertion. If
        we are too close to the start to move back, fail. When working with UTF-8
        we move back a number of characters, not bytes. */
        OP_REVERSE => {
            number = GET2!((*F).ecode, 1);
            if utf != 0 {
                /* We used to do a simpler `while (number-- > 0)` but that triggers
                clang's unsigned integer overflow sanitizer. */
                while number > 0 {
                    number = number.wrapping_sub(1);
                    if (*F).eptr <= (*mb).check_subject {
                        RRETURN!(MATCH_NOMATCH);
                    }
                    (*F).eptr = (*F).eptr.sub(1);
                    BACKCHAR!((*F).eptr);
                }
            } else {
                /* No UTF support, or not in UTF mode: count is code unit count */

                if number as isize > (*F).eptr.offset_from((*mb).start_subject) {
                    RRETURN!(MATCH_NOMATCH);
                }
                (*F).eptr = (*F).eptr.sub(number as usize);
            }

            /* Save the earliest consulted character, then skip to next opcode */

            if (*F).eptr < (*mb).start_used_ptr {
                (*mb).start_used_ptr = (*F).eptr;
            }
            (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Move the subject pointer back by a variable amount. This occurs at the
        start of each branch of a lookbehind assertion when the branch has a
        variable, but limited, length. A loop is needed to try matching the branch
        after moving back different numbers of characters. If we are too close to
        the start to move back even the minimum amount, fail. When working with
        UTF-8 we move back a number of characters, not bytes. */

        /* Lmin = F->fields.op_vreverse.min, Lmax = F->fields.op_vreverse.max */
        OP_VREVERSE => {
            (*F).fields.op_vreverse.min = GET2!((*F).ecode, 1);
            (*F).fields.op_vreverse.max = GET2!((*F).ecode, 1 + IMM2_SIZE);

            /* Move back by the maximum branch length and then work forwards. This
            ensures that items such as \d{3,5} get the maximum length, which is
            relevant for captures, and makes for Perl compatibility. */

            if utf != 0 {
                i = 0;
                while i < (*F).fields.op_vreverse.max {
                    if (*F).eptr == (*mb).start_subject {
                        if i < (*F).fields.op_vreverse.min {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        (*F).fields.op_vreverse.max = i;
                        break;
                    }
                    (*F).eptr = (*F).eptr.sub(1);
                    BACKCHAR!((*F).eptr);
                    i = i.wrapping_add(1);
                }
            } else {
                /* No UTF support or not in UTF mode */

                let diff: isize = (*F).eptr.offset_from((*mb).start_subject);
                let available: u32 = if diff > 65535 {
                    65535
                } else if diff > 0 {
                    diff as c_int as u32
                } else {
                    0
                };
                if (*F).fields.op_vreverse.min > available {
                    RRETURN!(MATCH_NOMATCH);
                }
                if (*F).fields.op_vreverse.max > available {
                    (*F).fields.op_vreverse.max = available;
                }
                (*F).eptr = (*F).eptr.sub((*F).fields.op_vreverse.max as usize);
            }

            /* Now try matching, moving forward one character on failure, until we
            reach the minimum back length. */

            state = ST_C11_2;
            continue 'sm;
        }

        /* Top of the OP_VREVERSE for (;;) loop */
        ST_C11_2 => {
            RMATCH!((*F).ecode.add(1 + 2 * IMM2_SIZE), RM37);
        }

        ST_L_RM37 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmax_old: u32 = (*F).fields.op_vreverse.max;
            (*F).fields.op_vreverse.max = lmax_old.wrapping_sub(1);
            if lmax_old <= (*F).fields.op_vreverse.min {
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).eptr = (*F).eptr.add(1);
            if utf != 0 {
                FORWARDCHARTEST!((*F).eptr, (*mb).end_subject);
            }
            state = ST_C11_2;
            continue 'sm;
        }

        _ => {}
    }
}
