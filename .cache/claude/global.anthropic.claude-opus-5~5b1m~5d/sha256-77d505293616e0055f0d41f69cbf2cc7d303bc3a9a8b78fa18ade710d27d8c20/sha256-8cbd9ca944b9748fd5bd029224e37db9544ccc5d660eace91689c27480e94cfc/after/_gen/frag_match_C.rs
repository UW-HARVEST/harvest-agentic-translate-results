// ==== EXTRA_STATE_CONSTS ====
/* Synthetic loop-entry / continuation states for chunk C (C lines 5489..6900). */
pub const S_BRAPOS_LOOP: u32 = 1200; /* top of the for(;;) at C 5561 */
pub const S_BRAPOS_AFTER: u32 = 1201; /* after that loop, C 5596 */
pub const S_BRA_LOOP: u32 = 1202; /* top of the for(;;) at C 5629 */
pub const S_RECURSE_LOOP: u32 = 1203; /* top of the for(;;) at C 5743 */
pub const S_ASSERT_LOOP: u32 = 1204; /* top of the for(;;) at C 5793 */
pub const S_ASSERTNOT_LOOP: u32 = 1205; /* top of the for(;;) at C 5822 */
pub const S_SCS_CREF_LOOP: u32 = 1206; /* top of the for(;;) at C 5878 */
pub const S_SCS_MATCH_LOOP: u32 = 1207; /* top of the for(;;) at C 5936 */
pub const S_COND_ASSERT_LOOP: u32 = 1208; /* top of the for(;;) at C 6102 */
pub const S_COND_CHOOSE: u32 = 1209; /* C 6159, after the condition switch */
pub const S_VREVERSE_LOOP: u32 = 1210; /* top of the for(;;) at C 6272 */
// ==== EXTRA_LOCALS ====
/* chunk C: the `ecode` local of the OP_ASSERT_SCS case (C 5870); it must
survive the goto to SCS_OFFSET_FOUND, which is a separate state here. */
let mut scs_ecode: PCRE2_SPTR = null();
// ==== ARMS ====

/* ===================================================================== */
/* BRAZERO, BRAMINZERO and SKIPZERO occur just before a non-possessive
bracket group, indicating that it may occur zero times. It may repeat
infinitely, or not at all - i.e. it could be ()* or ()? or even (){0} in
the pattern. Brackets with fixed upper repeat limits are compiled as a
number of copies, with the optional ones preceded by BRAZERO or BRAMINZERO.
Possessive groups with possible zero repeats are preceded by BRAPOSZERO. */

