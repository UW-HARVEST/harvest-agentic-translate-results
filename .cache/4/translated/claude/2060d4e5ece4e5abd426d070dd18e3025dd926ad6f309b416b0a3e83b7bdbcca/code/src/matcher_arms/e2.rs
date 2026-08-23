{
/* ===================================================================== */
/* Chunk e2: C lines 3776-4368 of pcre2_match.c -- the REPTYPE_MIN half of
the common "repeated character type" code (label REPEATTYPE).

The L-macros in force here are:

  #define Lstart_eptr  F->fields.type_repeat.start_eptr
  #define Lmin         F->fields.type_repeat.min
  #define Lmax         F->fields.type_repeat.max
  #define Lctype       F->fields.type_repeat.ctype
  #define Lpropvalue   F->fields.type_repeat.propvalue

Each C loop here has the shape

  for (;;)
    {
    RMATCH(Fecode, RMnnn);
    if (rrc != MATCH_NOMATCH) RRETURN(rrc);
    if (Lmin++ >= Lmax) RRETURN(MATCH_NOMATCH);
    ... consume one character ...
    }

so it is flattened into: this block performs the very first RMATCH, and the
matching `if lbl == LBL_RM_BASE + RMnnn` block does the tests, consumes a
character and re-issues the same RMATCH (i.e. loops back). */

if lbl == LBL_REPEATTYPE_2 {
    /* C 3776: If minimizing, we have to test the rest of the pattern before
    each subsequent match. This means we cannot use a local "notmatch" variable
    as in the other cases. As all 4 temporary 32-bit values in the frame are
    already in use, just test the type each time. */

    if reptype == REPTYPE_MIN {
        if proptype >= 0 {
            match proptype as u32 {
                /* C 3783: case PT_LAMP */
                PT_LAMP => {
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM208;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }

                /* C 3803: case PT_GC */
                PT_GC => {
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM209;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }

                /* C 3820: case PT_PC */
                PT_PC => {
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM210;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }

                /* C 3837: case PT_SC */
                PT_SC => {
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM211;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }

                /* C 3854: case PT_SCX */
                PT_SCX => {
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM224;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }

                /* C 3877: case PT_ALNUM */
                PT_ALNUM => {
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM212;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }

                /* Perl space used to exclude VT, but from Perl 5.18 it is
                included, which means that Perl space and POSIX space are now
                identical. PCRE was changed at release 8.34. */

                /* C 3900: case PT_SPACE / case PT_PXSPACE */
                PT_SPACE | PT_PXSPACE => {
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM213;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }

                /* C 3928: case PT_WORD */
                PT_WORD => {
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM214;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }

                /* C 3951: case PT_CLIST */
                PT_CLIST => {
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM215;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }

                /* C 3987: case PT_UCNC */
                PT_UCNC => {
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM216;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }

                /* C 4006: case PT_BIDICL */
                PT_BIDICL => {
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM223;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }

                /* C 4023: case PT_BOOL */
                PT_BOOL => {
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM222;
                    lbl = LBL_MATCH_RECURSE;
                    continue 'sw;
                }

                /* This should never occur */

                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }
        }
        /* Match extended Unicode sequences. We will get here only if the
        support is in the binary; otherwise a compile-time error occurs. */
        else if (*F).fields.type_repeat.ctype == OP_EXTUNI {
            /* C 4061: for (;;) { RMATCH(Fecode, RM217); ... } */
            start_ecode = (*F).ecode;
            (*F).return_id = RM217;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }
        /* UTF mode for non-property testing character types. */
        else if utf != FALSE {
            /* C 4088: for (;;) { RMATCH(Fecode, RM218); ... } */
            start_ecode = (*F).ecode;
            (*F).return_id = RM218;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }
        /* Not UTF mode */
        else {
            /* C 4218: for (;;) { RMATCH(Fecode, RM33); ... } */
            start_ecode = (*F).ecode;
            (*F).return_id = RM33;
            lbl = LBL_MATCH_RECURSE;
            continue 'sw;
        }

        /* C 4361: PCRE2_DEBUG_UNREACHABLE() -- control should never reach
        here; if it did, the C would fall through to the `break` at the end of
        the repeated-character-type processing. */
        #[allow(unreachable_code)]
        {
            lbl = LBL_TOP_OF_LOOP;
            continue 'sw;
        }
    }

    /* C 4369: the `else` (maximizing/possessive) branch lives in chunk e3. */
    lbl = LBL_REPEATTYPE_3;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM208) at C 3787: minimizing PT_LAMP. */

if lbl == LBL_RM_BASE + RM208 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    let chartype: u32 = UCD_CHARTYPE(fc);
    if (chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt)
        == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
    {
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    /* Loop back to RMATCH(Fecode, RM208) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM208;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM209) at C 3807: minimizing PT_GC. */

if lbl == LBL_RM_BASE + RM209 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    if (UCD_CATEGORY(fc) == (*F).fields.type_repeat.propvalue)
        == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
    {
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    /* Loop back to RMATCH(Fecode, RM209) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM209;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM210) at C 3824: minimizing PT_PC. */

if lbl == LBL_RM_BASE + RM210 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    if (UCD_CHARTYPE(fc) == (*F).fields.type_repeat.propvalue)
        == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
    {
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    /* Loop back to RMATCH(Fecode, RM210) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM210;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM211) at C 3841: minimizing PT_SC. */

if lbl == LBL_RM_BASE + RM211 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    if (UCD_SCRIPT(fc) == (*F).fields.type_repeat.propvalue)
        == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
    {
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    /* Loop back to RMATCH(Fecode, RM211) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM211;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM224) at C 3860: minimizing PT_SCX. */

if lbl == LBL_RM_BASE + RM224 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    let prop: &'static ucd_record = GET_UCD(fc);
    let ok: BOOL = ((prop.script as u32 == (*F).fields.type_repeat.propvalue)
        || crate::internal::script_set_bit(
            UCD_SCRIPTX_PROP(prop) as usize,
            (*F).fields.type_repeat.propvalue,
        )) as BOOL;
    if ok == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL) {
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    /* Loop back to RMATCH(Fecode, RM224) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM224;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM212) at C 3881: minimizing PT_ALNUM. */

if lbl == LBL_RM_BASE + RM212 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    let category: u32 = UCD_CATEGORY(fc);
    if (category == ucp_L || category == ucp_N)
        == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
    {
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    /* Loop back to RMATCH(Fecode, RM212) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM212;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM213) at C 3904: minimizing PT_SPACE and
PT_PXSPACE. */

if lbl == LBL_RM_BASE + RM213 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    match fc {
        /* HSPACE_CASES: */
        0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002 | 0x2003 | 0x2004
        | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f | 0x205f | 0x3000
        /* VSPACE_CASES: */
        | 0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => {
            if (*F).fields.type_repeat.ctype == OP_NOTPROP {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }

        _ => {
            if (UCD_CATEGORY(fc) == ucp_Z) == ((*F).fields.type_repeat.ctype == OP_NOTPROP) {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }
    }
    /* Loop back to RMATCH(Fecode, RM213) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM213;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM214) at C 3932: minimizing PT_WORD. */

if lbl == LBL_RM_BASE + RM214 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    let chartype: u32 = UCD_CHARTYPE(fc);
    let category: u32 = _pcre2_ucp_gentype_8[chartype as usize];
    if (category == ucp_L || category == ucp_N || chartype == ucp_Mn || chartype == ucp_Pc)
        == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
    {
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    /* Loop back to RMATCH(Fecode, RM214) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM214;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM215) at C 3955: minimizing PT_CLIST. */

if lbl == LBL_RM_BASE + RM215 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    let mut cp: *const u32 = _pcre2_ucd_caseless_sets_8
        .as_ptr()
        .add((*F).fields.type_repeat.propvalue as usize);
    loop {
        if fc < *cp {
            if (*F).fields.type_repeat.ctype == OP_NOTPROP {
                break;
            }
            rrc = MATCH_NOMATCH;
            lbl = LBL_RETURN_SWITCH;
            continue 'sw;
        }
        let v = *cp;
        cp = cp.add(1);
        if fc == v {
            if (*F).fields.type_repeat.ctype == OP_NOTPROP {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            break;
        }
    }
    /* Loop back to RMATCH(Fecode, RM215) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM215;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM216) at C 3991: minimizing PT_UCNC. */

if lbl == LBL_RM_BASE + RM216 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    if (fc == CHAR_DOLLAR_SIGN
        || fc == CHAR_COMMERCIAL_AT
        || fc == CHAR_GRAVE_ACCENT
        || (fc >= 0xa0 && fc <= 0xd7ff)
        || fc >= 0xe000)
        == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
    {
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    /* Loop back to RMATCH(Fecode, RM216) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM216;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM223) at C 4010: minimizing PT_BIDICL. */

if lbl == LBL_RM_BASE + RM223 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    if (UCD_BIDICLASS(fc) == (*F).fields.type_repeat.propvalue)
        == ((*F).fields.type_repeat.ctype == OP_NOTPROP)
    {
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    /* Loop back to RMATCH(Fecode, RM223) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM223;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM222) at C 4029: minimizing PT_BOOL. */

if lbl == LBL_RM_BASE + RM222 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    let prop: &'static ucd_record = GET_UCD(fc);
    let ok: BOOL = crate::internal::boolprop_set_bit(
        UCD_BPROPS_PROP(prop) as usize,
        (*F).fields.type_repeat.propvalue,
    ) as BOOL;
    if ok == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL) {
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    /* Loop back to RMATCH(Fecode, RM222) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM222;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM217) at C 4063: minimizing OP_EXTUNI. */

if lbl == LBL_RM_BASE + RM217 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    } else {
        /* GETCHARINCTEST(fc, Feptr) */
        fc = *(*F).eptr as u32;
        (*F).eptr = (*F).eptr.add(1);
        if utf != FALSE && fc >= 0xc0 {
            let r = getutf8inc(fc, (*F).eptr);
            fc = r.0;
            (*F).eptr = r.1;
        }
        (*F).eptr = crate::extuni::_pcre2_extuni_8(
            fc,
            (*F).eptr,
            (*mb).start_subject,
            (*mb).end_subject,
            utf,
            core::ptr::null_mut(),
        );
    }
    /* CHECK_PARTIAL() */
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
    }
    /* Loop back to RMATCH(Fecode, RM217) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM217;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM218) at C 4090: minimizing non-property
character types, UTF mode. */

if lbl == LBL_RM_BASE + RM218 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    if (*F).fields.type_repeat.ctype == OP_ANY
        && {
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
        }
    {
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
    match (*F).fields.type_repeat.ctype {
        /* This is the non-NL case */
        OP_ANY => {
            if (*mb).partial != 0   /* Take care with CRLF partial */
                && (*F).eptr >= (*mb).end_subject
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
            CHAR_CR => {
                if (*F).eptr < (*mb).end_subject && *(*F).eptr as u32 == CHAR_LF {
                    (*F).eptr = (*F).eptr.add(1);
                }
            }

            CHAR_LF => {}

            CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            }

            _ => {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        },

        OP_NOT_HSPACE => match fc {
            /* HSPACE_CASES: */
            0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002 | 0x2003
            | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f
            | 0x205f | 0x3000 => {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            _ => {}
        },

        OP_HSPACE => match fc {
            /* HSPACE_CASES: */
            0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002 | 0x2003
            | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f
            | 0x205f | 0x3000 => {}
            _ => {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        },

        OP_NOT_VSPACE => match fc {
            /* VSPACE_CASES: */
            0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            _ => {}
        },

        OP_VSPACE => match fc {
            /* VSPACE_CASES: */
            0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => {}
            _ => {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        },

        OP_NOT_DIGIT => {
            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }

        OP_DIGIT => {
            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }

        OP_NOT_WHITESPACE => {
            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }

        OP_WHITESPACE => {
            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }

        OP_NOT_WORDCHAR => {
            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }

        OP_WORDCHAR => {
            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }

        _ => {
            return PCRE2_ERROR_INTERNAL;
        }
    }
    /* Loop back to RMATCH(Fecode, RM218) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM218;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}

/* --------------------------------------------------------------------- */
/* Continuation of RMATCH(Fecode, RM33) at C 4220: minimizing non-property
character types, not UTF mode. */

if lbl == LBL_RM_BASE + RM33 as u32 {
    if rrc != MATCH_NOMATCH {
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    let oldmin = (*F).fields.type_repeat.min;
    (*F).fields.type_repeat.min = oldmin.wrapping_add(1);
    if oldmin >= (*F).fields.type_repeat.max {
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
    if (*F).fields.type_repeat.ctype == OP_ANY
        && {
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
        }
    {
        rrc = MATCH_NOMATCH;
        lbl = LBL_RETURN_SWITCH;
        continue 'sw;
    }
    fc = *(*F).eptr as u32;
    (*F).eptr = (*F).eptr.add(1);
    match (*F).fields.type_repeat.ctype {
        /* This is the non-NL case */
        OP_ANY => {
            if (*mb).partial != 0   /* Take care with CRLF partial */
                && (*F).eptr >= (*mb).end_subject
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
            CHAR_CR => {
                if (*F).eptr < (*mb).end_subject && *(*F).eptr as u32 == CHAR_LF {
                    (*F).eptr = (*F).eptr.add(1);
                }
            }

            CHAR_LF => {}

            CHAR_VT | CHAR_FF | CHAR_NEL => {
                if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                    rrc = MATCH_NOMATCH;
                    lbl = LBL_RETURN_SWITCH;
                    continue 'sw;
                }
            }

            _ => {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        },

        OP_NOT_HSPACE => match fc {
            /* HSPACE_BYTE_CASES: */
            0x09 | 0x20 | 0xa0 => {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            _ => {}
        },

        OP_HSPACE => match fc {
            /* HSPACE_BYTE_CASES: */
            0x09 | 0x20 | 0xa0 => {}
            _ => {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        },

        OP_NOT_VSPACE => match fc {
            /* VSPACE_BYTE_CASES: */
            0x0a | 0x0b | 0x0c | 0x0d | 0x85 => {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
            _ => {}
        },

        OP_VSPACE => match fc {
            /* VSPACE_BYTE_CASES: */
            0x0a | 0x0b | 0x0c | 0x0d | 0x85 => {}
            _ => {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        },

        OP_NOT_DIGIT => {
            if MAX_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }

        OP_DIGIT => {
            if !MAX_255(fc) || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }

        OP_NOT_WHITESPACE => {
            if MAX_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }

        OP_WHITESPACE => {
            if !MAX_255(fc) || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }

        OP_NOT_WORDCHAR => {
            if MAX_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }

        OP_WORDCHAR => {
            if !MAX_255(fc) || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                rrc = MATCH_NOMATCH;
                lbl = LBL_RETURN_SWITCH;
                continue 'sw;
            }
        }

        _ => {
            return PCRE2_ERROR_INTERNAL;
        }
    }
    /* Loop back to RMATCH(Fecode, RM33) */
    start_ecode = (*F).ecode;
    (*F).return_id = RM33;
    lbl = LBL_MATCH_RECURSE;
    continue 'sw;
}
}
