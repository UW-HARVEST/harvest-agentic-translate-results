{
    /* The cases in this fragment reassign RWS when the recursion workspace has to
    be extended (the C code does `RWS = (int *)rws;`); the skeleton declares the
    parameter as `mut RWS`, so the update persists exactly as in C. */

    match codevalue {
        /* ================================================================== */
        /* These are the class-handling opcodes */
        OP_CLASS | OP_NCLASS | OP_XCLASS | OP_ECLASS => {
            let mut isinclass: BOOL = FALSE;
            let next_state_offset: c_int;
            let ecode: PCRE2_SPTR;

            /* An extended class may have a table or a list of single characters,
            ranges, or both, and it may be positive or negative. There's a
            function that sorts all this out. */

            if codevalue == OP_XCLASS {
                ecode = code.add(GET!(code, 1) as usize);
                if clen > 0 {
                    isinclass =
                        _pcre2_xclass_8(c, code.add(1 + LINK_SIZE), (*mb).start_code, utf);
                }
            }
            /* A nested set-based class has internal opcodes for performing
            set operations. */
            else if codevalue == OP_ECLASS {
                ecode = code.add(GET!(code, 1) as usize);
                if clen > 0 {
                    isinclass =
                        _pcre2_eclass_8(c, code.add(1 + LINK_SIZE), ecode, (*mb).start_code, utf);
                }
            }
            /* For a simple class, there is always just a 32-byte table, and we
            can set isinclass from it. */
            else {
                ecode = code.add(1 + 32 / size_of::<PCRE2_UCHAR>());
                if clen > 0 {
                    isinclass = if c > 255 {
                        (codevalue == OP_NCLASS) as BOOL
                    } else {
                        ((*code.add(1).add((c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0)
                            as BOOL
                    };
                }
            }

            /* At this point, isinclass is set for all kinds of class, and ecode
            points to the byte after the end of the class. If there is a
            quantifier, this is where it will be. */

            next_state_offset = ecode.offset_from(start_code) as c_int;

            match *ecode as u32 {
                OP_CRSTAR | OP_CRMINSTAR | OP_CRPOSSTAR => {
                    ADD_ACTIVE!(next_state_offset + 1, 0);
                    if isinclass != 0 {
                        if *ecode as u32 == OP_CRPOSSTAR {
                            active_count -= 1; /* Remove non-match possibility */
                            next_active_state = next_active_state.sub(1);
                        }
                        ADD_NEW!(state_offset, 0);
                    }
                }

                OP_CRPLUS | OP_CRMINPLUS | OP_CRPOSPLUS => {
                    count = (*current_state).count; /* Already matched */
                    if count > 0 {
                        ADD_ACTIVE!(next_state_offset + 1, 0);
                    }
                    if isinclass != 0 {
                        if count > 0 && *ecode as u32 == OP_CRPOSPLUS {
                            active_count -= 1; /* Remove non-match possibility */
                            next_active_state = next_active_state.sub(1);
                        }
                        count += 1;
                        ADD_NEW!(state_offset, count);
                    }
                }

                OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSQUERY => {
                    ADD_ACTIVE!(next_state_offset + 1, 0);
                    if isinclass != 0 {
                        if *ecode as u32 == OP_CRPOSQUERY {
                            active_count -= 1; /* Remove non-match possibility */
                            next_active_state = next_active_state.sub(1);
                        }
                        ADD_NEW!(next_state_offset + 1, 0);
                    }
                }

                OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                    count = (*current_state).count; /* Already matched */
                    if count >= GET2!(ecode, 1) as c_int {
                        ADD_ACTIVE!(next_state_offset + 1 + 2 * IMM2_SIZE as c_int, 0);
                    }
                    if isinclass != 0 {
                        let max: c_int = GET2!(ecode, 1 + IMM2_SIZE) as c_int;

                        if *ecode as u32 == OP_CRPOSRANGE && count >= GET2!(ecode, 1) as c_int {
                            active_count -= 1; /* Remove non-match possibility */
                            next_active_state = next_active_state.sub(1);
                        }

                        count += 1;
                        if count >= max && max != 0
                        /* Max 0 => no limit */
                        {
                            ADD_NEW!(next_state_offset + 1 + 2 * IMM2_SIZE as c_int, 0);
                        } else {
                            ADD_NEW!(state_offset, count);
                        }
                    }
                }

                _ => {
                    if isinclass != 0 {
                        ADD_NEW!(next_state_offset, 0);
                    }
                }
            }
            break 'next_active_state;
        }

        /* ================================================================== */
        /* These are the opcodes for fancy brackets of various kinds. We have
        to use recursion in order to handle them. The "always failing" assertion
        (?!) is optimised to OP_FAIL when compiling, so we have to support that,
        though the other "backtracking verbs" are not supported. */
        OP_FAIL => {
            break 'next_active_state;
        }

        OP_ASSERT | OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT => {
            let mut rc: c_int;
            let local_workspace: *mut c_int;
            let local_offsets: *mut PCRE2_SIZE;
            let mut endasscode: PCRE2_SPTR = code.add(GET!(code, 1) as usize);
            let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;

            if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                rc = more_workspace(&mut rws, RWS_OVEC_OSIZE as c_uint, mb);
                if rc != 0 {
                    return rc;
                }
                RWS = rws as *mut c_int;
            }

            local_offsets = RWS
                .add((*rws).size as usize)
                .sub((*rws).free as usize) as *mut PCRE2_SIZE;
            local_workspace = (local_offsets as *mut c_int).add(RWS_OVEC_OSIZE);
            (*rws).free = (*rws).free.wrapping_sub((RWS_RSIZE + RWS_OVEC_OSIZE) as u32);

            while *endasscode as u32 == OP_ALT {
                endasscode = endasscode.add(GET!(endasscode, 1) as usize);
            }

            rc = internal_dfa_match(
                mb,                                        /* static match data */
                code,                                      /* this subexpression's code */
                ptr,                                       /* where we currently are */
                ptr.offset_from(start_subject) as PCRE2_SIZE, /* start offset */
                local_offsets,                             /* offset vector */
                (RWS_OVEC_OSIZE / OVEC_UNIT) as u32,       /* size of same */
                local_workspace,                           /* workspace vector */
                RWS_RSIZE as c_int,                        /* size of same */
                rlevel,                                    /* function recursion level */
                RWS,                                       /* recursion workspace */
            );

            (*rws).free = (*rws).free.wrapping_add((RWS_RSIZE + RWS_OVEC_OSIZE) as u32);

            if rc < 0 && rc != PCRE2_ERROR_NOMATCH {
                return rc;
            }
            if (rc >= 0) == (codevalue == OP_ASSERT || codevalue == OP_ASSERTBACK) {
                ADD_ACTIVE!(
                    endasscode.add(LINK_SIZE + 1).offset_from(start_code) as c_int,
                    0
                );
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_COND | OP_SCOND => {
            let codelink: c_int = GET!(code, 1) as c_int;
            let condcode: PCRE2_UCHAR;

            /* Because of the way auto-callout works during compile, a callout item
            is inserted between OP_COND and an assertion condition. This does not
            happen for the other conditions. */

            if *code.add(LINK_SIZE + 1) as u32 == OP_CALLOUT
                || *code.add(LINK_SIZE + 1) as u32 == OP_CALLOUT_STR
            {
                let mut callout_length: PCRE2_SIZE = 0;
                rrc = do_callout_dfa(
                    code,
                    offsets,
                    current_subject,
                    ptr,
                    mb,
                    1 + LINK_SIZE,
                    &mut callout_length,
                );
                if rrc < 0 {
                    return rrc; /* Abandon */
                }
                if rrc > 0 {
                    break 'next_active_state; /* Fail this thread */
                }
                code = code.add(callout_length); /* Skip callout data */
            }

            condcode = *code.add(LINK_SIZE + 1);

            /* Back reference conditions and duplicate named recursion conditions
            are not supported */

            if condcode as u32 == OP_CREF
                || condcode as u32 == OP_DNCREF
                || condcode as u32 == OP_DNRREF
            {
                return PCRE2_ERROR_DFA_UCOND;
            }

            /* The DEFINE condition is always false, and the assertion (?!) is
            converted to OP_FAIL. */

            if condcode as u32 == OP_FALSE || condcode as u32 == OP_FAIL {
                ADD_ACTIVE!(state_offset + codelink + LINK_SIZE as c_int + 1, 0);
            }
            /* There is also an always-true condition */
            else if condcode as u32 == OP_TRUE {
                ADD_ACTIVE!(state_offset + LINK_SIZE as c_int + 2, 0);
            }
            /* The only supported version of OP_RREF is for the value RREF_ANY,
            which means "test if in any recursion". We can't test for specifically
            recursed groups. */
            else if condcode as u32 == OP_RREF {
                let value: c_uint = GET2!(code, LINK_SIZE + 2) as c_uint;
                if value != RREF_ANY {
                    return PCRE2_ERROR_DFA_UCOND;
                }
                if !(*mb).recursive.is_null() {
                    ADD_ACTIVE!(
                        state_offset + LINK_SIZE as c_int + 2 + IMM2_SIZE as c_int,
                        0
                    );
                } else {
                    ADD_ACTIVE!(state_offset + codelink + LINK_SIZE as c_int + 1, 0);
                }
            }
            /* Otherwise, the condition is an assertion */
            else {
                let mut rc: c_int;
                let local_workspace: *mut c_int;
                let local_offsets: *mut PCRE2_SIZE;
                let asscode: PCRE2_SPTR = code.add(LINK_SIZE + 1);
                let mut endasscode: PCRE2_SPTR = asscode.add(GET!(asscode, 1) as usize);
                let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;

                if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                    rc = more_workspace(&mut rws, RWS_OVEC_OSIZE as c_uint, mb);
                    if rc != 0 {
                        return rc;
                    }
                    RWS = rws as *mut c_int;
                }

                local_offsets = RWS
                    .add((*rws).size as usize)
                    .sub((*rws).free as usize) as *mut PCRE2_SIZE;
                local_workspace = (local_offsets as *mut c_int).add(RWS_OVEC_OSIZE);
                (*rws).free = (*rws).free.wrapping_sub((RWS_RSIZE + RWS_OVEC_OSIZE) as u32);

                while *endasscode as u32 == OP_ALT {
                    endasscode = endasscode.add(GET!(endasscode, 1) as usize);
                }

                rc = internal_dfa_match(
                    mb,                                        /* fixed match data */
                    asscode,                                   /* this subexpression's code */
                    ptr,                                       /* where we currently are */
                    ptr.offset_from(start_subject) as PCRE2_SIZE, /* start offset */
                    local_offsets,                             /* offset vector */
                    (RWS_OVEC_OSIZE / OVEC_UNIT) as u32,       /* size of same */
                    local_workspace,                           /* workspace vector */
                    RWS_RSIZE as c_int,                        /* size of same */
                    rlevel,                                    /* function recursion level */
                    RWS,                                       /* recursion workspace */
                );

                (*rws).free = (*rws).free.wrapping_add((RWS_RSIZE + RWS_OVEC_OSIZE) as u32);

                if rc < 0 && rc != PCRE2_ERROR_NOMATCH {
                    return rc;
                }
                if (rc >= 0)
                    == (condcode as u32 == OP_ASSERT || condcode as u32 == OP_ASSERTBACK)
                {
                    ADD_ACTIVE!(
                        endasscode.add(LINK_SIZE + 1).offset_from(start_code) as c_int,
                        0
                    );
                } else {
                    ADD_ACTIVE!(state_offset + codelink + LINK_SIZE as c_int + 1, 0);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_RECURSE => {
            let mut rc: c_int;
            let local_workspace: *mut c_int;
            let local_offsets: *mut PCRE2_SIZE;
            let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;
            let callpat: PCRE2_SPTR = start_code.add(GET!(code, 1) as usize);
            let recno: u32 = if callpat == (*mb).start_code {
                0
            } else {
                GET2!(callpat, 1 + LINK_SIZE)
            };

            /* Argument list has not been supported yet. */
            if *code.add(1 + LINK_SIZE) as u32 == OP_CREF {
                return PCRE2_ERROR_DFA_UITEM;
            }

            if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_RSIZE {
                rc = more_workspace(&mut rws, RWS_OVEC_RSIZE as c_uint, mb);
                if rc != 0 {
                    return rc;
                }
                RWS = rws as *mut c_int;
            }

            local_offsets = RWS
                .add((*rws).size as usize)
                .sub((*rws).free as usize) as *mut PCRE2_SIZE;
            local_workspace = (local_offsets as *mut c_int).add(RWS_OVEC_RSIZE);
            (*rws).free = (*rws).free.wrapping_sub((RWS_RSIZE + RWS_OVEC_RSIZE) as u32);

            /* Check for repeating a recursion without advancing the subject
            pointer or last used character. This should catch convoluted mutual
            recursions. (Some simple cases are caught at compile time.) */

            let mut ri: *mut dfa_recursion_info = (*mb).recursive;
            while !ri.is_null() {
                if recno == (*ri).group_num
                    && ptr == (*ri).subject_position
                    && (*mb).last_used_ptr == (*ri).last_used_ptr
                {
                    return PCRE2_ERROR_RECURSELOOP;
                }
                ri = (*ri).prevrec;
            }

            /* Remember this recursion and where we started it so as to
            catch infinite loops. */

            new_recursive.group_num = recno;
            new_recursive.subject_position = ptr;
            new_recursive.last_used_ptr = (*mb).last_used_ptr;
            new_recursive.prevrec = (*mb).recursive;
            (*mb).recursive = &mut new_recursive;

            rc = internal_dfa_match(
                mb,                                        /* fixed match data */
                callpat,                                   /* this subexpression's code */
                ptr,                                       /* where we currently are */
                ptr.offset_from(start_subject) as PCRE2_SIZE, /* start offset */
                local_offsets,                             /* offset vector */
                (RWS_OVEC_RSIZE / OVEC_UNIT) as u32,       /* size of same */
                local_workspace,                           /* workspace vector */
                RWS_RSIZE as c_int,                        /* size of same */
                rlevel,                                    /* function recursion level */
                RWS,                                       /* recursion workspace */
            );

            (*rws).free = (*rws).free.wrapping_add((RWS_RSIZE + RWS_OVEC_RSIZE) as u32);
            (*mb).recursive = new_recursive.prevrec; /* Done this recursion */

            /* Ran out of internal offsets */

            if rc == 0 {
                return PCRE2_ERROR_DFA_RECURSE;
            }

            /* For each successful matched substring, set up the next state with a
            count of characters to skip before trying it. Note that the count is in
            characters, not bytes. */

            if rc > 0 {
                rc = rc * 2 - 2;
                while rc >= 0 {
                    let mut charcount: PCRE2_SIZE = (*local_offsets.add((rc + 1) as usize))
                        .wrapping_sub(*local_offsets.add(rc as usize));
                    if utf != 0 {
                        let mut p: PCRE2_SPTR = start_subject.add(*local_offsets.add(rc as usize));
                        let pp: PCRE2_SPTR =
                            start_subject.add(*local_offsets.add((rc + 1) as usize));
                        while p < pp {
                            let pc = *p;
                            p = p.add(1);
                            if NOT_FIRSTCU!(pc) {
                                charcount = charcount.wrapping_sub(1);
                            }
                        }
                    }
                    if charcount > 0 {
                        ADD_NEW_DATA!(
                            -(state_offset + LINK_SIZE as c_int + 1),
                            0,
                            charcount.wrapping_sub(1) as c_int
                        );
                    } else {
                        ADD_ACTIVE!(state_offset + LINK_SIZE as c_int + 1, 0);
                    }
                    rc -= 2;
                }
            } else if rc != PCRE2_ERROR_NOMATCH {
                return rc;
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_BRAPOS | OP_SBRAPOS | OP_CBRAPOS | OP_SCBRAPOS | OP_BRAPOSZERO => {
            let mut rc: c_int;
            let local_workspace: *mut c_int;
            let local_offsets: *mut PCRE2_SIZE;
            let mut charcount: PCRE2_SIZE;
            let mut matched_count: PCRE2_SIZE;
            let mut local_ptr: PCRE2_SPTR = ptr;
            let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;
            let allow_zero: BOOL;

            if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                rc = more_workspace(&mut rws, RWS_OVEC_OSIZE as c_uint, mb);
                if rc != 0 {
                    return rc;
                }
                RWS = rws as *mut c_int;
            }

            local_offsets = RWS
                .add((*rws).size as usize)
                .sub((*rws).free as usize) as *mut PCRE2_SIZE;
            local_workspace = (local_offsets as *mut c_int).add(RWS_OVEC_OSIZE);
            (*rws).free = (*rws).free.wrapping_sub((RWS_RSIZE + RWS_OVEC_OSIZE) as u32);

            if codevalue == OP_BRAPOSZERO {
                allow_zero = TRUE;
                code = code.add(1); /* The following opcode will be one of the above BRAs */
            } else {
                allow_zero = FALSE;
            }

            /* Loop to match the subpattern as many times as possible as if it were
            a complete pattern. */

            matched_count = 0;
            loop {
                rc = internal_dfa_match(
                    mb,                                        /* fixed match data */
                    code,                                      /* this subexpression's code */
                    local_ptr,                                 /* where we currently are */
                    ptr.offset_from(start_subject) as PCRE2_SIZE, /* start offset */
                    local_offsets,                             /* offset vector */
                    (RWS_OVEC_OSIZE / OVEC_UNIT) as u32,       /* size of same */
                    local_workspace,                           /* workspace vector */
                    RWS_RSIZE as c_int,                        /* size of same */
                    rlevel,                                    /* function recursion level */
                    RWS,                                       /* recursion workspace */
                );

                /* Failed to match */

                if rc < 0 {
                    if rc != PCRE2_ERROR_NOMATCH {
                        return rc;
                    }
                    break;
                }

                /* Matched: break the loop if zero characters matched. */

                charcount = (*local_offsets.add(1)).wrapping_sub(*local_offsets);
                if charcount == 0 {
                    break;
                }
                local_ptr = local_ptr.add(charcount); /* Advance temporary position ptr */
                matched_count += 1;
            }

            (*rws).free = (*rws).free.wrapping_add((RWS_RSIZE + RWS_OVEC_OSIZE) as u32);

            /* At this point we have matched the subpattern matched_count
            times, and local_ptr is pointing to the character after the end of the
            last match. */

            if matched_count > 0 || allow_zero != 0 {
                let mut end_subpattern: PCRE2_SPTR = code;
                let next_state_offset: c_int;

                loop {
                    end_subpattern = end_subpattern.add(GET!(end_subpattern, 1) as usize);
                    if *end_subpattern as u32 != OP_ALT {
                        break;
                    }
                }
                next_state_offset =
                    end_subpattern.offset_from(start_code) as c_int + LINK_SIZE as c_int + 1;

                /* Optimization: if there are no more active states, and there
                are no new states yet set up, then skip over the subject string
                right here, to save looping. Otherwise, set up the new state to swing
                into action when the end of the matched substring is reached. */

                if i + 1 >= active_count && new_count == 0 {
                    ptr = local_ptr;
                    clen = 0;
                    ADD_NEW!(next_state_offset, 0);
                } else {
                    let mut p: PCRE2_SPTR = ptr;
                    let pp: PCRE2_SPTR = local_ptr;
                    charcount = pp.offset_from(p) as PCRE2_SIZE;
                    if utf != 0 {
                        while p < pp {
                            let pc = *p;
                            p = p.add(1);
                            if NOT_FIRSTCU!(pc) {
                                charcount = charcount.wrapping_sub(1);
                            }
                        }
                    }
                    ADD_NEW_DATA!(-next_state_offset, 0, charcount.wrapping_sub(1) as c_int);
                }
            }
            break 'next_active_state;
        }

        /*-----------------------------------------------------------------*/
        OP_ONCE => {
            let mut rc: c_int;
            let local_workspace: *mut c_int;
            let local_offsets: *mut PCRE2_SIZE;
            let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;

            if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                rc = more_workspace(&mut rws, RWS_OVEC_OSIZE as c_uint, mb);
                if rc != 0 {
                    return rc;
                }
                RWS = rws as *mut c_int;
            }

            local_offsets = RWS
                .add((*rws).size as usize)
                .sub((*rws).free as usize) as *mut PCRE2_SIZE;
            local_workspace = (local_offsets as *mut c_int).add(RWS_OVEC_OSIZE);
            (*rws).free = (*rws).free.wrapping_sub((RWS_RSIZE + RWS_OVEC_OSIZE) as u32);

            rc = internal_dfa_match(
                mb,                                        /* fixed match data */
                code,                                      /* this subexpression's code */
                ptr,                                       /* where we currently are */
                ptr.offset_from(start_subject) as PCRE2_SIZE, /* start offset */
                local_offsets,                             /* offset vector */
                (RWS_OVEC_OSIZE / OVEC_UNIT) as u32,       /* size of same */
                local_workspace,                           /* workspace vector */
                RWS_RSIZE as c_int,                        /* size of same */
                rlevel,                                    /* function recursion level */
                RWS,                                       /* recursion workspace */
            );

            (*rws).free = (*rws).free.wrapping_add((RWS_RSIZE + RWS_OVEC_OSIZE) as u32);

            if rc >= 0 {
                let mut end_subpattern: PCRE2_SPTR = code;
                let mut charcount: PCRE2_SIZE =
                    (*local_offsets.add(1)).wrapping_sub(*local_offsets);
                let next_state_offset: c_int;
                let repeat_state_offset: c_int;

                loop {
                    end_subpattern = end_subpattern.add(GET!(end_subpattern, 1) as usize);
                    if *end_subpattern as u32 != OP_ALT {
                        break;
                    }
                }
                next_state_offset =
                    end_subpattern.offset_from(start_code) as c_int + LINK_SIZE as c_int + 1;

                /* If the end of this subpattern is KETRMAX or KETRMIN, we must
                arrange for the repeat state also to be added to the relevant list.
                Calculate the offset, or set -1 for no repeat. */

                repeat_state_offset = if *end_subpattern as u32 == OP_KETRMAX
                    || *end_subpattern as u32 == OP_KETRMIN
                {
                    end_subpattern.offset_from(start_code) as c_int
                        - GET!(end_subpattern, 1) as c_int
                } else {
                    -1
                };

                /* If we have matched an empty string, add the next state at the
                current character pointer. This is important so that the duplicate
                checking kicks in, which is what breaks infinite loops that match an
                empty string. */

                if charcount == 0 {
                    ADD_ACTIVE!(next_state_offset, 0);
                }
                /* Optimization: if there are no more active states, and there
                are no new states yet set up, then skip over the subject string
                right here, to save looping. Otherwise, set up the new state to swing
                into action when the end of the matched substring is reached. */
                else if i + 1 >= active_count && new_count == 0 {
                    ptr = ptr.add(charcount);
                    clen = 0;
                    ADD_NEW!(next_state_offset, 0);

                    /* If we are adding a repeat state at the new character position,
                    we must fudge things so that it is the only current state.
                    Otherwise, it might be a duplicate of one we processed before, and
                    that would cause it to be skipped. */

                    if repeat_state_offset >= 0 {
                        next_active_state = active_states;
                        active_count = 0;
                        i = -1;
                        ADD_ACTIVE!(repeat_state_offset, 0);
                    }
                } else {
                    if utf != 0 {
                        let mut p: PCRE2_SPTR = start_subject.add(*local_offsets);
                        let pp: PCRE2_SPTR = start_subject.add(*local_offsets.add(1));
                        while p < pp {
                            let pc = *p;
                            p = p.add(1);
                            if NOT_FIRSTCU!(pc) {
                                charcount = charcount.wrapping_sub(1);
                            }
                        }
                    }
                    ADD_NEW_DATA!(-next_state_offset, 0, charcount.wrapping_sub(1) as c_int);
                    if repeat_state_offset >= 0 {
                        ADD_NEW_DATA!(
                            -repeat_state_offset,
                            0,
                            charcount.wrapping_sub(1) as c_int
                        );
                    }
                }
            } else if rc != PCRE2_ERROR_NOMATCH {
                return rc;
            }
            break 'next_active_state;
        }

        /* ================================================================== */
        /* Handle callouts */
        OP_CALLOUT | OP_CALLOUT_STR => {
            let mut callout_length: PCRE2_SIZE = 0;
            rrc = do_callout_dfa(code, offsets, current_subject, ptr, mb, 0, &mut callout_length);
            if rrc < 0 {
                return rrc; /* Abandon */
            }
            if rrc == 0 {
                ADD_ACTIVE!(state_offset + callout_length as c_int, 0);
            }
            break 'next_active_state;
        }

        _ => {}
    }
}
