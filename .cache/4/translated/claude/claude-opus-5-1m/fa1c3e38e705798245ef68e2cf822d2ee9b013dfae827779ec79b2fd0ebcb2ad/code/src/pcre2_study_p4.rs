/* Translated from c_src/src/pcre2_study.c lines 1917-2086 */

/*************************************************
*          Study a compiled expression           *
*************************************************/

/* This function is handed a compiled expression that it must study to produce
information that will speed up the matching.

Argument:
  re       points to the compiled expression

Returns:   0 normally; non-zero should never normally occur
           1 unknown opcode in set_start_bits
           2 missing capturing bracket
           3 unknown opcode in find_minlength
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_study_8(re: *mut pcre2_real_code) -> c_int {
    let mut count: c_int = 0;
    let code: *mut PCRE2_UCHAR;
    let utf: BOOL = if ((*re).overall_options & PCRE2_UTF) != 0 {
        TRUE
    } else {
        FALSE
    };
    let ucp: BOOL = if ((*re).overall_options & PCRE2_UCP) != 0 {
        TRUE
    } else {
        FALSE
    };

    /* Find start of compiled code */

    code = (re as *mut u8).add((*re).code_start) as *mut PCRE2_UCHAR;

    /* For a pattern that has a first code unit, or a multiline pattern that
    matches only at "line start", there is no point in seeking a list of starting
    code units. */

    if ((*re).flags & (PCRE2_FIRSTSET | PCRE2_STARTLINE)) == 0 {
        let mut depth: c_int = 0;
        let rc: c_int = set_start_bits(re, code as PCRE2_SPTR, utf, ucp, &mut depth);
        /* LCOV_EXCL_START */
        if rc == SSB_UNKNOWN as c_int {
            /* PCRE2_DEBUG_UNREACHABLE(); */
            return 1;
        }
        /* LCOV_EXCL_STOP */

        /* If a list of starting code units was set up, scan the list to see if only
        one or two were listed. Having only one listed is rare because usually a
        single starting code unit will have been recognized and PCRE2_FIRSTSET set.
        If two are listed, see if they are caseless versions of the same character;
        if so we can replace the list with a caseless first code unit. This gives
        better performance and is plausibly worth doing for patterns such as [Ww]ord
        or (word|WORD). */

        if rc == SSB_DONE as c_int {
            let mut i: c_int;
            let mut a: c_int = -1;
            let mut b: c_int = -1;
            let mut p: *mut u8 = (*re).start_bitmap.as_mut_ptr();
            let mut flags: u32 = PCRE2_FIRSTMAPSET;

            'done: {
                i = 0;
                while i < 256 {
                    let x: u8 = *p;
                    if x != 0 {
                        let mut c: c_int;
                        let y: u8 = x & (!x).wrapping_add(1); /* Least significant bit */
                        if y != x {
                            break 'done;
                        } /* More than one bit set */

                        /* In the 16-bit and 32-bit libraries, the bit for 0xff means "0xff and
                        all wide characters", so we cannot use it here. */

                        /* Compute the character value */

                        c = i;
                        match x {
                            1 => {}
                            2 => c += 1,
                            4 => c += 2,
                            8 => c += 3,
                            16 => c += 4,
                            32 => c += 5,
                            64 => c += 6,
                            128 => c += 7,
                            _ => {}
                        }

                        /* c contains the code unit value, in the range 0-255. In 8-bit UTF
                        mode, only values < 128 can be used. In all the other cases, c is a
                        character value. */

                        if utf != 0 && c > 127 {
                            break 'done;
                        }

                        if a < 0 {
                            a = c; /* First one found, save in a */
                        } else if b < 0
                        /* Second one found */
                        {
                            let mut d: c_int =
                                TABLE_GET!(c as u32, (*re).tables.add(fcc_offset), c) as c_int;

                            if utf != 0 || ucp != 0 {
                                if UCD_CASESET(c as u32) != 0 {
                                    break 'done;
                                } /* Multiple case set */
                                if c > 127 {
                                    d = UCD_OTHERCASE(c as u32) as c_int;
                                }
                            }

                            if d != a {
                                break 'done;
                            } /* Not the other case of a */
                            b = c; /* Save second in b */
                        } else {
                            break 'done;
                        } /* More than two characters found */
                    }
                    p = p.add(1);
                    i += 8;
                }

                /* Replace the start code unit bits with a first code unit. If it is the
                same as a required later code unit, then clear the required later code
                unit. This is because a search for a required code unit starts after an
                explicit first code unit, but at a code unit found from the bitmap.
                Patterns such as /a*a/ don't work if both the start unit and required
                unit are the same. */

                if a >= 0 {
                    if ((*re).flags & PCRE2_LASTSET) != 0
                        && ((*re).last_codeunit == a as u32
                            || (b >= 0 && (*re).last_codeunit == b as u32))
                    {
                        (*re).flags &= !(PCRE2_LASTSET | PCRE2_LASTCASELESS);
                        (*re).last_codeunit = 0;
                    }
                    (*re).first_codeunit = a as u32;
                    flags = PCRE2_FIRSTSET;
                    if b >= 0 {
                        flags |= PCRE2_FIRSTCASELESS;
                    }
                }
            }

            /* DONE: */
            (*re).flags |= flags;
        }
    }

    /* Find the minimum length of subject string. If the pattern can match an empty
    string, the minimum length is already known. If the pattern contains (*ACCEPT)
    all bets are off, and we don't even try to find a minimum length. If there are
    more back references than the size of the vector we are going to cache them in,
    do nothing. A pattern that complicated will probably take a long time to
    analyze and may in any case turn out to be too complicated. Note that back
    reference minima are held as 16-bit numbers. */

    if ((*re).flags & (PCRE2_MATCH_EMPTY | PCRE2_HASACCEPT)) == 0
        && (*re).top_backref as c_int <= MAX_CACHE_BACKREF as c_int
    {
        let min: c_int;
        let mut backref_cache: [c_int; 129] = [0; 129]; /* MAX_CACHE_BACKREF+1 */
        backref_cache[0] = 0; /* Highest one that is set */
        min = find_minlength(
            re,
            code as PCRE2_SPTR,
            code as PCRE2_SPTR,
            utf,
            std::ptr::null_mut(),
            &mut count,
            backref_cache.as_mut_ptr(),
        );
        match min {
            -1 =>
            /* \C in UTF mode or over-complex regex */
            {
                /* Leave minlength unchanged (will be zero) */
            }

            /* LCOV_EXCL_START */
            -2 => {
                /* PCRE2_DEBUG_UNREACHABLE(); */
                return 2; /* missing capturing bracket */
            }
            /* LCOV_EXCL_STOP */

            /* LCOV_EXCL_START */
            -3 => {
                /* PCRE2_DEBUG_UNREACHABLE(); */
                return 3; /* unrecognized opcode */
            }
            /* LCOV_EXCL_STOP */
            _ => {
                (*re).minlength = if min > u16::MAX as c_int {
                    u16::MAX as c_int as u16
                } else {
                    min as u16
                };
            }
        }
    }

    0
}

/* End of pcre2_study.c */