OP_BRAZERO => {
    Fecode!() = Fecode!().add(1);
    /* RMATCH(Fecode, RM9) */
    start_ecode = Fecode!();
    Freturn_id!() = RM9 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_BRAMINZERO => {
    let mut next_ecode: PCRE2_SPTR = Fecode!().add(1);
    Fecode!() = next_ecode;
    loop {
        next_ecode = next_ecode.add(GET!(next_ecode, 1) as usize);
        if *next_ecode as u32 != OP_ALT {
            break;
        }
    }
    /* RMATCH(next_ecode + 1 + LINK_SIZE, RM10) */
    start_ecode = next_ecode.add(1 + LINK_SIZE);
    Freturn_id!() = RM10 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_SKIPZERO => {
    let mut next_ecode: PCRE2_SPTR = Fecode!().add(1);
    loop {
        next_ecode = next_ecode.add(GET!(next_ecode, 1) as usize);
        if *next_ecode as u32 != OP_ALT {
            break;
        }
    }
    Fecode!() = next_ecode.add(1 + LINK_SIZE);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Handle possessive brackets with an unlimited repeat. The end of these
brackets will always be OP_KETRPOS, which returns MATCH_KETRPOS without
going further in the pattern.

Lstart_eptr   = F->fields.op_brapos.start_eptr
Lstart_group  = F->fields.op_brapos.start_group
Lframe_type   = F->fields.op_brapos.frame_type
Lmatched_once = F->byte1
Lzero_allowed = F->byte2 */

OP_BRAPOSZERO => {
    (*F).byte2 = TRUE as u8; /* Zero repeat is allowed */
    Fecode!() = Fecode!().add(1);
    if *Fecode!() as u32 == OP_CBRAPOS || *Fecode!() as u32 == OP_SCBRAPOS {
        /* goto POSSESSIVE_CAPTURE */
        state = L_POSSESSIVE_CAPTURE;
        continue 'sm;
    }
    /* goto POSSESSIVE_NON_CAPTURE */
    state = L_POSSESSIVE_NON_CAPTURE;
    continue 'sm;
}

OP_BRAPOS | OP_SBRAPOS => {
    (*F).byte2 = FALSE as u8; /* Zero repeat not allowed */
    /* fall through to POSSESSIVE_NON_CAPTURE */
    state = L_POSSESSIVE_NON_CAPTURE;
    continue 'sm;
}

OP_CBRAPOS | OP_SCBRAPOS => {
    (*F).byte2 = FALSE as u8; /* Zero repeat not allowed */
    /* fall through to POSSESSIVE_CAPTURE */
    state = L_POSSESSIVE_CAPTURE;
    continue 'sm;
}

/* ===================================================================== */
/* Handle non-capturing brackets that cannot match an empty string. When we
get to the final alternative within the brackets, as long as there are no
THEN's in the pattern, we can optimize by not recording a new backtracking
point. (Ideally we should test for a THEN within this group, but we don't
have that information.) Don't do this if we are at the very top level,
however, because that would make handling assertions and once-only brackets
messier when there is nothing to go back to.

Lframe_type = F->fields.op_bra.frame_type */

OP_BRA => {
    if (*mb).hasthen != 0 || Frdepth!() == 0 {
        (*F).fields.op_bra.frame_type = 0;
        /* goto GROUPLOOP */
        state = L_GROUPLOOP;
        continue 'sm;
    }
    state = S_BRA_LOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Handle a capturing bracket, other than those that are possessive with an
unlimited repeat. */

OP_CBRA | OP_SCBRA => {
    (*F).fields.op_bra.frame_type = GF_CAPTURE | GET2!(Fecode!(), 1 + LINK_SIZE);
    /* goto GROUPLOOP */
    state = L_GROUPLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Atomic groups and non-capturing brackets that can match an empty string
must record a backtracking point and also set up a chained frame. */

OP_ONCE | OP_SCRIPT_RUN | OP_SBRA => {
    (*F).fields.op_bra.frame_type = GF_NOCAPTURE;
    /* fall through to GROUPLOOP */
    state = L_GROUPLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Pattern recursion either matches the current regex, or some
subexpression. The offset data is the offset to the starting bracket from
the start of the whole pattern. This is so that it works from duplicated
subpatterns. For a whole-pattern recursion, we have to infer the number
zero.

Lstart_branch = F->fields.op_recurse.start_branch
Lframe_type   = F->fields.op_recurse.frame_type */

OP_RECURSE => {
    bracode = (*mb).start_code.add(GET!(Fecode!(), 1) as usize);
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

    if Fcurrent_recurse!() != RECURSE_UNSET {
        offset = Flast_group_offset!();
        while offset != PCRE2_UNSET {
            N = ((*match_data).heapframes as *mut u8).add(offset) as *mut heapframe;
            P = ((N as *mut u8).sub(frame_size)) as *mut heapframe;
            if (*N).group_frame_type == (GF_RECURSE | number) {
                if Feptr!() == (*P).eptr
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

    state = S_RECURSE_LOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Positive assertions are like other groups except that PCRE doesn't allow
the effect of (*THEN) to escape beyond an assertion; it is therefore
treated as NOMATCH. (*ACCEPT) is treated as successful assertion, with its
captures and mark retained. Any other return is an error. */

OP_ASSERT | OP_ASSERTBACK | OP_ASSERT_NA | OP_ASSERTBACK_NA => {
    state = S_ASSERT_LOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Handle negative assertions. Loop for each non-matching branch as for
positive assertions. */

OP_ASSERT_NOT | OP_ASSERTBACK_NOT => {
    state = S_ASSERTNOT_LOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Handle scan substring operation.

Lsaved_end_subject = F->fields.op_assert_scs.saved_end_subject
Lsaved_eptr        = F->fields.op_assert_scs.saved_eptr
Ltrue_end_extra    = F->fields.op_assert_scs.true_end_extra
Lsaved_moptions    = F->fields.op_assert_scs.saved_moptions */

OP_ASSERT_SCS => {
    length = 0;
    scs_ecode = Fecode!().add(1 + LINK_SIZE);

    /* Disable compiler warning. */
    offset = 0;

    state = S_SCS_CREF_LOOP;
    continue 'sm;
}

/* ===================================================================== */
/* The callout item calls an external function, if one is provided, passing
details of the match so far. This is mainly for debugging, though the
function is able to force a failure. */

OP_CALLOUT | OP_CALLOUT_STR => {
    rrc = do_callout(F, mb, &mut length);
    if rrc > 0 {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if rrc < 0 {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(length);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Conditional group: compilation checked that there are no more than two
branches. If the condition is false, skipping the first branch takes us
past the end of the item if there is only one branch, but that's exactly
what we want.

Lstart_branch = F->fields.op_cond.start_branch
Llength       = F->fields.op_cond.length
Lpositive     = F->byte1 */

OP_COND | OP_SCOND => {
    /* The variable Llength will be added to Fecode when the condition is
    false, to get to the second branch. Setting it to the offset to the ALT or
    KET, then incrementing Fecode achieves this effect. However, if the second
    branch is non-existent, we must point to the KET so that the end of the
    group is correctly processed. We now have Fecode pointing to the condition
    or callout. */

    (*F).fields.op_cond.length = GET!(Fecode!(), 1) as PCRE2_SIZE; /* Offset to the second branch */
    if *Fecode!().add((*F).fields.op_cond.length) as u32 != OP_ALT {
        (*F).fields.op_cond.length -= 1 + LINK_SIZE;
    }
    Fecode!() = Fecode!().add(1 + LINK_SIZE); /* From this opcode */

    /* Because of the way auto-callout works during compile, a callout item is
    inserted between OP_COND and an assertion condition. Such a callout can
    also be inserted manually. */

    if *Fecode!() as u32 == OP_CALLOUT || *Fecode!() as u32 == OP_CALLOUT_STR {
        rrc = do_callout(F, mb, &mut length);
        if rrc > 0 {
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        if rrc < 0 {
            /* RRETURN(rrc) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }

        /* Advance Fecode past the callout, so it now points to the condition. We
        must adjust Llength so that the value of Fecode+Llength is unchanged. */

        Fecode!() = Fecode!().add(length);
        (*F).fields.op_cond.length -= length;
    }

    /* Test the various possible conditions */

    condition = FALSE;
    match *Fecode!() as u32 {
        OP_RREF => {
            /* Group recursion test */
            if Fcurrent_recurse!() != RECURSE_UNSET {
                number = GET2!(Fecode!(), 1);
                condition = (number == RREF_ANY || number == Fcurrent_recurse!()) as BOOL;
            }
        }

        OP_DNRREF => {
            /* Duplicate named group recursion test */
            if Fcurrent_recurse!() != RECURSE_UNSET {
                let mut count: i32 = GET2!(Fecode!(), 1 + IMM2_SIZE) as i32;
                let mut slot: PCRE2_SPTR = (*mb)
                    .name_table
                    .add((GET2!(Fecode!(), 1) as usize) * ((*mb).name_entry_size as usize));
                loop {
                    let c_ = count;
                    count -= 1;
                    if !(c_ > 0) {
                        break;
                    }
                    number = GET2!(slot, 0);
                    condition = (number == Fcurrent_recurse!()) as BOOL;
                    if condition != 0 {
                        break;
                    }
                    slot = slot.add((*mb).name_entry_size as usize);
                }
            }
        }

        OP_CREF => {
            /* Numbered group used test */
            offset = ((GET2!(Fecode!(), 1) << 1).wrapping_sub(2)) as PCRE2_SIZE; /* Doubled ref number */
            condition = (offset < Foffset_top!() && *Fovector!().add(offset) != PCRE2_UNSET) as BOOL;
        }

        OP_DNCREF => {
            /* Duplicate named group used test */
            let mut count: i32 = GET2!(Fecode!(), 1 + IMM2_SIZE) as i32;
            let mut slot: PCRE2_SPTR = (*mb)
                .name_table
                .add((GET2!(Fecode!(), 1) as usize) * ((*mb).name_entry_size as usize));
            loop {
                let c_ = count;
                count -= 1;
                if !(c_ > 0) {
                    break;
                }
                offset = ((GET2!(slot, 0) << 1).wrapping_sub(2)) as PCRE2_SIZE;
                condition =
                    (offset < Foffset_top!() && *Fovector!().add(offset) != PCRE2_UNSET) as BOOL;
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

        /* The condition is an assertion. Run code similar to the assertion code
        above. */
        _ => {
            (*F).byte1 =
                (*Fecode!() as u32 == OP_ASSERT || *Fecode!() as u32 == OP_ASSERTBACK) as u8;
            (*F).fields.op_cond.start_branch = Fecode!();
            state = S_COND_ASSERT_LOOP;
            continue 'sm;
        }
    }

    state = S_COND_CHOOSE;
    continue 'sm;
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
    number = GET2!(Fecode!(), 1);
    if utf != 0 {
        /* We used to do a simpler `while (number-- > 0)` but that triggers
        clang's unsigned integer overflow sanitizer. */
        while number > 0 {
            number -= 1;
            if Feptr!() <= (*mb).check_subject {
                /* RRETURN(MATCH_NOMATCH) */
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            Feptr!() = Feptr!().offset(-1);
            BACKCHAR!(Feptr!());
        }
    } else {
        /* No UTF support, or not in UTF mode: count is code unit count */
        if (number as isize) > Feptr!().offset_from((*mb).start_subject) {
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        Feptr!() = Feptr!().sub(number as usize);
    }

    /* Save the earliest consulted character, then skip to next opcode */

    if Feptr!() < (*mb).start_used_ptr {
        (*mb).start_used_ptr = Feptr!();
    }
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Move the subject pointer back by a variable amount. This occurs at the
start of each branch of a lookbehind assertion when the branch has a
variable, but limited, length. A loop is needed to try matching the branch
after moving back different numbers of characters. If we are too close to
the start to move back even the minimum amount, fail. When working with
UTF-8 we move back a number of characters, not bytes.

Lmin = F->fields.op_vreverse.min
Lmax = F->fields.op_vreverse.max */

OP_VREVERSE => {
    (*F).fields.op_vreverse.min = GET2!(Fecode!(), 1);
    (*F).fields.op_vreverse.max = GET2!(Fecode!(), 1 + IMM2_SIZE);

    /* Move back by the maximum branch length and then work forwards. This
    ensures that items such as \d{3,5} get the maximum length, which is
    relevant for captures, and makes for Perl compatibility. */

    if utf != 0 {
        i = 0;
        while i < (*F).fields.op_vreverse.max {
            if Feptr!() == (*mb).start_subject {
                if i < (*F).fields.op_vreverse.min {
                    /* RRETURN(MATCH_NOMATCH) */
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                (*F).fields.op_vreverse.max = i;
                break;
            }
            Feptr!() = Feptr!().offset(-1);
            BACKCHAR!(Feptr!());
            i += 1;
        }
    } else {
        /* No UTF support or not in UTF mode */
        let diff: isize = Feptr!().offset_from((*mb).start_subject);
        let available: u32 = if diff > 65535 {
            65535
        } else if diff > 0 {
            diff as i32 as u32
        } else {
            0
        };
        if (*F).fields.op_vreverse.min > available {
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        if (*F).fields.op_vreverse.max > available {
            (*F).fields.op_vreverse.max = available;
        }
        Feptr!() = Feptr!().sub((*F).fields.op_vreverse.max as usize);
    }

    /* Now try matching, moving forward one character on failure, until we
    reach the minimum back length. */

    state = S_VREVERSE_LOOP;
    continue 'sm;
}

/* ===================================================================== */
/* An alternation is the end of a branch; scan along to find the end of the
bracketed group. */

OP_ALT => {
    branch_end = Fecode!();
    loop {
        Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
        if *Fecode!() as u32 != OP_ALT {
            break;
        }
    }
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* The end of a parenthesized group. For all but OP_BRA and OP_COND, the
starting frame was added to the chained frames in order to remember the
starting subject position for the group. (Not true for OP_BRA when it's a
whole pattern recursion, but that is handled separately below.)*/

OP_KET | OP_KETRMIN | OP_KETRMAX | OP_KETRPOS => {
    bracode = Fecode!().sub(GET!(Fecode!(), 1) as usize);

    if branch_end.is_null() {
        branch_end = Fecode!();
    }
    branch_start = bracode;
    while branch_start.add(GET!(branch_start, 1) as usize) != branch_end {
        branch_start = branch_start.add(GET!(branch_start, 1) as usize);
    }
    branch_end = null();

    /* Point N to the frame at the start of the most recent group, and P to its
    predecessor. Remember the subject pointer at the start of the group. */

    if *bracode as u32 != OP_BRA && *bracode as u32 != OP_COND {
        N = ((*match_data).heapframes as *mut u8).add(Flast_group_offset!()) as *mut heapframe;
        P = ((N as *mut u8).sub(frame_size)) as *mut heapframe;
        Flast_group_offset!() = (*P).last_group_offset;

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
                && Feptr!() != (*P).eptr
            {
                /* RRETURN(MATCH_NOMATCH) */
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            copy_nonoverlapping(
                Fovector!() as *const u8,
                (P as *mut u8).add(OVECTOR_OFFSET_IN_HEAPFRAME),
                Foffset_top!() * core::mem::size_of::<PCRE2_SIZE>(),
            );
            (*P).offset_top = Foffset_top!();
            (*P).mark = Fmark!();
            Fback_frame!() = (F as usize) - (P as usize);
            /* RRETURN(MATCH_MATCH) */
            rrc = MATCH_MATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    } else {
        P = null_mut(); /* Indicates starting frame not recorded */
    }

    /* The group was not a conditional assertion. */

    'ket_switch: {
        match *bracode as u32 {
            /* Whole pattern recursion is handled as a recursion into group 0, but
            the entire pattern is wrapped in OP_BRA/OP_KET rather than a capturing
            group - a design mistake: it should perhaps have been capture group 0.
            Anyway, that means the end of such recursion must be handled here. It is
            detected by checking for an immediately following OP_END when we are
            recursing in group 0. If this is not the end of a whole-pattern
            recursion, there is nothing to be done. */
            OP_BRA => {
                if Fcurrent_recurse!() != 0 || *Fecode!().add(1 + LINK_SIZE) as u32 != OP_END {
                    break 'ket_switch;
                }

                /* It is the end of whole-pattern recursion. */

                offset = Flast_group_offset!();

                /* Corrupted heapframes?. Trigger an assert and return an error */
                if offset == PCRE2_UNSET {
                    return PCRE2_ERROR_INTERNAL;
                }

                N = ((*match_data).heapframes as *mut u8).add(offset) as *mut heapframe;
                P = ((N as *mut u8).sub(frame_size)) as *mut heapframe;
                Flast_group_offset!() = (*P).last_group_offset;

                /* Reinstate the previous set of captures and then carry on after the
                recursion call. */

                Fecode!() = (*P).ecode.add(1 + LINK_SIZE);

                if *Fecode!() as u32 != OP_CREF {
                    copy_nonoverlapping(
                        (*P).ovector.as_ptr() as *const u8,
                        (*F).ovector.as_mut_ptr() as *mut u8,
                        Foffset_top!() * core::mem::size_of::<PCRE2_SIZE>(),
                    );
                    Foffset_top!() = (*P).offset_top;
                } else {
                    recurse_update_offsets(F, P);
                }

                Fcapture_last!() = (*P).capture_last;
                Fcurrent_recurse!() = (*P).current_recurse;
                /* continue: with next opcode */
                state = S_MAINLOOP;
                continue 'sm;
            }

            OP_COND | OP_SCOND => {
                /* No need to do anything for these */
            }

            /* Non-atomic positive assertions are like OP_BRA, except that the
            subject pointer must be put back to where it was at the start of the
            assertion. For a variable lookbehind, check its end point. */
            OP_ASSERTBACK_NA => {
                if *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                    && Feptr!() != (*P).eptr
                {
                    /* RRETURN(MATCH_NOMATCH) */
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                /* Fall through to OP_ASSERT_NA */
                if Feptr!() > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = Feptr!();
                }
                Feptr!() = (*P).eptr;
            }

            OP_ASSERT_NA => {
                if Feptr!() > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = Feptr!();
                }
                Feptr!() = (*P).eptr;
            }

            /* Atomic positive assertions are like OP_ONCE, except that in addition
            the subject pointer must be put back to where it was at the start of the
            assertion. For a variable lookbehind, check its end point. */
            OP_ASSERTBACK => {
                if *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                    && Feptr!() != (*P).eptr
                {
                    /* RRETURN(MATCH_NOMATCH) */
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                /* Fall through to OP_ASSERT */
                if Feptr!() > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = Feptr!();
                }
                Feptr!() = (*P).eptr;
                /* Fall through to OP_ONCE */
                Fback_frame!() = (F as usize) - (P as usize);
                loop {
                    let y: u32 = GET!((*P).ecode, 1);
                    if *(*P).ecode.add(y as usize) as u32 != OP_ALT {
                        break;
                    }
                    (*P).ecode = (*P).ecode.add(y as usize);
                }
            }

            OP_ASSERT => {
                if Feptr!() > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = Feptr!();
                }
                Feptr!() = (*P).eptr;
                /* Fall through to OP_ONCE */
                Fback_frame!() = (F as usize) - (P as usize);
                loop {
                    let y: u32 = GET!((*P).ecode, 1);
                    if *(*P).ecode.add(y as usize) as u32 != OP_ALT {
                        break;
                    }
                    (*P).ecode = (*P).ecode.add(y as usize);
                }
            }

            /* For an atomic group, discard internal backtracking points. We must
            also ensure that any remaining branches within the top-level of the group
            are not tried. Do this by adjusting the code pointer within the backtrack
            frame so that it points to the final branch. */
            OP_ONCE => {
                Fback_frame!() = (F as usize) - (P as usize);
                loop {
                    let y: u32 = GET!((*P).ecode, 1);
                    if *(*P).ecode.add(y as usize) as u32 != OP_ALT {
                        break;
                    }
                    (*P).ecode = (*P).ecode.add(y as usize);
                }
            }

            /* A matching negative assertion returns MATCH, which is turned into
            NOMATCH at the assertion level. For a variable lookbehind, check its end
            point. */
            OP_ASSERTBACK_NOT => {
                if *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                    && Feptr!() != (*P).eptr
                {
                    /* RRETURN(MATCH_NOMATCH) */
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                /* Fall through to OP_ASSERT_NOT */
                /* RRETURN(MATCH_MATCH) */
                rrc = MATCH_MATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }

            OP_ASSERT_NOT => {
                /* RRETURN(MATCH_MATCH) */
                rrc = MATCH_MATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
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
                Feptr!() = (*P).fields.op_assert_scs.saved_eptr;

                /* RMATCH(Fecode + 1 + LINK_SIZE, RM39) */
                start_ecode = Fecode!().add(1 + LINK_SIZE);
                Freturn_id!() = RM39 as u8;
                state = S_MATCH_RECURSE;
                continue 'sm;
            }

            /* At the end of a script run, apply the script-checking rules. This code
            will never by exercised if Unicode support it not compiled, because in
            that environment script runs cause an error at compile time. */
            OP_SCRIPT_RUN => {
                if crate::script_run::_pcre2_script_run_8((*P).eptr, Feptr!(), utf) == 0 {
                    /* RRETURN(MATCH_NOMATCH) */
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            /* Whole-pattern recursion is coded as a recurse into group 0, and is
            handled with OP_BRA above. Other recursion is handled here. */
            OP_CBRA | OP_CBRAPOS | OP_SCBRA | OP_SCBRAPOS => {
                number = GET2!(bracode, 1 + LINK_SIZE);

                /* Handle a recursively called group. We reinstate the previous set of
                captures and then carry on after the recursion call. */

                if Fcurrent_recurse!() == number {
                    P = ((N as *mut u8).sub(frame_size)) as *mut heapframe;
                    Fecode!() = (*P).ecode.add(1 + LINK_SIZE);

                    if *Fecode!() as u32 != OP_CREF {
                        copy_nonoverlapping(
                            (*P).ovector.as_ptr() as *const u8,
                            (*F).ovector.as_mut_ptr() as *mut u8,
                            Foffset_top!() * core::mem::size_of::<PCRE2_SIZE>(),
                        );
                        Foffset_top!() = (*P).offset_top;
                    } else {
                        recurse_update_offsets(F, P);
                    }

                    Fcapture_last!() = (*P).capture_last;
                    Fcurrent_recurse!() = (*P).current_recurse;
                    /* continue: with next opcode */
                    state = S_MAINLOOP;
                    continue 'sm;
                }

                /* Deal with actual capturing. */

                offset = ((number << 1).wrapping_sub(2)) as PCRE2_SIZE;
                Fcapture_last!() = number;
                *Fovector!().add(offset) =
                    ((*P).eptr as usize) - ((*mb).start_subject as usize);
                *Fovector!().add(offset + 1) =
                    (Feptr!() as usize) - ((*mb).start_subject as usize);
                if offset >= Foffset_top!() {
                    Foffset_top!() = offset + 2;
                }
            }

            _ => {}
        } /* End actions relating to the starting opcode */
    }

    /* OP_KETRPOS is a possessive repeating ket. Remember the current position,
    and return the MATCH_KETRPOS. This makes it possible to do the repeats one
    at a time from the outer level. This must precede the empty string test -
    in this case that test is done at the outer level. */

    if *Fecode!() as u32 == OP_KETRPOS {
        copy_nonoverlapping(
            (F as *const u8).add(EPTR_OFFSET_IN_HEAPFRAME),
            (P as *mut u8).add(EPTR_OFFSET_IN_HEAPFRAME),
            frame_copy_size,
        );
        /* RRETURN(MATCH_KETRPOS) */
        rrc = MATCH_KETRPOS;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    /* Handle the different kinds of closing brackets. A non-repeating ket
    needs no special action, just continuing at this level. This also happens
    for the repeating kets if the group matched no characters, in order to
    forcibly break infinite loops. Otherwise, the repeating kets try the rest
    of the pattern or restart from the preceding bracket, in the appropriate
    order. */

    if Fop!() as u32 != OP_KET && (P.is_null() || Feptr!() != (*P).eptr) {
        if Fop!() as u32 == OP_KETRMIN {
            /* RMATCH(Fecode + 1 + LINK_SIZE, RM6) */
            start_ecode = Fecode!().add(1 + LINK_SIZE);
            Freturn_id!() = RM6 as u8;
            state = S_MATCH_RECURSE;
            continue 'sm;
        }

        /* Repeat the maximum number of times (KETRMAX) */

        /* RMATCH(bracode, RM7) */
        start_ecode = bracode;
        Freturn_id!() = RM7 as u8;
        state = S_MATCH_RECURSE;
        continue 'sm;
    }

    /* Carry on at this level for a non-repeating ket, or after matching an
    empty string, or after repeating for a maximum number of times. */

    Fecode!() = Fecode!().add(1 + LINK_SIZE);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Start and end of line assertions, not multiline mode. */

OP_CIRC => {
    /* Start of line, unless PCRE2_NOTBOL is set. */
    if Feptr!() != (*mb).start_subject || ((*mb).moptions & PCRE2_NOTBOL) != 0 {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

OP_SOD => {
    /* Unconditional start of subject */
    if Feptr!() != (*mb).start_subject {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* When PCRE2_NOTEOL is unset, assert before the subject end, or a
terminating newline unless PCRE2_DOLLAR_ENDONLY is set. */

OP_DOLL => {
    if ((*mb).moptions & PCRE2_NOTEOL) != 0 {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if ((*mb).poptions & PCRE2_DOLLAR_ENDONLY) == 0 {
        /* goto ASSERT_NL_OR_EOS */
        state = L_ASSERT_NL_OR_EOS;
        continue 'sm;
    }

    /* Fall through to OP_EOD */
    /* Unconditional end of subject assertion (\z). */
    if Feptr!() < (*mb).true_end_subject {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if (*mb).partial != 0 {
        (*mb).hitend = TRUE;
        if (*mb).partial > 1 {
            return PCRE2_ERROR_PARTIAL;
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

OP_EOD => {
    if Feptr!() < (*mb).true_end_subject {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if (*mb).partial != 0 {
        (*mb).hitend = TRUE;
        if (*mb).partial > 1 {
            return PCRE2_ERROR_PARTIAL;
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* End of subject or ending \n assertion (\Z) */

OP_EODN => {
    /* fall into ASSERT_NL_OR_EOS */
    state = L_ASSERT_NL_OR_EOS;
    continue 'sm;
}

/* ===================================================================== */
/* Start and end of line assertions, multiline mode. */

/* Start of subject unless notbol, or after any newline except for one at
the very end, unless PCRE2_ALT_CIRCUMFLEX is set. */

OP_CIRCM => {
    if ((*mb).moptions & PCRE2_NOTBOL) != 0 && Feptr!() == (*mb).start_subject {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() != (*mb).start_subject
        && ((Feptr!() == (*mb).end_subject && ((*mb).poptions & PCRE2_ALT_CIRCUMFLEX) == 0)
            || WAS_NEWLINE!(Feptr!()) == 0)
    {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* Assert before any newline, or before end of subject unless noteol is
set. */

OP_DOLLM => {
    if Feptr!() < (*mb).end_subject {
        if IS_NEWLINE!(Feptr!()) == 0 {
            if (*mb).partial != 0
                && Feptr!().add(1) >= (*mb).end_subject
                && (*mb).nltype == NLTYPE_FIXED
                && (*mb).nllen == 2
                && *Feptr!() == (*mb).nl[0]
            {
                (*mb).hitend = TRUE;
                if (*mb).partial > 1 {
                    return PCRE2_ERROR_PARTIAL;
                }
            }
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    } else {
        if ((*mb).moptions & PCRE2_NOTEOL) != 0 {
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        SCHECK_PARTIAL!();
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Start of match assertion */

OP_SOM => {
    if Feptr!() != (*mb).start_subject.add((*mb).start_offset) {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Reset the start of match point */

OP_SET_SOM => {
    Fstart_match!() = Feptr!();
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Word boundary assertions. Find out if the previous and current
characters are "word" characters. It takes a bit more work in UTF mode.
Characters > 255 are assumed to be "non-word" characters when PCRE2_UCP is
not set. When it is set, use Unicode properties if available, even when not
in UTF mode. Remember the earliest and latest consulted characters. */

OP_NOT_WORD_BOUNDARY | OP_WORD_BOUNDARY | OP_NOT_UCP_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY => {
    if Feptr!() == (*mb).check_subject {
        prev_is_word = FALSE;
    } else {
        let mut lastptr: PCRE2_SPTR = Feptr!().wrapping_sub(1);
        if utf != 0 {
            BACKCHAR!(lastptr);
            GETCHAR!(fc, lastptr);
        } else {
            fc = *lastptr as u32;
        }
        if lastptr < (*mb).start_used_ptr {
            (*mb).start_used_ptr = lastptr;
        }
        if Fop!() as u32 == OP_UCP_WORD_BOUNDARY || Fop!() as u32 == OP_NOT_UCP_WORD_BOUNDARY {
            let chartype: i32 = UCD_CHARTYPE!(fc) as i32;
            let category: i32 = crate::tables::_pcre2_ucp_gentype_8[chartype as usize] as i32;
            prev_is_word = (category == ucp_L as i32
                || category == ucp_N as i32
                || chartype == ucp_Mn as i32
                || chartype == ucp_Pc as i32) as BOOL;
        } else {
            prev_is_word = (CHMAX_255!(fc) != 0
                && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0) as BOOL;
        }
    }

    /* Get status of next character */

    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        cur_is_word = FALSE;
    } else {
        let mut nextptr: PCRE2_SPTR = Feptr!().add(1);
        if utf != 0 {
            FORWARDCHARTEST!(nextptr, (*mb).end_subject);
            GETCHAR!(fc, Feptr!());
        } else {
            fc = *Feptr!() as u32;
        }
        if nextptr > (*mb).last_used_ptr {
            (*mb).last_used_ptr = nextptr;
        }
        if Fop!() as u32 == OP_UCP_WORD_BOUNDARY || Fop!() as u32 == OP_NOT_UCP_WORD_BOUNDARY {
            let chartype: i32 = UCD_CHARTYPE!(fc) as i32;
            let category: i32 = crate::tables::_pcre2_ucp_gentype_8[chartype as usize] as i32;
            cur_is_word = (category == ucp_L as i32
                || category == ucp_N as i32
                || chartype == ucp_Mn as i32
                || chartype == ucp_Pc as i32) as BOOL;
        } else {
            cur_is_word = (CHMAX_255!(fc) != 0
                && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0) as BOOL;
        }
    }

    /* Now see if the situation is what we want */

    let this_op: u8 = *Fecode!();
    Fecode!() = Fecode!().add(1);
    let want: BOOL = if this_op as u32 == OP_WORD_BOUNDARY
        || Fop!() as u32 == OP_UCP_WORD_BOUNDARY
    {
        (cur_is_word == prev_is_word) as BOOL
    } else {
        (cur_is_word != prev_is_word) as BOOL
    };
    if want != 0 {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Backtracking (*VERB)s, with and without arguments. Note that if the
pattern is successfully matched, we do not come back from RMATCH. */

OP_MARK => {
    (*mb).nomatch_mark = Fecode!().add(2);
    Fmark!() = (*mb).nomatch_mark;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode] + Fecode[1], RM12) */
    start_ecode = Fecode!()
        .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize)
        .add(*Fecode!().add(1) as usize);
    Freturn_id!() = RM12 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_FAIL => {
    /* RRETURN(MATCH_NOMATCH) */
    rrc = MATCH_NOMATCH;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* Record the current recursing group number in mb->verb_current_recurse
when a backtracking return such as MATCH_COMMIT is given. This enables the
recurse processing to catch verbs from within the recursion. */

OP_COMMIT => {
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM13) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM13 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_COMMIT_ARG => {
    (*mb).nomatch_mark = Fecode!().add(2);
    Fmark!() = (*mb).nomatch_mark;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode] + Fecode[1], RM36) */
    start_ecode = Fecode!()
        .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize)
        .add(*Fecode!().add(1) as usize);
    Freturn_id!() = RM36 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_PRUNE => {
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM14) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM14 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_PRUNE_ARG => {
    (*mb).nomatch_mark = Fecode!().add(2);
    Fmark!() = (*mb).nomatch_mark;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode] + Fecode[1], RM15) */
    start_ecode = Fecode!()
        .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize)
        .add(*Fecode!().add(1) as usize);
    Freturn_id!() = RM15 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_SKIP => {
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM16) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM16 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

/* Note that, for Perl compatibility, SKIP with an argument does NOT set
nomatch_mark. When a pattern match ends with a SKIP_ARG for which there was
not a matching mark, we have to re-run the match, ignoring the SKIP_ARG
that failed and any that precede it (either they also failed, or were not
triggered). To do this, we maintain a count of executed SKIP_ARGs. If a
SKIP_ARG gets to top level, the match is re-run with mb->ignore_skip_arg
set to the count of the one that failed. */

OP_SKIP_ARG => {
    (*mb).skip_arg_count += 1;
    if (*mb).skip_arg_count <= (*mb).ignore_skip_arg {
        Fecode!() = Fecode!()
            .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize)
            .add(*Fecode!().add(1) as usize);
        state = S_MAINLOOP;
        continue 'sm;
    }
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode] + Fecode[1], RM17) */
    start_ecode = Fecode!()
        .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize)
        .add(*Fecode!().add(1) as usize);
    Freturn_id!() = RM17 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

/* For THEN (and THEN_ARG) we pass back the address of the opcode, so that
the branch in which it occurs can be determined. */

OP_THEN => {
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM18) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM18 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_THEN_ARG => {
    (*mb).nomatch_mark = Fecode!().add(2);
    Fmark!() = (*mb).nomatch_mark;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode] + Fecode[1], RM19) */
    start_ecode = Fecode!()
        .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize)
        .add(*Fecode!().add(1) as usize);
    Freturn_id!() = RM19 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

// ==== STATES ====

/* ---- chunk C: labels and RMATCH resume points ---- */

/* POSSESSIVE_NON_CAPTURE: C 5545 */
L_POSSESSIVE_NON_CAPTURE => {
    (*F).fields.op_brapos.frame_type = GF_NOCAPTURE; /* Remembered frame type */
    /* goto POSSESSIVE_GROUP */
    state = L_POSSESSIVE_GROUP;
    continue 'sm;
}

/* POSSESSIVE_CAPTURE: C 5553 */
L_POSSESSIVE_CAPTURE => {
    number = GET2!(Fecode!(), 1 + LINK_SIZE);
    (*F).fields.op_brapos.frame_type = GF_CAPTURE | number; /* Remembered frame type */
    /* fall through to POSSESSIVE_GROUP */
    state = L_POSSESSIVE_GROUP;
    continue 'sm;
}

/* POSSESSIVE_GROUP: C 5557 */
L_POSSESSIVE_GROUP => {
    (*F).byte1 = FALSE as u8; /* Lmatched_once: never matched */
    (*F).fields.op_brapos.start_group = Fecode!(); /* Start of this group */
    state = S_BRAPOS_LOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 5561 */
S_BRAPOS_LOOP => {
    (*F).fields.op_brapos.start_eptr = Feptr!(); /* Position at group start */
    group_frame_type = (*F).fields.op_brapos.frame_type;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM8) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM8 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM8 => {
    if rrc == MATCH_KETRPOS {
        (*F).byte1 = TRUE as u8; /* Matched at least once */
        if Feptr!() == (*F).fields.op_brapos.start_eptr {
            /* Empty match; skip to end */
            loop {
                Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
                if *Fecode!() as u32 != OP_ALT {
                    break;
                }
            }
            /* break out of the for(;;) */
            state = S_BRAPOS_AFTER;
            continue 'sm;
        }

        Fecode!() = (*F).fields.op_brapos.start_group;
        /* continue the for(;;) */
        state = S_BRAPOS_LOOP;
        continue 'sm;
    }

    /* See comment above about handling THEN. */

    if rrc == MATCH_THEN {
        let next_ecode: PCRE2_SPTR = Fecode!().add(GET!(Fecode!(), 1) as usize);
        if (*mb).verb_ecode_ptr < next_ecode
            && (*Fecode!() as u32 == OP_ALT || *next_ecode as u32 == OP_ALT)
        {
            rrc = MATCH_NOMATCH;
        }
    }

    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
    if *Fecode!() as u32 != OP_ALT {
        /* break out of the for(;;) */
        state = S_BRAPOS_AFTER;
        continue 'sm;
    }
    state = S_BRAPOS_LOOP;
    continue 'sm;
}

/* C 5594: success if matched something or zero repeat allowed */
S_BRAPOS_AFTER => {
    if (*F).byte1 != 0 || (*F).byte2 != 0 {
        Fecode!() = Fecode!().add(1 + LINK_SIZE);
        state = S_MAINLOOP;
        continue 'sm;
    }

    /* RRETURN(MATCH_NOMATCH) */
    rrc = MATCH_NOMATCH;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* Top of the for(;;) at C 5629 (OP_BRA, no THEN in the pattern) */
S_BRA_LOOP => {
    let current_branch: PCRE2_SPTR = Fecode!();
    let next_branch: PCRE2_SPTR = current_branch.add(GET!(current_branch, 1) as usize);

    if *next_branch as u32 != OP_ALT {
        /* break: hit the start of the final branch. Continue at this level. */
        Fecode!() = Fecode!().add(1 + LINK_SIZE);
        state = S_MAINLOOP;
        continue 'sm;
    }

    /* This is never the final branch. We do not need to test for MATCH_THEN
    here because this code is not used when there is a THEN in the pattern. */

    Fecode!() = next_branch;

    /* RMATCH(current_branch + 1 + LINK_SIZE, RM1) */
    start_ecode = current_branch.add(1 + LINK_SIZE);
    Freturn_id!() = RM1 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM1 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = S_BRA_LOOP;
    continue 'sm;
}

/* GROUPLOOP: C 5676 */
L_GROUPLOOP => {
    group_frame_type = (*F).fields.op_bra.frame_type;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM2) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM2 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM2 => {
    if rrc == MATCH_THEN {
        let next_ecode: PCRE2_SPTR = Fecode!().add(GET!(Fecode!(), 1) as usize);
        if (*mb).verb_ecode_ptr < next_ecode
            && (*Fecode!() as u32 == OP_ALT || *next_ecode as u32 == OP_ALT)
        {
            rrc = MATCH_NOMATCH;
        }
    }
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
    if *Fecode!() as u32 != OP_ALT {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_GROUPLOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 5743 (OP_RECURSE) */
S_RECURSE_LOOP => {
    group_frame_type = (*F).fields.op_recurse.frame_type;
    /* RMATCH(Lstart_branch + PRIV(OP_lengths)[*Lstart_branch], RM11) */
    start_ecode = (*F).fields.op_recurse.start_branch.add(
        crate::tables::_pcre2_OP_lengths_8[*(*F).fields.op_recurse.start_branch as usize] as usize,
    );
    Freturn_id!() = RM11 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM11 => {
    let next_ecode: PCRE2_SPTR = (*F)
        .fields
        .op_recurse
        .start_branch
        .add(GET!((*F).fields.op_recurse.start_branch, 1) as usize);

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
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }

    /* Note that carrying on after (*ACCEPT) in a recursion is handled in the
    OP_ACCEPT code. Nothing needs to be done here. */

    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*F).fields.op_recurse.start_branch = next_ecode;
    if *(*F).fields.op_recurse.start_branch as u32 != OP_ALT {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = S_RECURSE_LOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 5793 (positive assertions) */
S_ASSERT_LOOP => {
    group_frame_type = GF_NOCAPTURE;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM3) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM3 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM3 => {
    if rrc == MATCH_ACCEPT {
        copy_nonoverlapping(
            (assert_accept_frame as *const u8).add(OVECTOR_OFFSET_IN_HEAPFRAME),
            Fovector!() as *mut u8,
            (*assert_accept_frame).offset_top * core::mem::size_of::<PCRE2_SIZE>(),
        );
        Foffset_top!() = (*assert_accept_frame).offset_top;
        Fmark!() = (*assert_accept_frame).mark;
        /* break out of the for(;;) */
        loop {
            Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
            if *Fecode!() as u32 != OP_ALT {
                break;
            }
        }
        Fecode!() = Fecode!().add(1 + LINK_SIZE);
        state = S_MAINLOOP;
        continue 'sm;
    }
    if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
    if *Fecode!() as u32 != OP_ALT {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = S_ASSERT_LOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 5822 (negative assertions) */
S_ASSERTNOT_LOOP => {
    group_frame_type = GF_NOCAPTURE;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM4) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM4 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM4 => {
    match rrc {
        /* Assertion matched, therefore it fails. */
        MATCH_ACCEPT | MATCH_MATCH => {
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }

        /* Branch failed, try next if present. */
        MATCH_NOMATCH | MATCH_THEN => {
            Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
            if *Fecode!() as u32 != OP_ALT {
                /* goto ASSERT_NOT_FAILED */
                state = L_ASSERT_NOT_FAILED;
                continue 'sm;
            }
            /* break out of the switch; round the for(;;) again */
            state = S_ASSERTNOT_LOOP;
            continue 'sm;
        }

        /* Assertion forced to fail, therefore continue. */
        MATCH_COMMIT | MATCH_SKIP | MATCH_PRUNE => {
            loop {
                Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
                if *Fecode!() as u32 != OP_ALT {
                    break;
                }
            }
            /* goto ASSERT_NOT_FAILED */
            state = L_ASSERT_NOT_FAILED;
            continue 'sm;
        }

        /* Pass back any other return */
        _ => {
            /* RRETURN(rrc) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
}

/* ASSERT_NOT_FAILED: C 5853. None of the branches have matched or there was
a backtrack to (*COMMIT), (*SKIP), (*PRUNE), or (*THEN) in the last branch.
This is success for a negative assertion, so carry on. */
L_ASSERT_NOT_FAILED => {
    Fecode!() = Fecode!().add(1 + LINK_SIZE);
    state = S_MAINLOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 5878 (OP_ASSERT_SCS condition scan) */
S_SCS_CREF_LOOP => {
    if *scs_ecode as u32 == OP_CREF {
        length += 1 + IMM2_SIZE;
        offset = ((GET2!(scs_ecode, 1) << 1).wrapping_sub(2)) as PCRE2_SIZE;
        scs_ecode = scs_ecode.add(1 + IMM2_SIZE);
        if offset < Foffset_top!() && *Fovector!().add(offset) != PCRE2_UNSET {
            /* goto SCS_OFFSET_FOUND */
            state = L_SCS_OFFSET_FOUND;
            continue 'sm;
        }
        state = S_SCS_CREF_LOOP;
        continue 'sm;
    }

    if *scs_ecode as u32 != OP_DNCREF {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    let mut count: i32 = GET2!(scs_ecode, 1 + IMM2_SIZE) as i32;
    let mut slot: PCRE2_SPTR = (*mb)
        .name_table
        .add((GET2!(scs_ecode, 1) as usize) * ((*mb).name_entry_size as usize));
    length += 1 + 2 * IMM2_SIZE;
    scs_ecode = scs_ecode.add(1 + 2 * IMM2_SIZE);

    while count > 0 {
        offset = ((GET2!(slot, 0) << 1).wrapping_sub(2)) as PCRE2_SIZE;
        if offset < Foffset_top!() && *Fovector!().add(offset) != PCRE2_UNSET {
            /* goto SCS_OFFSET_FOUND */
            state = L_SCS_OFFSET_FOUND;
            continue 'sm;
        }
        slot = slot.add((*mb).name_entry_size as usize);
        count -= 1;
    }
    state = S_SCS_CREF_LOOP;
    continue 'sm;
}

/* SCS_OFFSET_FOUND: C 5907 */
L_SCS_OFFSET_FOUND => {
    /* Skip remaining options. */
    loop {
        if *scs_ecode as u32 == OP_CREF {
            length += 1 + IMM2_SIZE;
            scs_ecode = scs_ecode.add(1 + IMM2_SIZE);
        } else if *scs_ecode as u32 == OP_DNCREF {
            length += 1 + 2 * IMM2_SIZE;
            scs_ecode = scs_ecode.add(1 + 2 * IMM2_SIZE);
        } else {
            break;
        }
    }

    (*F).fields.op_assert_scs.saved_end_subject = (*mb).end_subject;
    (*F).fields.op_assert_scs.true_end_extra =
        ((*mb).true_end_subject as usize) - ((*mb).end_subject as usize);
    (*F).fields.op_assert_scs.saved_eptr = Feptr!();
    (*F).fields.op_assert_scs.saved_moptions = (*mb).moptions;

    Feptr!() = (*mb).start_subject.add(*Fovector!().add(offset));
    (*mb).end_subject = (*mb).start_subject.add(*Fovector!().add(offset + 1));
    (*mb).true_end_subject = (*mb).end_subject;
    (*mb).moptions &= !PCRE2_NOTEOL;

    state = S_SCS_MATCH_LOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 5936 */
S_SCS_MATCH_LOOP => {
    group_frame_type = GF_NOCAPTURE;
    /* RMATCH(Fecode + 1 + LINK_SIZE + length, RM38) */
    start_ecode = Fecode!().add(1 + LINK_SIZE + length);
    Freturn_id!() = RM38 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM38 => {
    if rrc == MATCH_ACCEPT {
        copy_nonoverlapping(
            (assert_accept_frame as *const u8).add(OVECTOR_OFFSET_IN_HEAPFRAME),
            Fovector!() as *mut u8,
            (*assert_accept_frame).offset_top * core::mem::size_of::<PCRE2_SIZE>(),
        );
        Foffset_top!() = (*assert_accept_frame).offset_top;
        Fmark!() = (*assert_accept_frame).mark;
        (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
        (*mb).true_end_subject = (*mb)
            .end_subject
            .add((*F).fields.op_assert_scs.true_end_extra);
        (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
        /* break out of the for(;;) */
        loop {
            Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
            if *Fecode!() as u32 != OP_ALT {
                break;
            }
        }
        Fecode!() = Fecode!().add(1 + LINK_SIZE);
        Feptr!() = (*F).fields.op_assert_scs.saved_eptr;
        state = S_MAINLOOP;
        continue 'sm;
    }

    if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
        (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
        (*mb).true_end_subject = (*mb)
            .end_subject
            .add((*F).fields.op_assert_scs.true_end_extra);
        (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
    if *Fecode!() as u32 != OP_ALT {
        (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
        (*mb).true_end_subject = (*mb)
            .end_subject
            .add((*F).fields.op_assert_scs.true_end_extra);
        (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    length = 0;
    state = S_SCS_MATCH_LOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 6102 (assertion condition of OP_COND) */
S_COND_ASSERT_LOOP => {
    group_frame_type = GF_CONDASSERT;
    /* RMATCH(Lstart_branch + PRIV(OP_lengths)[*Lstart_branch], RM5) */
    start_ecode = (*F).fields.op_cond.start_branch.add(
        crate::tables::_pcre2_OP_lengths_8[*(*F).fields.op_cond.start_branch as usize] as usize,
    );
    Freturn_id!() = RM5 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM5 => {
    match rrc {
        /* Save captures */
        MATCH_ACCEPT => {
            copy_nonoverlapping(
                (assert_accept_frame as *const u8).add(OVECTOR_OFFSET_IN_HEAPFRAME),
                Fovector!() as *mut u8,
                (*assert_accept_frame).offset_top * core::mem::size_of::<PCRE2_SIZE>(),
            );
            Foffset_top!() = (*assert_accept_frame).offset_top;

            /* Fall through into MATCH_MATCH */
            /* In the case of a match, the captures have already been put into
            the current frame. */
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
                .add(GET!((*F).fields.op_cond.start_branch, 1) as usize);
            if *(*F).fields.op_cond.start_branch as u32 == OP_ALT {
                /* Try next branch */
                state = S_COND_ASSERT_LOOP;
                continue 'sm;
            }
            condition = ((*F).byte1 == 0) as BOOL; /* TRUE for negative assertion */
        }

        /* These force no match without checking other branches. */
        MATCH_COMMIT | MATCH_SKIP | MATCH_PRUNE => {
            condition = ((*F).byte1 == 0) as BOOL;
        }

        _ => {
            /* RRETURN(rrc) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }

    /* break out of the branch loop */

    /* If the condition is true, find the end of the assertion so that
    advancing past it gets us to the start of the first branch. */

    if condition != 0 {
        loop {
            Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
            if *Fecode!() as u32 != OP_ALT {
                break;
            }
        }
    }
    /* End of assertion condition */
    state = S_COND_CHOOSE;
    continue 'sm;
}

/* C 6157: choose branch according to the condition. */
S_COND_CHOOSE => {
    Fecode!() = Fecode!().add(if condition != 0 {
        crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize
    } else {
        (*F).fields.op_cond.length
    });

    /* If the opcode is OP_SCOND it means we are at a repeated conditional
    group that might match an empty string. We must therefore descend a level
    so that the start is remembered for checking. For OP_COND we can just
    continue at this level. */

    if Fop!() as u32 == OP_SCOND {
        group_frame_type = GF_NOCAPTURE;
        /* RMATCH(Fecode, RM35) */
        start_ecode = Fecode!();
        Freturn_id!() = RM35 as u8;
        state = S_MATCH_RECURSE;
        continue 'sm;
    }
    state = S_MAINLOOP;
    continue 'sm;
}

RM35 => {
    /* RRETURN(rrc) */
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* Top of the for(;;) at C 6272 (OP_VREVERSE) */
S_VREVERSE_LOOP => {
    /* RMATCH(Fecode + 1 + 2 * IMM2_SIZE, RM37) */
    start_ecode = Fecode!().add(1 + 2 * IMM2_SIZE);
    Freturn_id!() = RM37 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM37 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    /* if (Lmax-- <= Lmin) RRETURN(MATCH_NOMATCH); */
    let old_max: u32 = (*F).fields.op_vreverse.max;
    (*F).fields.op_vreverse.max = old_max.wrapping_sub(1);
    if old_max <= (*F).fields.op_vreverse.min {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().add(1);
    if utf != 0 {
        FORWARDCHARTEST!(Feptr!(), (*mb).end_subject);
    }
    state = S_VREVERSE_LOOP;
    continue 'sm;
}

/* C 6469: resume after the RMATCH in the OP_ASSERT_SCS case of the OP_KET
starting-opcode switch. */
RM39 => {
    (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
    (*mb).true_end_subject = (*mb).end_subject;
    /* RRETURN(rrc) */
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6548: resume after the OP_KETRMIN RMATCH. */
RM6 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().sub(GET!(Fecode!(), 1) as usize);
    /* End of ket processing */
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 6556: resume after the OP_KETRMAX RMATCH. */
RM7 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1 + LINK_SIZE);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ASSERT_NL_OR_EOS: C 6604 */
L_ASSERT_NL_OR_EOS => {
    if Feptr!() < (*mb).true_end_subject
        && (IS_NEWLINE!(Feptr!()) == 0
            || Feptr!() != (*mb).true_end_subject.sub((*mb).nllen as usize))
    {
        if (*mb).partial != 0
            && Feptr!().add(1) >= (*mb).end_subject
            && (*mb).nltype == NLTYPE_FIXED
            && (*mb).nllen == 2
            && *Feptr!() == (*mb).nl[0]
        {
            (*mb).hitend = TRUE;
            if (*mb).partial > 1 {
                return PCRE2_ERROR_PARTIAL;
            }
        }
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    /* Either at end of string or \n before end. */

    if (*mb).partial != 0 {
        (*mb).hitend = TRUE;
        if (*mb).partial > 1 {
            return PCRE2_ERROR_PARTIAL;
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 6788: resume after the OP_MARK RMATCH. */
RM12 => {
    /* A return of MATCH_SKIP_ARG means that matching failed at SKIP with an
    argument, and we must check whether that argument matches this MARK's
    argument. It is passed back in mb->verb_skip_ptr. If it does match, we
    return MATCH_SKIP with mb->verb_skip_ptr now pointing to the subject
    position that corresponds to this mark. Otherwise, pass back the return
    code unaltered. */

    if rrc == MATCH_SKIP_ARG
        && crate::string_utils::_pcre2_strcmp_8(Fecode!().add(2), (*mb).verb_skip_ptr) == 0
    {
        (*mb).verb_skip_ptr = Feptr!(); /* Pass back current position */
        /* RRETURN(MATCH_SKIP) */
        rrc = MATCH_SKIP;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    /* RRETURN(rrc) */
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6805: resume after the OP_COMMIT RMATCH. */
RM13 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_COMMIT) */
    rrc = MATCH_COMMIT;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6812: resume after the OP_COMMIT_ARG RMATCH. */
RM36 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_COMMIT) */
    rrc = MATCH_COMMIT;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6818: resume after the OP_PRUNE RMATCH. */
RM14 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_PRUNE) */
    rrc = MATCH_PRUNE;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6825: resume after the OP_PRUNE_ARG RMATCH. */
RM15 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_PRUNE) */
    rrc = MATCH_PRUNE;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6831: resume after the OP_SKIP RMATCH. */
RM16 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_skip_ptr = Feptr!(); /* Pass back current position */
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_SKIP) */
    rrc = MATCH_SKIP;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6852: resume after the OP_SKIP_ARG RMATCH. */
RM17 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    /* Pass back the current skip name and return the special MATCH_SKIP_ARG
    return code. This will either be caught by a matching MARK, or get to the
    top, where it causes a rematch with mb->ignore_skip_arg set to the value of
    mb->skip_arg_count. */

    (*mb).verb_skip_ptr = Fecode!().add(2);
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_SKIP_ARG) */
    rrc = MATCH_SKIP_ARG;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6868: resume after the OP_THEN RMATCH. */
RM18 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_ecode_ptr = Fecode!();
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_THEN) */
    rrc = MATCH_THEN;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6876: resume after the OP_THEN_ARG RMATCH. */
RM19 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_ecode_ptr = Fecode!();
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_THEN) */
    rrc = MATCH_THEN;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 5495: resume after the OP_BRAZERO RMATCH. */
RM9 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    let mut next_ecode: PCRE2_SPTR = Fecode!();
    loop {
        next_ecode = next_ecode.add(GET!(next_ecode, 1) as usize);
        if *next_ecode as u32 != OP_ALT {
            break;
        }
    }
    Fecode!() = next_ecode.add(1 + LINK_SIZE);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 5510: resume after the OP_BRAMINZERO RMATCH. */
RM10 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = S_MAINLOOP;
    continue 'sm;
}
