{
if lbl == LBL_SWITCH {
    match (*F).op as u32 {
    /* ===================================================================== */
    /* Match a bit-mapped character class, possibly repeatedly. These opcodes
    are used when all the characters in the class have values in the range
    0-255, and either the matching is caseful, or the characters are in the
    range 0-127 when UTF processing is enabled. The only difference between
    OP_CLASS and OP_NCLASS occurs when a data character outside the range is
    encountered. */

    /*     #define Lbyte_map_address  F->fields.class_repeat.byte_map_address
           #define Lbyte_map          ((const unsigned char *)Lbyte_map_address)
           #define Lstart_eptr        F->fields.class_repeat.start_eptr
           #define Lmin               F->fields.class_repeat.min
           #define Lmax               F->fields.class_repeat.max        */

    /* case OP_NCLASS: case OP_CLASS: (C 2051) */
    OP_NCLASS | OP_CLASS => {
        (*F).fields.class_repeat.byte_map_address = (*F).ecode.add(1); /* Save for matching */
        (*F).ecode = (*F).ecode.add(1 + (32 / 1)); /* Advance past the item */

        /* Look past the end of the item to see if there is repeat information
        following. Then obey similar code to character type repeats. */

        match *(*F).ecode as u32 {
            OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
            | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                let cu = *(*F).ecode as u32;
                (*F).ecode = (*F).ecode.add(1);
                fc = cu.wrapping_sub(OP_CRSTAR);
                (*F).fields.class_repeat.min = rep_min[fc as usize];
                (*F).fields.class_repeat.max = rep_max[fc as usize];
                reptype = rep_typ[fc as usize];
            }

            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                (*F).fields.class_repeat.min = GET2((*F).ecode, 1);
                (*F).fields.class_repeat.max = GET2((*F).ecode, 1 + IMM2_SIZE);
                if (*F).fields.class_repeat.max == 0 {
                    (*F).fields.class_repeat.max = u32::MAX; /* Max 0 => infinity */
                }
                reptype = rep_typ
                    [(*(*F).ecode as u32).wrapping_sub(OP_CRSTAR) as usize];
                (*F).ecode = (*F).ecode.add(1 + 2 * IMM2_SIZE);
            }

            _ => {
                /* No repeat follows */
                (*F).fields.class_repeat.max = 1;
                (*F).fields.class_repeat.min = (*F).fields.class_repeat.max;
            }
        }

        /* First, ensure the minimum number of matches are present. */

        if utf != FALSE {
            i = 1;
            while i <= (*F).fields.class_repeat.min {
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
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                /* GETCHARINC(fc, Feptr) */
                fc = *(*F).eptr as u32;
                (*F).eptr = (*F).eptr.add(1);
                if fc >= 0xc0 {
                    let r = getutf8inc(fc, (*F).eptr);
                    fc = r.0;
                    (*F).eptr = r.1;
                }
                if fc > 255 {
                    if (*F).op as u32 == OP_CLASS {
                        rrc = MATCH_NOMATCH;
                        lbl = LBL_RETURN_SWITCH;
                        continue 'sw;
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
                        rrc = MATCH_NOMATCH;
                        lbl = LBL_RETURN_SWITCH;
                        continue 'sw;
                    }
                }
                i = i.wrapping_add(1);
            }
        }
        /* Not UTF mode */
        else {
            i = 1;
            while i <= (*F).fields.class_repeat.min {
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
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
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
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
                i = i.wrapping_add(1);
            }
        }

        /* If Lmax == Lmin we are done. Continue with main loop. */

        if (*F).fields.class_repeat.min == (*F).fields.class_repeat.max {
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* If minimizing, keep testing the rest of the expression and advancing
        the pointer while it matches the class. */

        if reptype == REPTYPE_MIN {
            if utf != FALSE {
                /* for (;;) { RMATCH(Fecode, RM200); ... } */
                start_ecode = (*F).ecode;
                (*F).return_id = RM200;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            }
            /* Not UTF mode */
            else {
                /* for (;;) { RMATCH(Fecode, RM23); ... } */
                start_ecode = (*F).ecode;
                (*F).return_id = RM23;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            }
        }
        /* If maximizing, find the longest possible run, then work backwards. */
        else {
            (*F).fields.class_repeat.start_eptr = (*F).eptr;

            if utf != FALSE {
                i = (*F).fields.class_repeat.min;
                while i < (*F).fields.class_repeat.max {
                    let mut len: c_int = 1;
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
                    /* GETCHARLEN(fc, Feptr, len) */
                    fc = *(*F).eptr as u32;
                    if fc >= 0xc0 {
                        len += utf8_extra(fc) as c_int;
                        fc = getutf8(fc, (*F).eptr);
                    }
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
                    lbl = LBL_TOP_OF_LOOP; /* No backtracking */
                    continue 'sw;
                }

                /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                Unicode character. Use <= Lstart_eptr to ensure backtracking
                doesn't go too far. */

                /* for (;;) { RMATCH(Fecode, RM201); ... } */
                start_ecode = (*F).ecode;
                (*F).return_id = RM201;
                lbl = LBL_MATCH_RECURSE;
                continue 'sw;
            }
            /* Not UTF mode */
            else {
                i = (*F).fields.class_repeat.min;
                while i < (*F).fields.class_repeat.max {
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
                    lbl = LBL_TOP_OF_LOOP; /* No backtracking */
                    continue 'sw;
                }

                /* while (Feptr >= Lstart_eptr) { RMATCH(Fecode, RM24); ... } */
                loop {
                    if !((*F).eptr >= (*F).fields.class_repeat.start_eptr) {
                        break;
                    }
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM24;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }
            }

            rrc = MATCH_NOMATCH; /* C 2269: RRETURN(MATCH_NOMATCH) */
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
    }

    /* ===================================================================== */
    /* Match an extended character class. In the 8-bit library, this opcode is
    encountered only when UTF-8 mode mode is supported. In the 16-bit and
    32-bit libraries, codepoints greater than 255 may be encountered even when
    UTF is not supported. */

    /*     #define Lstart_eptr  F->fields.xclass_repeat.start_eptr
           #define Lxclass_data F->fields.xclass_repeat.xclass_data
           #define Lmin         F->fields.xclass_repeat.min
           #define Lmax         F->fields.xclass_repeat.max        */

    /* case OP_XCLASS: (C 2294) */
    OP_XCLASS => {
        (*F).fields.xclass_repeat.xclass_data =
            (*F).ecode.add(1 + LINK_SIZE); /* Save for matching */
        (*F).ecode = (*F).ecode.add(GET((*F).ecode, 1) as usize); /* Advance past the item */

        match *(*F).ecode as u32 {
            OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
            | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                let cu = *(*F).ecode as u32;
                (*F).ecode = (*F).ecode.add(1);
                fc = cu.wrapping_sub(OP_CRSTAR);
                (*F).fields.xclass_repeat.min = rep_min[fc as usize];
                (*F).fields.xclass_repeat.max = rep_max[fc as usize];
                reptype = rep_typ[fc as usize];
            }

            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                (*F).fields.xclass_repeat.min = GET2((*F).ecode, 1);
                (*F).fields.xclass_repeat.max = GET2((*F).ecode, 1 + IMM2_SIZE);
                if (*F).fields.xclass_repeat.max == 0 {
                    (*F).fields.xclass_repeat.max = u32::MAX; /* Max 0 => infinity */
                }
                reptype = rep_typ
                    [(*(*F).ecode as u32).wrapping_sub(OP_CRSTAR) as usize];
                (*F).ecode = (*F).ecode.add(1 + 2 * IMM2_SIZE);
            }

            _ => {
                /* No repeat follows */
                (*F).fields.xclass_repeat.max = 1;
                (*F).fields.xclass_repeat.min = (*F).fields.xclass_repeat.max;
            }
        }

        /* First, ensure the minimum number of matches are present. */

        i = 1;
        while i <= (*F).fields.xclass_repeat.min {
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
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            if crate::xclass::_pcre2_xclass_8(
                fc,
                (*F).fields.xclass_repeat.xclass_data,
                (*mb).start_code as *const u8,
                utf,
            ) == FALSE
            {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            i = i.wrapping_add(1);
        }

        /* If Lmax == Lmin we can just continue with the main loop. */

        if (*F).fields.xclass_repeat.min == (*F).fields.xclass_repeat.max {
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* If minimizing, keep testing the rest of the expression and advancing
        the pointer while it matches the class. */

        if reptype == REPTYPE_MIN {
            /* for (;;) { RMATCH(Fecode, RM100); ... } */
            start_ecode = (*F).ecode;
            (*F).return_id = RM100;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }
        /* If maximizing, find the longest possible run, then work backwards. */
        else {
            (*F).fields.xclass_repeat.start_eptr = (*F).eptr;
            i = (*F).fields.xclass_repeat.min;
            while i < (*F).fields.xclass_repeat.max {
                let mut len: c_int = 1;
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
                /* GETCHARLENTEST(fc, Feptr, len) */
                fc = *(*F).eptr as u32;
                if utf != FALSE && fc >= 0xc0 {
                    len += utf8_extra(fc) as c_int;
                    fc = getutf8(fc, (*F).eptr);
                }
                if crate::xclass::_pcre2_xclass_8(
                    fc,
                    (*F).fields.xclass_repeat.xclass_data,
                    (*mb).start_code as *const u8,
                    utf,
                ) == FALSE
                {
                    break;
                }
                (*F).eptr = (*F).eptr.add(len as usize);
                i = i.wrapping_add(1);
            }

            if reptype == REPTYPE_POS {
                lbl = LBL_TOP_OF_LOOP; /* No backtracking */
                continue 'sw;
            }

            /* After \C in UTF mode, Lstart_eptr might be in the middle of a
            Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
            go too far. */

            /* for (;;) { RMATCH(Fecode, RM101); ... } */
            start_ecode = (*F).ecode;
            (*F).return_id = RM101;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }
    }

    /* ===================================================================== */
    /* Match a complex, set-based character class. This opcodes are used when
    there is complex nesting or logical operations within the character
    class. */

    /*     #define Lstart_eptr  F->fields.eclass_repeat.start_eptr
           #define Leclass_data F->fields.eclass_repeat.eclass_data
           #define Leclass_len  F->fields.eclass_repeat.eclass_len
           #define Lmin         F->fields.eclass_repeat.min
           #define Lmax         F->fields.eclass_repeat.max        */

    /* case OP_ECLASS: (C 2436) */
    OP_ECLASS => {
        (*F).fields.eclass_repeat.eclass_data =
            (*F).ecode.add(1 + LINK_SIZE); /* Save for matching */
        (*F).ecode = (*F).ecode.add(GET((*F).ecode, 1) as usize); /* Advance past the item */
        (*F).fields.eclass_repeat.eclass_len = (*F)
            .ecode
            .offset_from((*F).fields.eclass_repeat.eclass_data) as PCRE2_SIZE;

        match *(*F).ecode as u32 {
            OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
            | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                let cu = *(*F).ecode as u32;
                (*F).ecode = (*F).ecode.add(1);
                fc = cu.wrapping_sub(OP_CRSTAR);
                (*F).fields.eclass_repeat.min = rep_min[fc as usize];
                (*F).fields.eclass_repeat.max = rep_max[fc as usize];
                reptype = rep_typ[fc as usize];
            }

            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                (*F).fields.eclass_repeat.min = GET2((*F).ecode, 1);
                (*F).fields.eclass_repeat.max = GET2((*F).ecode, 1 + IMM2_SIZE);
                if (*F).fields.eclass_repeat.max == 0 {
                    (*F).fields.eclass_repeat.max = u32::MAX; /* Max 0 => infinity */
                }
                reptype = rep_typ
                    [(*(*F).ecode as u32).wrapping_sub(OP_CRSTAR) as usize];
                (*F).ecode = (*F).ecode.add(1 + 2 * IMM2_SIZE);
            }

            _ => {
                /* No repeat follows */
                (*F).fields.eclass_repeat.max = 1;
                (*F).fields.eclass_repeat.min = (*F).fields.eclass_repeat.max;
            }
        }

        /* First, ensure the minimum number of matches are present. */

        i = 1;
        while i <= (*F).fields.eclass_repeat.min {
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
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            /* GETCHARINCTEST(fc, Feptr) */
            fc = *(*F).eptr as u32;
            (*F).eptr = (*F).eptr.add(1);
            if utf != FALSE && fc >= 0xc0 {
                let r = getutf8inc(fc, (*F).eptr);
                fc = r.0;
                (*F).eptr = r.1;
            }
            if crate::xclass::_pcre2_eclass_8(
                fc,
                (*F).fields.eclass_repeat.eclass_data,
                (*F)
                    .fields
                    .eclass_repeat
                    .eclass_data
                    .add((*F).fields.eclass_repeat.eclass_len),
                (*mb).start_code as *const u8,
                utf,
            ) == FALSE
            {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            i = i.wrapping_add(1);
        }

        /* If Lmax == Lmin we can just continue with the main loop. */

        if (*F).fields.eclass_repeat.min == (*F).fields.eclass_repeat.max {
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }

        /* If minimizing, keep testing the rest of the expression and advancing
        the pointer while it matches the class. */

        if reptype == REPTYPE_MIN {
            /* for (;;) { RMATCH(Fecode, RM102); ... } */
            start_ecode = (*F).ecode;
            (*F).return_id = RM102;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }
        /* If maximizing, find the longest possible run, then work backwards. */
        else {
            (*F).fields.eclass_repeat.start_eptr = (*F).eptr;
            i = (*F).fields.eclass_repeat.min;
            while i < (*F).fields.eclass_repeat.max {
                let mut len: c_int = 1;
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
                /* GETCHARLENTEST(fc, Feptr, len) */
                fc = *(*F).eptr as u32;
                if utf != FALSE && fc >= 0xc0 {
                    len += utf8_extra(fc) as c_int;
                    fc = getutf8(fc, (*F).eptr);
                }
                if crate::xclass::_pcre2_eclass_8(
                    fc,
                    (*F).fields.eclass_repeat.eclass_data,
                    (*F)
                        .fields
                        .eclass_repeat
                        .eclass_data
                        .add((*F).fields.eclass_repeat.eclass_len),
                    (*mb).start_code as *const u8,
                    utf,
                ) == FALSE
                {
                    break;
                }
                (*F).eptr = (*F).eptr.add(len as usize);
                i = i.wrapping_add(1);
            }

            if reptype == REPTYPE_POS {
                lbl = LBL_TOP_OF_LOOP; /* No backtracking */
                continue 'sw;
            }

            /* After \C in UTF mode, Lstart_eptr might be in the middle of a
            Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
            go too far. */

            /* for (;;) { RMATCH(Fecode, RM103); ... } */
            start_ecode = (*F).ecode;
            (*F).return_id = RM103;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }
    }

    _ => {}
    }
}

