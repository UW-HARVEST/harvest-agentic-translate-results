/* Translated from c_src/src/pcre2_substitute.c lines 443-741 */

/* Helper to perform the call to the substitute_case_callout. We wrap the
user-provided callout because our internal arguments are slightly extended. We
don't want the user callout to handle the case of "\l" (first character only to
lowercase) or "\l\U" (first character to lowercase, rest to uppercase) because
those are not operations defined by Unicode. Instead the user callout simply
needs to provide the three Unicode primitives: lower, upper, titlecase. */

unsafe fn do_case_copy(
    input_output: *mut PCRE2_UCHAR,
    input_len: PCRE2_SIZE,
    output_cap: PCRE2_SIZE,
    state: *mut case_state,
    utf: BOOL,
    substitute_case_callout: pcre2_substitute_case_callout_fn,
    substitute_case_callout_data: *mut c_void,
) -> PCRE2_SIZE {
    let input: PCRE2_SPTR = input_output;
    let output: *mut PCRE2_UCHAR = input_output;
    let mut rc: PCRE2_SIZE;
    let mut rc2: PCRE2_SIZE;
    let ch1_to_case: c_int;
    let rest_to_case: c_int;
    let mut ch1: [PCRE2_UCHAR; 6] = [0; 6];
    let ch1_len: PCRE2_SIZE;
    let mut rest: PCRE2_SPTR;
    let rest_len: PCRE2_SIZE;
    let mut ch1_overflow: BOOL = FALSE;
    let mut rest_overflow: BOOL = FALSE;

    /* PCRE2_ASSERT(input_len != 0); */

    /* switch (state->to_case) */
    if (*state).to_case == PCRE2_SUBSTITUTE_CASE_LOWER as c_int
        || (*state).to_case == PCRE2_SUBSTITUTE_CASE_UPPER as c_int
        || (*state).to_case == PCRE2_SUBSTITUTE_CASE_TITLE_FIRST as c_int
    {
        /* case PCRE2_SUBSTITUTE_CASE_LOWER: // Can be single_char TRUE or FALSE
           case PCRE2_SUBSTITUTE_CASE_UPPER: // Can only be single_char FALSE
           case PCRE2_SUBSTITUTE_CASE_TITLE_FIRST: // Can be single_char TRUE or FALSE

        The easy case, where our internal casing operations align with those of
        the callout. */

        if (*state).single_char == FALSE {
            rc = (substitute_case_callout.unwrap())(
                input,
                input_len,
                output,
                output_cap,
                (*state).to_case,
                substitute_case_callout_data,
            );

            if (*state).to_case == PCRE2_SUBSTITUTE_CASE_TITLE_FIRST as c_int {
                (*state).to_case = PCRE2_SUBSTITUTE_CASE_LOWER as c_int;
            }

            return rc;
        }

        ch1_to_case = (*state).to_case;
        rest_to_case = PCRE2_SUBSTITUTE_CASE_NONE as c_int;
    } else if (*state).to_case == PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST as c_int {
        /* case PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST: // Can only be single_char FALSE */
        ch1_to_case = PCRE2_SUBSTITUTE_CASE_LOWER as c_int;
        rest_to_case = PCRE2_SUBSTITUTE_CASE_UPPER as c_int;
    } else {
        /* LCOV_EXCL_START */
        /* default: PCRE2_DEBUG_UNREACHABLE(); */
        return 0;
        /* LCOV_EXCL_STOP */
    }

    /* Identify the leading character. Take copy, because its storage overlaps with
    `output`, and hence may be scrambled by the callout. */

    {
        let mut ch_end: PCRE2_SPTR = input;
        let mut ch: u32;

        GETCHARINCTEST!(ch, ch_end, utf);
        let _ = ch;
        /* PCRE2_ASSERT(ch_end <= input + input_len && ch_end - input <= 6); */
        ch1_len = ch_end.offset_from(input) as PCRE2_SIZE;
        memcpy(
            ch1.as_mut_ptr() as *mut c_void,
            input as *const c_void,
            CU2BYTES!(ch1_len),
        );
    }

    rest = input.add(ch1_len);
    rest_len = input_len - ch1_len;

    /* Transform just ch1. The buffers are always in-place (input == output). With a
    custom callout, we need a loop to discover its required buffer size. The loop
    wouldn't be required if the callout were well-behaved, but it might be naughty
    and return "5" the first time, then "10" the next time we call it using the
    exact same input! */

    {
        let mut ch1_cap: PCRE2_SIZE;
        let max_ch1_cap: PCRE2_SIZE;

        ch1_cap = ch1_len; /* First attempt uses the space vacated by ch1. */
        /* PCRE2_ASSERT(output_cap >= input_len && input_len >= rest_len); */
        max_ch1_cap = output_cap - rest_len;

        loop {
            rc = (substitute_case_callout.unwrap())(
                ch1.as_ptr(),
                ch1_len,
                output,
                ch1_cap,
                ch1_to_case,
                substitute_case_callout_data,
            );
            if rc == !(0 as PCRE2_SIZE) {
                return rc;
            }

            if rc <= ch1_cap {
                break;
            }

            if rc > max_ch1_cap {
                ch1_overflow = TRUE;
                break;
            }

            /* Move the rest to the right, to make room for expanding ch1. */

            memmove(
                input_output.add(rc) as *mut c_void,
                rest as *const c_void,
                CU2BYTES!(rest_len),
            );
            rest = input.add(rc);

            ch1_cap = rc;

            /* Proof of loop termination: `ch1_cap` is growing on each iteration, but
            the loop ends if `rc` reaches the (unchanging) upper bound of output_cap. */
        }
    }

    if rest_to_case == PCRE2_SUBSTITUTE_CASE_NONE as c_int {
        if ch1_overflow == FALSE {
            /* PCRE2_ASSERT(rest_len <= output_cap - rc); */
            memmove(
                output.add(rc) as *mut c_void,
                rest as *const c_void,
                CU2BYTES!(rest_len),
            );
        }
        rc2 = rest_len;

        (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE as c_int;
    } else {
        let mut dummy: [PCRE2_UCHAR; 1] = [0; 1];

        rc2 = (substitute_case_callout.unwrap())(
            rest,
            rest_len,
            if ch1_overflow != 0 {
                dummy.as_mut_ptr()
            } else {
                output.add(rc)
            },
            if ch1_overflow != 0 {
                0 as PCRE2_SIZE
            } else {
                output_cap - rc
            },
            rest_to_case,
            substitute_case_callout_data,
        );
        if rc2 == !(0 as PCRE2_SIZE) {
            return rc2;
        }

        if ch1_overflow == FALSE && rc2 > output_cap - rc {
            rest_overflow = TRUE;
        }

        /* If ch1 grows so that `xform(ch1)+rest` can't fit in the buffer, but then
        `rest` shrinks, it's actually possible for the total calculated length of
        `xform(ch1)+xform(rest)` to come out at less than output_cap. But we can't
        report that, because it would make it seem that the operation succeeded.
        If either of xform(ch1) or xform(rest) won't fit in the buffer, our final
        result must be > output_cap. */
        if ch1_overflow != 0 && rc2 < rest_len {
            rc2 = rest_len;
        }

        (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER as c_int;
    }

    if rc2 > !(0 as PCRE2_SIZE) - rc
    /* Integer overflow */
    {
        return !(0 as PCRE2_SIZE);
    }

    /* PCRE2_ASSERT(!(ch1_overflow || rest_overflow) || rc + rc2 > output_cap); */
    let _ = rest_overflow;

    rc + rc2
}

/* c_src lines 608-741 contain only comments and the CHECKMEMCPY,
CHECKCASECPY_BASE, CHECKCASECPY_DEFAULT, CHECKCASECPY_CALLOUT and
DELAYEDFORCECASE macros. Those macros contain `goto NOROOM` /
`goto TOOLARGEREPLACE` / `goto CASEERROR` jumps into the body of
pcre2_substitute(), so they cannot be Rust macros (loop labels are hygienic);
they are expanded by hand at each use site inside pcre2_substitute() itself. */
