/* png.c lines 2280..2725 */

/* <float.h>: DBL_MIN_10_EXP for an IEEE 754 double. */
const DBL_MIN_10_EXP: c_int = -307;

/* Utility used below - a simple accurate power of ten from an integral
 * exponent.
 */
/* png_pow10 */
unsafe fn png_pow10(mut power: c_int) -> f64 {
    let mut recip: c_int = 0;
    let mut d: f64 = 1.0;

    /* Handle negative exponent with a reciprocal at the end because
     * 10 is exact whereas .1 is inexact in base 2
     */
    if power < 0 {
        if power < DBL_MIN_10_EXP {
            return 0.0;
        }
        recip = 1;
        power = -power;
    }

    if power > 0 {
        /* Decompose power bitwise. */
        let mut mult: f64 = 10.0;
        loop {
            if (power & 1) != 0 {
                d *= mult;
            }
            mult *= mult;
            power >>= 1;

            if !(power > 0) {
                break;
            }
        }

        if recip != 0 {
            d = 1.0 / d;
        }
    }
    /* else power is 0 and d is 1 */

    d
}

/* Function to format a floating point value in ASCII with a given
 * precision.
 */
/* png_ascii_from_fp */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_ascii_from_fp(
    png_ptr: png_const_structrp,
    mut ascii: png_charp,
    mut size: usize,
    mut fp: f64,
    mut precision: c_uint,
) {
    /* We use standard functions from math.h, but not printf because
     * that would require stdio.  The caller must supply a buffer of
     * sufficient size or we will png_error.  The tests on size and
     * the space in ascii[] consumed are indicated below.
     */
    if precision < 1 {
        precision = DBL_DIG as c_uint;
    }

    /* Enforce the limit of the implementation precision too. */
    if precision > (DBL_DIG + 1) as c_uint {
        precision = (DBL_DIG + 1) as c_uint;
    }

    /* Basic sanity checks */
    if size >= precision.wrapping_add(5) as usize
    /* See the requirements below. */
    {
        if fp < 0.0 {
            fp = -fp;
            *ascii = 45; /* '-'  PLUS 1 TOTAL 1 */
            ascii = ascii.add(1);
            size = size.wrapping_sub(1);
        }

        if fp >= DBL_MIN && fp <= DBL_MAX {
            let mut exp_b10: c_int = 0; /* A base 10 exponent */
            let mut base: f64; /* 10^exp_b10 */

            /* First extract a base 10 exponent of the number,
             * the calculation below rounds down when converting
             * from base 2 to base 10 (multiply by log10(2) -
             * 0.3010, but 77/256 is 0.3008, so exp_b10 needs to
             * be increased.  Note that the arithmetic shift
             * performs a floor() unlike C arithmetic - using a
             * C multiply would break the following for negative
             * exponents.
             */
            frexp(fp, &mut exp_b10); /* exponent to base 2 */

            exp_b10 = (exp_b10 * 77) >> 8; /* <= exponent to base 10 */

            /* Avoid underflow here. */
            base = png_pow10(exp_b10); /* May underflow */

            while base < DBL_MIN || base < fp {
                /* And this may overflow. */
                let test: f64 = png_pow10(exp_b10 + 1);

                if test <= DBL_MAX {
                    exp_b10 += 1;
                    base = test;
                } else {
                    break;
                }
            }

            /* Normalize fp and correct exp_b10, after this fp is in the
             * range [.1,1) and exp_b10 is both the exponent and the digit
             * *before* which the decimal point should be inserted
             * (starting with 0 for the first digit).  Note that this
             * works even if 10^exp_b10 is out of range because of the
             * test on DBL_MAX above.
             */
            fp /= base;
            while fp >= 1.0 {
                fp /= 10.0;
                exp_b10 += 1;
            }

            /* Because of the code above fp may, at this point, be
             * less than .1, this is ok because the code below can
             * handle the leading zeros this generates, so no attempt
             * is made to correct that here.
             */

            {
                let mut czero: c_uint;
                let mut clead: c_uint;
                let mut cdigits: c_uint;
                let mut exponent: [c_char; 10] = [0; 10];

                /* Allow up to two leading zeros - this will not lengthen
                 * the number compared to using E-n.
                 */
                if exp_b10 < 0 && exp_b10 > -3
                /* PLUS 3 TOTAL 4 */
                {
                    czero = (0 as c_uint).wrapping_sub(exp_b10 as c_uint); /* PLUS 2 digits: TOTAL 3 */
                    exp_b10 = 0; /* Dot added below before first output. */
                } else {
                    czero = 0; /* No zeros to add */
                }

                /* Generate the digit list, stripping trailing zeros and
                 * inserting a '.' before a digit if the exponent is 0.
                 */
                clead = czero; /* Count of leading zeros */
                cdigits = 0; /* Count of digits in list. */

                loop {
                    let mut d: f64 = 0.0;

                    fp *= 10.0;
                    /* Use modf here, not floor and subtract, so that
                     * the separation is done in one step.  At the end
                     * of the loop don't break the number into parts so
                     * that the final digit is rounded.
                     */
                    if cdigits.wrapping_add(czero).wrapping_add(1)
                        < precision.wrapping_add(clead)
                    {
                        fp = modf(fp, &mut d);
                    } else {
                        d = floor(fp + 0.5);

                        if d > 9.0 {
                            /* Rounding up to 10, handle that here. */
                            if czero > 0 {
                                czero -= 1;
                                d = 1.0;
                                if cdigits == 0 {
                                    clead -= 1;
                                }
                            } else {
                                while cdigits > 0 && d > 9.0 {
                                    ascii = ascii.sub(1);
                                    let mut ch: c_int = *ascii as c_int;

                                    if exp_b10 != -1 {
                                        exp_b10 += 1;
                                    } else if ch == 46 {
                                        ascii = ascii.sub(1);
                                        ch = *ascii as c_int;
                                        size = size.wrapping_add(1);
                                        /* Advance exp_b10 to '1', so that the
                                         * decimal point happens after the
                                         * previous digit.
                                         */
                                        exp_b10 = 1;
                                    }

                                    cdigits -= 1;
                                    d = (ch - 47) as f64; /* I.e. 1+(ch-48) */
                                }

                                /* Did we reach the beginning? If so adjust the
                                 * exponent but take into account the leading
                                 * decimal point.
                                 */
                                if d > 9.0
                                /* cdigits == 0 */
                                {
                                    if exp_b10 == -1 {
                                        /* Leading decimal point (plus zeros?), if
                                         * we lose the decimal point here it must
                                         * be reentered below.
                                         */
                                        ascii = ascii.sub(1);
                                        let ch: c_int = *ascii as c_int;

                                        if ch == 46 {
                                            size = size.wrapping_add(1);
                                            exp_b10 = 1;
                                        }

                                        /* Else lost a leading zero, so 'exp_b10' is
                                         * still ok at (-1)
                                         */
                                    } else {
                                        exp_b10 += 1;
                                    }

                                    /* In all cases we output a '1' */
                                    d = 1.0;
                                }
                            }
                        }
                        fp = 0.0; /* Guarantees termination below. */
                    }

                    if d == 0.0 {
                        czero += 1;
                        if cdigits == 0 {
                            clead += 1;
                        }
                    } else {
                        /* Included embedded zeros in the digit count. */
                        cdigits = cdigits.wrapping_add(czero.wrapping_sub(clead));
                        clead = 0;

                        while czero > 0 {
                            /* exp_b10 == (-1) means we just output the decimal
                             * place - after the DP don't adjust 'exp_b10' any
                             * more!
                             */
                            if exp_b10 != -1 {
                                if exp_b10 == 0 {
                                    *ascii = 46;
                                    ascii = ascii.add(1);
                                    size = size.wrapping_sub(1);
                                }
                                /* PLUS 1: TOTAL 4 */
                                exp_b10 -= 1;
                            }
                            *ascii = 48;
                            ascii = ascii.add(1);
                            czero -= 1;
                        }

                        if exp_b10 != -1 {
                            if exp_b10 == 0 {
                                *ascii = 46;
                                ascii = ascii.add(1);
                                size = size.wrapping_sub(1); /* counted above */
                            }

                            exp_b10 -= 1;
                        }
                        *ascii = (48 + d as c_int) as c_char;
                        ascii = ascii.add(1);
                        cdigits += 1;
                    }

                    if !(cdigits.wrapping_add(czero) < precision.wrapping_add(clead)
                        && fp > DBL_MIN)
                    {
                        break;
                    }
                }

                /* The total output count (max) is now 4+precision */

                /* Check for an exponent, if we don't need one we are
                 * done and just need to terminate the string.  At this
                 * point, exp_b10==(-1) is effectively a flag: it got
                 * to '-1' because of the decrement, after outputting
                 * the decimal point above. (The exponent required is
                 * *not* -1.)
                 */
                if exp_b10 >= -1 && exp_b10 <= 2 {
                    /* The following only happens if we didn't output the
                     * leading zeros above for negative exponent, so this
                     * doesn't add to the digit requirement.  Note that the
                     * two zeros here can only be output if the two leading
                     * zeros were *not* output, so this doesn't increase
                     * the output count.
                     */
                    loop {
                        let t: c_int = exp_b10;
                        exp_b10 -= 1;
                        if !(t > 0) {
                            break;
                        }
                        *ascii = 48;
                        ascii = ascii.add(1);
                    }

                    *ascii = 0;

                    /* Total buffer requirement (including the '\0') is
                     * 5+precision - see check at the start.
                     */
                    return;
                }

                /* Here if an exponent is required, adjust size for
                 * the digits we output but did not count.  The total
                 * digit output here so far is at most 1+precision - no
                 * decimal point and no leading or trailing zeros have
                 * been output.
                 */
                size = size.wrapping_sub(cdigits as usize);

                *ascii = 69; /* 'E': PLUS 1 TOTAL 2+precision */
                ascii = ascii.add(1);
                size = size.wrapping_sub(1);

                /* The following use of an unsigned temporary avoids ambiguities in
                 * the signed arithmetic on exp_b10 and permits GCC at least to do
                 * better optimization.
                 */
                {
                    let mut uexp_b10: c_uint;

                    if exp_b10 < 0 {
                        *ascii = 45; /* '-': PLUS 1 TOTAL 3+precision */
                        ascii = ascii.add(1);
                        size = size.wrapping_sub(1);
                        uexp_b10 = (0 as c_uint).wrapping_sub(exp_b10 as c_uint);
                    } else {
                        uexp_b10 = (0 as c_uint).wrapping_add(exp_b10 as c_uint);
                    }

                    cdigits = 0;

                    while uexp_b10 > 0 {
                        exponent[cdigits as usize] = (48 + uexp_b10 % 10) as c_char;
                        cdigits += 1;
                        uexp_b10 /= 10;
                    }
                }

                /* Need another size check here for the exponent digits, so
                 * this need not be considered above.
                 */
                if size > cdigits as usize {
                    while cdigits > 0 {
                        cdigits -= 1;
                        *ascii = exponent[cdigits as usize];
                        ascii = ascii.add(1);
                    }

                    *ascii = 0;

                    return;
                }
            }
        } else if !(fp >= DBL_MIN) {
            *ascii = 48; /* '0' */
            ascii = ascii.add(1);
            *ascii = 0;
            return;
        } else {
            *ascii = 105; /* 'i' */
            ascii = ascii.add(1);
            *ascii = 110; /* 'n' */
            ascii = ascii.add(1);
            *ascii = 102; /* 'f' */
            ascii = ascii.add(1);
            *ascii = 0;
            return;
        }
    }

    /* Here on buffer too small. */
    png_error(
        png_ptr,
        b"ASCII conversion buffer too small\0".as_ptr() as png_const_charp,
    );
}

