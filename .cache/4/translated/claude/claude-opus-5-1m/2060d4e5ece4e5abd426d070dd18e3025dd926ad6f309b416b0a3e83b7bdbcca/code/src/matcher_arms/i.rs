{
if lbl == LBL_SWITCH {
    match (*F).op as u32 {
        /* ===================================================================== */
        /* Start and end of line assertions, not multiline mode. */

        /* case OP_CIRC: Start of line, unless PCRE2_NOTBOL is set. */
        OP_CIRC => {
            if (*F).eptr != (*mb).start_subject || ((*mb).moptions & PCRE2_NOTBOL) != 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* case OP_SOD: Unconditional start of subject */
        OP_SOD => {
            if (*F).eptr != (*mb).start_subject {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* case OP_DOLL: falls through into case OP_EOD. */
        OP_DOLL | OP_EOD => {
            if (*F).op as u32 == OP_DOLL {
                if ((*mb).moptions & PCRE2_NOTEOL) != 0 {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                if ((*mb).poptions & PCRE2_DOLLAR_ENDONLY) == 0 {
                    lbl = LBL_ASSERT_NL_OR_EOS;
                    continue 'sw;
                }
                /* Fall through to OP_EOD */
            }

            /* Unconditional end of subject assertion (\z). */
            if (*F).eptr < (*mb).true_end_subject {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            if (*mb).partial != 0 {
                (*mb).hitend = TRUE;
                if (*mb).partial > 1 {
                    return PCRE2_ERROR_PARTIAL;
                }
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* End of subject or ending \n assertion (\Z) */
        OP_EODN => {
            lbl = LBL_ASSERT_NL_OR_EOS;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Start and end of line assertions, multiline mode. */

        OP_CIRCM => {
            if ((*mb).moptions & PCRE2_NOTBOL) != 0 && (*F).eptr == (*mb).start_subject {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            if (*F).eptr != (*mb).start_subject
                && (((*F).eptr == (*mb).end_subject
                    && ((*mb).poptions & PCRE2_ALT_CIRCUMFLEX) == 0)
                    || !{
                        /* WAS_NEWLINE(Feptr) */
                        let p = (*F).eptr;
                        if (*mb).nltype != NLTYPE_FIXED {
                            p > (*mb).start_subject
                                && crate::newline::_pcre2_was_newline_8(
                                    p,
                                    (*mb).nltype,
                                    (*mb).start_subject,
                                    &mut (*mb).nllen,
                                    utf,
                                ) != FALSE
                        } else {
                            p >= (*mb).start_subject.add((*mb).nllen as usize)
                                && *p.sub((*mb).nllen as usize) as u32 == (*mb).nl[0] as u32
                                && ((*mb).nllen == 1
                                    || *p.sub((*mb).nllen as usize).add(1) as u32
                                        == (*mb).nl[1] as u32)
                        }
                    })
            {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        OP_DOLLM => {
            if (*F).eptr < (*mb).end_subject {
                if !{
                    /* IS_NEWLINE(Feptr) */
                    let p = (*F).eptr;
                    if (*mb).nltype != NLTYPE_FIXED {
                        p < (*mb).end_subject
                            && crate::newline::_pcre2_is_newline_8(
                                p,
                                (*mb).nltype,
                                (*mb).end_subject,
                                &mut (*mb).nllen,
                                utf,
                            ) != FALSE
                    } else {
                        p <= (*mb).end_subject.sub((*mb).nllen as usize)
                            && *p as u32 == (*mb).nl[0] as u32
                            && ((*mb).nllen == 1 || *p.add(1) as u32 == (*mb).nl[1] as u32)
                    }
                } {
                    if (*mb).partial != 0
                        && (*F).eptr.add(1) >= (*mb).end_subject
                        && (*mb).nltype == NLTYPE_FIXED
                        && (*mb).nllen == 2
                        && *(*F).eptr as u32 == (*mb).nl[0] as u32
                    {
                        (*mb).hitend = TRUE;
                        if (*mb).partial > 1 {
                            return PCRE2_ERROR_PARTIAL;
                        }
                    }
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            } else {
                if ((*mb).moptions & PCRE2_NOTEOL) != 0 {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Start of match assertion */

        OP_SOM => {
            if (*F).eptr != (*mb).start_subject.add((*mb).start_offset) {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Reset the start of match point */

        OP_SET_SOM => {
            (*F).start_match = (*F).eptr;
            (*F).ecode = (*F).ecode.add(1);
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Word boundary assertions. */

        OP_NOT_WORD_BOUNDARY | OP_WORD_BOUNDARY | OP_NOT_UCP_WORD_BOUNDARY
        | OP_UCP_WORD_BOUNDARY => {
            if (*F).eptr == (*mb).check_subject {
                prev_is_word = FALSE;
            } else {
                let mut lastptr: PCRE2_SPTR = (*F).eptr.sub(1);
                if utf != FALSE {
                    /* BACKCHAR(lastptr) */
                    while (*lastptr & 0xc0) == 0x80 {
                        lastptr = lastptr.sub(1);
                    }
                    /* GETCHAR(fc, lastptr) */
                    fc = *lastptr as u32;
                    if fc >= 0xc0 {
                        fc = getutf8(fc, lastptr);
                    }
                } else {
                    fc = *lastptr as u32;
                }
                if lastptr < (*mb).start_used_ptr {
                    (*mb).start_used_ptr = lastptr;
                }
                if (*F).op as u32 == OP_UCP_WORD_BOUNDARY
                    || (*F).op as u32 == OP_NOT_UCP_WORD_BOUNDARY
                {
                    let chartype: c_int = UCD_CHARTYPE(fc) as c_int;
                    let category: c_int = _pcre2_ucp_gentype_8[chartype as usize] as c_int;
                    prev_is_word = ((category == ucp_L as c_int
                        || category == ucp_N as c_int
                        || chartype == ucp_Mn as c_int
                        || chartype == ucp_Pc as c_int)
                        as BOOL);
                } else {
                    prev_is_word =
                        ((CHMAX_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0)
                            as BOOL);
                }
            }

            /* Get status of next character */

            if (*F).eptr >= (*mb).end_subject {
                /* SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                cur_is_word = FALSE;
            } else {
                let mut nextptr: PCRE2_SPTR = (*F).eptr.add(1);
                if utf != FALSE {
                    /* FORWARDCHARTEST(nextptr, mb->end_subject) */
                    while nextptr < (*mb).end_subject && (*nextptr & 0xc0) == 0x80 {
                        nextptr = nextptr.add(1);
                    }
                    /* GETCHAR(fc, Feptr) */
                    fc = *(*F).eptr as u32;
                    if fc >= 0xc0 {
                        fc = getutf8(fc, (*F).eptr);
                    }
                } else {
                    fc = *(*F).eptr as u32;
                }
                if nextptr > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = nextptr;
                }
                if (*F).op as u32 == OP_UCP_WORD_BOUNDARY
                    || (*F).op as u32 == OP_NOT_UCP_WORD_BOUNDARY
                {
                    let chartype: c_int = UCD_CHARTYPE(fc) as c_int;
                    let category: c_int = _pcre2_ucp_gentype_8[chartype as usize] as c_int;
                    cur_is_word = ((category == ucp_L as c_int
                        || category == ucp_N as c_int
                        || chartype == ucp_Mn as c_int
                        || chartype == ucp_Pc as c_int)
                        as BOOL);
                } else {
                    cur_is_word =
                        ((CHMAX_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0)
                            as BOOL);
                }
            }

            /* Now see if the situation is what we want */

            {
                let ec = *(*F).ecode;
                (*F).ecode = (*F).ecode.add(1);
                let want = if ec as u32 == OP_WORD_BOUNDARY
                    || (*F).op as u32 == OP_UCP_WORD_BOUNDARY
                {
                    cur_is_word == prev_is_word
                } else {
                    cur_is_word != prev_is_word
                };
                if want {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            }
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* ===================================================================== */
        /* Backtracking (*VERB)s, with and without arguments. */

        OP_MARK => {
            (*F).mark = (*F).ecode.add(2);
            (*mb).nomatch_mark = (*F).ecode.add(2);
            start_ecode = (*F)
                .ecode
                .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize)
                .add(*(*F).ecode.add(1) as usize);
            (*F).return_id = RM12;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        OP_FAIL => {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }

        OP_COMMIT => {
            start_ecode = (*F)
                .ecode
                .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize);
            (*F).return_id = RM13;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        OP_COMMIT_ARG => {
            (*F).mark = (*F).ecode.add(2);
            (*mb).nomatch_mark = (*F).ecode.add(2);
            start_ecode = (*F)
                .ecode
                .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize)
                .add(*(*F).ecode.add(1) as usize);
            (*F).return_id = RM36;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        OP_PRUNE => {
            start_ecode = (*F)
                .ecode
                .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize);
            (*F).return_id = RM14;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        OP_PRUNE_ARG => {
            (*F).mark = (*F).ecode.add(2);
            (*mb).nomatch_mark = (*F).ecode.add(2);
            start_ecode = (*F)
                .ecode
                .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize)
                .add(*(*F).ecode.add(1) as usize);
            (*F).return_id = RM15;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        OP_SKIP => {
            start_ecode = (*F)
                .ecode
                .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize);
            (*F).return_id = RM16;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        OP_SKIP_ARG => {
            (*mb).skip_arg_count = (*mb).skip_arg_count.wrapping_add(1);
            if (*mb).skip_arg_count <= (*mb).ignore_skip_arg {
                (*F).ecode = (*F)
                    .ecode
                    .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize)
                    .add(*(*F).ecode.add(1) as usize);
                lbl = LBL_TOP_OF_LOOP;
                continue 'sw;
            }
            start_ecode = (*F)
                .ecode
                .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize)
                .add(*(*F).ecode.add(1) as usize);
            (*F).return_id = RM17;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        OP_THEN => {
            start_ecode = (*F)
                .ecode
                .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize);
            (*F).return_id = RM18;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        OP_THEN_ARG => {
            (*F).mark = (*F).ecode.add(2);
            (*mb).nomatch_mark = (*F).ecode.add(2);
            start_ecode = (*F)
                .ecode
                .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize)
                .add(*(*F).ecode.add(1) as usize);
            (*F).return_id = RM19;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        _ => {}
    }
}

/* ASSERT_NL_OR_EOS: (the label at the start of case OP_EODN) */
if lbl == LBL_ASSERT_NL_OR_EOS {
    if (*F).eptr < (*mb).true_end_subject
        && (!{
            /* IS_NEWLINE(Feptr) */
            let p = (*F).eptr;
            if (*mb).nltype != NLTYPE_FIXED {
                p < (*mb).end_subject
                    && crate::newline::_pcre2_is_newline_8(
                        p,
                        (*mb).nltype,
                        (*mb).end_subject,
                        &mut (*mb).nllen,
                        utf,
                    ) != FALSE
            } else {
                p <= (*mb).end_subject.sub((*mb).nllen as usize)
                    && *p as u32 == (*mb).nl[0] as u32
                    && ((*mb).nllen == 1 || *p.add(1) as u32 == (*mb).nl[1] as u32)
            }
        } || (*F).eptr != (*mb).true_end_subject.sub((*mb).nllen as usize))
    {
        if (*mb).partial != 0
            && (*F).eptr.add(1) >= (*mb).end_subject
            && (*mb).nltype == NLTYPE_FIXED
            && (*mb).nllen == 2
            && *(*F).eptr as u32 == (*mb).nl[0] as u32
        {
            (*mb).hitend = TRUE;
            if (*mb).partial > 1 {
                return PCRE2_ERROR_PARTIAL;
            }
        }
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }

    /* Either at end of string or \n before end. */

    if (*mb).partial != 0 {
        (*mb).hitend = TRUE;
        if (*mb).partial > 1 {
            return PCRE2_ERROR_PARTIAL;
        }
    }
    (*F).ecode = (*F).ecode.add(1);
    lbl = LBL_TOP_OF_LOOP;
    continue 'sw;
}

/* ---- RMATCH continuations owned by this chunk ---- */

if lbl == LBL_RM_BASE + RM12 as u32 {
    /* After RMATCH in case OP_MARK */
    if rrc == MATCH_SKIP_ARG
        && crate::string_utils::_pcre2_strcmp_8((*F).ecode.add(2), (*mb).verb_skip_ptr) == 0
    {
        (*mb).verb_skip_ptr = (*F).eptr; /* Pass back current position */
        rrc = MATCH_SKIP;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM13 as u32 {
    /* After RMATCH in case OP_COMMIT */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    (*mb).verb_current_recurse = (*F).current_recurse;
    rrc = MATCH_COMMIT;
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM36 as u32 {
    /* After RMATCH in case OP_COMMIT_ARG */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    (*mb).verb_current_recurse = (*F).current_recurse;
    rrc = MATCH_COMMIT;
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM14 as u32 {
    /* After RMATCH in case OP_PRUNE */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    (*mb).verb_current_recurse = (*F).current_recurse;
    rrc = MATCH_PRUNE;
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM15 as u32 {
    /* After RMATCH in case OP_PRUNE_ARG */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    (*mb).verb_current_recurse = (*F).current_recurse;
    rrc = MATCH_PRUNE;
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM16 as u32 {
    /* After RMATCH in case OP_SKIP */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    (*mb).verb_skip_ptr = (*F).eptr; /* Pass back current position */
    (*mb).verb_current_recurse = (*F).current_recurse;
    rrc = MATCH_SKIP;
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM17 as u32 {
    /* After RMATCH in case OP_SKIP_ARG */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    (*mb).verb_skip_ptr = (*F).ecode.add(2);
    (*mb).verb_current_recurse = (*F).current_recurse;
    rrc = MATCH_SKIP_ARG;
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM18 as u32 {
    /* After RMATCH in case OP_THEN */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    (*mb).verb_ecode_ptr = (*F).ecode;
    (*mb).verb_current_recurse = (*F).current_recurse;
    rrc = MATCH_THEN;
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

if lbl == LBL_RM_BASE + RM19 as u32 {
    /* After RMATCH in case OP_THEN_ARG */
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    (*mb).verb_ecode_ptr = (*F).ecode;
    (*mb).verb_current_recurse = (*F).current_recurse;
    rrc = MATCH_THEN;
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}
}
