// ==== EXTRA_STATE_CONSTS ====
/* Synthetic states for chunk A (C lines 900..2573). */

/* Shared fall-through target for OP_ACCEPT/OP_END (C line 976). */
pub const L_A_OP_END: u32 = 1000;
/* Shared fall-through target for OP_ANY/OP_ALLANY (C line 1076). */
pub const L_A_ALLANY: u32 = 1001;

/* REPEATCHAR: multi-code-unit UTF character (C lines 1430..1482). */
pub const L_A_UTF_MIN_LOOP: u32 = 1002; /* RM202 */
pub const L_A_UTF_MAX_LOOP: u32 = 1003; /* RM203 */
/* REPEATCHAR: single-code-unit, caseless (C lines 1532..1574). */
pub const L_A_CI_MIN_LOOP: u32 = 1004; /* RM25 */
pub const L_A_CI_MAX_LOOP: u32 = 1005; /* RM26 */
/* REPEATCHAR: single-code-unit, caseful (C lines 1593..1631). */
pub const L_A_CF_MIN_LOOP: u32 = 1006; /* RM27 */
pub const L_A_CF_MAX_LOOP: u32 = 1007; /* RM28 */

/* REPEATNOTCHAR: caseless (C lines 1788..1889). */
pub const L_A_N_CI_UMIN_LOOP: u32 = 1008; /* RM204 */
pub const L_A_N_CI_MIN_LOOP: u32 = 1009; /* RM29 */
pub const L_A_N_CI_UMAX_LOOP: u32 = 1010; /* RM205 */
pub const L_A_N_CI_MAX_LOOP: u32 = 1011; /* RM30 */
/* REPEATNOTCHAR: caseful (C lines 1928..2026). */
pub const L_A_N_CF_UMIN_LOOP: u32 = 1012; /* RM206 */
pub const L_A_N_CF_MIN_LOOP: u32 = 1013; /* RM31 */
pub const L_A_N_CF_UMAX_LOOP: u32 = 1014; /* RM207 */
pub const L_A_N_CF_MAX_LOOP: u32 = 1015; /* RM32 */

/* OP_CLASS/OP_NCLASS (C lines 2143..2270). */
pub const L_A_CLASS_UMIN_LOOP: u32 = 1016; /* RM200 */
pub const L_A_CLASS_MIN_LOOP: u32 = 1017; /* RM23 */
pub const L_A_CLASS_UMAX_LOOP: u32 = 1018; /* RM201 */
pub const L_A_CLASS_MAX_LOOP: u32 = 1019; /* RM24 */

/* OP_XCLASS (C lines 2353..2412). */
pub const L_A_XCLASS_MIN_LOOP: u32 = 1020; /* RM100 */
pub const L_A_XCLASS_MAX_LOOP: u32 = 1021; /* RM101 */

/* OP_ECLASS (C lines 2496..2556). */
pub const L_A_ECLASS_MIN_LOOP: u32 = 1022; /* RM102 */
pub const L_A_ECLASS_MAX_LOOP: u32 = 1023; /* RM103 */

/* memcmp(a, b, n) == 0 */
#[inline]
pub(crate) unsafe fn frag_a_memcmp_eq(a: PCRE2_SPTR, b: PCRE2_SPTR, n: PCRE2_SIZE) -> bool {
    let mut k: PCRE2_SIZE = 0;
    while k < n {
        if *a.add(k) != *b.add(k) {
            return false;
        }
        k += 1;
    }
    true
}
// ==== EXTRA_LOCALS ====
let mut othercase: u32 = 0; /* REPEATCHAR, C line 1406 */
// ==== ARMS ====
/* ===================================================================== */
/* Before OP_ACCEPT there may be any number of OP_CLOSE opcodes, to close
any currently open capturing brackets. Unlike reaching the end of a group,
where we know the starting frame is at the top of the chained frames, in
this case we have to search back for the relevant frame in case other types
of group that use chained frames have intervened. Multiple OP_CLOSEs always
come innermost first, which matches the chain order. We can ignore this in
a recursion, because captures are not passed out of recursions. */