/* Function to format a fixed point value in ASCII.
 */
/* png_ascii_from_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_ascii_from_fixed(
    png_ptr: png_const_structrp,
    mut ascii: png_charp,
    size: usize,
    fp: png_fixed_point,
) {
    /* Require space for 10 decimal digits, a decimal point, a minus sign and a
     * trailing \0, 13 characters:
     */
    if size > 12 {
        let mut num: png_uint_32;

        /* Avoid overflow here on the minimum integer. */
        if fp < 0 {
            *ascii = 45;
            ascii = ascii.add(1);
            num = fp.wrapping_neg() as png_uint_32;
        } else {
            num = fp as png_uint_32;
        }

        if num <= 0x80000000
        /* else overflowed */
        {
            let mut ndigits: c_uint = 0;
            let mut first: c_uint = 16; /* flag value */
            let mut digits: [c_char; 10] = [0; 10];

            while num != 0 {
                /* Split the low digit off num: */
                let tmp: c_uint = num / 10;
                num = num.wrapping_sub(tmp.wrapping_mul(10));
                digits[ndigits as usize] = (48 + num) as c_char;
                ndigits += 1;
                /* Record the first non-zero digit, note that this is a number
                 * starting at 1, it's not actually the array index.
                 */
                if first == 16 && num > 0 {
                    first = ndigits;
                }
                num = tmp;
            }

            if ndigits > 0 {
                while ndigits > 5 {
                    ndigits -= 1;
                    *ascii = digits[ndigits as usize];
                    ascii = ascii.add(1);
                }
                /* The remaining digits are fractional digits, ndigits is '5' or
                 * smaller at this point.  It is certainly not zero.  Check for a
                 * non-zero fractional digit:
                 */
                if first <= 5 {
                    let mut i: c_uint;
                    *ascii = 46; /* decimal point */
                    ascii = ascii.add(1);
                    /* ndigits may be <5 for small numbers, output leading zeros
                     * then ndigits digits to first:
                     */
                    i = 5;
                    while ndigits < i {
                        *ascii = 48;
                        ascii = ascii.add(1);
                        i -= 1;
                    }
                    while ndigits >= first {
                        ndigits -= 1;
                        *ascii = digits[ndigits as usize];
                        ascii = ascii.add(1);
                    }
                    /* Don't output the trailing zeros! */
                }
            } else {
                *ascii = 48;
                ascii = ascii.add(1);
            }

            /* And null terminate the string: */
            *ascii = 0;
            return;
        }
    }

    /* Here on buffer too small. */
    png_error(
        png_ptr,
        b"ASCII conversion buffer too small\0".as_ptr() as png_const_charp,
    );
}
