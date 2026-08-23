/* Fragment 4 of the match() opcode switch: OP_NCLASS/OP_CLASS, OP_XCLASS and
OP_ECLASS. Translated from c_src/src/pcre2_match.c lines 2045..2573.

The "Lxxx" macros of the C source are expanded by hand:

  class group  (Lbyte_map_address, Lbyte_map, Lstart_eptr, Lmin, Lmax)
                 -> (*F).fields.class_repeat.*
  xclass group (Lstart_eptr, Lxclass_data, Lmin, Lmax)
                 -> (*F).fields.xclass_repeat.*
  eclass group (Lstart_eptr, Leclass_data, Leclass_len, Lmin, Lmax)
                 -> (*F).fields.eclass_repeat.*

Loops whose body contains an RMATCH() are turned into explicit states:

  ST_C4_1  minimizing class repeat, UTF          (RM200)
  ST_C4_2  minimizing class repeat, not UTF      (RM23)
  ST_C4_3  maximizing class backtrack, UTF       (RM201)
  ST_C4_4  maximizing class backtrack, not UTF   (RM24)
  ST_C4_5  minimizing xclass repeat              (RM100)
  ST_C4_6  maximizing xclass backtrack           (RM101)
  ST_C4_7  minimizing eclass repeat              (RM102)
  ST_C4_8  maximizing eclass backtrack           (RM103)
*/
{
    match state {
        /* ===================================================================== */
        /* Match a bit-mapped character class, possibly repeatedly. These opcodes
        are used when all the characters in the class have values in the range
        0-255, and either the matching is caseful, or the characters are in the
        range 0-127 when UTF processing is enabled. The only difference between
        OP_CLASS and OP_NCLASS occurs when a data character outside the range is
        encountered. */
        OP_NCLASS | OP_CLASS => {
            (*F).fields.class_repeat.byte_map_address = (*F).ecode.add(1); /* Save for matching */
            (*F).ecode = (*F).ecode.add(1 + 32); /* Advance past the item */

            /* Look past the end of the item to see if there is repeat information
            following. Then obey similar code to character type repeats. */

            match *(*F).ecode as u32 {
                OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
                | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                    fc = *(*F).ecode as u32 - OP_CRSTAR;
                    (*F).ecode = (*F).ecode.add(1);
                    (*F).fields.class_repeat.min = *rep_min.as_ptr().add(fc as usize);
                    (*F).fields.class_repeat.max = *rep_max.as_ptr().add(fc as usize);
                    reptype = *rep_typ.as_ptr().add(fc as usize);
                }

                OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                    (*F).fields.class_repeat.min = GET2!((*F).ecode, 1);
                    (*F).fields.class_repeat.max = GET2!((*F).ecode, 1 + IMM2_SIZE);
                    if (*F).fields.class_repeat.max == 0 {
                        (*F).fields.class_repeat.max = u32::MAX; /* Max 0 => infinity */
                    }
                    reptype = *rep_typ
                        .as_ptr()
                        .add((*(*F).ecode as u32 - OP_CRSTAR) as usize);
                    (*F).ecode = (*F).ecode.add(1 + 2 * IMM2_SIZE);
                }

                _ => {
                    /* No repeat follows */
                    (*F).fields.class_repeat.min = 1;
                    (*F).fields.class_repeat.max = 1;
                }
            }

            /* First, ensure the minimum number of matches are present. */

            if utf != 0 {
                i = 1;
                while i <= (*F).fields.class_repeat.min {
                    if (*F).eptr >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        RRETURN!(MATCH_NOMATCH);
                    }
                    GETCHARINC!(fc, (*F).eptr);
                    if fc > 255 {
                        if (*F).op as u32 == OP_CLASS {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    } else {
                        if (*(*F)
                            .fields
                            .class_repeat
                            .byte_map_address
                            .add((fc / 8) as usize) as u32
                            & (1u32 << (fc & 7)))
                            == 0
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    i = i.wrapping_add(1);
                }
            } else
            /* Not UTF mode */
            {
                i = 1;
                while i <= (*F).fields.class_repeat.min {
                    if (*F).eptr >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        RRETURN!(MATCH_NOMATCH);
                    }
                    fc = *(*F).eptr as u32;
                    (*F).eptr = (*F).eptr.add(1);
                    if (*(*F)
                        .fields
                        .class_repeat
                        .byte_map_address
                        .add((fc / 8) as usize) as u32
                        & (1u32 << (fc & 7)))
                        == 0
                    {
                        RRETURN!(MATCH_NOMATCH);
                    }
                    i = i.wrapping_add(1);
                }
            }

            /* If Lmax == Lmin we are done. Continue with main loop. */

            if (*F).fields.class_repeat.min == (*F).fields.class_repeat.max {
                state = ST_TOP;
                continue 'sm;
            }

            /* If minimizing, keep testing the rest of the expression and advancing
            the pointer while it matches the class. */

            if reptype == REPTYPE_MIN {
                if utf != 0 {
                    state = ST_C4_1;
                    continue 'sm;
                } else
                /* Not UTF mode */
                {
                    state = ST_C4_2;
                    continue 'sm;
                }
            }
            /* If maximizing, find the longest possible run, then work backwards. */
            else {
                (*F).fields.class_repeat.start_eptr = (*F).eptr;

                if utf != 0 {
                    i = (*F).fields.class_repeat.min;
                    while i < (*F).fields.class_repeat.max {
                        let mut len: c_int = 1;
                        if (*F).eptr >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, (*F).eptr, len);
                        if fc > 255 {
                            if (*F).op as u32 == OP_CLASS {
                                break;
                            }
                        } else {
                            if (*(*F)
                                .fields
                                .class_repeat
                                .byte_map_address
                                .add((fc / 8) as usize) as u32
                                & (1u32 << (fc & 7)))
                                == 0
                            {
                                break;
                            }
                        }
                        (*F).eptr = (*F).eptr.add(len as usize);
                        i = i.wrapping_add(1);
                    }

                    if reptype == REPTYPE_POS {
                        state = ST_TOP;
                        continue 'sm;
                    } /* No backtracking */

                    /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                    Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
                    go too far. */

                    state = ST_C4_3;
                    continue 'sm;
                } else
                /* Not UTF mode */
                {
                    i = (*F).fields.class_repeat.min;
                    while i < (*F).fields.class_repeat.max {
                        if (*F).eptr >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        fc = *(*F).eptr as u32;
                        if (*(*F)
                            .fields
                            .class_repeat
                            .byte_map_address
                            .add((fc / 8) as usize) as u32
                            & (1u32 << (fc & 7)))
                            == 0
                        {
                            break;
                        }
                        (*F).eptr = (*F).eptr.add(1);
                        i = i.wrapping_add(1);
                    }

                    if reptype == REPTYPE_POS {
                        state = ST_TOP;
                        continue 'sm;
                    } /* No backtracking */

                    state = ST_C4_4;
                    continue 'sm;
                }
            }
        }

        /* Top of the minimizing class-repeat loop, UTF mode. */
        ST_C4_1 => {
            RMATCH!((*F).ecode, RM200);
        }

        ST_L_RM200 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin_old: u32 = (*F).fields.class_repeat.min;
            (*F).fields.class_repeat.min = lmin_old.wrapping_add(1);
            if lmin_old >= (*F).fields.class_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINC!(fc, (*F).eptr);
            if fc > 255 {
                if (*F).op as u32 == OP_CLASS {
                    RRETURN!(MATCH_NOMATCH);
                }
            } else {
                if (*(*F)
                    .fields
                    .class_repeat
                    .byte_map_address
                    .add((fc / 8) as usize) as u32
                    & (1u32 << (fc & 7)))
                    == 0
                {
                    RRETURN!(MATCH_NOMATCH);
                }
            }
            state = ST_C4_1;
            continue 'sm;
        }

        /* Top of the minimizing class-repeat loop, not UTF mode. */
        ST_C4_2 => {
            RMATCH!((*F).ecode, RM23);
        }

        ST_L_RM23 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin_old: u32 = (*F).fields.class_repeat.min;
            (*F).fields.class_repeat.min = lmin_old.wrapping_add(1);
            if lmin_old >= (*F).fields.class_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if (*(*F)
                .fields
                .class_repeat
                .byte_map_address
                .add((fc / 8) as usize) as u32
                & (1u32 << (fc & 7)))
                == 0
            {
                RRETURN!(MATCH_NOMATCH);
            }
            state = ST_C4_2;
            continue 'sm;
        }

        /* Top of the maximizing class backtracking loop, UTF mode. */
        ST_C4_3 => {
            RMATCH!((*F).ecode, RM201);
        }

        ST_L_RM201 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let eptr_old: PCRE2_SPTR = (*F).eptr;
            (*F).eptr = (*F).eptr.sub(1);
            if eptr_old <= (*F).fields.class_repeat.start_eptr {
                /* Tried at original position: leave the loop */
                RRETURN!(MATCH_NOMATCH);
            }
            BACKCHAR!((*F).eptr);
            state = ST_C4_3;
            continue 'sm;
        }

        /* Top of the maximizing class backtracking loop, not UTF mode:
        while (Feptr >= Lstart_eptr) { RMATCH(Fecode, RM24); ... } */
        ST_C4_4 => {
            if !((*F).eptr >= (*F).fields.class_repeat.start_eptr) {
                RRETURN!(MATCH_NOMATCH);
            }
            RMATCH!((*F).ecode, RM24);
        }

        ST_L_RM24 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            (*F).eptr = (*F).eptr.sub(1);
            state = ST_C4_4;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Match an extended character class. In the 8-bit library, this opcode is
        encountered only when UTF-8 mode mode is supported. In the 16-bit and
        32-bit libraries, codepoints greater than 255 may be encountered even when
        UTF is not supported. */
        OP_XCLASS => {
            (*F).fields.xclass_repeat.xclass_data = (*F).ecode.add(1 + LINK_SIZE); /* Save for matching */
            (*F).ecode = (*F).ecode.add(GET!((*F).ecode, 1) as usize); /* Advance past the item */

            match *(*F).ecode as u32 {
                OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
                | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                    fc = *(*F).ecode as u32 - OP_CRSTAR;
                    (*F).ecode = (*F).ecode.add(1);
                    (*F).fields.xclass_repeat.min = *rep_min.as_ptr().add(fc as usize);
                    (*F).fields.xclass_repeat.max = *rep_max.as_ptr().add(fc as usize);
                    reptype = *rep_typ.as_ptr().add(fc as usize);
                }

                OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                    (*F).fields.xclass_repeat.min = GET2!((*F).ecode, 1);
                    (*F).fields.xclass_repeat.max = GET2!((*F).ecode, 1 + IMM2_SIZE);
                    if (*F).fields.xclass_repeat.max == 0 {
                        (*F).fields.xclass_repeat.max = u32::MAX; /* Max 0 => infinity */
                    }
                    reptype = *rep_typ
                        .as_ptr()
                        .add((*(*F).ecode as u32 - OP_CRSTAR) as usize);
                    (*F).ecode = (*F).ecode.add(1 + 2 * IMM2_SIZE);
                }

                _ => {
                    /* No repeat follows */
                    (*F).fields.xclass_repeat.min = 1;
                    (*F).fields.xclass_repeat.max = 1;
                }
            }

            /* First, ensure the minimum number of matches are present. */

            i = 1;
            while i <= (*F).fields.xclass_repeat.min {
                if (*F).eptr >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                GETCHARINCTEST!(fc, (*F).eptr, utf);
                if _pcre2_xclass_8(
                    fc,
                    (*F).fields.xclass_repeat.xclass_data,
                    (*mb).start_code,
                    utf,
                ) == 0
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                i = i.wrapping_add(1);
            }

            /* If Lmax == Lmin we can just continue with the main loop. */

            if (*F).fields.xclass_repeat.min == (*F).fields.xclass_repeat.max {
                state = ST_TOP;
                continue 'sm;
            }

            /* If minimizing, keep testing the rest of the expression and advancing
            the pointer while it matches the class. */

            if reptype == REPTYPE_MIN {
                state = ST_C4_5;
                continue 'sm;
            }
            /* If maximizing, find the longest possible run, then work backwards. */
            else {
                (*F).fields.xclass_repeat.start_eptr = (*F).eptr;
                i = (*F).fields.xclass_repeat.min;
                while i < (*F).fields.xclass_repeat.max {
                    let mut len: c_int = 1;
                    if (*F).eptr >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        break;
                    }
                    GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                    if _pcre2_xclass_8(
                        fc,
                        (*F).fields.xclass_repeat.xclass_data,
                        (*mb).start_code,
                        utf,
                    ) == 0
                    {
                        break;
                    }
                    (*F).eptr = (*F).eptr.add(len as usize);
                    i = i.wrapping_add(1);
                }

                if reptype == REPTYPE_POS {
                    state = ST_TOP;
                    continue 'sm;
                } /* No backtracking */

                /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
                go too far. */

                state = ST_C4_6;
                continue 'sm;
            }
        }

        /* Top of the minimizing xclass-repeat loop. */
        ST_C4_5 => {
            RMATCH!((*F).ecode, RM100);
        }

        ST_L_RM100 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin_old: u32 = (*F).fields.xclass_repeat.min;
            (*F).fields.xclass_repeat.min = lmin_old.wrapping_add(1);
            if lmin_old >= (*F).fields.xclass_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            if _pcre2_xclass_8(
                fc,
                (*F).fields.xclass_repeat.xclass_data,
                (*mb).start_code,
                utf,
            ) == 0
            {
                RRETURN!(MATCH_NOMATCH);
            }
            state = ST_C4_5;
            continue 'sm;
        }

        /* Top of the maximizing xclass backtracking loop. */
        ST_C4_6 => {
            RMATCH!((*F).ecode, RM101);
        }

        ST_L_RM101 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let eptr_old: PCRE2_SPTR = (*F).eptr;
            (*F).eptr = (*F).eptr.sub(1);
            if eptr_old <= (*F).fields.xclass_repeat.start_eptr {
                /* Tried at original position: leave the loop */
                RRETURN!(MATCH_NOMATCH);
            }
            if utf != 0 {
                BACKCHAR!((*F).eptr);
            }
            state = ST_C4_6;
            continue 'sm;
        }

        /* ===================================================================== */
        /* Match a complex, set-based character class. This opcodes are used when
        there is complex nesting or logical operations within the character
        class. */
        OP_ECLASS => {
            (*F).fields.eclass_repeat.eclass_data = (*F).ecode.add(1 + LINK_SIZE); /* Save for matching */
            (*F).ecode = (*F).ecode.add(GET!((*F).ecode, 1) as usize); /* Advance past the item */
            (*F).fields.eclass_repeat.eclass_len = (*F)
                .ecode
                .offset_from((*F).fields.eclass_repeat.eclass_data)
                as PCRE2_SIZE;

            match *(*F).ecode as u32 {
                OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
                | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                    fc = *(*F).ecode as u32 - OP_CRSTAR;
                    (*F).ecode = (*F).ecode.add(1);
                    (*F).fields.eclass_repeat.min = *rep_min.as_ptr().add(fc as usize);
                    (*F).fields.eclass_repeat.max = *rep_max.as_ptr().add(fc as usize);
                    reptype = *rep_typ.as_ptr().add(fc as usize);
                }

                OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                    (*F).fields.eclass_repeat.min = GET2!((*F).ecode, 1);
                    (*F).fields.eclass_repeat.max = GET2!((*F).ecode, 1 + IMM2_SIZE);
                    if (*F).fields.eclass_repeat.max == 0 {
                        (*F).fields.eclass_repeat.max = u32::MAX; /* Max 0 => infinity */
                    }
                    reptype = *rep_typ
                        .as_ptr()
                        .add((*(*F).ecode as u32 - OP_CRSTAR) as usize);
                    (*F).ecode = (*F).ecode.add(1 + 2 * IMM2_SIZE);
                }

                _ => {
                    /* No repeat follows */
                    (*F).fields.eclass_repeat.min = 1;
                    (*F).fields.eclass_repeat.max = 1;
                }
            }

            /* First, ensure the minimum number of matches are present. */

            i = 1;
            while i <= (*F).fields.eclass_repeat.min {
                if (*F).eptr >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                GETCHARINCTEST!(fc, (*F).eptr, utf);
                if _pcre2_eclass_8(
                    fc,
                    (*F).fields.eclass_repeat.eclass_data,
                    (*F)
                        .fields
                        .eclass_repeat
                        .eclass_data
                        .add((*F).fields.eclass_repeat.eclass_len),
                    (*mb).start_code,
                    utf,
                ) == 0
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                i = i.wrapping_add(1);
            }

            /* If Lmax == Lmin we can just continue with the main loop. */

            if (*F).fields.eclass_repeat.min == (*F).fields.eclass_repeat.max {
                state = ST_TOP;
                continue 'sm;
            }

            /* If minimizing, keep testing the rest of the expression and advancing
            the pointer while it matches the class. */

            if reptype == REPTYPE_MIN {
                state = ST_C4_7;
                continue 'sm;
            }
            /* If maximizing, find the longest possible run, then work backwards. */
            else {
                (*F).fields.eclass_repeat.start_eptr = (*F).eptr;
                i = (*F).fields.eclass_repeat.min;
                while i < (*F).fields.eclass_repeat.max {
                    let mut len: c_int = 1;
                    if (*F).eptr >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        break;
                    }
                    GETCHARLENTEST!(fc, (*F).eptr, len, utf);
                    if _pcre2_eclass_8(
                        fc,
                        (*F).fields.eclass_repeat.eclass_data,
                        (*F)
                            .fields
                            .eclass_repeat
                            .eclass_data
                            .add((*F).fields.eclass_repeat.eclass_len),
                        (*mb).start_code,
                        utf,
                    ) == 0
                    {
                        break;
                    }
                    (*F).eptr = (*F).eptr.add(len as usize);
                    i = i.wrapping_add(1);
                }

                if reptype == REPTYPE_POS {
                    state = ST_TOP;
                    continue 'sm;
                } /* No backtracking */

                /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
                go too far. */

                state = ST_C4_8;
                continue 'sm;
            }
        }

        /* Top of the minimizing eclass-repeat loop. */
        ST_C4_7 => {
            RMATCH!((*F).ecode, RM102);
        }

        ST_L_RM102 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let lmin_old: u32 = (*F).fields.eclass_repeat.min;
            (*F).fields.eclass_repeat.min = lmin_old.wrapping_add(1);
            if lmin_old >= (*F).fields.eclass_repeat.max {
                RRETURN!(MATCH_NOMATCH);
            }
            if (*F).eptr >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                RRETURN!(MATCH_NOMATCH);
            }
            GETCHARINCTEST!(fc, (*F).eptr, utf);
            if _pcre2_eclass_8(
                fc,
                (*F).fields.eclass_repeat.eclass_data,
                (*F)
                    .fields
                    .eclass_repeat
                    .eclass_data
                    .add((*F).fields.eclass_repeat.eclass_len),
                (*mb).start_code,
                utf,
            ) == 0
            {
                RRETURN!(MATCH_NOMATCH);
            }
            state = ST_C4_7;
            continue 'sm;
        }

        /* Top of the maximizing eclass backtracking loop. */
        ST_C4_8 => {
            RMATCH!((*F).ecode, RM103);
        }

        ST_L_RM103 => {
            if rrc != MATCH_NOMATCH {
                RRETURN!(rrc);
            }
            let eptr_old: PCRE2_SPTR = (*F).eptr;
            (*F).eptr = (*F).eptr.sub(1);
            if eptr_old <= (*F).fields.eclass_repeat.start_eptr {
                /* Tried at original position: leave the loop */
                RRETURN!(MATCH_NOMATCH);
            }
            if utf != 0 {
                BACKCHAR!((*F).eptr);
            }
            state = ST_C4_8;
            continue 'sm;
        }

        _ => {}
    }
}
