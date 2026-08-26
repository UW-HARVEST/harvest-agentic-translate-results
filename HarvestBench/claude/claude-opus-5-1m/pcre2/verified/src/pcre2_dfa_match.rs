// Translated from c_src/src/pcre2_dfa_match.c
use crate::internal::*;

include!("pcre2_dfa_match_head.rs");
include!("pcre2_dfa_match_tables.rs");

/* The static helper functions do_callout_dfa() and more_workspace(). */
include!("pcre2_dfa_match_helpers.rs");

/*************************************************
*     Match a Regular Expression - DFA engine    *
*************************************************/

/* This is the main matching function. It is called recursively for some
patterns (assertions, recursion, possessive groups).

Returns:            > 0 => number of match offset pairs placed in offsets
                    = 0 => offsets overflowed; longest matches are present
                     -1 => failed to match
                   < -1 => some kind of unexpected problem
*/

unsafe fn internal_dfa_match(
    mb: *mut dfa_match_block,
    this_start_code: PCRE2_SPTR,
    current_subject_arg: PCRE2_SPTR,
    start_offset: PCRE2_SIZE,
    offsets: *mut PCRE2_SIZE,
    offsetcount_arg: u32,
    workspace: *mut c_int,
    wscount_arg: c_int,
    rlevel_arg: u32,
    mut RWS: *mut c_int,
) -> c_int {
    let mut current_subject: PCRE2_SPTR = current_subject_arg;
    let mut offsetcount: u32 = offsetcount_arg;
    let mut wscount: c_int = wscount_arg;
    let mut rlevel: u32 = rlevel_arg;

    let mut active_states: *mut stateblock;
    let mut new_states: *mut stateblock;
    let mut temp_states: *mut stateblock;
    let mut next_active_state: *mut stateblock;
    let mut next_new_state: *mut stateblock;
    let ctypes: *const u8;
    let lcc: *const u8;
    let fcc: *const u8;
    let mut ptr: PCRE2_SPTR;
    let mut end_code: PCRE2_SPTR;
    let mut new_recursive: dfa_recursion_info = core::mem::zeroed();
    let mut active_count: c_int;
    let mut new_count: c_int;
    let mut match_count: c_int;

    /* Some fields in the mb block are frequently referenced, so we load them into
    independent variables in the hope that this will perform better. */

    let start_subject: PCRE2_SPTR = (*mb).start_subject;
    let end_subject: PCRE2_SPTR = (*mb).end_subject;
    let start_code: PCRE2_SPTR = (*mb).start_code;

    let utf: BOOL = (((*mb).poptions & PCRE2_UTF) != 0) as BOOL;
    let utf_or_ucp: BOOL = (utf != 0 || ((*mb).poptions & PCRE2_UCP) != 0) as BOOL;

    let mut reset_could_continue: BOOL = FALSE;

    let mcc = (*mb).match_call_count;
    (*mb).match_call_count = mcc + 1;
    if mcc >= (*mb).match_limit {
        return PCRE2_ERROR_MATCHLIMIT;
    }
    {
        let old = rlevel;
        rlevel = old + 1;
        if old > (*mb).match_limit_depth {
            return PCRE2_ERROR_DEPTHLIMIT;
        }
    }
    offsetcount &= (-2i32) as u32; /* Round down */

    wscount -= 2;
    wscount = (wscount - (wscount % (INTS_PER_STATEBLOCK * 2))) / (2 * INTS_PER_STATEBLOCK);

    ctypes = (*mb).tables.add(ctypes_offset);
    lcc = (*mb).tables.add(lcc_offset);
    fcc = (*mb).tables.add(fcc_offset);

    match_count = PCRE2_ERROR_NOMATCH; /* A negative number */

    active_states = workspace.add(2) as *mut stateblock;
    new_states = active_states.add(wscount as usize);
    next_new_state = new_states;
    new_count = 0;
    active_count = 0;
    next_active_state = active_states;

    /* The macros used for adding states to the two state vectors (one for the
    current character, one for the following character). */

    macro_rules! ADD_ACTIVE {
        ($x:expr, $y:expr) => {{
            let ac__ = active_count;
            active_count = ac__ + 1;
            if ac__ < wscount {
                (*next_active_state).offset = $x;
                (*next_active_state).count = $y;
                next_active_state = next_active_state.add(1);
            } else {
                return PCRE2_ERROR_DFA_WSSIZE;
            }
        }};
    }

    #[allow(unused_macros)]
    macro_rules! ADD_ACTIVE_DATA {
        ($x:expr, $y:expr, $z:expr) => {{
            let ac__ = active_count;
            active_count = ac__ + 1;
            if ac__ < wscount {
                (*next_active_state).offset = $x;
                (*next_active_state).count = $y;
                (*next_active_state).data = $z;
                next_active_state = next_active_state.add(1);
            } else {
                return PCRE2_ERROR_DFA_WSSIZE;
            }
        }};
    }

    macro_rules! ADD_NEW {
        ($x:expr, $y:expr) => {{
            let nc__ = new_count;
            new_count = nc__ + 1;
            if nc__ < wscount {
                (*next_new_state).offset = $x;
                (*next_new_state).count = $y;
                next_new_state = next_new_state.add(1);
            } else {
                return PCRE2_ERROR_DFA_WSSIZE;
            }
        }};
    }

    macro_rules! ADD_NEW_DATA {
        ($x:expr, $y:expr, $z:expr) => {{
            let nc__ = new_count;
            new_count = nc__ + 1;
            if nc__ < wscount {
                (*next_new_state).offset = $x;
                (*next_new_state).count = $y;
                (*next_new_state).data = $z;
                next_new_state = next_new_state.add(1);
            } else {
                return PCRE2_ERROR_DFA_WSSIZE;
            }
        }};
    }

    /* The first thing in any (sub) pattern is a bracket of some sort. Push all
    the alternative states onto the list, and find out where the end is. This
    makes is possible to use this function recursively, when we want to stop at a
    matching internal ket rather than at the end.

    If we are dealing with a backward assertion we have to find out the maximum
    amount to move back, and set up each alternative appropriately. */

    if *this_start_code as u32 == OP_ASSERTBACK || *this_start_code as u32 == OP_ASSERTBACK_NOT {
        let mut max_back: usize = 0;
        let gone_back: usize;

        end_code = this_start_code;
        loop {
            let back: usize = GET2!(end_code, 2 + LINK_SIZE) as usize;
            if back > max_back {
                max_back = back;
            }
            end_code = end_code.add(GET!(end_code, 1) as usize);
            if *end_code as u32 != OP_ALT {
                break;
            }
        }

        /* If we can't go back the amount required for the longest lookbehind
        pattern, go back as far as we can; some alternatives may still be viable. */

        /* In character mode we have to step back character by character */

        if utf != 0 {
            let mut gb: usize = 0;
            while gb < max_back {
                if current_subject <= start_subject {
                    break;
                }
                current_subject = current_subject.sub(1);
                ACROSSCHAR!(
                    current_subject > start_subject,
                    current_subject,
                    current_subject = current_subject.sub(1)
                );
                gb += 1;
            }
            gone_back = gb;
        } else {
            /* In byte-mode we can do this quickly. */
            let current_offset: usize = current_subject.offset_from(start_subject) as usize;
            gone_back = if current_offset < max_back {
                current_offset
            } else {
                max_back
            };
            current_subject = current_subject.sub(gone_back);
        }

        /* Save the earliest consulted character */

        if current_subject < (*mb).start_used_ptr {
            (*mb).start_used_ptr = current_subject;
        }

        /* Now we can process the individual branches. There will be an OP_REVERSE at
        the start of each branch, except when the length of the branch is zero. */

        end_code = this_start_code;
        loop {
            let revlen: u32 = if *end_code.add(1 + LINK_SIZE) as u32 == OP_REVERSE {
                (1 + IMM2_SIZE) as u32
            } else {
                0
            };
            let back: usize = if revlen == 0 {
                0
            } else {
                GET2!(end_code, 2 + LINK_SIZE) as usize
            };
            if back <= gone_back {
                let bstate: c_int =
                    (end_code.offset_from(start_code) as usize + 1 + LINK_SIZE + revlen as usize)
                        as c_int;
                ADD_NEW_DATA!(-bstate, 0, (gone_back - back) as c_int);
            }
            end_code = end_code.add(GET!(end_code, 1) as usize);
            if *end_code as u32 != OP_ALT {
                break;
            }
        }
    }
    /* This is the code for a "normal" subpattern (not a backward assertion). The
    start of a whole pattern is always one of these. If we are at the top level,
    we may be asked to restart matching from the same point that we reached for a
    previous partial match. We still have to scan through the top-level branches to
    find the end state. */
    else {
        end_code = this_start_code;

        /* Restarting */

        if rlevel == 1 && ((*mb).moptions & PCRE2_DFA_RESTART) != 0 {
            loop {
                end_code = end_code.add(GET!(end_code, 1) as usize);
                if *end_code as u32 != OP_ALT {
                    break;
                }
            }
            new_count = *workspace.add(1);
            if *workspace == 0 {
                memcpy(
                    new_states as *mut c_void,
                    active_states as *const c_void,
                    new_count as usize * size_of::<stateblock>(),
                );
            }
        }
        /* Not restarting */
        else {
            let mut length: c_int = (1 + LINK_SIZE
                + (if *this_start_code as u32 == OP_CBRA
                    || *this_start_code as u32 == OP_SCBRA
                    || *this_start_code as u32 == OP_CBRAPOS
                    || *this_start_code as u32 == OP_SCBRAPOS
                {
                    IMM2_SIZE
                } else {
                    0
                })) as c_int;
            loop {
                ADD_NEW!(end_code.offset_from(start_code) as c_int + length, 0);
                end_code = end_code.add(GET!(end_code, 1) as usize);
                length = (1 + LINK_SIZE) as c_int;
                if *end_code as u32 != OP_ALT {
                    break;
                }
            }
        }
    }

    *workspace = 0; /* Bit indicating which vector is current */

    /* Loop for scanning the subject */

    ptr = current_subject;
    'subject_loop: loop {
        let mut i: c_int;
        let mut j: c_int;
        let mut clen: c_int;
        let mut dlen: c_int;
        let mut c: u32;
        let mut d: u32;
        let mut partial_newline: BOOL = FALSE;
        let mut could_continue: BOOL = reset_could_continue;
        reset_could_continue = FALSE;

        if ptr > (*mb).last_used_ptr {
            (*mb).last_used_ptr = ptr;
        }

        /* Make the new state list into the active state list and empty the
        new state list. */

        temp_states = active_states;
        active_states = new_states;
        new_states = temp_states;
        active_count = new_count;
        new_count = 0;

        *workspace ^= 1; /* Remember for the restarting feature */
        *workspace.add(1) = active_count;

        /* Set the pointers for adding new states */

        next_active_state = active_states.add(active_count as usize);
        next_new_state = new_states;

        /* Load the current character from the subject outside the loop, as many
        different states may want to look at it, and we assume that at least one
        will. */

        if ptr < end_subject {
            clen = 1; /* Number of data items in the character */
            GETCHARLENTEST!(c, ptr, clen, utf);
        } else {
            clen = 0; /* This indicates the end of the subject */
            c = NOTACHAR; /* This value should never actually be used */
        }

        /* Scan up the active states and act on each one. The result of an action
        may be to add more states to the currently active list (e.g. on hitting a
        parenthesis) or it may be to put states on the new list, for considering
        when we move the character pointer on. */

        i = 0;
        while i < active_count {
            /* The C code uses `continue` and `goto NEXT_ACTIVE_STATE` to go on to
            the next active state; both are `break 'next_active_state` here so that
            the loop increment below still happens. */
            'next_active_state: {
                let current_state: *mut stateblock = active_states.add(i as usize);
                let mut caseless: BOOL = FALSE;
                let mut code: PCRE2_SPTR;
                let mut codevalue: u32;
                let mut state_offset: c_int = (*current_state).offset;
                let mut rrc: c_int;
                let mut count: c_int;

                /* A negative offset is a special case meaning "hold off going to this
                (negated) state until the number of characters in the data field have
                been skipped". If the could_continue flag was passed over from a previous
                state, arrange for it to passed on. */

                if state_offset < 0 {
                    if (*current_state).data > 0 {
                        ADD_NEW_DATA!(
                            state_offset,
                            (*current_state).count,
                            (*current_state).data - 1
                        );
                        if could_continue != 0 {
                            reset_could_continue = TRUE;
                        }
                        break 'next_active_state;
                    } else {
                        state_offset = -state_offset;
                        (*current_state).offset = state_offset;
                    }
                }

                /* Check for a duplicate state with the same count, and skip if found. */

                j = 0;
                while j < i {
                    if (*active_states.add(j as usize)).offset == state_offset
                        && (*active_states.add(j as usize)).count == (*current_state).count
                    {
                        break 'next_active_state;
                    }
                    j += 1;
                }

                /* The state offset is the offset to the opcode */

                code = start_code.add(state_offset as usize);
                codevalue = *code as u32;

                /* If this opcode inspects a character, but we are at the end of the
                subject, remember the fact for use when testing for a partial match. */

                if clen == 0 && *poptable.as_ptr().add(codevalue as usize) != 0 {
                    could_continue = TRUE;
                }

                /* If this opcode is followed by an inline character, load it. */

                if *coptable.as_ptr().add(codevalue as usize) > 0 {
                    dlen = 1;
                    if utf != 0 {
                        let dptr = code.add(*coptable.as_ptr().add(codevalue as usize) as usize);
                        GETCHARLEN!(d, dptr, dlen);
                    } else {
                        d = *code.add(*coptable.as_ptr().add(codevalue as usize) as usize) as u32;
                    }
                    if codevalue >= OP_TYPESTAR {
                        if d == OP_ANYBYTE {
                            return PCRE2_ERROR_DFA_UITEM;
                        } else if d == OP_NOTPROP || d == OP_PROP {
                            codevalue += OP_PROP_EXTRA;
                        } else if d == OP_ANYNL {
                            codevalue += OP_ANYNL_EXTRA;
                        } else if d == OP_EXTUNI {
                            codevalue += OP_EXTUNI_EXTRA;
                        } else if d == OP_NOT_HSPACE || d == OP_HSPACE {
                            codevalue += OP_HSPACE_EXTRA;
                        } else if d == OP_NOT_VSPACE || d == OP_VSPACE {
                            codevalue += OP_VSPACE_EXTRA;
                        }
                    }
                } else {
                    dlen = 0; /* Not strictly necessary, but compilers moan */
                    d = NOTACHAR; /* if these variables are not set. */
                }

                /* Now process the individual opcodes. Each fragment handles a set of
                opcodes; an unhandled codevalue falls through to the next fragment and
                finally to the C `default:` case. */

                include!("pcre2_dfa_ops1.rs");
                include!("pcre2_dfa_ops2.rs");
                include!("pcre2_dfa_ops3.rs");
                include!("pcre2_dfa_ops4.rs");
                include!("pcre2_dfa_ops5.rs");
                include!("pcre2_dfa_ops6.rs");

                /* Unsupported opcode */
                return PCRE2_ERROR_DFA_UITEM;
            }
            i += 1;
        } /* End of loop scanning active states */

        /* We have finished the processing at the current subject character. If no
        new states have been set for the next character, we have found all the
        matches that we are going to find. If partial matching has been requested,
        check for appropriate conditions.

        The "could_continue" variable is true if a state could have continued but
        for the fact that the end of the subject was reached. */

        if new_count <= 0 {
            if could_continue != 0
                && (((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                    || (((*mb).moptions & PCRE2_PARTIAL_SOFT) != 0 && match_count < 0))
                && (partial_newline != 0
                    || (ptr >= end_subject
                        && (ptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)))
            {
                match_count = PCRE2_ERROR_PARTIAL;
            }
            break 'subject_loop; /* Exit from loop along the subject string */
        }

        /* One or more states are active for the next character. */

        ptr = ptr.add(clen as usize); /* Advance to next subject character */
    } /* Loop to move along the subject string */

    /* Control gets here from "break" a few lines above. If we have a match and
    PCRE2_ENDANCHORED is set, the match fails. */

    if match_count >= 0
        && (((*mb).moptions | (*mb).poptions) & PCRE2_ENDANCHORED) != 0
        && ptr < end_subject
    {
        match_count = PCRE2_ERROR_NOMATCH;
    }

    match_count
}

/*************************************************
*     Match a pattern using the DFA algorithm    *
*************************************************/

include!("pcre2_dfa_match_public.rs");
