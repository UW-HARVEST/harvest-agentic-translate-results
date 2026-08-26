/* Translated from c_src/src/pcre2_compile.c lines 1259-1488 */

/* The macro IS_DIGIT(x) ((x) >= CHAR_0 && (x) <= CHAR_9) (c_src line 276) is
expanded inline everywhere below, exactly as the C preprocessor does. */

/*************************************************
*   Read a number, possibly signed and constrained *
*************************************************/

/* This function is used to read numbers in the pattern. The initial pointer
must be at the sign or first digit of the number. When relative values
(introduced by "+" or "-") are allowed, they are relative group numbers, and the
result must be greater than zero.

Arguments:
  ptrptr      points to the character pointer variable
  ptrend      points to the end of the input string
  allow_sign  if < 0, sign not allowed; if >= 0, sign is relative to this
  max_value   the largest number allowed;
              you must not pass a value for max_value larger than
              INT_MAX/10 - 1 because this function relies on max_value to
              avoid integer overflow
  max_error   the error to give for an over-large number
  intptr      where to put the result
  errcodeptr  where to put an error code

Returns:      TRUE  - a number was read
              FALSE - errorcode == 0 => no number was found
                      errorcode != 0 => an error occurred
*/

unsafe fn read_number(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    allow_sign: i32,
    mut max_value: u32,
    max_error: u32,
    intptr: *mut c_int,
    errorcodeptr: *mut c_int,
) -> BOOL {
    let mut sign: c_int = 0;
    let mut n: u32 = 0;
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let mut yield_: BOOL = FALSE;

    /* PCRE2_ASSERT(max_value <= INT_MAX/10 - 1); */

    *errorcodeptr = 0;

    if allow_sign >= 0 && ptr < ptrend {
        if *ptr as u32 == CHAR_PLUS {
            sign = 1;
            max_value = max_value.wrapping_sub(allow_sign as u32);
            ptr = ptr.add(1);
        } else if *ptr as u32 == CHAR_MINUS {
            sign = -1;
            ptr = ptr.add(1);
        }
    }

    if ptr >= ptrend || !(*ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_9) {
        return FALSE;
    }

    'exit: {
        while ptr < ptrend && (*ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_9) {
            let c = *ptr;
            ptr = ptr.add(1);
            n = n
                .wrapping_mul(10)
                .wrapping_add((c as u32).wrapping_sub(CHAR_0));
            if n > max_value {
                *errorcodeptr = max_error as c_int;
                while ptr < ptrend && (*ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_9) {
                    ptr = ptr.add(1);
                }
                break 'exit;
            }
        }

        if allow_sign >= 0 && sign != 0 {
            if n == 0 {
                *errorcodeptr = ERR26; /* +0 and -0 are not allowed */
                break 'exit;
            }

            if sign > 0 {
                n = n.wrapping_add(allow_sign as u32);
            } else if n > allow_sign as u32 {
                *errorcodeptr = ERR15; /* Non-existent subpattern */
                break 'exit;
            } else {
                n = (allow_sign as u32).wrapping_add(1).wrapping_sub(n);
            }
        }

        yield_ = TRUE;
    }

    /* EXIT: */
    *intptr = n as c_int;
    *ptrptr = ptr;
    return yield_;
}

/*************************************************
*         Read repeat counts                     *
*************************************************/

/* Read an item of the form {n,m} and return the values when non-NULL pointers
are supplied. Repeat counts must be less than 65536 (MAX_REPEAT_COUNT); a
larger value is used for "unlimited". We have to use signed arguments for
read_number() because it is capable of returning a signed value. As of Perl
5.34.0 either n or m may be absent, but not both. Perl also allows spaces and
tabs after { and before } and between the numbers and the comma, so we do too.

Arguments:
  ptrptr         points to pointer to character after '{'
  ptrend         pointer to end of input
  minp           if not NULL, pointer to int for min
  maxp           if not NULL, pointer to int for max
  errorcodeptr   points to error code variable

Returns:         FALSE if not a repeat quantifier, errorcode set zero
                 FALSE on error, with errorcode set non-zero
                 TRUE on success, with pointer updated to point after '}'
*/

