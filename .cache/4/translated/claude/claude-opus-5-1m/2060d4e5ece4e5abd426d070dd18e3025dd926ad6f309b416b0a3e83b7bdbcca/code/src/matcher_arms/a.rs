{
/* ===================================================================== */
/* Opcode arms chunk "a": C lines 900-1659 of pcre2_match.c.
   Contains OP_CLOSE, OP_ASSERT_ACCEPT, OP_ACCEPT/OP_END, OP_ANY/OP_ALLANY,
   OP_ANYBYTE, OP_CHAR, OP_CHARI, OP_NOT/OP_NOTI, the single-character
   repeat opcodes, and the REPEATCHAR label block, plus the RMATCH
   continuations RM202, RM203, RM25, RM26, RM27, RM28.                    */
/* ===================================================================== */

if lbl == LBL_SWITCH {
    match (*F).op as u32 {

    /* ===================================================================== */
    /* Before OP_ACCEPT there may be any number of OP_CLOSE opcodes, to close
    any currently open capturing brackets. Unlike reaching the end of a group,
    where we know the starting frame is at the top of the chained frames, in
    this case we have to search back for the relevant frame in case other types
    of group that use chained frames have intervened. Multiple OP_CLOSEs always
    come innermost first, which matches the chain order. We can ignore this in
    a recursion, because captures are not passed out of recursions. */

    /* case OP_CLOSE: (C 900) */
    OP_CLOSE => {
        if (*F).current_recurse == RECURSE_UNSET {
            number = GET2((*F).ecode, 1);
            offset = (*F).last_group_offset;
            loop {
                /* Corrupted heapframes?. Trigger an assert and return an error */
                /* PCRE2_ASSERT(offset != PCRE2_UNSET); */
                if offset == PCRE2_UNSET {
                    return PCRE2_ERROR_INTERNAL;
                }

                N = frame_at((*match_data).heapframes, offset);
                P = frame_sub(N, frame_size);
                if (*N).group_frame_type == (GF_CAPTURE | number) {
                    break;
                }
                offset = (*P).last_group_offset;
            }
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
        (*F).ecode = (*F)
            .ecode
            .add(_pcre2_OP_lengths_8[*(*F).ecode as usize] as usize);
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    /* ===================================================================== */
    /* Real or forced end of the pattern, assertion, or recursion. In an
    assertion ACCEPT, update the last used pointer and remember the current
    frame so that the captures and mark can be fished out of it. */

    /* case OP_ASSERT_ACCEPT: (C 931) */
    OP_ASSERT_ACCEPT => {
        if (*F).eptr > (*mb).last_used_ptr {
            (*mb).last_used_ptr = (*F).eptr;
        }
        assert_accept_frame = F;
        rrc = MATCH_ACCEPT;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }

    /* For ACCEPT within a recursion, we have to find the most recent
    recursion. If not in a recursion, fall through to code that is common with
    OP_END. */

    /* case OP_ACCEPT: (C 940) / case OP_END: (C 976) */
    OP_ACCEPT | OP_END => {
        if (*F).op as u32 == OP_ACCEPT && (*F).current_recurse != RECURSE_UNSET {
            offset = (*F).last_group_offset;
            loop {
                /* Corrupted heapframes?. Trigger an assert and return an error */
                /* PCRE2_ASSERT(offset != PCRE2_UNSET); */
                if offset == PCRE2_UNSET {
                    return PCRE2_ERROR_INTERNAL;
                }

                N = frame_at((*match_data).heapframes, offset);
                P = frame_sub(N, frame_size);
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
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* OP_END itself can never be reached within a recursion because that is
        picked up when the OP_KET that always precedes OP_END is reached. */

        /* Fail for an empty string match if either PCRE2_NOTEMPTY is set, or if
        PCRE2_NOTEMPTY_ATSTART is set and we have matched at the start of the
        subject. In both cases, backtracking will then try other alternatives, if
        any. */

        if (*F).eptr == (*F).start_match
            && (((*mb).moptions & PCRE2_NOTEMPTY) != 0
                || (((*mb).moptions & PCRE2_NOTEMPTY_ATSTART) != 0
                    && (*F).start_match == (*mb).start_subject.add((*mb).start_offset)))
        {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }

        /* Fail if PCRE2_ENDANCHORED is set and the end of the match is not
        the end of the subject. After (*ACCEPT) we fail the entire match (at this
        position) but backtrack if we've reached the end of the pattern. This
        applies whether or not we are in a recursion. */

        if (*F).eptr < (*mb).end_subject
            && (((*mb).moptions | (*mb).poptions) & PCRE2_ENDANCHORED) != 0
        {
            if (*F).op as u32 == OP_END {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
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
            /* PCRE2_ASSERT(mb->hasbsk); */

            if (*mb).allowlookaroundbsk == FALSE {
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

        let md_ovector: *mut PCRE2_SIZE =
            core::ptr::addr_of_mut!((*match_data).ovector) as *mut PCRE2_SIZE;
        *md_ovector.add(0) =
            (*F).start_match.offset_from((*mb).start_subject) as PCRE2_SIZE;
        *md_ovector.add(1) = (*F).eptr.offset_from((*mb).start_subject) as PCRE2_SIZE;

        /* Set i to the smaller of the sizes of the external and frame ovectors. */

        i = 2u32.wrapping_mul(
            if (top_bracket as c_int + 1) > (*match_data).oveccount as c_int {
                (*match_data).oveccount as u32
            } else {
                (top_bracket as u32).wrapping_add(1)
            },
        );
        memcpy(
            md_ovector.add(2) as *mut c_void,
            ovec(F) as *const c_void,
            (i as usize).wrapping_sub(2) * core::mem::size_of::<PCRE2_SIZE>(),
        );
        loop {
            i = i.wrapping_sub(1);
            if !((i as PCRE2_SIZE) >= (*F).offset_top + 2) {
                break;
            }
            *md_ovector.add(i as usize) = PCRE2_UNSET;
        }
        return MATCH_MATCH; /* Note: NOT RRETURN */
    }

    /*===================================================================== */
    /* Match any single character type except newline; have to take care with
    CRLF newlines and partial matching. */

    /* case OP_ANY: (C 1061) / case OP_ALLANY: (C 1076) */
    OP_ANY | OP_ALLANY => {
        if (*F).op as u32 == OP_ANY {
            /* IS_NEWLINE(Feptr) */
            let is_nl: bool = if (*mb).nltype != NLTYPE_FIXED {
                (*F).eptr < (*mb).end_subject
                    && crate::newline::_pcre2_is_newline_8(
                        (*F).eptr,
                        (*mb).nltype,
                        (*mb).end_subject,
                        &mut (*mb).nllen,
                        utf,
                    ) != FALSE
            } else {
                (*F).eptr <= (*mb).end_subject.wrapping_sub((*mb).nllen as usize)
                    && *(*F).eptr as u32 == (*mb).nl[0] as u32
                    && ((*mb).nllen == 1 || *(*F).eptr.add(1) as u32 == (*mb).nl[1] as u32)
            };
            if is_nl {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            if (*mb).partial != 0
                && (*F).eptr == (*mb).end_subject.wrapping_sub(1)
                && (*mb).nltype == NLTYPE_FIXED
                && (*mb).nllen == 2
                && *(*F).eptr as u32 == (*mb).nl[0] as u32
            {
                (*mb).hitend = TRUE;
                if (*mb).partial > 1 {
                    return PCRE2_ERROR_PARTIAL;
                }
            }
        }

        /* Match any single character whatsoever. */

        if (*F).eptr >= (*mb).end_subject
        /* DO NOT merge the Feptr++ here; it must */
        {
            /* not be updated before SCHECK_PARTIAL. */
            /* SCHECK_PARTIAL() */
            if (*mb).partial != 0
                && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
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
        (*F).eptr = (*F).eptr.add(1);
        if utf != FALSE {
            /* ACROSSCHAR(Feptr < mb->end_subject, Feptr, Feptr++) */
            while (*F).eptr < (*mb).end_subject && (*(*F).eptr as u32 & 0xc0) == 0x80 {
                (*F).eptr = (*F).eptr.add(1);
            }
        }
        (*F).ecode = (*F).ecode.add(1);
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    /* ===================================================================== */
    /* Match a single code unit, even in UTF mode. This opcode really does
    match any code unit, even newline. (It really should be called ANYCODEUNIT,
    of course - the byte name is from pre-16 bit days.) */

    /* case OP_ANYBYTE: (C 1095) */
    OP_ANYBYTE => {
        if (*F).eptr >= (*mb).end_subject
        /* DO NOT merge the Feptr++ here; it must */
        {
            /* not be updated before SCHECK_PARTIAL. */
            /* SCHECK_PARTIAL() */
            if (*mb).partial != 0
                && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
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
        (*F).eptr = (*F).eptr.add(1);
        (*F).ecode = (*F).ecode.add(1);
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    /* ===================================================================== */
    /* Match a single character, casefully */

    /* case OP_CHAR: (C 1109) */
    OP_CHAR => {
        if utf != FALSE {
            length = 1;
            (*F).ecode = (*F).ecode.add(1);
            /* GETCHARLEN(fc, Fecode, length) */
            fc = *(*F).ecode as u32;
            if fc >= 0xc0 {
                length += utf8_extra(fc);
                fc = getutf8(fc, (*F).ecode);
            }
            if length > ((*mb).end_subject.offset_from((*F).eptr) as PCRE2_SIZE) {
                /* CHECK_PARTIAL() -- Not SCHECK_PARTIAL() */
                if (*F).eptr >= (*mb).end_subject {
                    if (*mb).partial != 0
                        && ((*F).eptr > (*mb).start_used_ptr
                            || (*mb).allowemptypartial != FALSE)
                    {
                        (*mb).hitend = TRUE;
                        if (*mb).partial > 1 {
                            return PCRE2_ERROR_PARTIAL;
                        }
                    }
                }
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            while length > 0 {
                let ec = *(*F).ecode;
                (*F).ecode = (*F).ecode.add(1);
                let sc = *(*F).eptr;
                (*F).eptr = (*F).eptr.add(1);
                if ec != sc {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                length -= 1;
            }
        } else {
            /* Not UTF mode */
            if (*mb).end_subject.offset_from((*F).eptr) < 1 {
                /* This one can use SCHECK_PARTIAL() */
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
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
            let sc = *(*F).eptr;
            (*F).eptr = (*F).eptr.add(1);
            if *(*F).ecode.add(1) != sc {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).ecode = (*F).ecode.add(2);
        }
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    /* ===================================================================== */
    /* Match a single character, caselessly. If we are at the end of the
    subject, give up immediately. We get here only when the pattern character
    has at most one other case. Characters with more than two cases are coded
    as OP_PROP with the pseudo-property PT_CLIST. */

    /* case OP_CHARI: (C 1148) */
    OP_CHARI => {
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
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }

        if utf != FALSE {
            length = 1;
            (*F).ecode = (*F).ecode.add(1);
            /* GETCHARLEN(fc, Fecode, length) */
            fc = *(*F).ecode as u32;
            if fc >= 0xc0 {
                length += utf8_extra(fc);
                fc = getutf8(fc, (*F).ecode);
            }

            /* If the pattern character's value is < 128, we know that its other case
            (if any) is also < 128 (and therefore only one code unit long in all
            code-unit widths), so we can use the fast lookup table. We checked above
            that there is at least one character left in the subject. */

            if fc < 128 {
                let cc: u32 = *(*F).eptr as u32;
                if *(*mb).lcc.add(fc as usize) as u32 != TABLE_GET(cc, (*mb).lcc, cc) {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                (*F).ecode = (*F).ecode.add(1);
                (*F).eptr = (*F).eptr.add(1);
            }
            /* Otherwise we must pick up the subject character and use Unicode
            property support to test its other case. Note that we cannot use the
            value of "length" to check for sufficient bytes left, because the other
            case of the character may have more or fewer code units. */
            else {
                /* GETCHARINC(dc, Feptr) */
                let mut dc: u32 = *(*F).eptr as u32;
                (*F).eptr = (*F).eptr.add(1);
                if dc >= 0xc0 {
                    let r = getutf8inc(dc, (*F).eptr);
                    dc = r.0;
                    (*F).eptr = r.1;
                }
                (*F).ecode = (*F).ecode.add(length);
                if dc != fc && dc != UCD_OTHERCASE(fc) {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            }
        }
        /* If UCP is set without UTF we must do the same as above, but with one
        character per code unit. */
        else if ucp != FALSE {
            let cc: u32 = *(*F).eptr as u32;
            fc = *(*F).ecode.add(1) as u32;
            if fc < 128 {
                if *(*mb).lcc.add(fc as usize) as u32 != TABLE_GET(cc, (*mb).lcc, cc) {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            } else {
                if cc != fc && cc != UCD_OTHERCASE(fc) {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            }
            (*F).eptr = (*F).eptr.add(1);
            (*F).ecode = (*F).ecode.add(2);
        }
        /* Not UTF or UCP mode; use the table for characters < 256. */
        else {
            if TABLE_GET(
                *(*F).ecode.add(1) as u32,
                (*mb).lcc,
                *(*F).ecode.add(1) as u32,
            ) != TABLE_GET(*(*F).eptr as u32, (*mb).lcc, *(*F).eptr as u32)
            {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).eptr = (*F).eptr.add(1);
            (*F).ecode = (*F).ecode.add(2);
        }
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    /* ===================================================================== */
    /* Match not a single character. */

    /* case OP_NOT: (C 1224) / case OP_NOTI: (C 1225) */
    OP_NOT | OP_NOTI => {
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
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }

        if utf != FALSE {
            let mut ch: u32;
            (*F).ecode = (*F).ecode.add(1);
            /* GETCHARINC(ch, Fecode) */
            ch = *(*F).ecode as u32;
            (*F).ecode = (*F).ecode.add(1);
            if ch >= 0xc0 {
                let r = getutf8inc(ch, (*F).ecode);
                ch = r.0;
                (*F).ecode = r.1;
            }
            /* GETCHARINC(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            if ch == fc {
                rrc = MATCH_NOMATCH; /* Caseful match */
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            } else if (*F).op as u32 == OP_NOTI
            /* If caseless */
            {
                if ch > 127 {
                    ch = UCD_OTHERCASE(ch);
                } else {
                    ch = *(*mb).fcc.add(ch as usize) as u32;
                }
                if ch == fc {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            }
        }
        /* UCP without UTF is as above, but with one character per code unit. */
        else if ucp != FALSE {
            let mut ch: u32;
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            ch = *(*F).ecode.add(1) as u32;
            (*F).ecode = (*F).ecode.add(2);

            if ch == fc {
                rrc = MATCH_NOMATCH; /* Caseful match */
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            } else if (*F).op as u32 == OP_NOTI
            /* If caseless */
            {
                if ch > 127 {
                    ch = UCD_OTHERCASE(ch);
                } else {
                    ch = *(*mb).fcc.add(ch as usize) as u32;
                }
                if ch == fc {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            }
        }
        /* Neither UTF nor UCP is set */
        else {
            let ch: u32 = *(*F).ecode.add(1) as u32;
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if ch == fc
                || ((*F).op as u32 == OP_NOTI && TABLE_GET(ch, (*mb).fcc, ch) == fc)
            {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).ecode = (*F).ecode.add(2);
        }
        lbl = LBL_TOP_OF_LOOP;
        continue 'sw;
    }

    /* ===================================================================== */
    /* Match a single character repeatedly. */

    /* #define Llength      F->byte1
       #define Loclength    F->byte2
       #define Lstart_eptr  F->fields.char_repeat.start_eptr
       #define Lcharptr     F->fields.char_repeat.charptr
       #define Lmin         F->fields.char_repeat.min
       #define Lmax         F->fields.char_repeat.max
       #define Lc           F->fields.char_repeat.c
       #define Loc          F->fields.char_repeat.oc.oc
       #define Loccu        F->fields.char_repeat.oc.occu               */

    /* case OP_EXACT: case OP_EXACTI: (C 1304) */
    OP_EXACT | OP_EXACTI => {
        (*F).fields.char_repeat.max = GET2((*F).ecode, 1);
        (*F).fields.char_repeat.min = (*F).fields.char_repeat.max;
        (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
        lbl = LBL_REPEATCHAR;
        continue 'sw;
    }

    /* case OP_POSUPTO: case OP_POSUPTOI: (C 1310) */
    OP_POSUPTO | OP_POSUPTOI => {
        reptype = REPTYPE_POS;
        (*F).fields.char_repeat.min = 0;
        (*F).fields.char_repeat.max = GET2((*F).ecode, 1);
        (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
        lbl = LBL_REPEATCHAR;
        continue 'sw;
    }

    /* case OP_UPTO: case OP_UPTOI: (C 1318) */
    OP_UPTO | OP_UPTOI => {
        reptype = REPTYPE_MAX;
        (*F).fields.char_repeat.min = 0;
        (*F).fields.char_repeat.max = GET2((*F).ecode, 1);
        (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
        lbl = LBL_REPEATCHAR;
        continue 'sw;
    }

    /* case OP_MINUPTO: case OP_MINUPTOI: (C 1326) */
    OP_MINUPTO | OP_MINUPTOI => {
        reptype = REPTYPE_MIN;
        (*F).fields.char_repeat.min = 0;
        (*F).fields.char_repeat.max = GET2((*F).ecode, 1);
        (*F).ecode = (*F).ecode.add(1 + IMM2_SIZE);
        lbl = LBL_REPEATCHAR;
        continue 'sw;
    }

    /* case OP_POSSTAR: case OP_POSSTARI: (C 1334) */
    OP_POSSTAR | OP_POSSTARI => {
        reptype = REPTYPE_POS;
        (*F).fields.char_repeat.min = 0;
        (*F).fields.char_repeat.max = u32::MAX;
        (*F).ecode = (*F).ecode.add(1);
        lbl = LBL_REPEATCHAR;
        continue 'sw;
    }

    /* case OP_POSPLUS: case OP_POSPLUSI: (C 1342) */
    OP_POSPLUS | OP_POSPLUSI => {
        reptype = REPTYPE_POS;
        (*F).fields.char_repeat.min = 1;
        (*F).fields.char_repeat.max = u32::MAX;
        (*F).ecode = (*F).ecode.add(1);
        lbl = LBL_REPEATCHAR;
        continue 'sw;
    }

    /* case OP_POSQUERY: case OP_POSQUERYI: (C 1350) */
    OP_POSQUERY | OP_POSQUERYI => {
        reptype = REPTYPE_POS;
        (*F).fields.char_repeat.min = 0;
        (*F).fields.char_repeat.max = 1;
        (*F).ecode = (*F).ecode.add(1);
        lbl = LBL_REPEATCHAR;
        continue 'sw;
    }

    /* case OP_STAR: ... case OP_MINQUERYI: (C 1358-1369) */
    OP_STAR | OP_STARI | OP_MINSTAR | OP_MINSTARI | OP_PLUS | OP_PLUSI | OP_MINPLUS
    | OP_MINPLUSI | OP_QUERY | OP_QUERYI | OP_MINQUERY | OP_MINQUERYI => {
        let opcu = *(*F).ecode as u32;
        (*F).ecode = (*F).ecode.add(1);
        fc = opcu.wrapping_sub(if ((*F).op as u32) < OP_STARI {
            OP_STAR
        } else {
            OP_STARI
        });
        (*F).fields.char_repeat.min = rep_min[fc as usize];
        (*F).fields.char_repeat.max = rep_max[fc as usize];
        reptype = rep_typ[fc as usize];

        lbl = LBL_REPEATCHAR;
        continue 'sw;
    }

    _ => {}
    }
}

/* --------------------------------------------------------------------- */
/* REPEATCHAR: (C 1392)

Common code for all repeated single-character matches. We first check
for the minimum number of characters. If the minimum equals the maximum, we
are done. Otherwise, if minimizing, check the rest of the pattern for a
match; if there isn't one, advance up to the maximum, one character at a
time.

If maximizing, advance up to the maximum number of matching characters,
until Feptr is past the end of the maximum run. If possessive, we are
then done (no backing up). Otherwise, match at this position; anything
other than no match is immediately returned. For nomatch, back up one
character, unless we are matching \R and the last thing matched was
\r\n, in which case, back up two code units until we reach the first
optional character position.

The various UTF/non-UTF and caseful/caseless cases are handled separately,
for speed. */

if lbl == LBL_REPEATCHAR {
    if utf != FALSE {
        length = 1;
        (*F).fields.char_repeat.charptr = (*F).ecode;
        /* GETCHARLEN(fc, Fecode, length) */
        fc = *(*F).ecode as u32;
        if fc >= 0xc0 {
            length += utf8_extra(fc);
            fc = getutf8(fc, (*F).ecode);
        }
        (*F).ecode = (*F).ecode.add(length);
        (*F).byte1 = length as u8;

        /* Handle multi-code-unit character matching, caseful and caseless. */

        if length > 1 {
            let mut othercase: u32 = 0;

            if (*F).op as u32 >= OP_STARI /* Caseless */
                && ({
                    othercase = UCD_OTHERCASE(fc);
                    othercase != fc
                })
            {
                (*F).byte2 = crate::ord2utf::_pcre2_ord2utf_8(
                    othercase,
                    core::ptr::addr_of_mut!((*F).fields.char_repeat.oc.occu) as *mut PCRE2_UCHAR,
                ) as u8;
            } else {
                (*F).byte2 = 0;
            }

            i = 1;
            while i <= (*F).fields.char_repeat.min {
                if (*F).eptr <= (*mb).end_subject.wrapping_sub(length)
                    && memcmp(
                        (*F).eptr as *const c_void,
                        (*F).fields.char_repeat.charptr as *const c_void,
                        CU2BYTES(length),
                    ) == 0
                {
                    (*F).eptr = (*F).eptr.add(length);
                } else if (*F).byte2 > 0
                    && (*F).eptr <= (*mb).end_subject.wrapping_sub((*F).byte2 as usize)
                    && memcmp(
                        (*F).eptr as *const c_void,
                        core::ptr::addr_of!((*F).fields.char_repeat.oc.occu) as *const c_void,
                        CU2BYTES((*F).byte2 as usize),
                    ) == 0
                {
                    (*F).eptr = (*F).eptr.add((*F).byte2 as usize);
                } else {
                    /* CHECK_PARTIAL() */
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial != 0
                            && ((*F).eptr > (*mb).start_used_ptr
                                || (*mb).allowemptypartial != FALSE)
                        {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                    }
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                i = i.wrapping_add(1);
            }

            if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
                lbl = LBL_TOP_OF_LOOP;
                continue 'sw;
            }

            if reptype == REPTYPE_MIN {
                /* for (;;) { RMATCH(Fecode, RM202); ... } */
                start_ecode = (*F).ecode;
                (*F).return_id = RM202;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            } else
            /* Maximize */
            {
                (*F).fields.char_repeat.start_eptr = (*F).eptr;
                i = (*F).fields.char_repeat.min;
                while i < (*F).fields.char_repeat.max {
                    if (*F).eptr <= (*mb).end_subject.wrapping_sub((*F).byte1 as usize)
                        && memcmp(
                            (*F).eptr as *const c_void,
                            (*F).fields.char_repeat.charptr as *const c_void,
                            CU2BYTES((*F).byte1 as usize),
                        ) == 0
                    {
                        (*F).eptr = (*F).eptr.add((*F).byte1 as usize);
                    } else if (*F).byte2 > 0
                        && (*F).eptr <= (*mb).end_subject.wrapping_sub((*F).byte2 as usize)
                        && memcmp(
                            (*F).eptr as *const c_void,
                            core::ptr::addr_of!((*F).fields.char_repeat.oc.occu)
                                as *const c_void,
                            CU2BYTES((*F).byte2 as usize),
                        ) == 0
                    {
                        (*F).eptr = (*F).eptr.add((*F).byte2 as usize);
                    } else {
                        /* CHECK_PARTIAL() */
                        if (*F).eptr >= (*mb).end_subject {
                            if (*mb).partial != 0
                                && ((*F).eptr > (*mb).start_used_ptr
                                    || (*mb).allowemptypartial != FALSE)
                            {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 {
                                    return PCRE2_ERROR_PARTIAL;
                                }
                            }
                        }
                        break;
                    }
                    i = i.wrapping_add(1);
                }

                /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
                go too far. */

                if reptype != REPTYPE_POS {
                    loop {
                        if (*F).eptr <= (*F).fields.char_repeat.start_eptr {
                            break;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM203;
                        lbl = LBL_MATCH_RECURSE;
                        continue 'sw;
                    }
                }
            }
            lbl = LBL_TOP_OF_LOOP; /* End of repeated wide character handling */
            continue 'sw;
        }

        /* Length of UTF character is 1. Put it into the preserved variable and
        fall through to the non-UTF code. */

        (*F).fields.char_repeat.c = fc;
    } else {
        /* When not in UTF mode, load a single-code-unit character. Then proceed as
        above, using Unicode casing if either UTF or UCP is set. */

        (*F).fields.char_repeat.c = *(*F).ecode as u32;
        (*F).ecode = (*F).ecode.add(1);
    }

    /* Caseless comparison */

    if (*F).op as u32 >= OP_STARI {
        if ucp != FALSE && utf == FALSE && (*F).fields.char_repeat.c > 127 {
            (*F).fields.char_repeat.oc.oc = UCD_OTHERCASE((*F).fields.char_repeat.c);
        } else {
            /* Lc will be < 128 in UTF-8 mode. */
            (*F).fields.char_repeat.oc.oc =
                *(*mb).fcc.add((*F).fields.char_repeat.c as usize) as u32;
        }

        i = 1;
        while i <= (*F).fields.char_repeat.min {
            let cc: u32; /* Faster than PCRE2_UCHAR */
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
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            cc = *(*F).eptr as u32;
            if (*F).fields.char_repeat.c != cc && (*F).fields.char_repeat.oc.oc != cc {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            (*F).eptr = (*F).eptr.add(1);
            i = i.wrapping_add(1);
        }
        if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        if reptype == REPTYPE_MIN {
            /* for (;;) { RMATCH(Fecode, RM25); ... } */
            start_ecode = (*F).ecode;
            (*F).return_id = RM25;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        } else
        /* Maximize */
        {
            (*F).fields.char_repeat.start_eptr = (*F).eptr;
            i = (*F).fields.char_repeat.min;
            while i < (*F).fields.char_repeat.max {
                let cc: u32; /* Faster than PCRE2_UCHAR */
                if (*F).eptr >= (*mb).end_subject {
                    /* SCHECK_PARTIAL() */
                    if (*mb).partial != 0
                        && ((*F).eptr > (*mb).start_used_ptr
                            || (*mb).allowemptypartial != FALSE)
                    {
                        (*mb).hitend = TRUE;
                        if (*mb).partial > 1 {
                            return PCRE2_ERROR_PARTIAL;
                        }
                    }
                    break;
                }
                cc = *(*F).eptr as u32;
                if (*F).fields.char_repeat.c != cc && (*F).fields.char_repeat.oc.oc != cc {
                    break;
                }
                (*F).eptr = (*F).eptr.add(1);
                i = i.wrapping_add(1);
            }
            if reptype != REPTYPE_POS {
                loop {
                    if (*F).eptr == (*F).fields.char_repeat.start_eptr {
                        break;
                    }
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM26;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }
            }
        }
    }
    /* Caseful comparisons (includes all multi-byte characters) */
    else {
        i = 1;
        while i <= (*F).fields.char_repeat.min {
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
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            let sc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if (*F).fields.char_repeat.c != sc {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            i = i.wrapping_add(1);
        }

        if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        if reptype == REPTYPE_MIN {
            /* for (;;) { RMATCH(Fecode, RM27); ... } */
            start_ecode = (*F).ecode;
            (*F).return_id = RM27;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        } else
        /* Maximize */
        {
            (*F).fields.char_repeat.start_eptr = (*F).eptr;
            i = (*F).fields.char_repeat.min;
            while i < (*F).fields.char_repeat.max {
                if (*F).eptr >= (*mb).end_subject {
                    /* SCHECK_PARTIAL() */
                    if (*mb).partial != 0
                        && ((*F).eptr > (*mb).start_used_ptr
                            || (*mb).allowemptypartial != FALSE)
                    {
                        (*mb).hitend = TRUE;
                        if (*mb).partial > 1 {
                            return PCRE2_ERROR_PARTIAL;
                        }
                    }
                    break;
                }

                if (*F).fields.char_repeat.c != *(*F).eptr as u32 {
                    break;
                }
                (*F).eptr = (*F).eptr.add(1);
                i = i.wrapping_add(1);
            }

            if reptype != REPTYPE_POS {
                loop {
                    if (*F).eptr <= (*F).fields.char_repeat.start_eptr {
                        break;
                    }
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM28;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }
            }
        }
    }
    lbl = LBL_TOP_OF_LOOP; /* C 1633: break */
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM202) at C 1434: the REPTYPE_MIN branch
of the multi-code-unit repeated character code. */

if lbl == LBL_RM_BASE + RM202 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let oldmin = (*F).fields.char_repeat.min;
        (*F).fields.char_repeat.min = oldmin.wrapping_add(1);
        if oldmin >= (*F).fields.char_repeat.max {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        if (*F).eptr <= (*mb).end_subject.wrapping_sub((*F).byte1 as usize)
            && memcmp(
                (*F).eptr as *const c_void,
                (*F).fields.char_repeat.charptr as *const c_void,
                CU2BYTES((*F).byte1 as usize),
            ) == 0
        {
            (*F).eptr = (*F).eptr.add((*F).byte1 as usize);
        } else if (*F).byte2 > 0
            && (*F).eptr <= (*mb).end_subject.wrapping_sub((*F).byte2 as usize)
            && memcmp(
                (*F).eptr as *const c_void,
                core::ptr::addr_of!((*F).fields.char_repeat.oc.occu) as *const c_void,
                CU2BYTES((*F).byte2 as usize),
            ) == 0
        {
            (*F).eptr = (*F).eptr.add((*F).byte2 as usize);
        } else {
            /* CHECK_PARTIAL() */
            if (*F).eptr >= (*mb).end_subject {
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
            }
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        /* Loop back to RMATCH(Fecode, RM202) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM202;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM203) at C 1477: backtracking for the
maximizing multi-code-unit repeated character code. */

if lbl == LBL_RM_BASE + RM203 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).eptr = (*F).eptr.wrapping_sub(1);
        /* BACKCHAR(Feptr) */
        while (*(*F).eptr as u32 & 0xc0) == 0x80 {
            (*F).eptr = (*F).eptr.wrapping_sub(1);
        }
        /* Top of the for(;;) loop */
        if (*F).eptr <= (*F).fields.char_repeat.start_eptr {
            break;
        }
        start_ecode = (*F).ecode;
        (*F).return_id = RM203;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    lbl = LBL_TOP_OF_LOOP; /* C 1483: break */
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM25) at C 1537: minimizing caseless
single-code-unit repeat. */

if lbl == LBL_RM_BASE + RM25 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let oldmin = (*F).fields.char_repeat.min;
        (*F).fields.char_repeat.min = oldmin.wrapping_add(1);
        if oldmin >= (*F).fields.char_repeat.max {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
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
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let cc: u32 = *(*F).eptr as u32;
        if (*F).fields.char_repeat.c != cc && (*F).fields.char_repeat.oc.oc != cc {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).eptr = (*F).eptr.add(1);
        /* Loop back to RMATCH(Fecode, RM25) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM25;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM26) at C 1570: backtracking for the
maximizing caseless single-code-unit repeat. */

if lbl == LBL_RM_BASE + RM26 as u32 {
    loop {
        (*F).eptr = (*F).eptr.wrapping_sub(1);
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        /* Top of the for(;;) loop */
        if (*F).eptr == (*F).fields.char_repeat.start_eptr {
            break;
        }
        start_ecode = (*F).ecode;
        (*F).return_id = RM26;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    lbl = LBL_TOP_OF_LOOP; /* C 1633: break */
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM27) at C 1597: minimizing caseful
single-code-unit repeat. */

if lbl == LBL_RM_BASE + RM27 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let oldmin = (*F).fields.char_repeat.min;
        (*F).fields.char_repeat.min = oldmin.wrapping_add(1);
        if oldmin >= (*F).fields.char_repeat.max {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
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
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let sc = *(*F).eptr as u32;
        (*F).eptr = (*F).eptr.add(1);
        if (*F).fields.char_repeat.c != sc {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        /* Loop back to RMATCH(Fecode, RM27) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM27;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM28) at C 1627: backtracking for the
maximizing caseful single-code-unit repeat. */

if lbl == LBL_RM_BASE + RM28 as u32 {
    loop {
        (*F).eptr = (*F).eptr.wrapping_sub(1);
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        /* Top of the for(;;) loop */
        if (*F).eptr <= (*F).fields.char_repeat.start_eptr {
            break;
        }
        start_ecode = (*F).ecode;
        (*F).return_id = RM28;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    lbl = LBL_TOP_OF_LOOP; /* C 1633: break */
    continue 'sw;
}
}
