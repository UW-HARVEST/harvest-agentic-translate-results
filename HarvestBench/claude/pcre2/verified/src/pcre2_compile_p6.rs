/* Translated from c_src/src/pcre2_compile.c lines 5967-6066 */

/*************************************************
*       Find first significant opcode            *
*************************************************/

/* This is called by several functions that scan a compiled expression looking
for a fixed first character, or an anchoring opcode etc. It skips over things
that do not influence this. For some calls, it makes sense to skip negative
forward and all backward assertions, and also the \b assertion; for others it
does not.

Arguments:
  code         pointer to the start of the group
  skipassert   TRUE if certain assertions are to be skipped

Returns:       pointer to the first significant opcode
*/

unsafe fn first_significant_code(code: PCRE2_SPTR, skipassert: BOOL) -> PCRE2_SPTR {
    let mut code: PCRE2_SPTR = code;
    loop {
        let op: u32 = *code as u32;

        if op == OP_ASSERT_NOT
            || op == OP_ASSERTBACK
            || op == OP_ASSERTBACK_NOT
            || op == OP_ASSERTBACK_NA
        {
            if skipassert == FALSE {
                return code;
            }
            loop {
                code = code.add(GET!(code, 1) as usize);
                if !(*code as u32 == OP_ALT) {
                    break;
                }
            }
            code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(*code as usize) as usize);
        } else if op == OP_WORD_BOUNDARY
            || op == OP_NOT_WORD_BOUNDARY
            || op == OP_UCP_WORD_BOUNDARY
            || op == OP_NOT_UCP_WORD_BOUNDARY
        {
            if skipassert == FALSE {
                return code;
            }
            /* Fall through */

            code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(*code as usize) as usize);
        } else if op == OP_CALLOUT
            || op == OP_CREF
            || op == OP_DNCREF
            || op == OP_RREF
            || op == OP_DNRREF
            || op == OP_FALSE
            || op == OP_TRUE
        {
            code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(*code as usize) as usize);
        } else if op == OP_CALLOUT_STR {
            code = code.add(GET!(code, 1 + 2 * LINK_SIZE) as usize);
        } else if op == OP_SKIPZERO {
            code = code.add(2 + GET!(code, 2) as usize + LINK_SIZE);
        } else if op == OP_COND || op == OP_SCOND {
            if *code.add(1 + LINK_SIZE) as u32 != OP_FALSE ||   /* Not DEFINE */
               *code.add(GET!(code, 1) as usize) as u32 != OP_KET
            /* More than one branch */
            {
                return code;
            }
            code = code.add(GET!(code, 1) as usize + 1 + LINK_SIZE);
        } else if op == OP_MARK
            || op == OP_COMMIT_ARG
            || op == OP_PRUNE_ARG
            || op == OP_SKIP_ARG
            || op == OP_THEN_ARG
        {
            code = code.add(
                *code.add(1) as usize
                    + *_pcre2_OP_lengths_8.as_ptr().add(*code as usize) as usize,
            );
        } else {
            return code;
        }
    }

    /* LCOV_EXCL_START */
    /* PCRE2_DEBUG_UNREACHABLE(); Control should never reach here */
    /* LCOV_EXCL_STOP */
}
