/* Translated from c_src/src/pcre2_auto_possess.c lines 1150-end */

/*************************************************
*    Scan compiled regex for auto-possession     *
*************************************************/

/* Replaces single character iterations with their possessive alternatives
if appropriate. This function modifies the compiled opcode! Hitting a
non-existent opcode may indicate a bug in PCRE2, but it can also be caused if a
bad UTF string was compiled with PCRE2_NO_UTF_CHECK. The rec_limit catches
overly complicated or large patterns. In these cases, the check just stops,
leaving the remainder of the pattern unpossessified.

Arguments:
  code        points to start of the byte code
  cb          compile data block

Returns:      0 for success
              -1 if a non-existant opcode is encountered
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_auto_possessify_8(
    code: *mut PCRE2_UCHAR,
    cb: *const compile_block,
) -> c_int {
    let mut code: *mut PCRE2_UCHAR = code;
    let mut c: PCRE2_UCHAR;
    let mut list: [u32; MAX_LIST] = [0; MAX_LIST];
    let mut rec_limit: c_int = 1000; /* Was 10,000 but clang+ASAN uses a lot of stack. */
    let utf: BOOL = if ((*cb).external_options & PCRE2_UTF) != 0 {
        TRUE
    } else {
        FALSE
    };
    let ucp: BOOL = if ((*cb).external_options & PCRE2_UCP) != 0 {
        TRUE
    } else {
        FALSE
    };

    loop {
        c = *code;

        /* LCOV_EXCL_START */
        if c as u32 >= OP_TABLE_LENGTH {
            /* PCRE2_DEBUG_UNREACHABLE(); */
            return -1; /* Something gone wrong */
        }
        /* LCOV_EXCL_STOP */

        if c as u32 >= OP_STAR && c as u32 <= OP_TYPEPOSUPTO {
            c = c.wrapping_sub((get_repeat_base(c) as u32).wrapping_sub(OP_STAR) as PCRE2_UCHAR);
            let end: PCRE2_SPTR = if c as u32 <= OP_MINUPTO {
                get_chr_property_list(
                    code as PCRE2_SPTR,
                    utf,
                    ucp,
                    (*cb).fcc,
                    list.as_mut_ptr(),
                )
            } else {
                std::ptr::null()
            };
            list[1] = (c as u32 == OP_STAR
                || c as u32 == OP_PLUS
                || c as u32 == OP_QUERY
                || c as u32 == OP_UPTO) as u32;

            if !end.is_null()
                && compare_opcodes(
                    end,
                    utf,
                    ucp,
                    cb,
                    list.as_ptr(),
                    end,
                    &mut rec_limit as *mut c_int,
                ) != 0
            {
                match c as u32 {
                    OP_STAR => {
                        *code = (*code).wrapping_add((OP_POSSTAR - OP_STAR) as PCRE2_UCHAR);
                    }

                    OP_MINSTAR => {
                        *code = (*code).wrapping_add((OP_POSSTAR - OP_MINSTAR) as PCRE2_UCHAR);
                    }

                    OP_PLUS => {
                        *code = (*code).wrapping_add((OP_POSPLUS - OP_PLUS) as PCRE2_UCHAR);
                    }

                    OP_MINPLUS => {
                        *code = (*code).wrapping_add((OP_POSPLUS - OP_MINPLUS) as PCRE2_UCHAR);
                    }

                    OP_QUERY => {
                        *code = (*code).wrapping_add((OP_POSQUERY - OP_QUERY) as PCRE2_UCHAR);
                    }

                    OP_MINQUERY => {
                        *code = (*code).wrapping_add((OP_POSQUERY - OP_MINQUERY) as PCRE2_UCHAR);
                    }

                    OP_UPTO => {
                        *code = (*code).wrapping_add((OP_POSUPTO - OP_UPTO) as PCRE2_UCHAR);
                    }

                    OP_MINUPTO => {
                        *code = (*code).wrapping_add((OP_POSUPTO - OP_MINUPTO) as PCRE2_UCHAR);
                    }

                    _ => {}
                }
            }
            c = *code;
        } else if c as u32 == OP_CLASS
            || c as u32 == OP_NCLASS
            || c as u32 == OP_XCLASS
            || c as u32 == OP_ECLASS
        {
            let repeat_opcode: *mut PCRE2_UCHAR = if c as u32 == OP_XCLASS
                || c as u32 == OP_ECLASS
            {
                code.add(GET!(code, 1) as usize)
            } else {
                code.add(1 + (32 / size_of::<PCRE2_UCHAR>()))
            };

            c = *repeat_opcode;
            if c as u32 >= OP_CRSTAR && c as u32 <= OP_CRMINRANGE {
                /* The return from get_chr_property_list() will never be NULL when
                *code (aka c) is one of the four class opcodes. However, gcc with
                -fanalyzer notes that a NULL return is possible, and grumbles. Hence we
                put in a check. */

                let end: PCRE2_SPTR = get_chr_property_list(
                    code as PCRE2_SPTR,
                    utf,
                    ucp,
                    (*cb).fcc,
                    list.as_mut_ptr(),
                );
                list[1] = ((c & 1) == 0) as u32;

                if !end.is_null()
                    && compare_opcodes(
                        end,
                        utf,
                        ucp,
                        cb,
                        list.as_ptr(),
                        end,
                        &mut rec_limit as *mut c_int,
                    ) != 0
                {
                    match c as u32 {
                        OP_CRSTAR | OP_CRMINSTAR => {
                            *repeat_opcode = OP_CRPOSSTAR as PCRE2_UCHAR;
                        }

                        OP_CRPLUS | OP_CRMINPLUS => {
                            *repeat_opcode = OP_CRPOSPLUS as PCRE2_UCHAR;
                        }

                        OP_CRQUERY | OP_CRMINQUERY => {
                            *repeat_opcode = OP_CRPOSQUERY as PCRE2_UCHAR;
                        }

                        OP_CRRANGE | OP_CRMINRANGE => {
                            *repeat_opcode = OP_CRPOSRANGE as PCRE2_UCHAR;
                        }

                        _ => {}
                    }
                }
            }
            c = *code;
        }

        match c as u32 {
            OP_END => {
                return 0;
            }

            OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
            | OP_TYPEMINQUERY | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS | OP_TYPEPOSQUERY => {
                if *code.add(1) as u32 == OP_PROP || *code.add(1) as u32 == OP_NOTPROP {
                    code = code.add(2);
                }
            }

            OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT | OP_TYPEPOSUPTO => {
                if *code.add(1 + IMM2_SIZE) as u32 == OP_PROP
                    || *code.add(1 + IMM2_SIZE) as u32 == OP_NOTPROP
                {
                    code = code.add(2);
                }
            }

            OP_CALLOUT_STR => {
                code = code.add(GET!(code, 1 + 2 * LINK_SIZE) as usize);
            }

            OP_XCLASS | OP_ECLASS => {
                code = code.add(GET!(code, 1) as usize);
            }

            OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                code = code.add(*code.add(1) as usize);
            }

            _ => {}
        }

        /* Add in the fixed length from the table */

        code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(c as usize) as usize);

        /* In UTF-8 and UTF-16 modes, opcodes that are followed by a character may be
        followed by a multi-byte character. The length in the table is a minimum, so
        we have to arrange to skip the extra code units. */

        if utf != 0 {
            match c as u32 {
                OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_STAR | OP_MINSTAR | OP_PLUS
                | OP_MINPLUS | OP_QUERY | OP_MINQUERY | OP_UPTO | OP_MINUPTO | OP_EXACT
                | OP_POSSTAR | OP_POSPLUS | OP_POSQUERY | OP_POSUPTO | OP_STARI | OP_MINSTARI
                | OP_PLUSI | OP_MINPLUSI | OP_QUERYI | OP_MINQUERYI | OP_UPTOI | OP_MINUPTOI
                | OP_EXACTI | OP_POSSTARI | OP_POSPLUSI | OP_POSQUERYI | OP_POSUPTOI
                | OP_NOTSTAR | OP_NOTMINSTAR | OP_NOTPLUS | OP_NOTMINPLUS | OP_NOTQUERY
                | OP_NOTMINQUERY | OP_NOTUPTO | OP_NOTMINUPTO | OP_NOTEXACT | OP_NOTPOSSTAR
                | OP_NOTPOSPLUS | OP_NOTPOSQUERY | OP_NOTPOSUPTO | OP_NOTSTARI | OP_NOTMINSTARI
                | OP_NOTPLUSI | OP_NOTMINPLUSI | OP_NOTQUERYI | OP_NOTMINQUERYI | OP_NOTUPTOI
                | OP_NOTMINUPTOI | OP_NOTEXACTI | OP_NOTPOSSTARI | OP_NOTPOSPLUSI
                | OP_NOTPOSQUERYI | OP_NOTPOSUPTOI => {
                    if HAS_EXTRALEN!(*code.offset(-1)) {
                        code = code.add(GET_EXTRALEN!(*code.offset(-1)) as usize);
                    }
                }

                _ => {}
            }
        }
    }
}

/* End of pcre2_auto_possess.c */
