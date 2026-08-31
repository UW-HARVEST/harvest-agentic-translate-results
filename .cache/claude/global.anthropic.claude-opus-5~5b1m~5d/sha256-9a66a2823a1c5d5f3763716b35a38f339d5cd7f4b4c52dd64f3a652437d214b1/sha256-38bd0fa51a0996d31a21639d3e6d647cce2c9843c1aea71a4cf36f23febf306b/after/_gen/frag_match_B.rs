// ==== EXTRA_STATE_CONSTS ====
/* ------------------------------------------------------------------ *
 * Chunk B (C lines 2574..5488) synthetic states, 1100..1199.
 *
 * Every one of these is a "loop entry" state: the C code has an
 * RMATCH() inside a for(;;) loop, so instead of duplicating the loop
 * body we give the top of the loop its own state and the RMxx state
 * jumps back to it.
 * ------------------------------------------------------------------ */

/* REPEATTYPE, minimizing (REPTYPE_MIN) repeats - property tests */
pub const L_B_TR_MIN_LAMP: u32 = 1100; /* C 3784, RM208 */
pub const L_B_TR_MIN_GC: u32 = 1101; /* C 3805, RM209 */
pub const L_B_TR_MIN_PC: u32 = 1102; /* C 3822, RM210 */
pub const L_B_TR_MIN_SC: u32 = 1103; /* C 3839, RM211 */
pub const L_B_TR_MIN_SCX: u32 = 1104; /* C 3856, RM224 */
pub const L_B_TR_MIN_ALNUM: u32 = 1105; /* C 3878, RM212 */
pub const L_B_TR_MIN_SPACE: u32 = 1106; /* C 3902, RM213 */
pub const L_B_TR_MIN_WORD: u32 = 1107; /* C 3929, RM214 */
pub const L_B_TR_MIN_CLIST: u32 = 1108; /* C 3952, RM215 */
pub const L_B_TR_MIN_UCNC: u32 = 1109; /* C 3989, RM216 */
pub const L_B_TR_MIN_BIDICL: u32 = 1110; /* C 4008, RM223 */
pub const L_B_TR_MIN_BOOL: u32 = 1111; /* C 4025, RM222 */
/* REPEATTYPE, minimizing repeats - other character types */
pub const L_B_TR_MIN_EXTUNI: u32 = 1112; /* C 4061, RM217 */
pub const L_B_TR_MIN_UTF: u32 = 1113; /* C 4088, RM218 */
pub const L_B_TR_MIN_NOUTF: u32 = 1114; /* C 4218, RM33 */
/* REPEATTYPE, maximizing repeats - the backtracking loops */
pub const L_B_TR_MAX_PROP_BT: u32 = 1115; /* C 4638, RM221 */
pub const L_B_TR_MAX_EXTUNI_BT: u32 = 1116; /* C 4678, RM219 */
pub const L_B_TR_MAX_UTF_BT: u32 = 1117; /* C 4957, RM220 */
pub const L_B_TR_MAX_NOUTF_BT: u32 = 1118; /* C 5213, RM34 */
/* Repeated back references */
pub const L_B_REF_MINLOOP: u32 = 1119; /* C 5360, RM20 */
pub const L_B_REF_SAMELEN: u32 = 1120; /* C 5421, RM21 */
pub const L_B_REF_DIFFLEN: u32 = 1121; /* C 5435, RM22 */

/* The HSPACE_xxx_CASES / VSPACE_xxx_CASES lists of pcre2_internal.h,
8-bit mode, not EBCDIC. They are used in pattern position, so they must
be macros expanding to or-patterns. The scrutinee is always cast to u32. */

macro_rules! B_HSPACE_BYTE_CASES {
    () => {
        0x09u32 | 0x20u32 | 0xa0u32
    };
}
macro_rules! B_HSPACE_CASES {
    () => {
        0x09u32
            | 0x20u32
            | 0xa0u32
            | 0x1680u32
            | 0x180eu32
            | 0x2000u32
            | 0x2001u32
            | 0x2002u32
            | 0x2003u32
            | 0x2004u32
            | 0x2005u32
            | 0x2006u32
            | 0x2007u32
            | 0x2008u32
            | 0x2009u32
            | 0x200au32
            | 0x202fu32
            | 0x205fu32
            | 0x3000u32
    };
}
macro_rules! B_VSPACE_BYTE_CASES {
    () => {
        0x0au32 | 0x0bu32 | 0x0cu32 | 0x0du32 | 0x85u32
    };
}
macro_rules! B_VSPACE_CASES {
    () => {
        0x0au32 | 0x0bu32 | 0x0cu32 | 0x0du32 | 0x85u32 | 0x2028u32 | 0x2029u32
    };
}
macro_rules! B_HSPACE_VSPACE_CASES {
    () => {
        0x09u32
            | 0x20u32
            | 0xa0u32
            | 0x1680u32
            | 0x180eu32
            | 0x2000u32
            | 0x2001u32
            | 0x2002u32
            | 0x2003u32
            | 0x2004u32
            | 0x2005u32
            | 0x2006u32
            | 0x2007u32
            | 0x2008u32
            | 0x2009u32
            | 0x200au32
            | 0x202fu32
            | 0x205fu32
            | 0x3000u32
            | 0x0au32
            | 0x0bu32
            | 0x0cu32
            | 0x0du32
            | 0x85u32
            | 0x2028u32
            | 0x2029u32
    };
}
// ==== EXTRA_LOCALS ====
/* Chunk B needs no extra function-scope locals: every variable that C
declares inside one of its case blocks and that has to survive an
RMATCH() split point is already a field of the heap frame. */
// ==== ARMS ====
/* ===================================================================== */
/* Match various character types when PCRE2_UCP is not set. These opcodes
are not generated when PCRE2_UCP is set - instead appropriate property
tests are compiled. */

