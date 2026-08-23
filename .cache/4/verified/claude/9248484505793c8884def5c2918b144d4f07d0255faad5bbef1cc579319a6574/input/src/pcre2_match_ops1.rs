/* Fragment 1 of the big switch in match(): c_src/src/pcre2_match.c lines
880-1293. Opcodes OP_CLOSE, OP_ASSERT_ACCEPT, OP_ACCEPT, OP_END, OP_ANY,
OP_ALLANY, OP_ANYBYTE, OP_CHAR, OP_CHARI, OP_NOT/OP_NOTI. */
{
    match state {
        /* ===================================================================== */
        /* Before OP_ACCEPT there may be any number of OP_CLOSE opcodes, to close
        any currently open capturing brackets. Unlike reaching the end of a group,
        where we know the starting frame is at the top of the chained frames, in
        this case we have to search back for the relevant frame in case other types
        of group that use chained frames have intervened. Multiple OP_CLOSEs always
        come innermost first, which matches the chain order. We can ignore this in
        a recursion, because captures are not passed out of recursions. */
        OP_CLOSE => {
            if (*F).current_recurse == RECURSE_UNSET {
                number = GET2!((*F).ecode, 1);
                offset = (*F).last_group_offset;
                loop {
                    /* Corrupted heapframes?. Trigger an assert and return an error */
                    if offset == PCRE2_UNSET {
                        return PCRE2_ERROR_INTERNAL;
                    }

                    N = ((*match_data).heapframes as *mut u8).add(offset) as *mut heapframe;
                    P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                    if (*N).group_frame_type == (GF_CAPTURE | number) {
                        break;
                    }
                    offset = (*P).last_group_offset;
                }
                offset = ((number << 1) - 2) as PCRE2_SIZE;
                (*F).capture_last = number;
                Fov!(offset) =
                    (*P).eptr.offset_from((*mb).start_subject) as PCRE2_SIZE;
                Fov!(offset + 1) =
                    (*F).eptr.offset_from((*mb).start_subject) as PCRE2_SIZE;
                if offset >= (*F).offset_top {
                    (*F).offset_top = offset + 2;
                }
            }
            (*F).ecode = (*F)
                .ecode
                .add(*_pcre2_OP_lengths_8.as_ptr().add(*(*F).ecode as usize) as usize);
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Real or forced end of the pattern, assertion, or recursion. In an
        assertion ACCEPT, update the last used pointer and remember the current
        frame so that the captures and mark can be fished out of it. */
        OP_ASSERT_ACCEPT => {
            if (*F).eptr > (*mb).last_used_ptr {
                (*mb).last_used_ptr = (*F).eptr;
            }
            assert_accept_frame = F;
            RRETURN!(MATCH_ACCEPT);
        }

        /* For ACCEPT within a recursion, we have to find the most recent
        recursion. If not in a recursion, fall through to code that is common with
        OP_END. */
        OP_ACCEPT => {
            if (*F).current_recurse != RECURSE_UNSET {
                offset = (*F).last_group_offset;
                loop {
                    /* Corrupted heapframes?. Trigger an assert and return an error */
                    if offset == PCRE2_UNSET {
                        return PCRE2_ERROR_INTERNAL;
                    }

                    N = ((*match_data).heapframes as *mut u8).add(offset) as *mut heapframe;
                    P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                    if GF_IDMASK((*N).group_frame_type) == GF_RECURSE {
                        break;
                    }
                    offset = (*P).last_group_offset;
                }

                /* N is now the frame of the recursion; the previous frame is at the
                OP_RECURSE position. Go back there, copying the current subject position
                and mark, and the start_match position (\K might have changed it), and
                then move on past the OP_RECURSE. */

                (*P).eptr = (*F).eptr;
                (*P).mark = (*F).mark;
                (*P).start_match = (*F).start_match;
                F = P;
                (*F).ecode = (*F).ecode.add(1 + LINK_SIZE);
                state = ST_TOP;
                continue 'sm;
            }
            /* Fall through */
            state = ST_C1_1;
            continue 'sm;
        }

        /* OP_END itself can never be reached within a recursion because that is
        picked up when the OP_KET that always precedes OP_END is reached. */
        OP_END | ST_C1_1 => {
            /* Fail for an empty string match if either PCRE2_NOTEMPTY is set, or if
            PCRE2_NOTEMPTY_ATSTART is set and we have matched at the start of the
            subject. In both cases, backtracking will then try other alternatives, if
            any. */

            if (*F).eptr == (*F).start_match
                && (((*mb).moptions & PCRE2_NOTEMPTY) != 0
                    || (((*mb).moptions & PCRE2_NOTEMPTY_ATSTART) != 0
                        && (*F).start_match == (*mb).start_subject.add((*mb).start_offset)))
            {
                RRETURN!(MATCH_NOMATCH);
            }

            /* Fail if PCRE2_ENDANCHORED is set and the end of the match is not
            the end of the subject. After (*ACCEPT) we fail the entire match (at this
            position) but backtrack if we've reached the end of the pattern. This
            applies whether or not we are in a recursion. */

            if (*F).eptr < (*mb).end_subject
                && (((*mb).moptions | (*mb).poptions) & PCRE2_ENDANCHORED) != 0
            {
                if (*F).op as u32 == OP_END {
                    RRETURN!(MATCH_NOMATCH);
                }

                return MATCH_NOMATCH; /* (*ACCEPT) */
            }

            /* Fail if we detect that the start position was moved to be either after
            the end position (\K in lookahead) or before the start offset (\K in
            lookbehind). If this occurs, the pattern must have used \K in a somewhat
            sneaky way (e.g. by pattern recursion), because if the \K is actually
            syntactically inside the lookaround, it's blocked at compile-time. */

            if (*F).start_match < (*mb).start_subject.add((*mb).start_offset)
                || (*F).start_match > (*F).eptr
            {
                /* The \K expression is fairly rare. We assert it was used so that we
                catch any unexpected invalid data in start_match. */

                if (*mb).allowlookaroundbsk == 0 {
                    return PCRE2_ERROR_BAD_BACKSLASH_K;
                }
            }

            /* We have a successful match of the whole pattern. Record the result and
            then do a direct return from the function. If there is space in the offset
            vector, set any pairs that follow the highest-numbered captured string but
            are less than the number of capturing groups in the pattern to PCRE2_UNSET.
            It is documented that this happens. "Gaps" are set to PCRE2_UNSET
            dynamically. It is only those at the end that need setting here. */

            (*mb).end_match_ptr = (*F).eptr; /* Record where we ended */
            (*mb).end_offset_top = (*F).offset_top; /* and how many extracts were taken */
            (*mb).mark = (*F).mark; /* and the last success mark */
            if (*F).eptr > (*mb).last_used_ptr {
                (*mb).last_used_ptr = (*F).eptr;
            }

            *(*match_data).ovector.as_mut_ptr().add(0) =
                (*F).start_match.offset_from((*mb).start_subject) as PCRE2_SIZE;
            *(*match_data).ovector.as_mut_ptr().add(1) =
                (*F).eptr.offset_from((*mb).start_subject) as PCRE2_SIZE;

            /* Set i to the smaller of the sizes of the external and frame ovectors. */

            i = (2 * (if top_bracket as c_int + 1 > (*match_data).oveccount as c_int {
                (*match_data).oveccount as c_int
            } else {
                top_bracket as c_int + 1
            })) as u32;
            memcpy(
                (*match_data).ovector.as_mut_ptr().add(2) as *mut c_void,
                (*F).ovector.as_ptr() as *const c_void,
                (i - 2) as usize * size_of::<PCRE2_SIZE>(),
            );
            loop {
                i = i - 1;
                if !((i as PCRE2_SIZE) >= (*F).offset_top + 2) {
                    break;
                }
                *(*match_data).ovector.as_mut_ptr().add(i as usize) = PCRE2_UNSET;
            }
            return MATCH_MATCH; /* Note: NOT RRETURN */
        }

        /*===================================================================== */
        /* Match any single character type except newline; have to take care with
        CRLF newlines and partial matching. */
        OP_ANY => {
            let is_nl__: bool = IS_NEWLINE!((*F).eptr, mb, (*mb).end_subject, utf);
            if is_nl__ {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*mb).partial != 0
                && (*F).eptr == (*mb).end_subject.sub(1)
                && (*mb).nltype == NLTYPE_FIXED
                && (*mb).nllen == 2
                && *(*F).eptr == (*mb).nl[0]
            {
                (*mb).hitend = TRUE;
                if (*mb).partial > 1 {
                    return PCRE2_ERROR_PARTIAL;
                }
            }
            /* Fall through */
            state = ST_C1_2;
            continue 'sm;
        }

        /* Match any single character whatsoever. */
        OP_ALLANY | ST_C1_2 => {
            if (*F).eptr >= (*mb).end_subject
            /* DO NOT merge the Feptr++ here; it must */
            {
                /* not be updated before SCHECK_PARTIAL. */
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).eptr = (*F).eptr.add(1);
            if utf != 0 {
                ACROSSCHAR!(
                    (*F).eptr < (*mb).end_subject,
                    (*F).eptr,
                    (*F).eptr = (*F).eptr.add(1)
                );
            }
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Match a single code unit, even in UTF mode. This opcode really does
        match any code unit, even newline. (It really should be called ANYCODEUNIT,
        of course - the byte name is from pre-16 bit days.) */
        OP_ANYBYTE => {
            if (*F).eptr >= (*mb).end_subject
            /* DO NOT merge the Feptr++ here; it must */
            {
                /* not be updated before SCHECK_PARTIAL. */
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            (*F).eptr = (*F).eptr.add(1);
            (*F).ecode = (*F).ecode.add(1);
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Match a single character, casefully */
        OP_CHAR => {
            if utf != 0 {
                length = 1;
                (*F).ecode = (*F).ecode.add(1);
                GETCHARLEN!(fc, (*F).ecode, length);
                if length > (*mb).end_subject.offset_from((*F).eptr) as PCRE2_SIZE {
                    CHECK_PARTIAL!(); /* Not SCHECK_PARTIAL() */
                    RRETURN!(MATCH_NOMATCH);
                }
                while length > 0 {
                    let ec__ = *(*F).ecode;
                    (*F).ecode = (*F).ecode.add(1);
                    let ep__ = *(*F).eptr;
                    (*F).eptr = (*F).eptr.add(1);
                    if ec__ != ep__ {
                        RRETURN!(MATCH_NOMATCH);
                    }
                    length -= 1;
                }
            }
            /* Not UTF mode */
            else {
                if (*mb).end_subject.offset_from((*F).eptr) < 1 {
                    SCHECK_PARTIAL!(); /* This one can use SCHECK_PARTIAL() */
                    RRETURN!(MATCH_NOMATCH);
                }
                let ep__ = *(*F).eptr;
                (*F).eptr = (*F).eptr.add(1);
                if *(*F).ecode.add(1) != ep__ {
                    RRETURN!(MATCH_NOMATCH);
                }
                (*F).ecode = (*F).ecode.add(2);
            }
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Match a single character, caselessly. If we are at the end of the
        subject, give up immediately. We get here only when the pattern character
        has at most one other case. Characters with more than two cases are coded
        as OP_PROP with the pseudo-property PT_CLIST. */
        OP_CHARI => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }

            if utf != 0 {
                length = 1;
                (*F).ecode = (*F).ecode.add(1);
                GETCHARLEN!(fc, (*F).ecode, length);

                /* If the pattern character's value is < 128, we know that its other case
                (if any) is also < 128 (and therefore only one code unit long in all
                code-unit widths), so we can use the fast lookup table. We checked above
                that there is at least one character left in the subject. */

                if fc < 128 {
                    let cc: u32 = *(*F).eptr as u32;
                    if *(*mb).lcc.add(fc as usize) != TABLE_GET!(cc, (*mb).lcc, cc) {
                        RRETURN!(MATCH_NOMATCH);
                    }
                    (*F).ecode = (*F).ecode.add(1);
                    (*F).eptr = (*F).eptr.add(1);
                }
                /* Otherwise we must pick up the subject character and use Unicode
                property support to test its other case. Note that we cannot use the
                value of "length" to check for sufficient bytes left, because the other
                case of the character may have more or fewer code units. */
                else {
                    let mut dc: u32;
                    GETCHARINC!(dc, (*F).eptr);
                    (*F).ecode = (*F).ecode.add(length);
                    if dc != fc && dc != UCD_OTHERCASE(fc) {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }
            }
            /* If UCP is set without UTF we must do the same as above, but with one
            character per code unit. */
            else if ucp != 0 {
                let cc: u32 = *(*F).eptr as u32;
                fc = *(*F).ecode.add(1) as u32;
                if fc < 128 {
                    if *(*mb).lcc.add(fc as usize) != TABLE_GET!(cc, (*mb).lcc, cc) {
                        RRETURN!(MATCH_NOMATCH);
                    }
                } else {
                    if cc != fc && cc != UCD_OTHERCASE(fc) {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }
                (*F).eptr = (*F).eptr.add(1);
                (*F).ecode = (*F).ecode.add(2);
            }
            /* Not UTF or UCP mode; use the table for characters < 256. */
            else {
                if TABLE_GET!(*(*F).ecode.add(1), (*mb).lcc, *(*F).ecode.add(1))
                    != TABLE_GET!(*(*F).eptr, (*mb).lcc, *(*F).eptr)
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                (*F).eptr = (*F).eptr.add(1);
                (*F).ecode = (*F).ecode.add(2);
            }
            state = ST_TOP;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Match not a single character. */
        OP_NOT | OP_NOTI => {
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }

            if utf != 0 {
                let mut ch: u32;
                (*F).ecode = (*F).ecode.add(1);
                GETCHARINC!(ch, (*F).ecode);
                GETCHARINC!(fc, (*F).eptr);
                if ch == fc {
                    RRETURN!(MATCH_NOMATCH); /* Caseful match */
                } else if (*F).op as u32 == OP_NOTI
                /* If caseless */
                {
                    if ch > 127 {
                        ch = UCD_OTHERCASE(ch);
                    } else {
                        ch = *(*mb).fcc.add(ch as usize) as u32;
                    }
                    if ch == fc {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }
            }
            /* UCP without UTF is as above, but with one character per code unit. */
            else if ucp != 0 {
                let mut ch: u32;
                fc = *(*F).eptr as u32;
                (*F).eptr = (*F).eptr.add(1);
                ch = *(*F).ecode.add(1) as u32;
                (*F).ecode = (*F).ecode.add(2);

                if ch == fc {
                    RRETURN!(MATCH_NOMATCH); /* Caseful match */
                } else if (*F).op as u32 == OP_NOTI
                /* If caseless */
                {
                    if ch > 127 {
                        ch = UCD_OTHERCASE(ch);
                    } else {
                        ch = *(*mb).fcc.add(ch as usize) as u32;
                    }
                    if ch == fc {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }
            }
            /* Neither UTF nor UCP is set */
            else {
                let ch: u32 = *(*F).ecode.add(1) as u32;
                fc = *(*F).eptr as u32;
                (*F).eptr = (*F).eptr.add(1);
                if ch == fc
                    || ((*F).op as u32 == OP_NOTI
                        && TABLE_GET!(ch, (*mb).fcc, ch) as u32 == fc)
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                (*F).ecode = (*F).ecode.add(2);
            }
            state = ST_TOP;
            continue 'sm;
        }

        _ => {}
    }
}
