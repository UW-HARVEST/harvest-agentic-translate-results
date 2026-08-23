{
    match state {
        /* ===================================================================== */
        /* Start and end of line assertions, not multiline mode. */

        OP_CIRC => {
            /* Start of line, unless PCRE2_NOTBOL is set. */
            if (*F).eptr != (*mb).start_subject || ((*mb).moptions & PCRE2_NOTBOL) != 0 {
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        OP_SOD => {
            /* Unconditional start of subject */
            if (*F).eptr != (*mb).start_subject {
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        /* When PCRE2_NOTEOL is unset, assert before the subject end, or a
        terminating newline unless PCRE2_DOLLAR_ENDONLY is set. */

        OP_DOLL => {
            if ((*mb).moptions & PCRE2_NOTEOL) != 0 {
                RRETURN!(MATCH_NOMATCH);
            }
            if ((*mb).poptions & PCRE2_DOLLAR_ENDONLY) == 0 {
                state = ST_ASSERT_NL_OR_EOS;
                continue 'sm;
            }

            /* Fall through */
            state = OP_EOD;
            continue 'sm;
        }

        /* Unconditional end of subject assertion (\z). */

        OP_EOD => {
            if (*F).eptr < (*mb).true_end_subject {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*mb).partial != 0 {
                (*mb).hitend = TRUE;
                if (*mb).partial > 1 {
                    return PCRE2_ERROR_PARTIAL;
                }
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        /* End of subject or ending \n assertion (\Z) */

        OP_EODN => {
            state = ST_ASSERT_NL_OR_EOS;
            continue 'sm;
        }

        ST_ASSERT_NL_OR_EOS => {
            if (*F).eptr < (*mb).true_end_subject
                && (!(IS_NEWLINE!((*F).eptr, mb, (*mb).end_subject, utf))
                    || (*F).eptr != (*mb).true_end_subject.sub((*mb).nllen as usize))
            {
                if (*mb).partial != 0
                    && (*F).eptr.add(1) >= (*mb).end_subject
                    && (*mb).nltype == NLTYPE_FIXED
                    && (*mb).nllen == 2
                    && *(*F).eptr == (*mb).nl[0]
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                RRETURN!(MATCH_NOMATCH);
            }

            /* Either at end of string or \n before end. */

            if (*mb).partial != 0 {
                (*mb).hitend = TRUE;
                if (*mb).partial > 1 {
                    return PCRE2_ERROR_PARTIAL;
                }
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Start and end of line assertions, multiline mode. */

        /* Start of subject unless notbol, or after any newline except for one at
        the very end, unless PCRE2_ALT_CIRCUMFLEX is set. */

        OP_CIRCM => {
            if ((*mb).moptions & PCRE2_NOTBOL) != 0 && (*F).eptr == (*mb).start_subject {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr != (*mb).start_subject
                && (((*F).eptr == (*mb).end_subject
                    && ((*mb).poptions & PCRE2_ALT_CIRCUMFLEX) == 0)
                    || !(WAS_NEWLINE!((*F).eptr, mb, (*mb).start_subject, utf)))
            {
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        /* Assert before any newline, or before end of subject unless noteol is
        set. */

        OP_DOLLM => {
            if (*F).eptr < (*mb).end_subject {
                if !(IS_NEWLINE!((*F).eptr, mb, (*mb).end_subject, utf)) {
                    if (*mb).partial != 0
                        && (*F).eptr.add(1) >= (*mb).end_subject
                        && (*mb).nltype == NLTYPE_FIXED
                        && (*mb).nllen == 2
                        && *(*F).eptr == (*mb).nl[0]
                    {
                        (*mb).hitend = TRUE;
                        if (*mb).partial > 1 {
                            return PCRE2_ERROR_PARTIAL;
                        }
                    }
                    RRETURN!(MATCH_NOMATCH);
                }
            } else {
                if ((*mb).moptions & PCRE2_NOTEOL) != 0 {
                    RRETURN!(MATCH_NOMATCH);
                }
                SCHECK_PARTIAL!();
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Start of match assertion */

        OP_SOM => {
            if (*F).eptr != (*mb).start_subject.add((*mb).start_offset) {
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Reset the start of match point */

        OP_SET_SOM => {
            (*F).start_match = (*F).eptr;
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Word boundary assertions. Find out if the previous and current
        characters are "word" characters. It takes a bit more work in UTF mode.
        Characters > 255 are assumed to be "non-word" characters when PCRE2_UCP is
        not set. When it is set, use Unicode properties if available, even when not
        in UTF mode. Remember the earliest and latest consulted characters. */

        OP_NOT_WORD_BOUNDARY | OP_WORD_BOUNDARY | OP_NOT_UCP_WORD_BOUNDARY
        | OP_UCP_WORD_BOUNDARY => {
            if (*F).eptr == (*mb).check_subject {
                prev_is_word = FALSE;
            } else {
                let mut lastptr: PCRE2_SPTR = (*F).eptr.sub(1);
                if utf != 0 {
                    BACKCHAR!(lastptr);
                    GETCHAR!(fc, lastptr);
                } else {
                    fc = *lastptr as u32;
                }
                if lastptr < (*mb).start_used_ptr {
                    (*mb).start_used_ptr = lastptr;
                }
                if (*F).op as u32 == OP_UCP_WORD_BOUNDARY
                    || (*F).op as u32 == OP_NOT_UCP_WORD_BOUNDARY
                {
                    let chartype: u32 = UCD_CHARTYPE(fc);
                    let category: u32 = *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize);
                    prev_is_word = (category == ucp_L
                        || category == ucp_N
                        || chartype == ucp_Mn
                        || chartype == ucp_Pc) as BOOL;
                } else {
                    prev_is_word = (CHMAX_255!(fc) != 0
                        && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0)
                        as BOOL;
                }
            }

            /* Get status of next character */

            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                cur_is_word = FALSE;
            } else {
                let mut nextptr: PCRE2_SPTR = (*F).eptr.add(1);
                if utf != 0 {
                    FORWARDCHARTEST!(nextptr, (*mb).end_subject);
                    GETCHAR!(fc, (*F).eptr);
                } else {
                    fc = *(*F).eptr as u32;
                }
                if nextptr > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = nextptr;
                }
                if (*F).op as u32 == OP_UCP_WORD_BOUNDARY
                    || (*F).op as u32 == OP_NOT_UCP_WORD_BOUNDARY
                {
                    let chartype: u32 = UCD_CHARTYPE(fc);
                    let category: u32 = *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize);
                    cur_is_word = (category == ucp_L
                        || category == ucp_N
                        || chartype == ucp_Mn
                        || chartype == ucp_Pc) as BOOL;
                } else {
                    cur_is_word = (CHMAX_255!(fc) != 0
                        && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0)
                        as BOOL;
                }
            }

            /* Now see if the situation is what we want */

            let op_here: u32 = {
                let t = *(*F).ecode;
                (*F).ecode = (*F).ecode.add(1);
                t as u32
            };
            let boundary_fails: bool =
                if op_here == OP_WORD_BOUNDARY || (*F).op as u32 == OP_UCP_WORD_BOUNDARY {
                    cur_is_word == prev_is_word
                } else {
                    cur_is_word != prev_is_word
                };
            if boundary_fails {
                RRETURN!(MATCH_NOMATCH);
            }
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Backtracking (*VERB)s, with and without arguments. Note that if the
        pattern is successfully matched, we do not come back from RMATCH. */

        OP_MARK => {
            (*mb).nomatch_mark = (*F).ecode.add(2);
            (*F).mark = (*mb).nomatch_mark;
            RMATCH!(
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize)
                    .add(*(*F).ecode.add(1) as usize),
                RM12
            );
        }

        ST_L_RM12 => {
            /* A return of MATCH_SKIP_ARG means that matching failed at SKIP with an
            argument, and we must check whether that argument matches this MARK's
            argument. It is passed back in mb->verb_skip_ptr. If it does match, we
            return MATCH_SKIP with mb->verb_skip_ptr now pointing to the subject
            position that corresponds to this mark. Otherwise, pass back the return
            code unaltered. */

            if rrc == MATCH_SKIP_ARG
                && _pcre2_strcmp_8((*F).ecode.add(2), (*mb).verb_skip_ptr) == 0
            {
                (*mb).verb_skip_ptr = (*F).eptr; /* Pass back current position */
                RRETURN!(MATCH_SKIP);
            }
            RRETURN!(rrc);
        }

        OP_FAIL => {
            RRETURN!(MATCH_NOMATCH);
        }

        /* Record the current recursing group number in mb->verb_current_recurse
        when a backtracking return such as MATCH_COMMIT is given. This enables the
        recurse processing to catch verbs from within the recursion. */

        OP_COMMIT => {
            RMATCH!(
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize),
                RM13
            );
        }

        ST_L_RM13 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*mb).verb_current_recurse = (*F).current_recurse;
            RRETURN!(MATCH_COMMIT);
        }

        OP_COMMIT_ARG => {
            (*mb).nomatch_mark = (*F).ecode.add(2);
            (*F).mark = (*mb).nomatch_mark;
            RMATCH!(
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize)
                    .add(*(*F).ecode.add(1) as usize),
                RM36
            );
        }

        ST_L_RM36 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*mb).verb_current_recurse = (*F).current_recurse;
            RRETURN!(MATCH_COMMIT);
        }

        OP_PRUNE => {
            RMATCH!(
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize),
                RM14
            );
        }

        ST_L_RM14 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*mb).verb_current_recurse = (*F).current_recurse;
            RRETURN!(MATCH_PRUNE);
        }

        OP_PRUNE_ARG => {
            (*mb).nomatch_mark = (*F).ecode.add(2);
            (*F).mark = (*mb).nomatch_mark;
            RMATCH!(
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize)
                    .add(*(*F).ecode.add(1) as usize),
                RM15
            );
        }

        ST_L_RM15 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*mb).verb_current_recurse = (*F).current_recurse;
            RRETURN!(MATCH_PRUNE);
        }

        OP_SKIP => {
            RMATCH!(
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize),
                RM16
            );
        }

        ST_L_RM16 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*mb).verb_skip_ptr = (*F).eptr; /* Pass back current position */
            (*mb).verb_current_recurse = (*F).current_recurse;
            RRETURN!(MATCH_SKIP);
        }

        /* Note that, for Perl compatibility, SKIP with an argument does NOT set
        nomatch_mark. When a pattern match ends with a SKIP_ARG for which there was
        not a matching mark, we have to re-run the match, ignoring the SKIP_ARG
        that failed and any that precede it (either they also failed, or were not
        triggered). To do this, we maintain a count of executed SKIP_ARGs. If a
        SKIP_ARG gets to top level, the match is re-run with mb->ignore_skip_arg
        set to the count of the one that failed. */

        OP_SKIP_ARG => {
            (*mb).skip_arg_count = (*mb).skip_arg_count + 1;
            if (*mb).skip_arg_count <= (*mb).ignore_skip_arg {
                (*F).ecode = (*F).ecode.add(
                    *_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize
                        + *(*F).ecode.add(1) as usize,
                );
                state = ST_TOP;
                continue 'sm;
            }
            RMATCH!(
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize)
                    .add(*(*F).ecode.add(1) as usize),
                RM17
            );
        }

        ST_L_RM17 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }

            /* Pass back the current skip name and return the special MATCH_SKIP_ARG
            return code. This will either be caught by a matching MARK, or get to the
            top, where it causes a rematch with mb->ignore_skip_arg set to the value of
            mb->skip_arg_count. */

            (*mb).verb_skip_ptr = (*F).ecode.add(2);
            (*mb).verb_current_recurse = (*F).current_recurse;
            RRETURN!(MATCH_SKIP_ARG);
        }

        /* For THEN (and THEN_ARG) we pass back the address of the opcode, so that
        the branch in which it occurs can be determined. */

        OP_THEN => {
            RMATCH!(
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize),
                RM18
            );
        }

        ST_L_RM18 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*mb).verb_ecode_ptr = (*F).ecode;
            (*mb).verb_current_recurse = (*F).current_recurse;
            RRETURN!(MATCH_THEN);
        }

        OP_THEN_ARG => {
            (*mb).nomatch_mark = (*F).ecode.add(2);
            (*F).mark = (*mb).nomatch_mark;
            RMATCH!(
                (*F)
                    .ecode
                    .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize)
                    .add(*(*F).ecode.add(1) as usize),
                RM19
            );
        }

        ST_L_RM19 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*mb).verb_ecode_ptr = (*F).ecode;
            (*mb).verb_current_recurse = (*F).current_recurse;
            RRETURN!(MATCH_THEN);
        }

        _ => {}
    }
}