OP_CLOSE => {
    if Fcurrent_recurse!() == RECURSE_UNSET {
        number = GET2!(Fecode!(), 1);
        offset = Flast_group_offset!();
        loop {
            /* Corrupted heapframes?. Trigger an assert and return an error */
            /* PCRE2_ASSERT(offset != PCRE2_UNSET); */
            if offset == PCRE2_UNSET {
                return PCRE2_ERROR_INTERNAL;
            }

            N = ((*match_data).heapframes as *mut u8).add(offset) as *mut heapframe;
            P = ((N as *mut u8).sub(frame_size)) as *mut heapframe;
            if (*N).group_frame_type == (GF_CAPTURE | number) {
                break;
            }
            offset = (*P).last_group_offset;
        }
        offset = ((number << 1) - 2) as PCRE2_SIZE;
        Fcapture_last!() = number;
        *Fovector!().add(offset) =
            ((*P).eptr as usize) - ((*mb).start_subject as usize);
        *Fovector!().add(offset + 1) =
            (Feptr!() as usize) - ((*mb).start_subject as usize);
        if offset >= Foffset_top!() {
            Foffset_top!() = offset + 2;
        }
    }
    Fecode!() = Fecode!()
        .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Real or forced end of the pattern, assertion, or recursion. In an
assertion ACCEPT, update the last used pointer and remember the current
frame so that the captures and mark can be fished out of it. */

OP_ASSERT_ACCEPT => {
    if Feptr!() > (*mb).last_used_ptr {
        (*mb).last_used_ptr = Feptr!();
    }
    assert_accept_frame = F;
    rrc = MATCH_ACCEPT; /* RRETURN(MATCH_ACCEPT) */
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* For ACCEPT within a recursion, we have to find the most recent
recursion. If not in a recursion, fall through to code that is common with
OP_END. */

OP_ACCEPT => {
    if Fcurrent_recurse!() != RECURSE_UNSET {
        offset = Flast_group_offset!();
        loop {
            /* Corrupted heapframes?. Trigger an assert and return an error */
            /* PCRE2_ASSERT(offset != PCRE2_UNSET); */
            if offset == PCRE2_UNSET {
                return PCRE2_ERROR_INTERNAL;
            }

            N = ((*match_data).heapframes as *mut u8).add(offset) as *mut heapframe;
            P = ((N as *mut u8).sub(frame_size)) as *mut heapframe;
            if GF_IDMASK((*N).group_frame_type) == GF_RECURSE {
                break;
            }
            offset = (*P).last_group_offset;
        }

        /* N is now the frame of the recursion; the previous frame is at the
        OP_RECURSE position. Go back there, copying the current subject position
        and mark, and the start_match position (\K might have changed it), and
        then move on past the OP_RECURSE. */

        (*P).eptr = Feptr!();
        (*P).mark = Fmark!();
        (*P).start_match = Fstart_match!();
        F = P;
        Fecode!() = Fecode!().add(1 + LINK_SIZE);
        state = S_MAINLOOP; /* continue */
        continue 'sm;
    }
    /* Fall through */
    state = L_A_OP_END;
    continue 'sm;
}

/* OP_END itself can never be reached within a recursion because that is
picked up when the OP_KET that always precedes OP_END is reached. */

OP_END => {
    state = L_A_OP_END;
    continue 'sm;
}

/*===================================================================== */
/* Match any single character type except newline; have to take care with
CRLF newlines and partial matching. */

OP_ANY => {
    if IS_NEWLINE!(Feptr!()) != 0 {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if (*mb).partial != 0
        && Feptr!() == (*mb).end_subject.wrapping_sub(1)
        && (*mb).nltype == NLTYPE_FIXED
        && (*mb).nllen == 2
        && *Feptr!() == (*mb).nl[0]
    {
        (*mb).hitend = TRUE;
        if (*mb).partial > 1 {
            return PCRE2_ERROR_PARTIAL;
        }
    }
    /* Fall through */
    state = L_A_ALLANY;
    continue 'sm;
}

/* Match any single character whatsoever. */

OP_ALLANY => {
    state = L_A_ALLANY;
    continue 'sm;
}

/* ===================================================================== */
/* Match a single code unit, even in UTF mode. This opcode really does
match any code unit, even newline. */

OP_ANYBYTE => {
    if Feptr!() >= (*mb).end_subject
    /* DO NOT merge the Feptr++ here; it must */
    {
        /* not be updated before SCHECK_PARTIAL. */
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().add(1);
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Match a single character, casefully */

OP_CHAR => {
    if utf != 0 {
        length = 1;
        Fecode!() = Fecode!().add(1);
        GETCHARLEN!(fc, Fecode!(), length);
        if length > (((*mb).end_subject as usize) - (Feptr!() as usize)) as PCRE2_SIZE {
            CHECK_PARTIAL!(); /* Not SCHECK_PARTIAL() */
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        while length > 0 {
            let a_ = *Fecode!();
            Fecode!() = Fecode!().add(1);
            let b_ = *Feptr!();
            Feptr!() = Feptr!().add(1);
            if a_ != b_ {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            length -= 1;
        }
    }
    /* Not UTF mode */
    else {
        if (*mb).end_subject.offset_from(Feptr!()) < 1 {
            SCHECK_PARTIAL!(); /* This one can use SCHECK_PARTIAL() */
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        let b_ = *Feptr!();
        Feptr!() = Feptr!().add(1);
        if *Fecode!().add(1) != b_ {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        Fecode!() = Fecode!().add(2);
    }
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Match a single character, caselessly. If we are at the end of the
subject, give up immediately. We get here only when the pattern character
has at most one other case. */

OP_CHARI => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    if utf != 0 {
        length = 1;
        Fecode!() = Fecode!().add(1);
        GETCHARLEN!(fc, Fecode!(), length);

        /* If the pattern character's value is < 128, we know that its other case
        (if any) is also < 128, so we can use the fast lookup table. */

        if fc < 128 {
            let cc: u32 = *Feptr!() as u32;
            if *(*mb).lcc.add(fc as usize) != TABLE_GET!(cc, (*mb).lcc, cc) {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            Fecode!() = Fecode!().add(1);
            Feptr!() = Feptr!().add(1);
        }
        /* Otherwise we must pick up the subject character and use Unicode
        property support to test its other case. */
        else {
            let mut dc: u32 = 0;
            GETCHARINC!(dc, Feptr!());
            Fecode!() = Fecode!().add(length);
            if dc != fc && dc != UCD_OTHERCASE!(fc) {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }
    }
    /* If UCP is set without UTF we must do the same as above, but with one
    character per code unit. */
    else if ucp != 0 {
        let cc: u32 = *Feptr!() as u32;
        fc = *Fecode!().add(1) as u32;
        if fc < 128 {
            if *(*mb).lcc.add(fc as usize) != TABLE_GET!(cc, (*mb).lcc, cc) {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        } else {
            if cc != fc && cc != UCD_OTHERCASE!(fc) {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }
        Feptr!() = Feptr!().add(1);
        Fecode!() = Fecode!().add(2);
    }
    /* Not UTF or UCP mode; use the table for characters < 256. */
    else {
        if TABLE_GET!(*Fecode!().add(1), (*mb).lcc, *Fecode!().add(1))
            != TABLE_GET!(*Feptr!(), (*mb).lcc, *Feptr!())
        {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        Feptr!() = Feptr!().add(1);
        Fecode!() = Fecode!().add(2);
    }
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Match not a single character. */

OP_NOT | OP_NOTI => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    if utf != 0 {
        let mut ch: u32 = 0;
        Fecode!() = Fecode!().add(1);
        GETCHARINC!(ch, Fecode!());
        GETCHARINC!(fc, Feptr!());
        if ch == fc {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) - caseful match */
            state = S_RETURN_SWITCH;
            continue 'sm;
        } else if Fop!() as u32 == OP_NOTI
        /* If caseless */
        {
            if ch > 127 {
                ch = UCD_OTHERCASE!(ch);
            } else {
                ch = *(*mb).fcc.add(ch as usize) as u32;
            }
            if ch == fc {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }
    }
    /* UCP without UTF is as above, but with one character per code unit. */
    else if ucp != 0 {
        let mut ch: u32;
        fc = *Feptr!() as u32;
        Feptr!() = Feptr!().add(1);
        ch = *Fecode!().add(1) as u32;
        Fecode!() = Fecode!().add(2);

        if ch == fc {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) - caseful match */
            state = S_RETURN_SWITCH;
            continue 'sm;
        } else if Fop!() as u32 == OP_NOTI
        /* If caseless */
        {
            if ch > 127 {
                ch = UCD_OTHERCASE!(ch);
            } else {
                ch = *(*mb).fcc.add(ch as usize) as u32;
            }
            if ch == fc {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }
    }
    /* Neither UTF nor UCP is set */
    else {
        let ch: u32 = *Fecode!().add(1) as u32;
        fc = *Feptr!() as u32;
        Feptr!() = Feptr!().add(1);
        if ch == fc
            || (Fop!() as u32 == OP_NOTI
                && TABLE_GET!(ch, (*mb).fcc, ch) as u32 == fc)
        {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        Fecode!() = Fecode!().add(2);
    }
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Match a single character repeatedly.
   Llength     -> (*F).byte1
   Loclength   -> (*F).byte2
   Lstart_eptr -> (*F).fields.char_repeat.start_eptr
   Lcharptr    -> (*F).fields.char_repeat.charptr
   Lmin        -> (*F).fields.char_repeat.min
   Lmax        -> (*F).fields.char_repeat.max
   Lc          -> (*F).fields.char_repeat.c
   Loc         -> (*F).fields.char_repeat.oc.oc
   Loccu       -> (*F).fields.char_repeat.oc.occu
*/

OP_EXACT | OP_EXACTI => {
    (*F).fields.char_repeat.max = GET2!(Fecode!(), 1);
    (*F).fields.char_repeat.min = (*F).fields.char_repeat.max;
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_POSUPTO | OP_POSUPTOI => {
    reptype = REPTYPE_POS;
    (*F).fields.char_repeat.min = 0;
    (*F).fields.char_repeat.max = GET2!(Fecode!(), 1);
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_UPTO | OP_UPTOI => {
    reptype = REPTYPE_MAX;
    (*F).fields.char_repeat.min = 0;
    (*F).fields.char_repeat.max = GET2!(Fecode!(), 1);
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_MINUPTO | OP_MINUPTOI => {
    reptype = REPTYPE_MIN;
    (*F).fields.char_repeat.min = 0;
    (*F).fields.char_repeat.max = GET2!(Fecode!(), 1);
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_POSSTAR | OP_POSSTARI => {
    reptype = REPTYPE_POS;
    (*F).fields.char_repeat.min = 0;
    (*F).fields.char_repeat.max = u32::MAX;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_POSPLUS | OP_POSPLUSI => {
    reptype = REPTYPE_POS;
    (*F).fields.char_repeat.min = 1;
    (*F).fields.char_repeat.max = u32::MAX;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_POSQUERY | OP_POSQUERYI => {
    reptype = REPTYPE_POS;
    (*F).fields.char_repeat.min = 0;
    (*F).fields.char_repeat.max = 1;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_STAR | OP_STARI | OP_MINSTAR | OP_MINSTARI | OP_PLUS | OP_PLUSI | OP_MINPLUS
| OP_MINPLUSI | OP_QUERY | OP_QUERYI | OP_MINQUERY | OP_MINQUERYI => {
    let t_ = *Fecode!();
    Fecode!() = Fecode!().add(1);
    fc = (t_ as u32)
        - (if (Fop!() as u32) < OP_STARI {
            OP_STAR
        } else {
            OP_STARI
        });
    (*F).fields.char_repeat.min = rep_min[fc as usize];
    (*F).fields.char_repeat.max = rep_max[fc as usize];
    reptype = rep_typ[fc as usize];
    /* Fall through to REPEATCHAR */
    state = L_REPEATCHAR;
    continue 'sm;
}

/* ===================================================================== */
/* Match a negated single one-byte character repeatedly.
   Lstart_eptr -> (*F).fields.charnot_repeat.start_eptr
   Lmin        -> (*F).fields.charnot_repeat.min
   Lmax        -> (*F).fields.charnot_repeat.max
   Lc          -> (*F).fields.charnot_repeat.c
   Loc         -> (*F).fields.charnot_repeat.oc
*/

OP_NOTEXACT | OP_NOTEXACTI => {
    (*F).fields.charnot_repeat.max = GET2!(Fecode!(), 1);
    (*F).fields.charnot_repeat.min = (*F).fields.charnot_repeat.max;
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTUPTO | OP_NOTUPTOI => {
    (*F).fields.charnot_repeat.min = 0;
    (*F).fields.charnot_repeat.max = GET2!(Fecode!(), 1);
    reptype = REPTYPE_MAX;
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTMINUPTO | OP_NOTMINUPTOI => {
    (*F).fields.charnot_repeat.min = 0;
    (*F).fields.charnot_repeat.max = GET2!(Fecode!(), 1);
    reptype = REPTYPE_MIN;
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTPOSSTAR | OP_NOTPOSSTARI => {
    reptype = REPTYPE_POS;
    (*F).fields.charnot_repeat.min = 0;
    (*F).fields.charnot_repeat.max = u32::MAX;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTPOSPLUS | OP_NOTPOSPLUSI => {
    reptype = REPTYPE_POS;
    (*F).fields.charnot_repeat.min = 1;
    (*F).fields.charnot_repeat.max = u32::MAX;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
    reptype = REPTYPE_POS;
    (*F).fields.charnot_repeat.min = 0;
    (*F).fields.charnot_repeat.max = 1;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTPOSUPTO | OP_NOTPOSUPTOI => {
    reptype = REPTYPE_POS;
    (*F).fields.charnot_repeat.min = 0;
    (*F).fields.charnot_repeat.max = GET2!(Fecode!(), 1);
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTSTAR | OP_NOTSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI | OP_NOTPLUS
| OP_NOTPLUSI | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_NOTQUERY | OP_NOTQUERYI
| OP_NOTMINQUERY | OP_NOTMINQUERYI => {
    let t_ = *Fecode!();
    Fecode!() = Fecode!().add(1);
    fc = (t_ as u32)
        - (if (Fop!() as u32) >= OP_NOTSTARI {
            OP_NOTSTARI
        } else {
            OP_NOTSTAR
        });
    (*F).fields.charnot_repeat.min = rep_min[fc as usize];
    (*F).fields.charnot_repeat.max = rep_max[fc as usize];
    reptype = rep_typ[fc as usize];
    /* Fall through to REPEATNOTCHAR */
    state = L_REPEATNOTCHAR;
    continue 'sm;
}

/* ===================================================================== */
/* Match a bit-mapped character class, possibly repeatedly.
   Lbyte_map_address -> (*F).fields.class_repeat.byte_map_address
   Lstart_eptr       -> (*F).fields.class_repeat.start_eptr
   Lmin              -> (*F).fields.class_repeat.min
   Lmax              -> (*F).fields.class_repeat.max
*/

OP_NCLASS | OP_CLASS => {
    (*F).fields.class_repeat.byte_map_address = Fecode!().add(1); /* Save for matching */
    Fecode!() = Fecode!().add(1 + 32); /* Advance past the item */

    /* Look past the end of the item to see if there is repeat information
    following. Then obey similar code to character type repeats. */

    match *Fecode!() as u32 {
        OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
        | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
            let t_ = *Fecode!();
            Fecode!() = Fecode!().add(1);
            fc = (t_ as u32) - OP_CRSTAR;
            (*F).fields.class_repeat.min = rep_min[fc as usize];
            (*F).fields.class_repeat.max = rep_max[fc as usize];
            reptype = rep_typ[fc as usize];
        }

        OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
            (*F).fields.class_repeat.min = GET2!(Fecode!(), 1);
            (*F).fields.class_repeat.max = GET2!(Fecode!(), 1 + IMM2_SIZE);
            if (*F).fields.class_repeat.max == 0 {
                (*F).fields.class_repeat.max = u32::MAX; /* Max 0 => infinity */
            }
            reptype = rep_typ[((*Fecode!() as u32) - OP_CRSTAR) as usize];
            Fecode!() = Fecode!().add(1 + 2 * IMM2_SIZE);
        }

        _ => {
            /* No repeat follows */
            (*F).fields.class_repeat.max = 1;
            (*F).fields.class_repeat.min = 1;
        }
    }

    /* First, ensure the minimum number of matches are present. */

    if utf != 0 {
        i = 1;
        while i <= (*F).fields.class_repeat.min {
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            GETCHARINC!(fc, Feptr!());
            if fc > 255 {
                if Fop!() as u32 == OP_CLASS {
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            } else if (*(*F).fields.class_repeat.byte_map_address.add((fc / 8) as usize)
                as u32
                & (1u32 << (fc & 7)))
                == 0
            {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            i += 1;
        }
    }
    /* Not UTF mode */
    else {
        i = 1;
        while i <= (*F).fields.class_repeat.min {
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            fc = *Feptr!() as u32;
            Feptr!() = Feptr!().add(1);
            if (*(*F).fields.class_repeat.byte_map_address.add((fc / 8) as usize) as u32
                & (1u32 << (fc & 7)))
                == 0
            {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            i += 1;
        }
    }

    /* If Lmax == Lmin we are done. Continue with main loop. */

    if (*F).fields.class_repeat.min == (*F).fields.class_repeat.max {
        state = S_MAINLOOP; /* continue */
        continue 'sm;
    }

    /* If minimizing, keep testing the rest of the expression and advancing
    the pointer while it matches the class. */

    if reptype == REPTYPE_MIN {
        if utf != 0 {
            state = L_A_CLASS_UMIN_LOOP;
            continue 'sm;
        }
        /* Not UTF mode */
        else {
            state = L_A_CLASS_MIN_LOOP;
            continue 'sm;
        }
    }
    /* If maximizing, find the longest possible run, then work backwards. */
    else {
        (*F).fields.class_repeat.start_eptr = Feptr!();

        if utf != 0 {
            i = (*F).fields.class_repeat.min;
            while i < (*F).fields.class_repeat.max {
                let mut len: i32 = 1;
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    break;
                }
                GETCHARLEN!(fc, Feptr!(), len);
                if fc > 255 {
                    if Fop!() as u32 == OP_CLASS {
                        break;
                    }
                } else if (*(*F)
                    .fields
                    .class_repeat
                    .byte_map_address
                    .add((fc / 8) as usize) as u32
                    & (1u32 << (fc & 7)))
                    == 0
                {
                    break;
                }
                Feptr!() = Feptr!().add(len as usize);
                i += 1;
            }

            if reptype == REPTYPE_POS {
                state = S_MAINLOOP; /* continue - no backtracking */
                continue 'sm;
            }

            state = L_A_CLASS_UMAX_LOOP;
            continue 'sm;
        }
        /* Not UTF mode */
        else {
            i = (*F).fields.class_repeat.min;
            while i < (*F).fields.class_repeat.max {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    break;
                }
                fc = *Feptr!() as u32;
                if (*(*F).fields.class_repeat.byte_map_address.add((fc / 8) as usize)
                    as u32
                    & (1u32 << (fc & 7)))
                    == 0
                {
                    break;
                }
                Feptr!() = Feptr!().add(1);
                i += 1;
            }

            if reptype == REPTYPE_POS {
                state = S_MAINLOOP; /* continue - no backtracking */
                continue 'sm;
            }

            state = L_A_CLASS_MAX_LOOP;
            continue 'sm;
        }
    }
}

/* ===================================================================== */
/* Match an extended character class.
   Lstart_eptr  -> (*F).fields.xclass_repeat.start_eptr
   Lxclass_data -> (*F).fields.xclass_repeat.xclass_data
   Lmin         -> (*F).fields.xclass_repeat.min
   Lmax         -> (*F).fields.xclass_repeat.max
*/

OP_XCLASS => {
    (*F).fields.xclass_repeat.xclass_data = Fecode!().add(1 + LINK_SIZE); /* Save for matching */
    Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize); /* Advance past the item */

    match *Fecode!() as u32 {
        OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
        | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
            let t_ = *Fecode!();
            Fecode!() = Fecode!().add(1);
            fc = (t_ as u32) - OP_CRSTAR;
            (*F).fields.xclass_repeat.min = rep_min[fc as usize];
            (*F).fields.xclass_repeat.max = rep_max[fc as usize];
            reptype = rep_typ[fc as usize];
        }

        OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
            (*F).fields.xclass_repeat.min = GET2!(Fecode!(), 1);
            (*F).fields.xclass_repeat.max = GET2!(Fecode!(), 1 + IMM2_SIZE);
            if (*F).fields.xclass_repeat.max == 0 {
                (*F).fields.xclass_repeat.max = u32::MAX; /* Max 0 => infinity */
            }
            reptype = rep_typ[((*Fecode!() as u32) - OP_CRSTAR) as usize];
            Fecode!() = Fecode!().add(1 + 2 * IMM2_SIZE);
        }

        _ => {
            /* No repeat follows */
            (*F).fields.xclass_repeat.max = 1;
            (*F).fields.xclass_repeat.min = 1;
        }
    }

    /* First, ensure the minimum number of matches are present. */

    i = 1;
    while i <= (*F).fields.xclass_repeat.min {
        if Feptr!() >= (*mb).end_subject {
            SCHECK_PARTIAL!();
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        GETCHARINCTEST!(fc, Feptr!(), utf);
        if crate::xclass::_pcre2_xclass_8(
            fc,
            (*F).fields.xclass_repeat.xclass_data,
            (*mb).start_code,
            utf,
        ) == 0
        {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        i += 1;
    }

    /* If Lmax == Lmin we can just continue with the main loop. */

    if (*F).fields.xclass_repeat.min == (*F).fields.xclass_repeat.max {
        state = S_MAINLOOP; /* continue */
        continue 'sm;
    }

    /* If minimizing, keep testing the rest of the expression and advancing
    the pointer while it matches the class. */

    if reptype == REPTYPE_MIN {
        state = L_A_XCLASS_MIN_LOOP;
        continue 'sm;
    }
    /* If maximizing, find the longest possible run, then work backwards. */
    else {
        (*F).fields.xclass_repeat.start_eptr = Feptr!();
        i = (*F).fields.xclass_repeat.min;
        while i < (*F).fields.xclass_repeat.max {
            let mut len: i32 = 1;
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                break;
            }
            GETCHARLENTEST!(fc, Feptr!(), len, utf);
            if crate::xclass::_pcre2_xclass_8(
                fc,
                (*F).fields.xclass_repeat.xclass_data,
                (*mb).start_code,
                utf,
            ) == 0
            {
                break;
            }
            Feptr!() = Feptr!().add(len as usize);
            i += 1;
        }

        if reptype == REPTYPE_POS {
            state = S_MAINLOOP; /* continue - no backtracking */
            continue 'sm;
        }

        state = L_A_XCLASS_MAX_LOOP;
        continue 'sm;
    }
}

/* ===================================================================== */
/* Match a complex, set-based character class.
   Lstart_eptr  -> (*F).fields.eclass_repeat.start_eptr
   Leclass_data -> (*F).fields.eclass_repeat.eclass_data
   Leclass_len  -> (*F).fields.eclass_repeat.eclass_len
   Lmin         -> (*F).fields.eclass_repeat.min
   Lmax         -> (*F).fields.eclass_repeat.max
*/

OP_ECLASS => {
    (*F).fields.eclass_repeat.eclass_data = Fecode!().add(1 + LINK_SIZE); /* Save for matching */
    Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize); /* Advance past the item */
    (*F).fields.eclass_repeat.eclass_len =
        ((Fecode!() as usize) - ((*F).fields.eclass_repeat.eclass_data as usize)) as PCRE2_SIZE;

    match *Fecode!() as u32 {
        OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
        | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
            let t_ = *Fecode!();
            Fecode!() = Fecode!().add(1);
            fc = (t_ as u32) - OP_CRSTAR;
            (*F).fields.eclass_repeat.min = rep_min[fc as usize];
            (*F).fields.eclass_repeat.max = rep_max[fc as usize];
            reptype = rep_typ[fc as usize];
        }

        OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
            (*F).fields.eclass_repeat.min = GET2!(Fecode!(), 1);
            (*F).fields.eclass_repeat.max = GET2!(Fecode!(), 1 + IMM2_SIZE);
            if (*F).fields.eclass_repeat.max == 0 {
                (*F).fields.eclass_repeat.max = u32::MAX; /* Max 0 => infinity */
            }
            reptype = rep_typ[((*Fecode!() as u32) - OP_CRSTAR) as usize];
            Fecode!() = Fecode!().add(1 + 2 * IMM2_SIZE);
        }

        _ => {
            /* No repeat follows */
            (*F).fields.eclass_repeat.max = 1;
            (*F).fields.eclass_repeat.min = 1;
        }
    }

    /* First, ensure the minimum number of matches are present. */

    i = 1;
    while i <= (*F).fields.eclass_repeat.min {
        if Feptr!() >= (*mb).end_subject {
            SCHECK_PARTIAL!();
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        GETCHARINCTEST!(fc, Feptr!(), utf);
        if crate::xclass::_pcre2_eclass_8(
            fc,
            (*F).fields.eclass_repeat.eclass_data,
            (*F).fields
                .eclass_repeat
                .eclass_data
                .add((*F).fields.eclass_repeat.eclass_len),
            (*mb).start_code,
            utf,
        ) == 0
        {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        i += 1;
    }

    /* If Lmax == Lmin we can just continue with the main loop. */

    if (*F).fields.eclass_repeat.min == (*F).fields.eclass_repeat.max {
        state = S_MAINLOOP; /* continue */
        continue 'sm;
    }

    /* If minimizing, keep testing the rest of the expression and advancing
    the pointer while it matches the class. */

    if reptype == REPTYPE_MIN {
        state = L_A_ECLASS_MIN_LOOP;
        continue 'sm;
    }
    /* If maximizing, find the longest possible run, then work backwards. */
    else {
        (*F).fields.eclass_repeat.start_eptr = Feptr!();
        i = (*F).fields.eclass_repeat.min;
        while i < (*F).fields.eclass_repeat.max {
            let mut len: i32 = 1;
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                break;
            }
            GETCHARLENTEST!(fc, Feptr!(), len, utf);
            if crate::xclass::_pcre2_eclass_8(
                fc,
                (*F).fields.eclass_repeat.eclass_data,
                (*F).fields
                    .eclass_repeat
                    .eclass_data
                    .add((*F).fields.eclass_repeat.eclass_len),
                (*mb).start_code,
                utf,
            ) == 0
            {
                break;
            }
            Feptr!() = Feptr!().add(len as usize);
            i += 1;
        }

        if reptype == REPTYPE_POS {
            state = S_MAINLOOP; /* continue - no backtracking */
            continue 'sm;
        }

        state = L_A_ECLASS_MAX_LOOP;
        continue 'sm;
    }
}
// ==== STATES ====
/* ------------------------------------------------------------------ */
/* Common code for OP_ACCEPT (not in a recursion) and OP_END. C line 976. */

L_A_OP_END => {
    /* Fail for an empty string match if either PCRE2_NOTEMPTY is set, or if
    PCRE2_NOTEMPTY_ATSTART is set and we have matched at the start of the
    subject. */

    if Feptr!() == Fstart_match!()
        && (((*mb).moptions & PCRE2_NOTEMPTY) != 0
            || (((*mb).moptions & PCRE2_NOTEMPTY_ATSTART) != 0
                && Fstart_match!() == (*mb).start_subject.add((*mb).start_offset)))
    {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    /* Fail if PCRE2_ENDANCHORED is set and the end of the match is not
    the end of the subject. */

    if Feptr!() < (*mb).end_subject
        && (((*mb).moptions | (*mb).poptions) & PCRE2_ENDANCHORED) != 0
    {
        if Fop!() as u32 == OP_END {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }

        return MATCH_NOMATCH; /* (*ACCEPT) */
    }

    /* Fail if we detect that the start position was moved to be either after
    the end position (\K in lookahead) or before the start offset (\K in
    lookbehind). */

    if Fstart_match!() < (*mb).start_subject.add((*mb).start_offset)
        || Fstart_match!() > Feptr!()
    {
        /* PCRE2_ASSERT(mb->hasbsk); */

        if (*mb).allowlookaroundbsk == 0 {
            return PCRE2_ERROR_BAD_BACKSLASH_K;
        }
    }

    /* We have a successful match of the whole pattern. */

    (*mb).end_match_ptr = Feptr!(); /* Record where we ended */
    (*mb).end_offset_top = Foffset_top!(); /* and how many extracts were taken */
    (*mb).mark = Fmark!(); /* and the last success mark */
    if Feptr!() > (*mb).last_used_ptr {
        (*mb).last_used_ptr = Feptr!();
    }

    *(*match_data).ovector.as_mut_ptr().add(0) =
        (Fstart_match!() as usize) - ((*mb).start_subject as usize);
    *(*match_data).ovector.as_mut_ptr().add(1) =
        (Feptr!() as usize) - ((*mb).start_subject as usize);

    /* Set i to the smaller of the sizes of the external and frame ovectors. */

    i = (2 * (if (top_bracket as i32 + 1) > (*match_data).oveccount as i32 {
        (*match_data).oveccount as i32
    } else {
        top_bracket as i32 + 1
    })) as u32;
    copy_nonoverlapping(
        Fovector!() as *const u8,
        (*match_data).ovector.as_mut_ptr().add(2) as *mut u8,
        ((i as usize) - 2) * core::mem::size_of::<PCRE2_SIZE>(),
    );
    loop {
        i = i.wrapping_sub(1);
        if !((i as PCRE2_SIZE) >= Foffset_top!() + 2) {
            break;
        }
        *(*match_data).ovector.as_mut_ptr().add(i as usize) = PCRE2_UNSET;
    }
    return MATCH_MATCH; /* Note: NOT RRETURN */
}

/* ------------------------------------------------------------------ */
/* OP_ALLANY, also reached by falling through from OP_ANY. C line 1076. */

L_A_ALLANY => {
    if Feptr!() >= (*mb).end_subject
    /* DO NOT merge the Feptr++ here; it must */
    {
        /* not be updated before SCHECK_PARTIAL. */
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().add(1);
    if utf != 0 {
        /* ACROSSCHAR(Feptr < mb->end_subject, Feptr, Feptr++) */
        while Feptr!() < (*mb).end_subject && (*Feptr!() & 0xc0u8) == 0x80u8 {
            Feptr!() = Feptr!().add(1);
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ------------------------------------------------------------------ */
/* REPEATCHAR: common code for all repeated single-character matches.
C line 1392. */

L_REPEATCHAR => {
    if utf != 0 {
        length = 1;
        (*F).fields.char_repeat.charptr = Fecode!();
        GETCHARLEN!(fc, Fecode!(), length);
        Fecode!() = Fecode!().add(length);
        (*F).byte1 = length as u8; /* Llength */

        /* Handle multi-code-unit character matching, caseful and caseless. */

        if length > 1 {
            /* if (Fop >= OP_STARI && (othercase = UCD_OTHERCASE(fc)) != fc) */
            let mut oc_cond_: bool = false;
            if (Fop!() as u32) >= OP_STARI
            /* Caseless */
            {
                othercase = UCD_OTHERCASE!(fc);
                oc_cond_ = othercase != fc;
            }
            if oc_cond_ {
                (*F).byte2 = crate::ord2utf::_pcre2_ord2utf_8(
                    othercase,
                    core::ptr::addr_of_mut!((*F).fields.char_repeat.oc.occu) as *mut u8,
                ) as u8;
            } else {
                (*F).byte2 = 0;
            }

            i = 1;
            while i <= (*F).fields.char_repeat.min {
                if Feptr!() <= (*mb).end_subject.wrapping_sub(length)
                    && frag_a_memcmp_eq(
                        Feptr!(),
                        (*F).fields.char_repeat.charptr,
                        length,
                    )
                {
                    Feptr!() = Feptr!().add(length);
                } else if (*F).byte2 > 0
                    && Feptr!() <= (*mb).end_subject.wrapping_sub((*F).byte2 as usize)
                    && frag_a_memcmp_eq(
                        Feptr!(),
                        core::ptr::addr_of!((*F).fields.char_repeat.oc.occu) as *const u8,
                        (*F).byte2 as usize,
                    )
                {
                    Feptr!() = Feptr!().add((*F).byte2 as usize);
                } else {
                    CHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                i += 1;
            }

            if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
                state = S_MAINLOOP; /* continue */
                continue 'sm;
            }

            if reptype == REPTYPE_MIN {
                state = L_A_UTF_MIN_LOOP;
                continue 'sm;
            } else
            /* Maximize */
            {
                (*F).fields.char_repeat.start_eptr = Feptr!();
                i = (*F).fields.char_repeat.min;
                while i < (*F).fields.char_repeat.max {
                    if Feptr!() <= (*mb).end_subject.wrapping_sub((*F).byte1 as usize)
                        && frag_a_memcmp_eq(
                            Feptr!(),
                            (*F).fields.char_repeat.charptr,
                            (*F).byte1 as usize,
                        )
                    {
                        Feptr!() = Feptr!().add((*F).byte1 as usize);
                    } else if (*F).byte2 > 0
                        && Feptr!()
                            <= (*mb).end_subject.wrapping_sub((*F).byte2 as usize)
                        && frag_a_memcmp_eq(
                            Feptr!(),
                            core::ptr::addr_of!((*F).fields.char_repeat.oc.occu)
                                as *const u8,
                            (*F).byte2 as usize,
                        )
                    {
                        Feptr!() = Feptr!().add((*F).byte2 as usize);
                    } else {
                        CHECK_PARTIAL!();
                        break;
                    }
                    i += 1;
                }

                /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                Unicode character. Use <= Lstart_eptr to ensure backtracking
                doesn't go too far. */

                if reptype != REPTYPE_POS {
                    state = L_A_UTF_MAX_LOOP;
                    continue 'sm;
                }
            }
            state = S_MAINLOOP; /* break: end of repeated wide character handling */
            continue 'sm;
        }

        /* Length of UTF character is 1. Put it into the preserved variable and
        fall through to the non-UTF code. */

        (*F).fields.char_repeat.c = fc;
    }
    /* When not in UTF mode, load a single-code-unit character. */
    else {
        (*F).fields.char_repeat.c = *Fecode!() as u32;
        Fecode!() = Fecode!().add(1);
    }

    /* Caseless comparison */

    if (Fop!() as u32) >= OP_STARI {
        if ucp != 0 && utf == 0 && (*F).fields.char_repeat.c > 127 {
            (*F).fields.char_repeat.oc.oc = UCD_OTHERCASE!((*F).fields.char_repeat.c);
        } else {
            /* Lc will be < 128 in UTF-8 mode. */
            (*F).fields.char_repeat.oc.oc =
                *(*mb).fcc.add((*F).fields.char_repeat.c as usize) as u32;
        }

        i = 1;
        while i <= (*F).fields.char_repeat.min {
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            let cc: u32 = *Feptr!() as u32;
            if (*F).fields.char_repeat.c != cc && (*F).fields.char_repeat.oc.oc != cc {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            Feptr!() = Feptr!().add(1);
            i += 1;
        }
        if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
            state = S_MAINLOOP; /* continue */
            continue 'sm;
        }

        if reptype == REPTYPE_MIN {
            state = L_A_CI_MIN_LOOP;
            continue 'sm;
        } else
        /* Maximize */
        {
            (*F).fields.char_repeat.start_eptr = Feptr!();
            i = (*F).fields.char_repeat.min;
            while i < (*F).fields.char_repeat.max {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    break;
                }
                let cc: u32 = *Feptr!() as u32;
                if (*F).fields.char_repeat.c != cc
                    && (*F).fields.char_repeat.oc.oc != cc
                {
                    break;
                }
                Feptr!() = Feptr!().add(1);
                i += 1;
            }
            if reptype != REPTYPE_POS {
                state = L_A_CI_MAX_LOOP;
                continue 'sm;
            }
        }
    }
    /* Caseful comparisons (includes all multi-byte characters) */
    else {
        i = 1;
        while i <= (*F).fields.char_repeat.min {
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            let cc: u32 = *Feptr!() as u32;
            Feptr!() = Feptr!().add(1);
            if (*F).fields.char_repeat.c != cc {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            i += 1;
        }

        if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
            state = S_MAINLOOP; /* continue */
            continue 'sm;
        }

        if reptype == REPTYPE_MIN {
            state = L_A_CF_MIN_LOOP;
            continue 'sm;
        } else
        /* Maximize */
        {
            (*F).fields.char_repeat.start_eptr = Feptr!();
            i = (*F).fields.char_repeat.min;
            while i < (*F).fields.char_repeat.max {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    break;
                }

                if (*F).fields.char_repeat.c != *Feptr!() as u32 {
                    break;
                }
                Feptr!() = Feptr!().add(1);
                i += 1;
            }

            if reptype != REPTYPE_POS {
                state = L_A_CF_MAX_LOOP;
                continue 'sm;
            }
        }
    }
    state = S_MAINLOOP; /* break */
    continue 'sm;
}

/* --- REPEATCHAR, UTF multi-unit, minimizing (C lines 1432..1448) --- */

L_A_UTF_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM202 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM202 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.char_repeat.min;
    (*F).fields.char_repeat.min = t_ + 1;
    if t_ >= (*F).fields.char_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() <= (*mb).end_subject.wrapping_sub((*F).byte1 as usize)
        && frag_a_memcmp_eq(
            Feptr!(),
            (*F).fields.char_repeat.charptr,
            (*F).byte1 as usize,
        )
    {
        Feptr!() = Feptr!().add((*F).byte1 as usize);
    } else if (*F).byte2 > 0
        && Feptr!() <= (*mb).end_subject.wrapping_sub((*F).byte2 as usize)
        && frag_a_memcmp_eq(
            Feptr!(),
            core::ptr::addr_of!((*F).fields.char_repeat.oc.occu) as *const u8,
            (*F).byte2 as usize,
        )
    {
        Feptr!() = Feptr!().add((*F).byte2 as usize);
    } else {
        CHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_UTF_MIN_LOOP;
    continue 'sm;
}

/* --- REPEATCHAR, UTF multi-unit, maximizing (C lines 1474..1481) --- */

L_A_UTF_MAX_LOOP => {
    if Feptr!() <= (*F).fields.char_repeat.start_eptr {
        state = S_MAINLOOP; /* break out of the for(;;), then break the switch */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM203 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM203 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub(1);
    BACKCHAR!(Feptr!());
    state = L_A_UTF_MAX_LOOP;
    continue 'sm;
}

/* --- REPEATCHAR, caseless, minimizing (C lines 1534..1548) --- */

L_A_CI_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM25 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM25 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.char_repeat.min;
    (*F).fields.char_repeat.min = t_ + 1;
    if t_ >= (*F).fields.char_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    let cc: u32 = *Feptr!() as u32;
    if (*F).fields.char_repeat.c != cc && (*F).fields.char_repeat.oc.oc != cc {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().add(1);
    state = L_A_CI_MIN_LOOP;
    continue 'sm;
}

/* --- REPEATCHAR, caseless, maximizing (C lines 1567..1573) --- */

L_A_CI_MAX_LOOP => {
    if Feptr!() == (*F).fields.char_repeat.start_eptr {
        state = S_MAINLOOP; /* break */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM26 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM26 => {
    Feptr!() = Feptr!().wrapping_sub(1);
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    state = L_A_CI_MAX_LOOP;
    continue 'sm;
}

/* --- REPEATCHAR, caseful, minimizing (C lines 1595..1606) --- */

L_A_CF_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM27 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM27 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.char_repeat.min;
    (*F).fields.char_repeat.min = t_ + 1;
    if t_ >= (*F).fields.char_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    let cc: u32 = *Feptr!() as u32;
    Feptr!() = Feptr!().add(1);
    if (*F).fields.char_repeat.c != cc {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_CF_MIN_LOOP;
    continue 'sm;
}

/* --- REPEATCHAR, caseful, maximizing (C lines 1624..1630) --- */

L_A_CF_MAX_LOOP => {
    if Feptr!() <= (*F).fields.char_repeat.start_eptr {
        state = S_MAINLOOP; /* break */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM28 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM28 => {
    Feptr!() = Feptr!().wrapping_sub(1);
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    state = L_A_CF_MAX_LOOP;
    continue 'sm;
}

/* ------------------------------------------------------------------ */
/* REPEATNOTCHAR: common code for all repeated single-character
non-matches. C line 1733. */

L_REPEATNOTCHAR => {
    GETCHARINCTEST!((*F).fields.charnot_repeat.c, Fecode!(), utf);

    /* The code is duplicated for the caseless and caseful cases, for speed. */

    if (Fop!() as u32) >= OP_NOTSTARI
    /* Caseless */
    {
        if (utf != 0 || ucp != 0) && (*F).fields.charnot_repeat.c > 127 {
            (*F).fields.charnot_repeat.oc =
                UCD_OTHERCASE!((*F).fields.charnot_repeat.c);
        } else {
            (*F).fields.charnot_repeat.oc = TABLE_GET!(
                (*F).fields.charnot_repeat.c,
                (*mb).fcc,
                (*F).fields.charnot_repeat.c
            ) as u32; /* Other case from table */
        }

        if utf != 0 {
            let mut d: u32 = 0;
            i = 1;
            while i <= (*F).fields.charnot_repeat.min {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                GETCHARINC!(d, Feptr!());
                if (*F).fields.charnot_repeat.c == d
                    || (*F).fields.charnot_repeat.oc == d
                {
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                i += 1;
            }
        }
        /* Not UTF mode */
        else {
            i = 1;
            while i <= (*F).fields.charnot_repeat.min {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                if (*F).fields.charnot_repeat.c == *Feptr!() as u32
                    || (*F).fields.charnot_repeat.oc == *Feptr!() as u32
                {
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                Feptr!() = Feptr!().add(1);
                i += 1;
            }
        }

        if (*F).fields.charnot_repeat.min == (*F).fields.charnot_repeat.max {
            state = S_MAINLOOP; /* continue - finished for exact count */
            continue 'sm;
        }

        if reptype == REPTYPE_MIN {
            if utf != 0 {
                state = L_A_N_CI_UMIN_LOOP;
                continue 'sm;
            }
            /* Not UTF mode */
            else {
                state = L_A_N_CI_MIN_LOOP;
                continue 'sm;
            }
        }
        /* Maximize case */
        else {
            (*F).fields.charnot_repeat.start_eptr = Feptr!();

            if utf != 0 {
                let mut d: u32 = 0;
                i = (*F).fields.charnot_repeat.min;
                while i < (*F).fields.charnot_repeat.max {
                    let mut len: i32 = 1;
                    if Feptr!() >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        break;
                    }
                    GETCHARLEN!(d, Feptr!(), len);
                    if (*F).fields.charnot_repeat.c == d
                        || (*F).fields.charnot_repeat.oc == d
                    {
                        break;
                    }
                    Feptr!() = Feptr!().add(len as usize);
                    i += 1;
                }

                /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                Unicode character. Use <= Lstart_eptr to ensure backtracking
                doesn't go too far. */

                if reptype != REPTYPE_POS {
                    state = L_A_N_CI_UMAX_LOOP;
                    continue 'sm;
                }
            }
            /* Not UTF mode */
            else {
                i = (*F).fields.charnot_repeat.min;
                while i < (*F).fields.charnot_repeat.max {
                    if Feptr!() >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        break;
                    }
                    if (*F).fields.charnot_repeat.c == *Feptr!() as u32
                        || (*F).fields.charnot_repeat.oc == *Feptr!() as u32
                    {
                        break;
                    }
                    Feptr!() = Feptr!().add(1);
                    i += 1;
                }
                if reptype != REPTYPE_POS {
                    state = L_A_N_CI_MAX_LOOP;
                    continue 'sm;
                }
            }
        }
    }
    /* Caseful comparisons */
    else {
        if utf != 0 {
            let mut d: u32 = 0;
            i = 1;
            while i <= (*F).fields.charnot_repeat.min {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                GETCHARINC!(d, Feptr!());
                if (*F).fields.charnot_repeat.c == d {
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                i += 1;
            }
        }
        /* Not UTF mode */
        else {
            i = 1;
            while i <= (*F).fields.charnot_repeat.min {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                let cc_ = *Feptr!() as u32;
                Feptr!() = Feptr!().add(1);
                if (*F).fields.charnot_repeat.c == cc_ {
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                i += 1;
            }
        }

        if (*F).fields.charnot_repeat.min == (*F).fields.charnot_repeat.max {
            state = S_MAINLOOP; /* continue */
            continue 'sm;
        }

        if reptype == REPTYPE_MIN {
            if utf != 0 {
                state = L_A_N_CF_UMIN_LOOP;
                continue 'sm;
            }
            /* Not UTF mode */
            else {
                state = L_A_N_CF_MIN_LOOP;
                continue 'sm;
            }
        }
        /* Maximize case */
        else {
            (*F).fields.charnot_repeat.start_eptr = Feptr!();

            if utf != 0 {
                let mut d: u32 = 0;
                i = (*F).fields.charnot_repeat.min;
                while i < (*F).fields.charnot_repeat.max {
                    let mut len: i32 = 1;
                    if Feptr!() >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        break;
                    }
                    GETCHARLEN!(d, Feptr!(), len);
                    if (*F).fields.charnot_repeat.c == d {
                        break;
                    }
                    Feptr!() = Feptr!().add(len as usize);
                    i += 1;
                }

                /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                Unicode character. */

                if reptype != REPTYPE_POS {
                    state = L_A_N_CF_UMAX_LOOP;
                    continue 'sm;
                }
            }
            /* Not UTF mode */
            else {
                i = (*F).fields.charnot_repeat.min;
                while i < (*F).fields.charnot_repeat.max {
                    if Feptr!() >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        break;
                    }
                    if (*F).fields.charnot_repeat.c == *Feptr!() as u32 {
                        break;
                    }
                    Feptr!() = Feptr!().add(1);
                    i += 1;
                }
                if reptype != REPTYPE_POS {
                    state = L_A_N_CF_MAX_LOOP;
                    continue 'sm;
                }
            }
        }
    }
    state = S_MAINLOOP; /* break */
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseless, UTF, minimizing (C 1794..1806) --- */

L_A_N_CI_UMIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM204 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM204 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.charnot_repeat.min;
    (*F).fields.charnot_repeat.min = t_ + 1;
    if t_ >= (*F).fields.charnot_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    let mut d: u32 = 0;
    GETCHARINC!(d, Feptr!());
    if (*F).fields.charnot_repeat.c == d || (*F).fields.charnot_repeat.oc == d {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_N_CI_UMIN_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseless, non-UTF, minimizing (C 1813..1825) --- */

L_A_N_CI_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM29 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM29 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.charnot_repeat.min;
    (*F).fields.charnot_repeat.min = t_ + 1;
    if t_ >= (*F).fields.charnot_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if (*F).fields.charnot_repeat.c == *Feptr!() as u32
        || (*F).fields.charnot_repeat.oc == *Feptr!() as u32
    {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().add(1);
    state = L_A_N_CI_MIN_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseless, UTF, maximizing (C 1857..1864) --- */

L_A_N_CI_UMAX_LOOP => {
    if Feptr!() <= (*F).fields.charnot_repeat.start_eptr {
        state = S_MAINLOOP; /* break */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM205 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM205 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub(1);
    BACKCHAR!(Feptr!());
    state = L_A_N_CI_UMAX_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseless, non-UTF, maximizing (C 1881..1887) --- */

L_A_N_CI_MAX_LOOP => {
    if Feptr!() == (*F).fields.charnot_repeat.start_eptr {
        state = S_MAINLOOP; /* break */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM30 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM30 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub(1);
    state = L_A_N_CI_MAX_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseful, UTF, minimizing (C 1934..1946) --- */

L_A_N_CF_UMIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM206 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM206 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.charnot_repeat.min;
    (*F).fields.charnot_repeat.min = t_ + 1;
    if t_ >= (*F).fields.charnot_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    let mut d: u32 = 0;
    GETCHARINC!(d, Feptr!());
    if (*F).fields.charnot_repeat.c == d {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_N_CF_UMIN_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseful, non-UTF, minimizing (C 1952..1963) --- */

L_A_N_CF_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM31 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM31 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.charnot_repeat.min;
    (*F).fields.charnot_repeat.min = t_ + 1;
    if t_ >= (*F).fields.charnot_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    let cc_ = *Feptr!() as u32;
    Feptr!() = Feptr!().add(1);
    if (*F).fields.charnot_repeat.c == cc_ {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_N_CF_MIN_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseful, UTF, maximizing (C 1995..2002) --- */

L_A_N_CF_UMAX_LOOP => {
    if Feptr!() <= (*F).fields.charnot_repeat.start_eptr {
        state = S_MAINLOOP; /* break */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM207 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM207 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub(1);
    BACKCHAR!(Feptr!());
    state = L_A_N_CF_UMAX_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseful, non-UTF, maximizing (C 2018..2024) --- */

L_A_N_CF_MAX_LOOP => {
    if Feptr!() == (*F).fields.charnot_repeat.start_eptr {
        state = S_MAINLOOP; /* break */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM32 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM32 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub(1);
    state = L_A_N_CF_MAX_LOOP;
    continue 'sm;
}

/* --- OP_CLASS/OP_NCLASS, UTF, minimizing (C 2148..2165) --- */

L_A_CLASS_UMIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM200 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM200 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.class_repeat.min;
    (*F).fields.class_repeat.min = t_ + 1;
    if t_ >= (*F).fields.class_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINC!(fc, Feptr!());
    if fc > 255 {
        if Fop!() as u32 == OP_CLASS {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    } else if (*(*F).fields.class_repeat.byte_map_address.add((fc / 8) as usize) as u32
        & (1u32 << (fc & 7)))
        == 0
    {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_CLASS_UMIN_LOOP;
    continue 'sm;
}

/* --- OP_CLASS/OP_NCLASS, non-UTF, minimizing (C 2171..2190) --- */

L_A_CLASS_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM23 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM23 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.class_repeat.min;
    (*F).fields.class_repeat.min = t_ + 1;
    if t_ >= (*F).fields.class_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    fc = *Feptr!() as u32;
    Feptr!() = Feptr!().add(1);
    if (*(*F).fields.class_repeat.byte_map_address.add((fc / 8) as usize) as u32
        & (1u32 << (fc & 7)))
        == 0
    {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_CLASS_MIN_LOOP;
    continue 'sm;
}

/* --- OP_CLASS/OP_NCLASS, UTF, maximizing (C 2228..2234) --- */

L_A_CLASS_UMAX_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM201 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM201 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let old_ = Feptr!();
    Feptr!() = Feptr!().wrapping_sub(1);
    if old_ <= (*F).fields.class_repeat.start_eptr {
        /* break: tried at original position; then RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    BACKCHAR!(Feptr!());
    state = L_A_CLASS_UMAX_LOOP;
    continue 'sm;
}

/* --- OP_CLASS/OP_NCLASS, non-UTF, maximizing (C 2261..2266) --- */

L_A_CLASS_MAX_LOOP => {
    if !(Feptr!() >= (*F).fields.class_repeat.start_eptr) {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM24 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM24 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub(1);
    state = L_A_CLASS_MAX_LOOP;
    continue 'sm;
}

/* --- OP_XCLASS, minimizing (C 2355..2369) --- */

L_A_XCLASS_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM100 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM100 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.xclass_repeat.min;
    (*F).fields.xclass_repeat.min = t_ + 1;
    if t_ >= (*F).fields.xclass_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if crate::xclass::_pcre2_xclass_8(
        fc,
        (*F).fields.xclass_repeat.xclass_data,
        (*mb).start_code,
        utf,
    ) == 0
    {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_XCLASS_MIN_LOOP;
    continue 'sm;
}

/* --- OP_XCLASS, maximizing (C 2402..2411) --- */

L_A_XCLASS_MAX_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM101 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM101 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let old_ = Feptr!();
    Feptr!() = Feptr!().wrapping_sub(1);
    if old_ <= (*F).fields.xclass_repeat.start_eptr {
        /* break: tried at original position; then RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if utf != 0 {
        BACKCHAR!(Feptr!());
    }
    state = L_A_XCLASS_MAX_LOOP;
    continue 'sm;
}

/* --- OP_ECLASS, minimizing (C 2498..2512) --- */

L_A_ECLASS_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM102 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM102 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.eclass_repeat.min;
    (*F).fields.eclass_repeat.min = t_ + 1;
    if t_ >= (*F).fields.eclass_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if crate::xclass::_pcre2_eclass_8(
        fc,
        (*F).fields.eclass_repeat.eclass_data,
        (*F).fields
            .eclass_repeat
            .eclass_data
            .add((*F).fields.eclass_repeat.eclass_len),
        (*mb).start_code,
        utf,
    ) == 0
    {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_ECLASS_MIN_LOOP;
    continue 'sm;
}

/* --- OP_ECLASS, maximizing (C 2546..2555) --- */

L_A_ECLASS_MAX_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM103 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM103 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let old_ = Feptr!();
    Feptr!() = Feptr!().wrapping_sub(1);
    if old_ <= (*F).fields.eclass_repeat.start_eptr {
        /* break: tried at original position; then RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if utf != 0 {
        BACKCHAR!(Feptr!());
    }
    state = L_A_ECLASS_MAX_LOOP;
    continue 'sm;
}