/* C 2574 */
OP_NOT_DIGIT => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if CHMAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2586 */
OP_DIGIT => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if CHMAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2598 */
OP_NOT_WHITESPACE => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if CHMAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2610 */
OP_WHITESPACE => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if CHMAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2622 */
OP_NOT_WORDCHAR => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if CHMAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2634 */
OP_WORDCHAR => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if CHMAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2646 */
OP_ANYNL => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    match fc {
        0x0du32 /* CHAR_CR */ => {
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
            } else if *Feptr!() == 0x0au8 /* CHAR_LF */ {
                Feptr!() = Feptr!().add(1);
            }
        }

        0x0au32 /* CHAR_LF */ => {}

        0x0bu32 | 0x0cu32 | 0x85u32 | 0x2028u32 | 0x2029u32 => {
            if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        _ => {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2681 */
OP_NOT_HSPACE => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    match fc {
        B_HSPACE_CASES!() => {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        _ => {}
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2696 */
OP_HSPACE => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    match fc {
        B_HSPACE_CASES!() => {}
        _ => {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2711 */
OP_NOT_VSPACE => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    match fc {
        B_VSPACE_CASES!() => {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        _ => {}
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2726 */
OP_VSPACE => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    match fc {
        B_VSPACE_CASES!() => {}
        _ => {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Check the next character by Unicode property. We will get here only
if the support is in the binary; otherwise a compile-time error occurs. */

/* C 2748 */
OP_PROP | OP_NOTPROP => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let chartype: u32;
        let prop: *const ucd_record = GET_UCD!(fc);
        let notmatch: BOOL = (Fop!() as u32 == OP_NOTPROP) as BOOL;

        match *Fecode!().add(1) as u32 {
            PT_LAMP => {
                chartype = (*prop).chartype as u32;
                if ((chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt) as BOOL)
                    == notmatch
                {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_GC => {
                if ((*Fecode!().add(2) as u32
                    == crate::tables::_pcre2_ucp_gentype_8[(*prop).chartype as usize])
                    as BOOL)
                    == notmatch
                {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_PC => {
                if ((*Fecode!().add(2) == (*prop).chartype) as BOOL) == notmatch {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_SC => {
                if ((*Fecode!().add(2) == (*prop).script) as BOOL) == notmatch {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_SCX => {
                let ok: BOOL = ((*Fecode!().add(2) == (*prop).script)
                    || MAPBIT!(
                        crate::ucd::_pcre2_ucd_script_sets_8
                            .as_ptr()
                            .add(UCD_SCRIPTX_PROP!(prop) as usize),
                        *Fecode!().add(2)
                    ) != 0) as BOOL;
                if ok == notmatch {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            /* These are specials */
            PT_ALNUM => {
                chartype = (*prop).chartype as u32;
                if ((crate::tables::_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                    || crate::tables::_pcre2_ucp_gentype_8[chartype as usize] == ucp_N)
                    as BOOL)
                    == notmatch
                {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            /* Perl space used to exclude VT, but from Perl 5.18 it is included,
            which means that Perl space and POSIX space are now identical. PCRE
            was changed at release 8.34. */
            PT_SPACE | PT_PXSPACE => match fc {
                B_HSPACE_VSPACE_CASES!() => {
                    if notmatch != 0 {
                        rrc = MATCH_NOMATCH;
                        state = S_RETURN_SWITCH;
                        continue 'sm;
                    }
                }

                _ => {
                    if ((crate::tables::_pcre2_ucp_gentype_8[(*prop).chartype as usize] == ucp_Z)
                        as BOOL)
                        == notmatch
                    {
                        rrc = MATCH_NOMATCH;
                        state = S_RETURN_SWITCH;
                        continue 'sm;
                    }
                }
            },

            PT_WORD => {
                chartype = (*prop).chartype as u32;
                if ((crate::tables::_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                    || crate::tables::_pcre2_ucp_gentype_8[chartype as usize] == ucp_N
                    || chartype == ucp_Mn
                    || chartype == ucp_Pc) as BOOL)
                    == notmatch
                {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_CLIST => {
                let mut cp: *const u32 = crate::ucd::_pcre2_ucd_caseless_sets_8
                    .as_ptr()
                    .add(*Fecode!().add(2) as usize);
                loop {
                    if fc < *cp {
                        if notmatch != 0 {
                            break;
                        } else {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                    }
                    let t = *cp;
                    cp = cp.add(1);
                    if fc == t {
                        if notmatch != 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        } else {
                            break;
                        }
                    }
                }
            }

            PT_UCNC => {
                if ((fc == 0x24u32 /* CHAR_DOLLAR_SIGN */
                    || fc == 0x40u32 /* CHAR_COMMERCIAL_AT */
                    || fc == 0x60u32 /* CHAR_GRAVE_ACCENT */
                    || (fc >= 0xa0 && fc <= 0xd7ff)
                    || fc >= 0xe000) as BOOL)
                    == notmatch
                {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_BIDICL => {
                if ((UCD_BIDICLASS_PROP!(prop) == *Fecode!().add(2) as u32) as BOOL) == notmatch {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_BOOL => {
                let ok: BOOL = (MAPBIT!(
                    crate::ucd::_pcre2_ucd_boolprop_sets_8
                        .as_ptr()
                        .add(UCD_BPROPS_PROP!(prop) as usize),
                    *Fecode!().add(2)
                ) != 0) as BOOL;
                if ok == notmatch {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            /* This should never occur */
            _ => {
                return PCRE2_ERROR_INTERNAL;
            }
        }

        Fecode!() = Fecode!().add(3);
    }
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Match an extended Unicode sequence. We will get here only if the support
is in the binary; otherwise a compile-time error occurs. */

/* C 2889 */
OP_EXTUNI => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    } else {
        GETCHARINCTEST!(fc, Feptr!(), utf);
        Feptr!() = crate::extuni::_pcre2_extuni_8(
            fc,
            Feptr!(),
            (*mb).start_subject,
            (*mb).end_subject,
            utf,
            null_mut(),
        );
    }
    CHECK_PARTIAL!();
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Match a single character type repeatedly. Note that the property type
does not need to be in a stack frame as it is not used within an RMATCH()
loop.

  Lstart_eptr -> (*F).fields.type_repeat.start_eptr
  Lmin        -> (*F).fields.type_repeat.min
  Lmax        -> (*F).fields.type_repeat.max
  Lctype      -> (*F).fields.type_repeat.ctype
  Lpropvalue  -> (*F).fields.type_repeat.propvalue                     */

/* C 2919 */
OP_TYPEEXACT => {
    (*F).fields.type_repeat.max = GET2!(Fecode!(), 1);
    (*F).fields.type_repeat.min = (*F).fields.type_repeat.max;
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATTYPE; /* goto REPEATTYPE */
    continue 'sm;
}

/* C 2924 */
OP_TYPEUPTO | OP_TYPEMINUPTO => {
    (*F).fields.type_repeat.min = 0;
    (*F).fields.type_repeat.max = GET2!(Fecode!(), 1);
    reptype = if *Fecode!() as u32 == OP_TYPEMINUPTO {
        REPTYPE_MIN
    } else {
        REPTYPE_MAX
    };
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATTYPE; /* goto REPEATTYPE */
    continue 'sm;
}

/* C 2932 */
OP_TYPEPOSSTAR => {
    reptype = REPTYPE_POS;
    (*F).fields.type_repeat.min = 0;
    (*F).fields.type_repeat.max = u32::MAX;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATTYPE; /* goto REPEATTYPE */
    continue 'sm;
}

/* C 2939 */
OP_TYPEPOSPLUS => {
    reptype = REPTYPE_POS;
    (*F).fields.type_repeat.min = 1;
    (*F).fields.type_repeat.max = u32::MAX;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATTYPE; /* goto REPEATTYPE */
    continue 'sm;
}

/* C 2946 */
OP_TYPEPOSQUERY => {
    reptype = REPTYPE_POS;
    (*F).fields.type_repeat.min = 0;
    (*F).fields.type_repeat.max = 1;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATTYPE; /* goto REPEATTYPE */
    continue 'sm;
}

/* C 2953 */
OP_TYPEPOSUPTO => {
    reptype = REPTYPE_POS;
    (*F).fields.type_repeat.min = 0;
    (*F).fields.type_repeat.max = GET2!(Fecode!(), 1);
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATTYPE; /* goto REPEATTYPE */
    continue 'sm;
}

/* C 2960 */
OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
| OP_TYPEMINQUERY => {
    /* fc = *Fecode++ - OP_TYPESTAR; */
    {
        let t = *Fecode!();
        Fecode!() = Fecode!().add(1);
        fc = (t as u32).wrapping_sub(OP_TYPESTAR);
    }
    (*F).fields.type_repeat.min = rep_min[fc as usize];
    (*F).fields.type_repeat.max = rep_max[fc as usize];
    reptype = rep_typ[fc as usize];

    /* Fall through into the common code at REPEATTYPE. */
    state = L_REPEATTYPE;
    continue 'sm;
}

/* ===================================================================== */
/* Match a back reference, possibly repeatedly. Look past the end of the
item to see if there is repeat information following. The OP_REF and
OP_REFI opcodes are used for a reference to a numbered group or to a
non-duplicated named group. For a duplicated named group, OP_DNREF and
OP_DNREFI are used. In this case we must scan the list of groups to which
the name refers, and use the first one that is set.

  Lstart    -> (*F).fields.ref_repeat.start
  Loffset   -> (*F).fields.ref_repeat.offset
  Llength   -> (*F).fields.ref_repeat.length
  Lmin      -> (*F).fields.ref_repeat.min
  Lmax      -> (*F).fields.ref_repeat.max
  Lcaseless -> (*F).byte1
  Lcaseopts -> (*F).byte2                                              */

/* C 5249 */
OP_DNREF | OP_DNREFI => {
    (*F).byte1 = (Fop!() as u32 == OP_DNREFI) as u8;
    (*F).byte2 = if Fop!() as u32 == OP_DNREFI {
        *Fecode!().add(1 + 2 * IMM2_SIZE)
    } else {
        0
    };
    {
        let mut count: i32 = GET2!(Fecode!(), 1 + IMM2_SIZE) as i32;
        let mut slot: PCRE2_SPTR = (*mb)
            .name_table
            .add((GET2!(Fecode!(), 1) * (*mb).name_entry_size as u32) as usize);
        Fecode!() = Fecode!()
            .add(1 + 2 * IMM2_SIZE + (if Fop!() as u32 == OP_DNREFI { 1 } else { 0 }));

        loop {
            let t = count;
            count = count - 1;
            if !(t > 0) {
                break;
            }
            (*F).fields.ref_repeat.offset =
                (GET2!(slot, 0) << 1).wrapping_sub(2) as PCRE2_SIZE;
            if (*F).fields.ref_repeat.offset < Foffset_top!()
                && *Fovector!().add((*F).fields.ref_repeat.offset) != PCRE2_UNSET
            {
                break;
            }
            slot = slot.add((*mb).name_entry_size as usize);
        }
    }
    state = L_REF_REPEAT; /* goto REF_REPEAT */
    continue 'sm;
}

/* C 5267 */
OP_REF | OP_REFI => {
    (*F).byte1 = (Fop!() as u32 == OP_REFI) as u8;
    (*F).byte2 = if Fop!() as u32 == OP_REFI {
        *Fecode!().add(1 + IMM2_SIZE)
    } else {
        0
    };
    (*F).fields.ref_repeat.offset = (GET2!(Fecode!(), 1) << 1).wrapping_sub(2) as PCRE2_SIZE;
    Fecode!() =
        Fecode!().add(1 + IMM2_SIZE + (if Fop!() as u32 == OP_REFI { 1 } else { 0 }));

    /* Fall through into the common code at REF_REPEAT. */
    state = L_REF_REPEAT;
    continue 'sm;
}
// ==== STATES ====
/* ===================================================================== *
 * C 2973: REPEATTYPE - common code for all repeated character type
 * matches. The four inner labels ENDLOOP99 (C 4511), GOT_MAX (C 4567)
 * and ENDLOOP00..ENDLOOP03 (C 5054..5117) are each jumped to only from
 * inside the single loop that immediately precedes them, so they are
 * translated as Rust labeled blocks with `break 'label`, not as states.
 * ===================================================================== */
L_REPEATTYPE => {
    /* Lctype = *Fecode++;  Code for the character type */
    (*F).fields.type_repeat.ctype = *Fecode!() as u32;
    Fecode!() = Fecode!().add(1);

    if (*F).fields.type_repeat.ctype == OP_PROP || (*F).fields.type_repeat.ctype == OP_NOTPROP {
        proptype = *Fecode!() as i32;
        Fecode!() = Fecode!().add(1);
        (*F).fields.type_repeat.propvalue = *Fecode!() as u32;
        Fecode!() = Fecode!().add(1);
    } else {
        proptype = -1;
    }

    /* First, ensure the minimum number of matches are present. Use inline
    code for maximizing the speed, and do the type test once at the start
    (i.e. keep it out of the loops). As there are no calls to RMATCH in the
    loops, we can use an ordinary variable for "notmatch". The code for UTF
    mode is separated out for tidiness, except for Unicode property tests. */

    if (*F).fields.type_repeat.min > 0 {
        if proptype >= 0
        /* Property tests in all modes */
        {
            let notmatch: BOOL = ((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL;
            match proptype as u32 {
                PT_LAMP => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let chartype: i32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        chartype = UCD_CHARTYPE!(fc) as i32;
                        if ((chartype == ucp_Lu as i32
                            || chartype == ucp_Ll as i32
                            || chartype == ucp_Lt as i32) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_GC => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        if ((UCD_CATEGORY!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_PC => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        if ((UCD_CHARTYPE!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_SC => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        if ((UCD_SCRIPT!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_SCX => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let ok: BOOL;
                        let prop: *const ucd_record;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        prop = GET_UCD!(fc);
                        ok = (((*prop).script as u32 == (*F).fields.type_repeat.propvalue)
                            || MAPBIT!(
                                crate::ucd::_pcre2_ucd_script_sets_8
                                    .as_ptr()
                                    .add(UCD_SCRIPTX_PROP!(prop) as usize),
                                (*F).fields.type_repeat.propvalue
                            ) != 0) as BOOL;
                        if ok == notmatch {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_ALNUM => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let category: i32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        category = UCD_CATEGORY!(fc) as i32;
                        if ((category == ucp_L as i32 || category == ucp_N as i32) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                which means that Perl space and POSIX space are now identical. PCRE
                was changed at release 8.34. */
                PT_SPACE | PT_PXSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        match fc {
                            B_HSPACE_VSPACE_CASES!() => {
                                if notmatch != 0 {
                                    rrc = MATCH_NOMATCH;
                                    state = S_RETURN_SWITCH;
                                    continue 'sm;
                                }
                            }
                            _ => {
                                if ((UCD_CATEGORY!(fc) == ucp_Z) as BOOL) == notmatch {
                                    rrc = MATCH_NOMATCH;
                                    state = S_RETURN_SWITCH;
                                    continue 'sm;
                                }
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_WORD => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let chartype: i32;
                        let category: i32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        chartype = UCD_CHARTYPE!(fc) as i32;
                        category = crate::tables::_pcre2_ucp_gentype_8[chartype as usize] as i32;
                        if ((category == ucp_L as i32
                            || category == ucp_N as i32
                            || chartype == ucp_Mn as i32
                            || chartype == ucp_Pc as i32) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_CLIST => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let mut cp: *const u32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        cp = crate::ucd::_pcre2_ucd_caseless_sets_8
                            .as_ptr()
                            .add((*F).fields.type_repeat.propvalue as usize);
                        loop {
                            if fc < *cp {
                                if notmatch != 0 {
                                    break;
                                }
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                            let t = *cp;
                            cp = cp.add(1);
                            if fc == t {
                                if notmatch != 0 {
                                    rrc = MATCH_NOMATCH;
                                    state = S_RETURN_SWITCH;
                                    continue 'sm;
                                }
                                break;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_UCNC => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        if ((fc == 0x24u32 /* CHAR_DOLLAR_SIGN */
                            || fc == 0x40u32 /* CHAR_COMMERCIAL_AT */
                            || fc == 0x60u32 /* CHAR_GRAVE_ACCENT */
                            || (fc >= 0xa0 && fc <= 0xd7ff)
                            || fc >= 0xe000) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_BIDICL => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        if ((UCD_BIDICLASS!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_BOOL => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let ok: BOOL;
                        let prop: *const ucd_record;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        prop = GET_UCD!(fc);
                        ok = (MAPBIT!(
                            crate::ucd::_pcre2_ucd_boolprop_sets_8
                                .as_ptr()
                                .add(UCD_BPROPS_PROP!(prop) as usize),
                            (*F).fields.type_repeat.propvalue
                        ) != 0) as BOOL;
                        if ok == notmatch {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* This should not occur */
                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }
        }
        /* Match extended Unicode sequences. We will get here only if the
        support is in the binary; otherwise a compile-time error occurs. */
        else if (*F).fields.type_repeat.ctype == OP_EXTUNI {
            i = 1;
            while i <= (*F).fields.type_repeat.min {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                } else {
                    GETCHARINCTEST!(fc, Feptr!(), utf);
                    Feptr!() = crate::extuni::_pcre2_extuni_8(
                        fc,
                        Feptr!(),
                        (*mb).start_subject,
                        (*mb).end_subject,
                        utf,
                        null_mut(),
                    );
                }
                CHECK_PARTIAL!();
                i = i.wrapping_add(1);
            }
        }
        /* Handle all other cases in UTF mode */
        else if utf != 0 {
            match (*F).fields.type_repeat.ctype {
                OP_ANY => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if IS_NEWLINE!(Feptr!()) != 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
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
                        Feptr!() = Feptr!().add(1);
                        ACROSSCHAR!(
                            Feptr!() < (*mb).end_subject,
                            Feptr!(),
                            Feptr!() = Feptr!().add(1)
                        );
                        i = i.wrapping_add(1);
                    }
                }

                OP_ALLANY => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        ACROSSCHAR!(
                            Feptr!() < (*mb).end_subject,
                            Feptr!(),
                            Feptr!() = Feptr!().add(1)
                        );
                        i = i.wrapping_add(1);
                    }
                }

                OP_ANYBYTE => {
                    if Feptr!()
                        > (*mb)
                            .end_subject
                            .wrapping_sub((*F).fields.type_repeat.min as usize)
                    {
                        rrc = MATCH_NOMATCH;
                        state = S_RETURN_SWITCH;
                        continue 'sm;
                    }
                    Feptr!() = Feptr!().add((*F).fields.type_repeat.min as usize);
                }

                OP_ANYNL => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINC!(fc, Feptr!());
                        match fc {
                            0x0du32 /* CHAR_CR */ => {
                                if Feptr!() < (*mb).end_subject && *Feptr!() == 0x0au8 {
                                    Feptr!() = Feptr!().add(1);
                                }
                            }
                            0x0au32 /* CHAR_LF */ => {}
                            0x0bu32 | 0x0cu32 | 0x85u32 | 0x2028u32 | 0x2029u32 => {
                                if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                                    rrc = MATCH_NOMATCH;
                                    state = S_RETURN_SWITCH;
                                    continue 'sm;
                                }
                            }
                            _ => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_HSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINC!(fc, Feptr!());
                        match fc {
                            B_HSPACE_CASES!() => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                            _ => {}
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_HSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINC!(fc, Feptr!());
                        match fc {
                            B_HSPACE_CASES!() => {}
                            _ => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_VSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINC!(fc, Feptr!());
                        match fc {
                            B_VSPACE_CASES!() => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                            _ => {}
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_VSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINC!(fc, Feptr!());
                        match fc {
                            B_VSPACE_CASES!() => {}
                            _ => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_DIGIT => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINC!(fc, Feptr!());
                        if fc < 128 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_DIGIT => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        cc = *Feptr!() as u32;
                        if cc >= 128 || (*(*mb).ctypes.add(cc as usize) & ctype_digit) == 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        /* No need to skip more code units - we know it has only one. */
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WHITESPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        cc = *Feptr!() as u32;
                        if cc < 128 && (*(*mb).ctypes.add(cc as usize) & ctype_space) != 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        ACROSSCHAR!(
                            Feptr!() < (*mb).end_subject,
                            Feptr!(),
                            Feptr!() = Feptr!().add(1)
                        );
                        i = i.wrapping_add(1);
                    }
                }

                OP_WHITESPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        cc = *Feptr!() as u32;
                        if cc >= 128 || (*(*mb).ctypes.add(cc as usize) & ctype_space) == 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        /* No need to skip more code units - we know it has only one. */
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WORDCHAR => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        cc = *Feptr!() as u32;
                        if cc < 128 && (*(*mb).ctypes.add(cc as usize) & ctype_word) != 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        ACROSSCHAR!(
                            Feptr!() < (*mb).end_subject,
                            Feptr!(),
                            Feptr!() = Feptr!().add(1)
                        );
                        i = i.wrapping_add(1);
                    }
                }

                OP_WORDCHAR => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        cc = *Feptr!() as u32;
                        if cc >= 128 || (*(*mb).ctypes.add(cc as usize) & ctype_word) == 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        /* No need to skip more code units - we know it has only one. */
                        i = i.wrapping_add(1);
                    }
                }

                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            } /* End switch(Lctype) */
        }
        /* Code for the non-UTF case for minimum matching of operators other
        than OP_PROP and OP_NOTPROP. */
        else {
            match (*F).fields.type_repeat.ctype {
                OP_ANY => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if IS_NEWLINE!(Feptr!()) != 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
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
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_ALLANY => {
                    if Feptr!()
                        > (*mb)
                            .end_subject
                            .wrapping_sub((*F).fields.type_repeat.min as usize)
                    {
                        SCHECK_PARTIAL!();
                        rrc = MATCH_NOMATCH;
                        state = S_RETURN_SWITCH;
                        continue 'sm;
                    }
                    Feptr!() = Feptr!().add((*F).fields.type_repeat.min as usize);
                }

                /* The OP_ANYBYTE case is cut out in C because \C gets turned into
                OP_ALLANY in non-UTF mode. */
                OP_ANYNL => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        let cc = *Feptr!();
                        Feptr!() = Feptr!().add(1);
                        match cc as u32 {
                            0x0du32 /* CHAR_CR */ => {
                                if Feptr!() < (*mb).end_subject && *Feptr!() == 0x0au8 {
                                    Feptr!() = Feptr!().add(1);
                                }
                            }
                            0x0au32 /* CHAR_LF */ => {}
                            0x0bu32 | 0x0cu32 | 0x85u32 => {
                                if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                                    rrc = MATCH_NOMATCH;
                                    state = S_RETURN_SWITCH;
                                    continue 'sm;
                                }
                            }
                            _ => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_HSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        let cc = *Feptr!();
                        Feptr!() = Feptr!().add(1);
                        match cc as u32 {
                            B_HSPACE_BYTE_CASES!() => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                            _ => {}
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_HSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        let cc = *Feptr!();
                        Feptr!() = Feptr!().add(1);
                        match cc as u32 {
                            B_HSPACE_BYTE_CASES!() => {}
                            _ => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_VSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        let cc = *Feptr!();
                        Feptr!() = Feptr!().add(1);
                        match cc as u32 {
                            B_VSPACE_BYTE_CASES!() => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                            _ => {}
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_VSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        let cc = *Feptr!();
                        Feptr!() = Feptr!().add(1);
                        match cc as u32 {
                            B_VSPACE_BYTE_CASES!() => {}
                            _ => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_DIGIT => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if MAX_255!(*Feptr!()) != 0
                            && (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_digit) != 0
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_DIGIT => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if MAX_255!(*Feptr!()) == 0
                            || (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_digit) == 0
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WHITESPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if MAX_255!(*Feptr!()) != 0
                            && (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_space) != 0
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_WHITESPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if MAX_255!(*Feptr!()) == 0
                            || (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_space) == 0
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WORDCHAR => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if MAX_255!(*Feptr!()) != 0
                            && (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_word) != 0
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_WORDCHAR => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if MAX_255!(*Feptr!()) == 0
                            || (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_word) == 0
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }
        }
    }

    /* If Lmin = Lmax we are done. Continue with the main loop. */

    if (*F).fields.type_repeat.min == (*F).fields.type_repeat.max {
        state = S_MAINLOOP; /* continue */
        continue 'sm;
    }

    /* If minimizing, we have to test the rest of the pattern before each
    subsequent match. This means we cannot use a local "notmatch" variable as
    in the other cases. As all 4 temporary 32-bit values in the frame are
    already in use, just test the type each time. */

    if reptype == REPTYPE_MIN {
        if proptype >= 0 {
            match proptype as u32 {
                PT_LAMP => {
                    state = L_B_TR_MIN_LAMP;
                    continue 'sm;
                }
                PT_GC => {
                    state = L_B_TR_MIN_GC;
                    continue 'sm;
                }
                PT_PC => {
                    state = L_B_TR_MIN_PC;
                    continue 'sm;
                }
                PT_SC => {
                    state = L_B_TR_MIN_SC;
                    continue 'sm;
                }
                PT_SCX => {
                    state = L_B_TR_MIN_SCX;
                    continue 'sm;
                }
                PT_ALNUM => {
                    state = L_B_TR_MIN_ALNUM;
                    continue 'sm;
                }
                PT_SPACE | PT_PXSPACE => {
                    state = L_B_TR_MIN_SPACE;
                    continue 'sm;
                }
                PT_WORD => {
                    state = L_B_TR_MIN_WORD;
                    continue 'sm;
                }
                PT_CLIST => {
                    state = L_B_TR_MIN_CLIST;
                    continue 'sm;
                }
                PT_UCNC => {
                    state = L_B_TR_MIN_UCNC;
                    continue 'sm;
                }
                PT_BIDICL => {
                    state = L_B_TR_MIN_BIDICL;
                    continue 'sm;
                }
                PT_BOOL => {
                    state = L_B_TR_MIN_BOOL;
                    continue 'sm;
                }
                /* This should never occur */
                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }
        }
        /* Match extended Unicode sequences. */
        else if (*F).fields.type_repeat.ctype == OP_EXTUNI {
            state = L_B_TR_MIN_EXTUNI;
            continue 'sm;
        }
        /* UTF mode for non-property testing character types. */
        else if utf != 0 {
            state = L_B_TR_MIN_UTF;
            continue 'sm;
        }
        /* Not UTF mode */
        else {
            state = L_B_TR_MIN_NOUTF;
            continue 'sm;
        }
    }
    /* If maximizing, it is worth using inline code for speed, doing the type
    test once at the start (i.e. keep it out of the loops). Once again,
    "notmatch" can be an ordinary local variable because the loops do not call
    RMATCH. */
    else {
        (*F).fields.type_repeat.start_eptr = Feptr!(); /* Remember where we started */

        if proptype >= 0 {
            let notmatch: BOOL = ((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL;
            match proptype as u32 {
                PT_LAMP => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let chartype: i32;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        chartype = UCD_CHARTYPE!(fc) as i32;
                        if ((chartype == ucp_Lu as i32
                            || chartype == ucp_Ll as i32
                            || chartype == ucp_Lt as i32) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_GC => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        if ((UCD_CATEGORY!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_PC => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        if ((UCD_CHARTYPE!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_SC => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        if ((UCD_SCRIPT!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_SCX => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let ok: BOOL;
                        let prop: *const ucd_record;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        prop = GET_UCD!(fc);
                        ok = (((*prop).script as u32 == (*F).fields.type_repeat.propvalue)
                            || MAPBIT!(
                                crate::ucd::_pcre2_ucd_script_sets_8
                                    .as_ptr()
                                    .add(UCD_SCRIPTX_PROP!(prop) as usize),
                                (*F).fields.type_repeat.propvalue
                            ) != 0) as BOOL;
                        if ok == notmatch {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_ALNUM => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let category: i32;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        category = UCD_CATEGORY!(fc) as i32;
                        if ((category == ucp_L as i32 || category == ucp_N as i32) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                which means that Perl space and POSIX space are now identical. PCRE
                was changed at release 8.34. */
                PT_SPACE | PT_PXSPACE => {
                    /* The C label ENDLOOP99 (C 4511) sits just after this loop; it
                    is reached only by the two gotos inside it. */
                    'endloop99: {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: i32 = 1;
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, Feptr!(), len, utf);
                            match fc {
                                B_HSPACE_VSPACE_CASES!() => {
                                    if notmatch != 0 {
                                        break 'endloop99; /* goto ENDLOOP99 - break the loop */
                                    }
                                }
                                _ => {
                                    if ((UCD_CATEGORY!(fc) == ucp_Z) as BOOL) == notmatch {
                                        break 'endloop99; /* goto ENDLOOP99 - break the loop */
                                    }
                                }
                            }
                            Feptr!() = Feptr!().add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }
                }

                PT_WORD => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let chartype: i32;
                        let category: i32;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        chartype = UCD_CHARTYPE!(fc) as i32;
                        category = crate::tables::_pcre2_ucp_gentype_8[chartype as usize] as i32;
                        if ((category == ucp_L as i32
                            || category == ucp_N as i32
                            || chartype == ucp_Mn as i32
                            || chartype == ucp_Pc as i32) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_CLIST => {
                    /* The C label GOT_MAX (C 4567) sits just after this loop; it is
                    reached only by the two gotos inside it. */
                    'got_max: {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut cp: *const u32;
                            let mut len: i32 = 1;
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, Feptr!(), len, utf);
                            {
                                cp = crate::ucd::_pcre2_ucd_caseless_sets_8
                                    .as_ptr()
                                    .add((*F).fields.type_repeat.propvalue as usize);
                                loop {
                                    if fc < *cp {
                                        if notmatch != 0 {
                                            break;
                                        } else {
                                            break 'got_max; /* goto GOT_MAX */
                                        }
                                    }
                                    let t = *cp;
                                    cp = cp.add(1);
                                    if fc == t {
                                        if notmatch != 0 {
                                            break 'got_max; /* goto GOT_MAX */
                                        } else {
                                            break;
                                        }
                                    }
                                }
                            }

                            Feptr!() = Feptr!().add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }
                }

                PT_UCNC => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        if ((fc == 0x24u32 /* CHAR_DOLLAR_SIGN */
                            || fc == 0x40u32 /* CHAR_COMMERCIAL_AT */
                            || fc == 0x60u32 /* CHAR_GRAVE_ACCENT */
                            || (fc >= 0xa0 && fc <= 0xd7ff)
                            || fc >= 0xe000) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_BIDICL => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        if ((UCD_BIDICLASS!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_BOOL => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let ok: BOOL;
                        let prop: *const ucd_record;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        prop = GET_UCD!(fc);
                        ok = (MAPBIT!(
                            crate::ucd::_pcre2_ucd_boolprop_sets_8
                                .as_ptr()
                                .add(UCD_BPROPS_PROP!(prop) as usize),
                            (*F).fields.type_repeat.propvalue
                        ) != 0) as BOOL;
                        if ok == notmatch {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }

            /* Feptr is now past the end of the maximum run */

            if reptype == REPTYPE_POS {
                state = S_MAINLOOP; /* continue - No backtracking */
                continue 'sm;
            }

            /* After \C in UTF mode, Lstart_eptr might be in the middle of a
            Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
            go too far. */

            state = L_B_TR_MAX_PROP_BT;
            continue 'sm;
        }
        /* Match extended Unicode grapheme clusters. We will get here only if the
        support is in the binary; otherwise a compile-time error occurs. */
        else if (*F).fields.type_repeat.ctype == OP_EXTUNI {
            i = (*F).fields.type_repeat.min;
            while i < (*F).fields.type_repeat.max {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    break;
                } else {
                    GETCHARINCTEST!(fc, Feptr!(), utf);
                    Feptr!() = crate::extuni::_pcre2_extuni_8(
                        fc,
                        Feptr!(),
                        (*mb).start_subject,
                        (*mb).end_subject,
                        utf,
                        null_mut(),
                    );
                }
                CHECK_PARTIAL!();
                i = i.wrapping_add(1);
            }

            /* Feptr is now past the end of the maximum run */

            if reptype == REPTYPE_POS {
                state = S_MAINLOOP; /* continue - No backtracking */
                continue 'sm;
            }

            state = L_B_TR_MAX_EXTUNI_BT;
            continue 'sm;
        } else if utf != 0 {
            match (*F).fields.type_repeat.ctype {
                OP_ANY => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if IS_NEWLINE!(Feptr!()) != 0 {
                            break;
                        }
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
                        Feptr!() = Feptr!().add(1);
                        ACROSSCHAR!(
                            Feptr!() < (*mb).end_subject,
                            Feptr!(),
                            Feptr!() = Feptr!().add(1)
                        );
                        i = i.wrapping_add(1);
                    }
                }

                OP_ALLANY => {
                    if (*F).fields.type_repeat.max < u32::MAX {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            Feptr!() = Feptr!().add(1);
                            ACROSSCHAR!(
                                Feptr!() < (*mb).end_subject,
                                Feptr!(),
                                Feptr!() = Feptr!().add(1)
                            );
                            i = i.wrapping_add(1);
                        }
                    } else {
                        Feptr!() = (*mb).end_subject; /* Unlimited UTF-8 repeat */
                        SCHECK_PARTIAL!();
                    }
                }

                /* The "byte" (i.e. "code unit") case is the same as non-UTF */
                OP_ANYBYTE => {
                    fc = (*F)
                        .fields
                        .type_repeat
                        .max
                        .wrapping_sub((*F).fields.type_repeat.min);
                    if fc
                        > (((*mb).end_subject as usize).wrapping_sub(Feptr!() as usize)) as u32
                    {
                        Feptr!() = (*mb).end_subject;
                        SCHECK_PARTIAL!();
                    } else {
                        Feptr!() = Feptr!().add(fc as usize);
                    }
                }

                OP_ANYNL => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc == 0x0du32
                        /* CHAR_CR */
                        {
                            Feptr!() = Feptr!().add(1);
                            if Feptr!() >= (*mb).end_subject {
                                break;
                            }
                            if *Feptr!() == 0x0au8 {
                                Feptr!() = Feptr!().add(1);
                            }
                        } else {
                            if fc != 0x0au32
                                && ((*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF
                                    || (fc != 0x0bu32
                                        && fc != 0x0cu32
                                        && fc != 0x85u32
                                        && fc != 0x2028u32
                                        && fc != 0x2029u32))
                            {
                                break;
                            }
                            Feptr!() = Feptr!().add(len as usize);
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_HSPACE | OP_HSPACE => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let gotspace: BOOL;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        match fc {
                            B_HSPACE_CASES!() => {
                                gotspace = TRUE;
                            }
                            _ => {
                                gotspace = FALSE;
                            }
                        }
                        if gotspace
                            == (((*F).fields.type_repeat.ctype == OP_NOT_HSPACE) as BOOL)
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_VSPACE | OP_VSPACE => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let gotspace: BOOL;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        match fc {
                            B_VSPACE_CASES!() => {
                                gotspace = TRUE;
                            }
                            _ => {
                                gotspace = FALSE;
                            }
                        }
                        if gotspace
                            == (((*F).fields.type_repeat.ctype == OP_NOT_VSPACE) as BOOL)
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_DIGIT => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_DIGIT => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WHITESPACE => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_WHITESPACE => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WORDCHAR => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_WORDCHAR => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }

            if reptype == REPTYPE_POS {
                state = S_MAINLOOP; /* continue - No backtracking */
                continue 'sm;
            }

            /* After \C in UTF mode, Lstart_eptr might be in the middle of a
            Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't go
            too far. */

            state = L_B_TR_MAX_UTF_BT;
            continue 'sm;
        }
        /* Not UTF mode */
        else {
            match (*F).fields.type_repeat.ctype {
                OP_ANY => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if IS_NEWLINE!(Feptr!()) != 0 {
                            break;
                        }
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
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_ALLANY | OP_ANYBYTE => {
                    fc = (*F)
                        .fields
                        .type_repeat
                        .max
                        .wrapping_sub((*F).fields.type_repeat.min);
                    if fc
                        > (((*mb).end_subject as usize).wrapping_sub(Feptr!() as usize)) as u32
                    {
                        Feptr!() = (*mb).end_subject;
                        SCHECK_PARTIAL!();
                    } else {
                        Feptr!() = Feptr!().add(fc as usize);
                    }
                }

                OP_ANYNL => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        fc = *Feptr!() as u32;
                        if fc == 0x0du32
                        /* CHAR_CR */
                        {
                            Feptr!() = Feptr!().add(1);
                            if Feptr!() >= (*mb).end_subject {
                                break;
                            }
                            if *Feptr!() == 0x0au8 {
                                Feptr!() = Feptr!().add(1);
                            }
                        } else {
                            if fc != 0x0au32
                                && ((*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF
                                    || (fc != 0x0bu32 && fc != 0x0cu32 && fc != 0x85u32))
                            {
                                break;
                            }
                            Feptr!() = Feptr!().add(1);
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_HSPACE => {
                    /* C label ENDLOOP00 (C 5054) */
                    'endloop00: {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            match *Feptr!() as u32 {
                                B_HSPACE_BYTE_CASES!() => {
                                    break 'endloop00; /* goto ENDLOOP00 */
                                }
                                _ => {
                                    Feptr!() = Feptr!().add(1);
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                    }
                }

                OP_HSPACE => {
                    /* C label ENDLOOP01 (C 5075) */
                    'endloop01: {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            match *Feptr!() as u32 {
                                B_HSPACE_BYTE_CASES!() => {
                                    Feptr!() = Feptr!().add(1);
                                }
                                _ => {
                                    break 'endloop01; /* goto ENDLOOP01 */
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                    }
                }

                OP_NOT_VSPACE => {
                    /* C label ENDLOOP02 (C 5096) */
                    'endloop02: {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            match *Feptr!() as u32 {
                                B_VSPACE_BYTE_CASES!() => {
                                    break 'endloop02; /* goto ENDLOOP02 */
                                }
                                _ => {
                                    Feptr!() = Feptr!().add(1);
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                    }
                }

                OP_VSPACE => {
                    /* C label ENDLOOP03 (C 5117) */
                    'endloop03: {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            match *Feptr!() as u32 {
                                B_VSPACE_BYTE_CASES!() => {
                                    Feptr!() = Feptr!().add(1);
                                }
                                _ => {
                                    break 'endloop03; /* goto ENDLOOP03 */
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                    }
                }

                OP_NOT_DIGIT => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if MAX_255!(*Feptr!()) != 0
                            && (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_digit) != 0
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_DIGIT => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if MAX_255!(*Feptr!()) == 0
                            || (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_digit) == 0
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WHITESPACE => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if MAX_255!(*Feptr!()) != 0
                            && (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_space) != 0
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_WHITESPACE => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if MAX_255!(*Feptr!()) == 0
                            || (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_space) == 0
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WORDCHAR => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if MAX_255!(*Feptr!()) != 0
                            && (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_word) != 0
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_WORDCHAR => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if MAX_255!(*Feptr!()) == 0
                            || (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_word) == 0
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }

            if reptype == REPTYPE_POS {
                state = S_MAINLOOP; /* continue - No backtracking */
                continue 'sm;
            }

            state = L_B_TR_MAX_NOUTF_BT;
            continue 'sm;
        }
    }
}

/* ===================================================================== *
 * REPEATTYPE, minimizing repeats with property tests (C 3776..4053).
 * Each C `for(;;) { RMATCH(Fecode, RMnn); ... }` becomes a loop-entry
 * state holding the RMATCH plus the RMnn state holding the loop body.
 * ===================================================================== */

/* C 3783 PT_LAMP */
L_B_TR_MIN_LAMP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM208 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM208 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let chartype: i32 = UCD_CHARTYPE!(fc) as i32;
        if ((chartype == ucp_Lu as i32
            || chartype == ucp_Ll as i32
            || chartype == ucp_Lt as i32) as BOOL)
            == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
        {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    state = L_B_TR_MIN_LAMP;
    continue 'sm;
}

/* C 3804 PT_GC */
L_B_TR_MIN_GC => {
    start_ecode = Fecode!();
    Freturn_id!() = RM209 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM209 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if ((UCD_CATEGORY!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
        == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
    {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_B_TR_MIN_GC;
    continue 'sm;
}

/* C 3821 PT_PC */
L_B_TR_MIN_PC => {
    start_ecode = Fecode!();
    Freturn_id!() = RM210 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM210 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if ((UCD_CHARTYPE!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
        == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
    {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_B_TR_MIN_PC;
    continue 'sm;
}

/* C 3838 PT_SC */
L_B_TR_MIN_SC => {
    start_ecode = Fecode!();
    Freturn_id!() = RM211 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM211 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if ((UCD_SCRIPT!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
        == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
    {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_B_TR_MIN_SC;
    continue 'sm;
}

/* C 3855 PT_SCX */
L_B_TR_MIN_SCX => {
    start_ecode = Fecode!();
    Freturn_id!() = RM224 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM224 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let prop: *const ucd_record = GET_UCD!(fc);
        let ok: BOOL = (((*prop).script as u32 == (*F).fields.type_repeat.propvalue)
            || MAPBIT!(
                crate::ucd::_pcre2_ucd_script_sets_8
                    .as_ptr()
                    .add(UCD_SCRIPTX_PROP!(prop) as usize),
                (*F).fields.type_repeat.propvalue
            ) != 0) as BOOL;
        if ok == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL) {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    state = L_B_TR_MIN_SCX;
    continue 'sm;
}

/* C 3877 PT_ALNUM */
L_B_TR_MIN_ALNUM => {
    start_ecode = Fecode!();
    Freturn_id!() = RM212 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM212 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let category: i32 = UCD_CATEGORY!(fc) as i32;
        if ((category == ucp_L as i32 || category == ucp_N as i32) as BOOL)
            == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
        {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    state = L_B_TR_MIN_ALNUM;
    continue 'sm;
}

/* C 3900 PT_SPACE / PT_PXSPACE */
L_B_TR_MIN_SPACE => {
    start_ecode = Fecode!();
    Freturn_id!() = RM213 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM213 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    match fc {
        B_HSPACE_VSPACE_CASES!() => {
            if (*F).fields.type_repeat.ctype == OP_NOTPROP {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }
        _ => {
            if ((UCD_CATEGORY!(fc) == ucp_Z) as BOOL)
                == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
            {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }
    }
    state = L_B_TR_MIN_SPACE;
    continue 'sm;
}

/* C 3928 PT_WORD */
L_B_TR_MIN_WORD => {
    start_ecode = Fecode!();
    Freturn_id!() = RM214 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM214 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let chartype: i32 = UCD_CHARTYPE!(fc) as i32;
        let category: i32 = crate::tables::_pcre2_ucp_gentype_8[chartype as usize] as i32;
        if ((category == ucp_L as i32
            || category == ucp_N as i32
            || chartype == ucp_Mn as i32
            || chartype == ucp_Pc as i32) as BOOL)
            == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
        {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    state = L_B_TR_MIN_WORD;
    continue 'sm;
}

/* C 3951 PT_CLIST */
L_B_TR_MIN_CLIST => {
    start_ecode = Fecode!();
    Freturn_id!() = RM215 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM215 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let mut cp: *const u32 = crate::ucd::_pcre2_ucd_caseless_sets_8
            .as_ptr()
            .add((*F).fields.type_repeat.propvalue as usize);
        loop {
            if fc < *cp {
                if (*F).fields.type_repeat.ctype == OP_NOTPROP {
                    break;
                }
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            let t = *cp;
            cp = cp.add(1);
            if fc == t {
                if (*F).fields.type_repeat.ctype == OP_NOTPROP {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                break;
            }
        }
    }
    state = L_B_TR_MIN_CLIST;
    continue 'sm;
}

/* C 3988 PT_UCNC */
L_B_TR_MIN_UCNC => {
    start_ecode = Fecode!();
    Freturn_id!() = RM216 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM216 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if ((fc == 0x24u32 /* CHAR_DOLLAR_SIGN */
        || fc == 0x40u32 /* CHAR_COMMERCIAL_AT */
        || fc == 0x60u32 /* CHAR_GRAVE_ACCENT */
        || (fc >= 0xa0 && fc <= 0xd7ff)
        || fc >= 0xe000) as BOOL)
        == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
    {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_B_TR_MIN_UCNC;
    continue 'sm;
}

/* C 4007 PT_BIDICL */
L_B_TR_MIN_BIDICL => {
    start_ecode = Fecode!();
    Freturn_id!() = RM223 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM223 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if ((UCD_BIDICLASS!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
        == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
    {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_B_TR_MIN_BIDICL;
    continue 'sm;
}

/* C 4024 PT_BOOL */
L_B_TR_MIN_BOOL => {
    start_ecode = Fecode!();
    Freturn_id!() = RM222 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM222 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let prop: *const ucd_record = GET_UCD!(fc);
        let ok: BOOL = (MAPBIT!(
            crate::ucd::_pcre2_ucd_boolprop_sets_8
                .as_ptr()
                .add(UCD_BPROPS_PROP!(prop) as usize),
            (*F).fields.type_repeat.propvalue
        ) != 0) as BOOL;
        if ok == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL) {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    state = L_B_TR_MIN_BOOL;
    continue 'sm;
}

/* C 4059: minimizing repeat of extended Unicode sequences */
L_B_TR_MIN_EXTUNI => {
    start_ecode = Fecode!();
    Freturn_id!() = RM217 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM217 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    } else {
        GETCHARINCTEST!(fc, Feptr!(), utf);
        Feptr!() = crate::extuni::_pcre2_extuni_8(
            fc,
            Feptr!(),
            (*mb).start_subject,
            (*mb).end_subject,
            utf,
            null_mut(),
        );
    }
    CHECK_PARTIAL!();
    state = L_B_TR_MIN_EXTUNI;
    continue 'sm;
}

/* C 4086: UTF mode for non-property testing character types */
L_B_TR_MIN_UTF => {
    start_ecode = Fecode!();
    Freturn_id!() = RM218 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM218 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if (*F).fields.type_repeat.ctype == OP_ANY && IS_NEWLINE!(Feptr!()) != 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINC!(fc, Feptr!());
    match (*F).fields.type_repeat.ctype {
        OP_ANY => {
            /* This is the non-NL case */
            if (*mb).partial != 0 /* Take care with CRLF partial */
                && Feptr!() >= (*mb).end_subject
                && (*mb).nltype == NLTYPE_FIXED
                && (*mb).nllen == 2
                && fc == (*mb).nl[0] as u32
            {
                (*mb).hitend = TRUE;
                if (*mb).partial > 1 {
                    return PCRE2_ERROR_PARTIAL;
                }
            }
        }

        OP_ALLANY | OP_ANYBYTE => {}

        OP_ANYNL => match fc {
            0x0du32 /* CHAR_CR */ => {
                if Feptr!() < (*mb).end_subject && *Feptr!() == 0x0au8 {
                    Feptr!() = Feptr!().add(1);
                }
            }
            0x0au32 /* CHAR_LF */ => {}
            0x0bu32 | 0x0cu32 | 0x85u32 | 0x2028u32 | 0x2029u32 => {
                if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }
            _ => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        },

        OP_NOT_HSPACE => match fc {
            B_HSPACE_CASES!() => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            _ => {}
        },

        OP_HSPACE => match fc {
            B_HSPACE_CASES!() => {}
            _ => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        },

        OP_NOT_VSPACE => match fc {
            B_VSPACE_CASES!() => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            _ => {}
        },

        OP_VSPACE => match fc {
            B_VSPACE_CASES!() => {}
            _ => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        },

        OP_NOT_DIGIT => {
            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_DIGIT => {
            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_NOT_WHITESPACE => {
            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_WHITESPACE => {
            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_NOT_WORDCHAR => {
            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_WORDCHAR => {
            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        _ => {
            return PCRE2_ERROR_INTERNAL;
        }
    }
    state = L_B_TR_MIN_UTF;
    continue 'sm;
}

/* C 4218: not UTF mode */
L_B_TR_MIN_NOUTF => {
    start_ecode = Fecode!();
    Freturn_id!() = RM33 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM33 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if (*F).fields.type_repeat.ctype == OP_ANY && IS_NEWLINE!(Feptr!()) != 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    fc = *Feptr!() as u32;
    Feptr!() = Feptr!().add(1);
    match (*F).fields.type_repeat.ctype {
        OP_ANY => {
            /* This is the non-NL case */
            if (*mb).partial != 0 /* Take care with CRLF partial */
                && Feptr!() >= (*mb).end_subject
                && (*mb).nltype == NLTYPE_FIXED
                && (*mb).nllen == 2
                && fc == (*mb).nl[0] as u32
            {
                (*mb).hitend = TRUE;
                if (*mb).partial > 1 {
                    return PCRE2_ERROR_PARTIAL;
                }
            }
        }

        OP_ALLANY | OP_ANYBYTE => {}

        OP_ANYNL => match fc {
            0x0du32 /* CHAR_CR */ => {
                if Feptr!() < (*mb).end_subject && *Feptr!() == 0x0au8 {
                    Feptr!() = Feptr!().add(1);
                }
            }
            0x0au32 /* CHAR_LF */ => {}
            0x0bu32 | 0x0cu32 | 0x85u32 => {
                if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }
            _ => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        },

        OP_NOT_HSPACE => match fc {
            B_HSPACE_BYTE_CASES!() => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            _ => {}
        },

        OP_HSPACE => match fc {
            B_HSPACE_BYTE_CASES!() => {}
            _ => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        },

        OP_NOT_VSPACE => match fc {
            B_VSPACE_BYTE_CASES!() => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            _ => {}
        },

        OP_VSPACE => match fc {
            B_VSPACE_BYTE_CASES!() => {}
            _ => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        },

        OP_NOT_DIGIT => {
            if MAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_DIGIT => {
            if MAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_NOT_WHITESPACE => {
            if MAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_WHITESPACE => {
            if MAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_NOT_WORDCHAR => {
            if MAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_WORDCHAR => {
            if MAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        _ => {
            return PCRE2_ERROR_INTERNAL;
        }
    }
    state = L_B_TR_MIN_NOUTF;
    continue 'sm;
}

/* ===================================================================== *
 * REPEATTYPE, maximizing repeats: the four backtracking loops. Leaving
 * any of these loops in C falls out of the enclosing block and reaches
 * the `break` at C 5224, i.e. the main loop.
 * ===================================================================== */

/* C 4638, property tests */
L_B_TR_MAX_PROP_BT => {
    if Feptr!() <= (*F).fields.type_repeat.start_eptr {
        state = S_MAINLOOP; /* break out of the for(;;) -> break (C 5224) */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM221 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM221 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().sub(1);
    if utf != 0 {
        BACKCHAR!(Feptr!());
    }
    state = L_B_TR_MAX_PROP_BT;
    continue 'sm;
}

/* C 4678, extended grapheme clusters */
L_B_TR_MAX_EXTUNI_BT => {
    if Feptr!() <= (*F).fields.type_repeat.start_eptr {
        state = S_MAINLOOP; /* At start of char run -> break (C 5224) */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM219 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM219 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    /* Backtracking over an extended grapheme cluster involves inspecting
    the previous two characters (if present) to see if a break is
    permitted between them. */

    {
        let mut lgb: i32;
        let mut rgb: i32;
        let mut fptr: PCRE2_SPTR;

        Feptr!() = Feptr!().sub(1);
        if utf == 0 {
            fc = *Feptr!() as u32;
        } else {
            BACKCHAR!(Feptr!());
            GETCHAR!(fc, Feptr!());
        }
        rgb = UCD_GRAPHBREAK!(fc) as i32;

        loop {
            if Feptr!() <= (*F).fields.type_repeat.start_eptr {
                break; /* At start of char run */
            }
            fptr = Feptr!().sub(1);
            if utf == 0 {
                fc = *fptr as u32;
            } else {
                BACKCHAR!(fptr);
                GETCHAR!(fc, fptr);
            }
            lgb = UCD_GRAPHBREAK!(fc) as i32;
            if (crate::tables::_pcre2_ucp_gbtable_8[lgb as usize] & (1u32 << rgb)) == 0 {
                break;
            }
            Feptr!() = fptr;
            rgb = lgb;
        }
    }
    state = L_B_TR_MAX_EXTUNI_BT;
    continue 'sm;
}

/* C 4957, UTF mode */
L_B_TR_MAX_UTF_BT => {
    if Feptr!() <= (*F).fields.type_repeat.start_eptr {
        state = S_MAINLOOP; /* break -> break (C 5224) */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM220 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM220 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().sub(1);
    BACKCHAR!(Feptr!());
    if (*F).fields.type_repeat.ctype == OP_ANYNL
        && Feptr!() > (*F).fields.type_repeat.start_eptr
        && *Feptr!() == 0x0au8 /* CHAR_NL */
        && *Feptr!().offset(-1) == 0x0du8
    /* CHAR_CR */
    {
        Feptr!() = Feptr!().sub(1);
    }
    state = L_B_TR_MAX_UTF_BT;
    continue 'sm;
}

/* C 5213, not UTF mode */
L_B_TR_MAX_NOUTF_BT => {
    if Feptr!() == (*F).fields.type_repeat.start_eptr {
        state = S_MAINLOOP; /* break -> break (C 5224) */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM34 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM34 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().sub(1);
    if (*F).fields.type_repeat.ctype == OP_ANYNL
        && Feptr!() > (*F).fields.type_repeat.start_eptr
        && *Feptr!() == 0x0au8 /* CHAR_LF */
        && *Feptr!().offset(-1) == 0x0du8
    /* CHAR_CR */
    {
        Feptr!() = Feptr!().sub(1);
    }
    state = L_B_TR_MAX_NOUTF_BT;
    continue 'sm;
}

/* ===================================================================== *
 * C 5278: REF_REPEAT - set up for repetition of a back reference, or
 * handle the non-repeated case.
 * ===================================================================== */
L_REF_REPEAT => {
    match *Fecode!() as u32 {
        OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY | OP_CRMINQUERY
        | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
            /* fc = *Fecode++ - OP_CRSTAR; */
            {
                let t = *Fecode!();
                Fecode!() = Fecode!().add(1);
                fc = (t as u32).wrapping_sub(OP_CRSTAR);
            }
            (*F).fields.ref_repeat.min = rep_min[fc as usize];
            (*F).fields.ref_repeat.max = rep_max[fc as usize];
            reptype = rep_typ[fc as usize];
        }

        OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
            (*F).fields.ref_repeat.min = GET2!(Fecode!(), 1);
            (*F).fields.ref_repeat.max = GET2!(Fecode!(), 1 + IMM2_SIZE);
            reptype = rep_typ[(*Fecode!() as u32).wrapping_sub(OP_CRSTAR) as usize];
            if (*F).fields.ref_repeat.max == 0 {
                (*F).fields.ref_repeat.max = u32::MAX; /* Max 0 => infinity */
            }
            Fecode!() = Fecode!().add(1 + 2 * IMM2_SIZE);
        }

        /* No repeat follows */
        _ => {
            {
                rrc = match_ref(
                    (*F).fields.ref_repeat.offset,
                    (*F).byte1 as BOOL,
                    (*F).byte2 as i32,
                    F,
                    mb,
                    &mut length,
                );
                if rrc != 0 {
                    if rrc > 0 {
                        Feptr!() = (*mb).end_subject; /* Partial match */
                    }
                    CHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }
            Feptr!() = Feptr!().add(length);
            state = S_MAINLOOP; /* continue - With the main loop */
            continue 'sm;
        }
    }

    /* Handle repeated back references. If a set group has length zero, just
    continue with the main loop, because it matches however many times. For an
    unset reference, if the minimum is zero, we can also just continue. We can
    also continue if PCRE2_MATCH_UNSET_BACKREF is set, because this makes unset
    group behave as a zero-length group. For any other unset cases, carrying
    on will result in NOMATCH. */

    if (*F).fields.ref_repeat.offset < Foffset_top!()
        && *Fovector!().add((*F).fields.ref_repeat.offset) != PCRE2_UNSET
    {
        if *Fovector!().add((*F).fields.ref_repeat.offset)
            == *Fovector!().add((*F).fields.ref_repeat.offset + 1)
        {
            state = S_MAINLOOP; /* continue */
            continue 'sm;
        }
    } else
    /* Group is not set */
    {
        if (*F).fields.ref_repeat.min == 0
            || ((*mb).poptions & PCRE2_MATCH_UNSET_BACKREF) != 0
        {
            state = S_MAINLOOP; /* continue */
            continue 'sm;
        }
    }

    /* First, ensure the minimum number of matches are present. */

    i = 1;
    while i <= (*F).fields.ref_repeat.min {
        let mut slength: PCRE2_SIZE = 0;
        rrc = match_ref(
            (*F).fields.ref_repeat.offset,
            (*F).byte1 as BOOL,
            (*F).byte2 as i32,
            F,
            mb,
            &mut slength,
        );
        if rrc != 0 {
            if rrc > 0 {
                Feptr!() = (*mb).end_subject; /* Partial match */
            }
            CHECK_PARTIAL!();
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        Feptr!() = Feptr!().add(slength);
        i = i.wrapping_add(1);
    }

    /* If min = max, we are done. They are not both allowed to be zero. */

    if (*F).fields.ref_repeat.min == (*F).fields.ref_repeat.max {
        state = S_MAINLOOP; /* continue */
        continue 'sm;
    }

    /* If minimizing, keep trying and advancing the pointer. */

    if reptype == REPTYPE_MIN {
        state = L_B_REF_MINLOOP;
        continue 'sm;
    }
    /* If maximizing, find the longest string and work backwards, as long as
    the matched lengths for each iteration are the same. */
    else {
        let mut samelengths: BOOL = TRUE;
        (*F).fields.ref_repeat.start = Feptr!(); /* Starting position */
        (*F).fields.ref_repeat.length = (*Fovector!()
            .add((*F).fields.ref_repeat.offset + 1))
        .wrapping_sub(*Fovector!().add((*F).fields.ref_repeat.offset));

        i = (*F).fields.ref_repeat.min;
        while i < (*F).fields.ref_repeat.max {
            let mut slength: PCRE2_SIZE = 0;
            rrc = match_ref(
                (*F).fields.ref_repeat.offset,
                (*F).byte1 as BOOL,
                (*F).byte2 as i32,
                F,
                mb,
                &mut slength,
            );
            if rrc != 0 {
                /* Can't use CHECK_PARTIAL because we don't want to update Feptr in
                the soft partial matching case. */

                if rrc > 0 && (*mb).partial != 0 && (*mb).end_subject > (*mb).start_used_ptr {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                break;
            }

            if slength != (*F).fields.ref_repeat.length {
                samelengths = FALSE;
            }
            Feptr!() = Feptr!().add(slength);
            i = i.wrapping_add(1);
        }

        /* No recursion if the repeat type is possessive. */
        if reptype == REPTYPE_POS {
            state = S_MAINLOOP; /* break */
            continue 'sm;
        }

        /* If the length matched for each repetition is the same as the length of
        the captured group, we can easily work backwards. This is the normal
        case. However, in caseless UTF-8 mode there are pairs of case-equivalent
        characters whose lengths (in terms of code units) differ. However, this
        is very rare, so we handle it by re-matching fewer and fewer times. */

        if samelengths != 0 {
            state = L_B_REF_SAMELEN;
            continue 'sm;
        }
        /* The rare case of non-matching lengths. Re-scan the repetition for each
        iteration. We know that match_ref() will succeed every time. */
        else {
            (*F).fields.ref_repeat.max = i;
            state = L_B_REF_DIFFLEN;
            continue 'sm;
        }
    }
}

/* C 5360: minimizing repeat of a back reference */
L_B_REF_MINLOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM20 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM20 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.ref_repeat.min;
        (*F).fields.ref_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.ref_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    {
        let mut slength: PCRE2_SIZE = 0;
        rrc = match_ref(
            (*F).fields.ref_repeat.offset,
            (*F).byte1 as BOOL,
            (*F).byte2 as i32,
            F,
            mb,
            &mut slength,
        );
        if rrc != 0 {
            if rrc > 0 {
                Feptr!() = (*mb).end_subject; /* Partial match */
            }
            CHECK_PARTIAL!();
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        Feptr!() = Feptr!().add(slength);
    }
    state = L_B_REF_MINLOOP;
    continue 'sm;
}

/* C 5421: maximizing, all repetitions the same length */
L_B_REF_SAMELEN => {
    if !(Feptr!() >= (*F).fields.ref_repeat.start) {
        /* End of the while loop: fall through to RRETURN(MATCH_NOMATCH) at
        C 5451. */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM21 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM21 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub((*F).fields.ref_repeat.length);
    state = L_B_REF_SAMELEN;
    continue 'sm;
}

/* C 5435: maximizing, the rare case of differing lengths */
L_B_REF_DIFFLEN => {
    start_ecode = Fecode!();
    Freturn_id!() = RM22 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM22 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() == (*F).fields.ref_repeat.start {
        /* Failed after minimal repetition: break out of the for(;;) and fall
        through to RRETURN(MATCH_NOMATCH) at C 5451. */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = (*F).fields.ref_repeat.start;
    (*F).fields.ref_repeat.max = (*F).fields.ref_repeat.max.wrapping_sub(1);
    i = (*F).fields.ref_repeat.min;
    while i < (*F).fields.ref_repeat.max {
        let mut slength: PCRE2_SIZE = 0;
        let _ = match_ref(
            (*F).fields.ref_repeat.offset,
            (*F).byte1 as BOOL,
            (*F).byte2 as i32,
            F,
            mb,
            &mut slength,
        );
        Feptr!() = Feptr!().add(slength);
        i = i.wrapping_add(1);
    }
    state = L_B_REF_DIFFLEN;
    continue 'sm;
}
