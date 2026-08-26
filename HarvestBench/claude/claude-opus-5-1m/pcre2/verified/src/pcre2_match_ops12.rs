{
    match state {
        /* ===================================================================== */
        /* An alternation is the end of a branch; scan along to find the end of the
        bracketed group. */

        OP_ALT => {
            branch_end = (*F).ecode;
            loop {
                (*F).ecode = (*F).ecode.add(GET!((*F).ecode, 1) as usize);
                if !(*(*F).ecode as u32 == OP_ALT) {
                    break;
                }
            }
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* The end of a parenthesized group. For all but OP_BRA and OP_COND, the
        starting frame was added to the chained frames in order to remember the
        starting subject position for the group. (Not true for OP_BRA when it's a
        whole pattern recursion, but that is handled separately below.)*/

        OP_KET | OP_KETRMIN | OP_KETRMAX | OP_KETRPOS => {
            bracode = (*F).ecode.sub(GET!((*F).ecode, 1) as usize);

            if branch_end.is_null() {
                branch_end = (*F).ecode;
            }
            branch_start = bracode;
            while branch_start.add(GET!(branch_start, 1) as usize) != branch_end {
                branch_start = branch_start.add(GET!(branch_start, 1) as usize);
            }
            branch_end = core::ptr::null();

            /* Point N to the frame at the start of the most recent group, and P to its
            predecessor. Remember the subject pointer at the start of the group. */

            if *bracode as u32 != OP_BRA && *bracode as u32 != OP_COND {
                N = ((*match_data).heapframes as *mut u8).add((*F).last_group_offset)
                    as *mut heapframe;
                P = (N as *mut u8).sub(frame_size) as *mut heapframe;
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
                    if (*bracode as u32 == OP_ASSERTBACK || *bracode as u32 == OP_ASSERTBACK_NOT)
                        && *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                        && (*F).eptr != (*P).eptr
                    {
                        RRETURN!(MATCH_NOMATCH);
                    }
                    memcpy(
                        (P as *mut u8).add(offset_of!(heapframe, ovector)) as *mut c_void,
                        (*F).ovector.as_mut_ptr() as *const c_void,
                        (*F).offset_top * size_of::<PCRE2_SIZE>(),
                    );
                    (*P).offset_top = (*F).offset_top;
                    (*P).mark = (*F).mark;
                    (*F).back_frame = (F as *mut u8).offset_from(P as *mut u8) as PCRE2_SIZE;
                    RRETURN!(MATCH_MATCH);
                }
            } else {
                P = core::ptr::null_mut(); /* Indicates starting frame not recorded */
            }

            /* The group was not a conditional assertion. */

            'ket_switch: {
                let bc: u32 = *bracode as u32;

                /* Whole pattern recursion is handled as a recursion into group 0, but
                the entire pattern is wrapped in OP_BRA/OP_KET rather than a capturing
                group - a design mistake: it should perhaps have been capture group 0.
                Anyway, that means the end of such recursion must be handled here. It is
                detected by checking for an immediately following OP_END when we are
                recursing in group 0. If this is not the end of a whole-pattern
                recursion, there is nothing to be done. */

                if bc == OP_BRA {
                    if (*F).current_recurse != 0 || *(*F).ecode.add(1 + LINK_SIZE) as u32 != OP_END
                    {
                        break 'ket_switch;
                    }

                    /* It is the end of whole-pattern recursion. */

                    offset = (*F).last_group_offset;

                    /* Corrupted heapframes?. Trigger an assert and return an error */
                    if offset == PCRE2_UNSET {
                        return PCRE2_ERROR_INTERNAL;
                    }

                    N = ((*match_data).heapframes as *mut u8).add(offset) as *mut heapframe;
                    P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                    (*F).last_group_offset = (*P).last_group_offset;

                    /* Reinstate the previous set of captures and then carry on after the
                    recursion call. */

                    (*F).ecode = (*P).ecode.add(1 + LINK_SIZE);

                    if *(*F).ecode as u32 != OP_CREF {
                        memcpy(
                            (*F).ovector.as_mut_ptr() as *mut c_void,
                            (*P).ovector.as_mut_ptr() as *const c_void,
                            (*F).offset_top * size_of::<PCRE2_SIZE>(),
                        );
                        (*F).offset_top = (*P).offset_top;
                    } else {
                        recurse_update_offsets(F, P);
                    }

                    (*F).capture_last = (*P).capture_last;
                    (*F).current_recurse = (*P).current_recurse;
                    state = ST_TOP;
                    continue 'sm; /* With next opcode */
                }
                /* No need to do anything for these */
                else if bc == OP_COND || bc == OP_SCOND {
                    break 'ket_switch;
                }
                /* Non-atomic positive assertions are like OP_BRA, except that the
                subject pointer must be put back to where it was at the start of the
                assertion. For a variable lookbehind, check its end point. */
                else if bc == OP_ASSERTBACK_NA || bc == OP_ASSERT_NA {
                    if bc == OP_ASSERTBACK_NA {
                        if *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                            && (*F).eptr != (*P).eptr
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        /* Fall through */
                    }

                    if (*F).eptr > (*mb).last_used_ptr {
                        (*mb).last_used_ptr = (*F).eptr;
                    }
                    (*F).eptr = (*P).eptr;
                    break 'ket_switch;
                }
                /* Atomic positive assertions are like OP_ONCE, except that in addition
                the subject pointer must be put back to where it was at the start of the
                assertion. For a variable lookbehind, check its end point. */
                else if bc == OP_ASSERTBACK || bc == OP_ASSERT || bc == OP_ONCE {
                    if bc == OP_ASSERTBACK {
                        if *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                            && (*F).eptr != (*P).eptr
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        /* Fall through */
                    }

                    if bc == OP_ASSERTBACK || bc == OP_ASSERT {
                        if (*F).eptr > (*mb).last_used_ptr {
                            (*mb).last_used_ptr = (*F).eptr;
                        }
                        (*F).eptr = (*P).eptr;
                        /* Fall through */
                    }

                    /* For an atomic group, discard internal backtracking points. We must
                    also ensure that any remaining branches within the top-level of the
                    group are not tried. Do this by adjusting the code pointer within the
                    backtrack frame so that it points to the final branch. */

                    (*F).back_frame = (F as *mut u8).offset_from(P as *mut u8) as PCRE2_SIZE;
                    loop {
                        let y: u32 = GET!((*P).ecode, 1);
                        if *(*P).ecode.add(y as usize) as u32 != OP_ALT {
                            break;
                        }
                        (*P).ecode = (*P).ecode.add(y as usize);
                    }
                    break 'ket_switch;
                }
                /* A matching negative assertion returns MATCH, which is turned into
                NOMATCH at the assertion level. For a variable lookbehind, check its end
                point. */
                else if bc == OP_ASSERTBACK_NOT || bc == OP_ASSERT_NOT {
                    if bc == OP_ASSERTBACK_NOT {
                        if *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                            && (*F).eptr != (*P).eptr
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        /* Fall through */
                    }

                    RRETURN!(MATCH_MATCH);
                }
                /* A scan substring group must preserve the current end_subject,
                and restore it before the backtracking is performed into its sub
                pattern. */
                else if bc == OP_ASSERT_SCS {
                    (*F).fields.op_assert_scs.saved_end_subject = (*mb).end_subject;
                    (*mb).end_subject = (*P).fields.op_assert_scs.saved_end_subject;
                    (*mb).true_end_subject =
                        (*mb).end_subject.add((*P).fields.op_assert_scs.true_end_extra);
                    (*F).eptr = (*P).fields.op_assert_scs.saved_eptr;

                    RMATCH!((*F).ecode.add(1 + LINK_SIZE), RM39);
                }
                /* At the end of a script run, apply the script-checking rules. This code
                will never by exercised if Unicode support it not compiled, because in
                that environment script runs cause an error at compile time. */
                else if bc == OP_SCRIPT_RUN {
                    if _pcre2_script_run_8((*P).eptr, (*F).eptr, utf) == 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                    break 'ket_switch;
                }
                /* Whole-pattern recursion is coded as a recurse into group 0, and is
                handled with OP_BRA above. Other recursion is handled here. */
                else if bc == OP_CBRA || bc == OP_CBRAPOS || bc == OP_SCBRA || bc == OP_SCBRAPOS {
                    number = GET2!(bracode, 1 + LINK_SIZE);

                    /* Handle a recursively called group. We reinstate the previous set of
                    captures and then carry on after the recursion call. */

                    if (*F).current_recurse == number {
                        P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                        (*F).ecode = (*P).ecode.add(1 + LINK_SIZE);

                        if *(*F).ecode as u32 != OP_CREF {
                            memcpy(
                                (*F).ovector.as_mut_ptr() as *mut c_void,
                                (*P).ovector.as_mut_ptr() as *const c_void,
                                (*F).offset_top * size_of::<PCRE2_SIZE>(),
                            );
                            (*F).offset_top = (*P).offset_top;
                        } else {
                            recurse_update_offsets(F, P);
                        }

                        (*F).capture_last = (*P).capture_last;
                        (*F).current_recurse = (*P).current_recurse;
                        state = ST_TOP;
                        continue 'sm; /* With next opcode */
                    }

                    /* Deal with actual capturing. */

                    offset = ((number << 1).wrapping_sub(2)) as PCRE2_SIZE;
                    (*F).capture_last = number;
                    Fov!(offset) = (*P).eptr.offset_from((*mb).start_subject) as PCRE2_SIZE;
                    Fov!(offset + 1) = (*F).eptr.offset_from((*mb).start_subject) as PCRE2_SIZE;
                    if offset >= (*F).offset_top {
                        (*F).offset_top = offset + 2;
                    }
                    break 'ket_switch;
                }
            } /* End actions relating to the starting opcode */

            /* OP_KETRPOS is a possessive repeating ket. Remember the current position,
            and return the MATCH_KETRPOS. This makes it possible to do the repeats one
            at a time from the outer level. This must precede the empty string test -
            in this case that test is done at the outer level. */

            if *(*F).ecode as u32 == OP_KETRPOS {
                memcpy(
                    (P as *mut u8).add(offset_of!(heapframe, eptr)) as *mut c_void,
                    (F as *mut u8).add(offset_of!(heapframe, eptr)) as *const c_void,
                    frame_copy_size,
                );
                RRETURN!(MATCH_KETRPOS);
            }

            /* Handle the different kinds of closing brackets. A non-repeating ket
            needs no special action, just continuing at this level. This also happens
            for the repeating kets if the group matched no characters, in order to
            forcibly break infinite loops. Otherwise, the repeating kets try the rest
            of the pattern or restart from the preceding bracket, in the appropriate
            order. */

            if (*F).op as u32 != OP_KET && (P.is_null() || (*F).eptr != (*P).eptr) {
                if (*F).op as u32 == OP_KETRMIN {
                    RMATCH!((*F).ecode.add(1 + LINK_SIZE), RM6);
                }

                /* Repeat the maximum number of times (KETRMAX) */

                RMATCH!(bracode, RM7);
            }

            /* Carry on at this level for a non-repeating ket, or after matching an
            empty string, or after repeating for a maximum number of times. */

            (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
            state = ST_TOP;
            continue 'sm;
        }

        ST_L_RM39 => {
            (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
            (*mb).true_end_subject = (*mb).end_subject;
            RRETURN!(rrc);
        }

        ST_L_RM6 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).ecode = (*F).ecode.sub(GET!((*F).ecode, 1) as usize);
            state = ST_TOP;
            continue 'sm; /* End of ket processing */
        }

        ST_L_RM7 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }

            /* Carry on at this level for a non-repeating ket, or after matching an
            empty string, or after repeating for a maximum number of times. */

            (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
            state = ST_TOP;
            continue 'sm;
        }

        _ => {}
    }
}