/* ---- RMATCH continuations owned by this chunk ---- */

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM200) at C 2150: minimizing bit-mapped
class repeat, UTF mode. */

if lbl == LBL_RM_BASE + RM200 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let oldmin = (*F).fields.class_repeat.min;
        (*F).fields.class_repeat.min = oldmin.wrapping_add(1);
        if oldmin >= (*F).fields.class_repeat.max {
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
        /* GETCHARINC(fc, Feptr) */
        fc = *(*F).eptr as u32;
        (*F).eptr = (*F).eptr.add(1);
        if fc >= 0xc0 {
            let r = getutf8inc(fc, (*F).eptr);
            fc = r.0;
            (*F).eptr = r.1;
        }
        if fc > 255 {
            if (*F).op as u32 == OP_CLASS {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
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
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }
        /* Loop back to RMATCH(Fecode, RM200) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM200;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM23) at C 2173: minimizing bit-mapped
class repeat, not UTF mode. */

if lbl == LBL_RM_BASE + RM23 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let oldmin = (*F).fields.class_repeat.min;
        (*F).fields.class_repeat.min = oldmin.wrapping_add(1);
        if oldmin >= (*F).fields.class_repeat.max {
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
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        /* Loop back to RMATCH(Fecode, RM23) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM23;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM201) at C 2230: backtracking for the
maximizing bit-mapped class repeat, UTF mode. */

if lbl == LBL_RM_BASE + RM201 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        /* if (Feptr-- <= Lstart_eptr) break; */
        let old_eptr = (*F).eptr;
        (*F).eptr = (*F).eptr.wrapping_sub(1);
        if old_eptr <= (*F).fields.class_repeat.start_eptr {
            break; /* Tried at original position */
        }
        /* BACKCHAR(Feptr) */
        while (*(*F).eptr as u32 & 0xc0) == 0x80 {
            (*F).eptr = (*F).eptr.wrapping_sub(1);
        }
        /* Loop back to RMATCH(Fecode, RM201) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM201;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    rrc = MATCH_NOMATCH; /* C 2269: RRETURN(MATCH_NOMATCH) */
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM24) at C 2263: backtracking for the
maximizing bit-mapped class repeat, not UTF mode. */

if lbl == LBL_RM_BASE + RM24 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        (*F).eptr = (*F).eptr.wrapping_sub(1);
        /* Top of the while loop */
        if !((*F).eptr >= (*F).fields.class_repeat.start_eptr) {
            break;
        }
        start_ecode = (*F).ecode;
        (*F).return_id = RM24;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    rrc = MATCH_NOMATCH; /* C 2269: RRETURN(MATCH_NOMATCH) */
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM100) at C 2357: minimizing XCLASS
repeat. */

if lbl == LBL_RM_BASE + RM100 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let oldmin = (*F).fields.xclass_repeat.min;
        (*F).fields.xclass_repeat.min = oldmin.wrapping_add(1);
        if oldmin >= (*F).fields.xclass_repeat.max {
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
        /* GETCHARINCTEST(fc, Feptr) */
        fc = *(*F).eptr as u32;
        (*F).eptr = (*F).eptr.add(1);
        if utf != FALSE && fc >= 0xc0 {
            let r = getutf8inc(fc, (*F).eptr);
            fc = r.0;
            (*F).eptr = r.1;
        }
        if crate::xclass::_pcre2_xclass_8(
            fc,
            (*F).fields.xclass_repeat.xclass_data,
            (*mb).start_code as *const u8,
            utf,
        ) == FALSE
        {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        /* Loop back to RMATCH(Fecode, RM100) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM100;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM101) at C 2404: backtracking for the
maximizing XCLASS repeat. */

if lbl == LBL_RM_BASE + RM101 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        /* if (Feptr-- <= Lstart_eptr) break; */
        let old_eptr = (*F).eptr;
        (*F).eptr = (*F).eptr.wrapping_sub(1);
        if old_eptr <= (*F).fields.xclass_repeat.start_eptr {
            break; /* Tried at original position */
        }
        if utf != FALSE {
            /* BACKCHAR(Feptr) */
            while (*(*F).eptr as u32 & 0xc0) == 0x80 {
                (*F).eptr = (*F).eptr.wrapping_sub(1);
            }
        }
        /* Loop back to RMATCH(Fecode, RM101) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM101;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    rrc = MATCH_NOMATCH; /* C 2411: RRETURN(MATCH_NOMATCH) */
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM102) at C 2500: minimizing ECLASS
repeat. */

if lbl == LBL_RM_BASE + RM102 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let oldmin = (*F).fields.eclass_repeat.min;
        (*F).fields.eclass_repeat.min = oldmin.wrapping_add(1);
        if oldmin >= (*F).fields.eclass_repeat.max {
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
        /* GETCHARINCTEST(fc, Feptr) */
        fc = *(*F).eptr as u32;
        (*F).eptr = (*F).eptr.add(1);
        if utf != FALSE && fc >= 0xc0 {
            let r = getutf8inc(fc, (*F).eptr);
            fc = r.0;
            (*F).eptr = r.1;
        }
        if crate::xclass::_pcre2_eclass_8(
            fc,
            (*F).fields.eclass_repeat.eclass_data,
            (*F)
                .fields
                .eclass_repeat
                .eclass_data
                .add((*F).fields.eclass_repeat.eclass_len),
            (*mb).start_code as *const u8,
            utf,
        ) == FALSE
        {
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        /* Loop back to RMATCH(Fecode, RM102) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM102;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM103) at C 2548: backtracking for the
maximizing ECLASS repeat. */

if lbl == LBL_RM_BASE + RM103 as u32 {
    loop {
        if rrc != MATCH_NOMATCH {
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        /* if (Feptr-- <= Lstart_eptr) break; */
        let old_eptr = (*F).eptr;
        (*F).eptr = (*F).eptr.wrapping_sub(1);
        if old_eptr <= (*F).fields.eclass_repeat.start_eptr {
            break; /* Tried at original position */
        }
        if utf != FALSE {
            /* BACKCHAR(Feptr) */
            while (*(*F).eptr as u32 & 0xc0) == 0x80 {
                (*F).eptr = (*F).eptr.wrapping_sub(1);
            }
        }
        /* Loop back to RMATCH(Fecode, RM103) */
        start_ecode = (*F).ecode;
        (*F).return_id = RM103;
        lbl = LBL_MATCH_RECURSE;
        continue 'sw;
    }
    rrc = MATCH_NOMATCH; /* C 2555: RRETURN(MATCH_NOMATCH) */
    lbl = LBL_RETURN_SWITCH;
    continue 'sw;
}
}
