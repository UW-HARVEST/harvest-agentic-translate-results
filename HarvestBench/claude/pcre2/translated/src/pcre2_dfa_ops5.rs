/* Fragment 5 of the switch(codevalue) in internal_dfa_match():
c_src/src/pcre2_dfa_match.c lines 2411-2639.

OP_NOT, OP_NOTI and the single-character repeat families (PLUS, QUERY, STAR,
EXACT, UPTO), both caseful and caseless. In the C code the caseless opcodes set
"caseless = TRUE", subtract OP_STARI - OP_STAR from codevalue, and then fall
through into the caseful code; here the caseless and caseful labels are grouped
into one arm and codevalue is tested to reproduce the fall-through exactly. */
{
    match codevalue {
        /*-----------------------------------------------------------------*/
        /* Match a negated single character casefully. */
        OP_NOT => {
            if clen > 0 && c != d {
                ADD_NEW!(state_offset + dlen + 1, 0);
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        /* Match a negated single character caselessly. */
        OP_NOTI => {
            if clen > 0 {
                let otherd: u32;
                if utf_or_ucp != 0 && d >= 128 {
                    otherd = UCD_OTHERCASE(d);
                } else {
                    otherd = TABLE_GET!(d, fcc, d) as u32;
                }
                if c != d && c != otherd {
                    ADD_NEW!(state_offset + dlen + 1, 0);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_PLUSI
        | OP_MINPLUSI
        | OP_POSPLUSI
        | OP_NOTPLUSI
        | OP_NOTMINPLUSI
        | OP_NOTPOSPLUSI
        | OP_PLUS
        | OP_MINPLUS
        | OP_POSPLUS
        | OP_NOTPLUS
        | OP_NOTMINPLUS
        | OP_NOTPOSPLUS => {
            if codevalue == OP_PLUSI
                || codevalue == OP_MINPLUSI
                || codevalue == OP_POSPLUSI
                || codevalue == OP_NOTPLUSI
                || codevalue == OP_NOTMINPLUSI
                || codevalue == OP_NOTPOSPLUSI
            {
                caseless = TRUE;
                codevalue -= OP_STARI - OP_STAR;
                /* Fall through */
            }
            count = (*current_state).count; /* Already matched */
            if count > 0 {
                ADD_ACTIVE!(state_offset + dlen + 1, 0);
            }
            if clen > 0 {
                let mut otherd: u32 = NOTACHAR;
                if caseless != 0 {
                    if utf_or_ucp != 0 && d >= 128 {
                        otherd = UCD_OTHERCASE(d);
                    } else {
                        otherd = TABLE_GET!(d, fcc, d) as u32;
                    }
                }
                if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                    if count > 0 && (codevalue == OP_POSPLUS || codevalue == OP_NOTPOSPLUS) {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    count += 1;
                    ADD_NEW!(state_offset, count);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_QUERYI
        | OP_MINQUERYI
        | OP_POSQUERYI
        | OP_NOTQUERYI
        | OP_NOTMINQUERYI
        | OP_NOTPOSQUERYI
        | OP_QUERY
        | OP_MINQUERY
        | OP_POSQUERY
        | OP_NOTQUERY
        | OP_NOTMINQUERY
        | OP_NOTPOSQUERY => {
            if codevalue == OP_QUERYI
                || codevalue == OP_MINQUERYI
                || codevalue == OP_POSQUERYI
                || codevalue == OP_NOTQUERYI
                || codevalue == OP_NOTMINQUERYI
                || codevalue == OP_NOTPOSQUERYI
            {
                caseless = TRUE;
                codevalue -= OP_STARI - OP_STAR;
                /* Fall through */
            }
            ADD_ACTIVE!(state_offset + dlen + 1, 0);
            if clen > 0 {
                let mut otherd: u32 = NOTACHAR;
                if caseless != 0 {
                    if utf_or_ucp != 0 && d >= 128 {
                        otherd = UCD_OTHERCASE(d);
                    } else {
                        otherd = TABLE_GET!(d, fcc, d) as u32;
                    }
                }
                if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                    if codevalue == OP_POSQUERY || codevalue == OP_NOTPOSQUERY {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    ADD_NEW!(state_offset + dlen + 1, 0);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_STARI
        | OP_MINSTARI
        | OP_POSSTARI
        | OP_NOTSTARI
        | OP_NOTMINSTARI
        | OP_NOTPOSSTARI
        | OP_STAR
        | OP_MINSTAR
        | OP_POSSTAR
        | OP_NOTSTAR
        | OP_NOTMINSTAR
        | OP_NOTPOSSTAR => {
            if codevalue == OP_STARI
                || codevalue == OP_MINSTARI
                || codevalue == OP_POSSTARI
                || codevalue == OP_NOTSTARI
                || codevalue == OP_NOTMINSTARI
                || codevalue == OP_NOTPOSSTARI
            {
                caseless = TRUE;
                codevalue -= OP_STARI - OP_STAR;
                /* Fall through */
            }
            ADD_ACTIVE!(state_offset + dlen + 1, 0);
            if clen > 0 {
                let mut otherd: u32 = NOTACHAR;
                if caseless != 0 {
                    if utf_or_ucp != 0 && d >= 128 {
                        otherd = UCD_OTHERCASE(d);
                    } else {
                        otherd = TABLE_GET!(d, fcc, d) as u32;
                    }
                }
                if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                    if codevalue == OP_POSSTAR || codevalue == OP_NOTPOSSTAR {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    ADD_NEW!(state_offset, 0);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_EXACTI | OP_NOTEXACTI | OP_EXACT | OP_NOTEXACT => {
            if codevalue == OP_EXACTI || codevalue == OP_NOTEXACTI {
                caseless = TRUE;
                codevalue -= OP_STARI - OP_STAR;
                /* Fall through */
            }
            count = (*current_state).count; /* Number already matched */
            if clen > 0 {
                let mut otherd: u32 = NOTACHAR;
                if caseless != 0 {
                    if utf_or_ucp != 0 && d >= 128 {
                        otherd = UCD_OTHERCASE(d);
                    } else {
                        otherd = TABLE_GET!(d, fcc, d) as u32;
                    }
                }
                if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                    count += 1;
                    if count >= GET2!(code, 1) as c_int {
                        ADD_NEW!(state_offset + dlen + 1 + IMM2_SIZE as c_int, 0);
                    } else {
                        ADD_NEW!(state_offset, count);
                    }
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_UPTOI
        | OP_MINUPTOI
        | OP_POSUPTOI
        | OP_NOTUPTOI
        | OP_NOTMINUPTOI
        | OP_NOTPOSUPTOI
        | OP_UPTO
        | OP_MINUPTO
        | OP_POSUPTO
        | OP_NOTUPTO
        | OP_NOTMINUPTO
        | OP_NOTPOSUPTO => {
            if codevalue == OP_UPTOI
                || codevalue == OP_MINUPTOI
                || codevalue == OP_POSUPTOI
                || codevalue == OP_NOTUPTOI
                || codevalue == OP_NOTMINUPTOI
                || codevalue == OP_NOTPOSUPTOI
            {
                caseless = TRUE;
                codevalue -= OP_STARI - OP_STAR;
                /* Fall through */
            }
            ADD_ACTIVE!(state_offset + dlen + 1 + IMM2_SIZE as c_int, 0);
            count = (*current_state).count; /* Number already matched */
            if clen > 0 {
                let mut otherd: u32 = NOTACHAR;
                if caseless != 0 {
                    if utf_or_ucp != 0 && d >= 128 {
                        otherd = UCD_OTHERCASE(d);
                    } else {
                        otherd = TABLE_GET!(d, fcc, d) as u32;
                    }
                }
                if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                    if codevalue == OP_POSUPTO || codevalue == OP_NOTPOSUPTO {
                        active_count -= 1; /* Remove non-match possibility */
                        next_active_state = next_active_state.sub(1);
                    }
                    count += 1;
                    if count >= GET2!(code, 1) as c_int {
                        ADD_NEW!(state_offset + dlen + 1 + IMM2_SIZE as c_int, 0);
                    } else {
                        ADD_NEW!(state_offset, count);
                    }
                }
            }
            break 'next_active_state;
        }

        _ => {}
    }
}