unsafe fn read_repeat_counts(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    minp: *mut u32,
    maxp: *mut u32,
    errorcodeptr: *mut c_int,
) -> BOOL {
    let mut p: PCRE2_SPTR = *ptrptr;
    let mut pp: PCRE2_SPTR;
    let mut yield_: BOOL = FALSE;
    let mut had_minimum: BOOL = FALSE;
    let mut min: i32 = 0;
    let mut max: i32 = 65536; /* REPEAT_UNLIMITED: this value is larger than MAX_REPEAT_COUNT */

    *errorcodeptr = 0;
    while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
        p = p.add(1);
    }

    /* Check the syntax before interpreting. Otherwise, a non-quantifier sequence
    such as "X{123456ABC" would incorrectly give a "number too big in quantifier"
    error. */

    pp = p;
    if pp < ptrend && (*pp as u32 >= CHAR_0 && *pp as u32 <= CHAR_9) {
        had_minimum = TRUE;
        loop {
            pp = pp.add(1);
            if !(pp < ptrend && (*pp as u32 >= CHAR_0 && *pp as u32 <= CHAR_9)) {
                break;
            }
        }
    }

    while pp < ptrend && (*pp as u32 == CHAR_SPACE || *pp as u32 == CHAR_HT) {
        pp = pp.add(1);
    }
    if pp >= ptrend {
        return FALSE;
    }

    if *pp as u32 == CHAR_RIGHT_CURLY_BRACKET {
        if had_minimum == FALSE {
            return FALSE;
        }
    } else {
        let c = *pp;
        pp = pp.add(1);
        if c as u32 != CHAR_COMMA {
            return FALSE;
        }
        while pp < ptrend && (*pp as u32 == CHAR_SPACE || *pp as u32 == CHAR_HT) {
            pp = pp.add(1);
        }
        if pp >= ptrend {
            return FALSE;
        }
        if *pp as u32 >= CHAR_0 && *pp as u32 <= CHAR_9 {
            loop {
                pp = pp.add(1);
                if !(pp < ptrend && (*pp as u32 >= CHAR_0 && *pp as u32 <= CHAR_9)) {
                    break;
                }
            }
        } else if had_minimum == FALSE {
            return FALSE;
        }
        while pp < ptrend && (*pp as u32 == CHAR_SPACE || *pp as u32 == CHAR_HT) {
            pp = pp.add(1);
        }
        if pp >= ptrend || *pp as u32 != CHAR_RIGHT_CURLY_BRACKET {
            return FALSE;
        }
    }

    /* Now process the quantifier for real. We know it must be {n} or {n,} or {,m}
    or {n,m}. The only error that read_number() can return is for a number that is
    too big. If *errorcodeptr is returned as zero it means no number was found. */

    /* Deal with {,m} or n too big. If we successfully read m there is no need to
    check m >= n because n defaults to zero. */

    'exit: {
        if read_number(
            &mut p,
            ptrend,
            -1,
            65535, /* MAX_REPEAT_COUNT */
            ERR5 as u32,
            &mut min,
            errorcodeptr,
        ) == FALSE
        {
            if *errorcodeptr != 0 {
                break 'exit;
            } /* n too big */
            p = p.add(1); /* Skip comma and subsequent spaces */
            while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                p = p.add(1);
            }
            if read_number(
                &mut p,
                ptrend,
                -1,
                65535, /* MAX_REPEAT_COUNT */
                ERR5 as u32,
                &mut max,
                errorcodeptr,
            ) == FALSE
            {
                if *errorcodeptr != 0 {
                    break 'exit;
                } /* m too big */
            }
        }
        /* Have read one number. Deal with {n} or {n,} or {n,m} */
        else {
            while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                p = p.add(1);
            }
            if *p as u32 == CHAR_RIGHT_CURLY_BRACKET {
                max = min;
            } else
            /* Handle {n,} or {n,m} */
            {
                p = p.add(1); /* Skip comma and subsequent spaces */
                while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                    p = p.add(1);
                }
                if read_number(
                    &mut p,
                    ptrend,
                    -1,
                    65535, /* MAX_REPEAT_COUNT */
                    ERR5 as u32,
                    &mut max,
                    errorcodeptr,
                ) == FALSE
                {
                    if *errorcodeptr != 0 {
                        break 'exit;
                    } /* m too big */
                }

                if max < min {
                    *errorcodeptr = ERR4;
                    break 'exit;
                }
            }
        }

        /* Valid quantifier exists */

        while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
            p = p.add(1);
        }
        p = p.add(1);
        yield_ = TRUE;
        if !minp.is_null() {
            *minp = min as u32;
        }
        if !maxp.is_null() {
            *maxp = max as u32;
        }
    }

    /* Update the pattern pointer */

    /* EXIT: */
    *ptrptr = p;
    return yield_;
}
