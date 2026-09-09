//! png.c lines 1960-2759: IHDR validation, ASCII <-> floating point conversion.
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

/* <float.h> values used below. */
const DBL_DIG: c_uint = 15;
const DBL_MIN_10_EXP: c_int = -307;

/* `frexp(value, &exp)`: returns the mantissa in [0.5,1) and stores the base 2
 * exponent, such that value == mantissa * 2^exp.  frexp(0) == (0, 0).
 */
fn png_frexp(v: f64) -> (f64, c_int) {
    if v == 0.0 || !v.is_finite() {
        return (v, 0);
    }

    let bits = v.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as c_int;

    if exp == 0 {
        /* Subnormal: scale up by 2^64 then correct the exponent. */
        let scaled = v * 18446744073709551616.0f64; /* 2^64 */
        let sb = scaled.to_bits();
        let e = ((sb >> 52) & 0x7ff) as c_int;
        let m = f64::from_bits((sb & !(0x7ffu64 << 52)) | (1022u64 << 52));
        return (m, e - 1022 - 64);
    }

    let m = f64::from_bits((bits & !(0x7ffu64 << 52)) | (1022u64 << 52));
    (m, exp - 1022)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_check_IHDR(
    png_ptr: png_const_structrp,
    width: png_uint_32,
    height: png_uint_32,
    bit_depth: c_int,
    color_type: c_int,
    interlace_type: c_int,
    compression_type: c_int,
    filter_type: c_int,
) {
    let mut error: c_int = 0;

    /* Check for width and height valid values */
    if width == 0 {
        png_warning(png_ptr, c"Image width is zero in IHDR".as_ptr());
        error = 1;
    }

    if width > PNG_UINT_31_MAX {
        png_warning(png_ptr, c"Invalid image width in IHDR".as_ptr());
        error = 1;
    }

    /* The bit mask on the first line below must be at least as big as a
     * png_uint_32.  "~7U" is not adequate on 16-bit systems because it will
     * be an unsigned 16-bit value.  Casting to (png_alloc_size_t) makes the
     * type of the result at least as bit (in bits) as the RHS of the > operator
     * which also avoids a common warning on 64-bit systems that the comparison
     * of (png_uint_32) against the constant value on the RHS will always be
     * false.
     */
    if ((width.wrapping_add(7) as png_alloc_size_t) & !(7 as png_alloc_size_t))
        > (((PNG_SIZE_MAX
            - 48        /* big_row_buf hack */
            - 1)        /* filter byte */
            / 8)        /* 8-byte RGBA pixels */
            - 1)
    /* extra max_pixel_depth pad */
    {
        /* The size of the row must be within the limits of this architecture.
         * Because the read code can perform arbitrary transformations the
         * maximum size is checked here.  Because the code in png_read_start_row
         * adds extra space "for safety's sake" in several places a conservative
         * limit is used here.
         *
         * NOTE: it would be far better to check the size that is actually used,
         * but the effect in the real world is minor and the changes are more
         * extensive, therefore much more dangerous and much more difficult to
         * write in a way that avoids compiler warnings.
         */
        png_warning(
            png_ptr,
            c"Image width is too large for this architecture".as_ptr(),
        );
        error = 1;
    }

    if width > (*png_ptr).user_width_max {
        png_warning(png_ptr, c"Image width exceeds user limit in IHDR".as_ptr());
        error = 1;
    }

    if height == 0 {
        png_warning(png_ptr, c"Image height is zero in IHDR".as_ptr());
        error = 1;
    }

    if height > PNG_UINT_31_MAX {
        png_warning(png_ptr, c"Invalid image height in IHDR".as_ptr());
        error = 1;
    }

    if height > (*png_ptr).user_height_max {
        png_warning(png_ptr, c"Image height exceeds user limit in IHDR".as_ptr());
        error = 1;
    }

    /* Check other values */
    if bit_depth != 1 && bit_depth != 2 && bit_depth != 4 && bit_depth != 8 && bit_depth != 16 {
        png_warning(png_ptr, c"Invalid bit depth in IHDR".as_ptr());
        error = 1;
    }

    if color_type < 0 || color_type == 1 || color_type == 5 || color_type > 6 {
        png_warning(png_ptr, c"Invalid color type in IHDR".as_ptr());
        error = 1;
    }

    if ((color_type == PNG_COLOR_TYPE_PALETTE) && bit_depth > 8)
        || ((color_type == PNG_COLOR_TYPE_RGB
            || color_type == PNG_COLOR_TYPE_GRAY_ALPHA
            || color_type == PNG_COLOR_TYPE_RGB_ALPHA)
            && bit_depth < 8)
    {
        png_warning(
            png_ptr,
            c"Invalid color type/bit depth combination in IHDR".as_ptr(),
        );
        error = 1;
    }

    if interlace_type >= PNG_INTERLACE_LAST {
        png_warning(png_ptr, c"Unknown interlace method in IHDR".as_ptr());
        error = 1;
    }

    if compression_type != PNG_COMPRESSION_TYPE_BASE {
        png_warning(png_ptr, c"Unknown compression method in IHDR".as_ptr());
        error = 1;
    }

    /* Accept filter_method 64 (intrapixel differencing) only if
     * 1. Libpng was compiled with PNG_MNG_FEATURES_SUPPORTED and
     * 2. Libpng did not read a PNG signature (this filter_method is only
     *    used in PNG datastreams that are embedded in MNG datastreams) and
     * 3. The application called png_permit_mng_features with a mask that
     *    included PNG_FLAG_MNG_FILTER_64 and
     * 4. The filter_method is 64 and
     * 5. The color_type is RGB or RGBA
     */
    if ((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) != 0 && (*png_ptr).mng_features_permitted != 0 {
        png_warning(
            png_ptr,
            c"MNG features are not allowed in a PNG datastream".as_ptr(),
        );
    }

    if filter_type != PNG_FILTER_TYPE_BASE {
        if !(((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
            && (filter_type == PNG_INTRAPIXEL_DIFFERENCING)
            && (((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) == 0)
            && (color_type == PNG_COLOR_TYPE_RGB || color_type == PNG_COLOR_TYPE_RGB_ALPHA))
        {
            png_warning(png_ptr, c"Unknown filter method in IHDR".as_ptr());
            error = 1;
        }

        if ((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) != 0 {
            png_warning(png_ptr, c"Invalid filter method in IHDR".as_ptr());
            error = 1;
        }
    }

    if error == 1 {
        png_error(png_ptr, c"Invalid IHDR data".as_ptr());
    }
}

/* ASCII to fp functions */
/* Check an ASCII formatted floating point value, see the more detailed
 * comments in pngpriv.h
 */
/* The following is used internally to preserve the sticky flags:
 *   png_fp_add(state, flags) -> state |= flags
 *   png_fp_set(state, value) -> state = value | (state & PNG_FP_STICKY)
 * Both are expanded inline below.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_check_fp_number(
    string: png_const_charp,
    size: usize,
    statep: *mut c_int,
    whereami: *mut usize,
) -> c_int {
    let mut state: c_int = *statep;
    let mut i: usize = *whereami;

    'PNG_FP_End: while i < size {
        let type_: c_int;
        /* First find the type of the next character */
        match *string.add(i) as c_int {
            43 => type_ = PNG_FP_SAW_SIGN,
            45 => type_ = PNG_FP_SAW_SIGN + PNG_FP_NEGATIVE,
            46 => type_ = PNG_FP_SAW_DOT,
            48 => type_ = PNG_FP_SAW_DIGIT,
            49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 => {
                type_ = PNG_FP_SAW_DIGIT + PNG_FP_NONZERO
            }
            69 | 101 => type_ = PNG_FP_SAW_E,
            _ => break 'PNG_FP_End,
        }

        /* Now deal with this type according to the current
         * state, the type is arranged to not overlap the
         * bits of the PNG_FP_STATE.
         */
        let key: c_int = (state & PNG_FP_STATE) + (type_ & PNG_FP_SAW_ANY);

        if key == PNG_FP_INTEGER + PNG_FP_SAW_SIGN {
            if (state & PNG_FP_SAW_ANY) != 0 {
                break 'PNG_FP_End; /* not a part of the number */
            }

            state |= type_;
        } else if key == PNG_FP_INTEGER + PNG_FP_SAW_DOT {
            /* Ok as trailer, ok as lead of fraction. */
            if (state & PNG_FP_SAW_DOT) != 0
            /* two dots */
            {
                break 'PNG_FP_End;
            } else if (state & PNG_FP_SAW_DIGIT) != 0
            /* trailing dot? */
            {
                state |= type_;
            } else {
                state = (PNG_FP_FRACTION | type_) | (state & PNG_FP_STICKY);
            }
        } else if key == PNG_FP_INTEGER + PNG_FP_SAW_DIGIT {
            if (state & PNG_FP_SAW_DOT) != 0
            /* delayed fraction */
            {
                state = (PNG_FP_FRACTION | PNG_FP_SAW_DOT) | (state & PNG_FP_STICKY);
            }

            state |= type_ | PNG_FP_WAS_VALID;
        } else if key == PNG_FP_INTEGER + PNG_FP_SAW_E {
            if (state & PNG_FP_SAW_DIGIT) == 0 {
                break 'PNG_FP_End;
            }

            state = PNG_FP_EXPONENT | (state & PNG_FP_STICKY);
        }
        /* case PNG_FP_FRACTION + PNG_FP_SAW_SIGN:
              goto PNG_FP_End; ** no sign in fraction */
        /* case PNG_FP_FRACTION + PNG_FP_SAW_DOT:
              goto PNG_FP_End; ** Because SAW_DOT is always set */
        else if key == PNG_FP_FRACTION + PNG_FP_SAW_DIGIT {
            state |= type_ | PNG_FP_WAS_VALID;
        } else if key == PNG_FP_FRACTION + PNG_FP_SAW_E {
            /* This is correct because the trailing '.' on an
             * integer is handled above - so we can only get here
             * with the sequence ".E" (with no preceding digits).
             */
            if (state & PNG_FP_SAW_DIGIT) == 0 {
                break 'PNG_FP_End;
            }

            state = PNG_FP_EXPONENT | (state & PNG_FP_STICKY);
        } else if key == PNG_FP_EXPONENT + PNG_FP_SAW_SIGN {
            if (state & PNG_FP_SAW_ANY) != 0 {
                break 'PNG_FP_End; /* not a part of the number */
            }

            state |= PNG_FP_SAW_SIGN;
        }
        /* case PNG_FP_EXPONENT + PNG_FP_SAW_DOT:
              goto PNG_FP_End; */
        else if key == PNG_FP_EXPONENT + PNG_FP_SAW_DIGIT {
            state |= PNG_FP_SAW_DIGIT | PNG_FP_WAS_VALID;
        }
        /* case PNG_FP_EXPONEXT + PNG_FP_SAW_E:
              goto PNG_FP_End; */
        else {
            break 'PNG_FP_End; /* I.e. break 2 */
        }

        /* The character seems ok, continue. */
        i += 1;
    }

    /* PNG_FP_End: */
    /* Here at the end, update the state and return the correct
     * return code.
     */
    *statep = state;
    *whereami = i;

    ((state & PNG_FP_SAW_DIGIT) != 0) as c_int
}

/* The same but for a complete string. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_check_fp_string(string: png_const_charp, size: usize) -> c_int {
    let mut state: c_int = 0;
    let mut char_index: usize = 0;

    if png_check_fp_number(string, size, &mut state, &mut char_index) != 0
        && (char_index == size || *string.add(char_index) == 0)
    {
        return state; /* must be non-zero - see above */
    }

    0 /* i.e. fail */
}

/* Utility used below - a simple accurate power of ten from an integral
 * exponent.
 */
pub fn png_pow10(power: c_int) -> f64 {
    let mut recip: c_int = 0;
    let mut d: f64 = 1.0;
    let mut power: c_int = power;

    /* Handle negative exponent with a reciprocal at the end because
     * 10 is exact whereas .1 is inexact in base 2
     */
    if power < 0 {
        if power < DBL_MIN_10_EXP {
            return 0.0;
        }
        recip = 1;
        power = power.wrapping_neg();
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
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_ascii_from_fp(
    png_ptr: png_const_structrp,
    ascii: png_charp,
    size: usize,
    fp: f64,
    precision: c_uint,
) {
    let mut ascii = ascii;
    let mut size = size;
    let mut fp = fp;
    let mut precision = precision;

    /* We use standard functions from math.h, but not printf because
     * that would require stdio.  The caller must supply a buffer of
     * sufficient size or we will png_error.  The tests on size and
     * the space in ascii[] consumed are indicated below.
     */
    if precision < 1 {
        precision = DBL_DIG;
    }

    /* Enforce the limit of the implementation precision too. */
    if precision > DBL_DIG + 1 {
        precision = DBL_DIG + 1;
    }

    /* Basic sanity checks */
    if size >= (precision.wrapping_add(5)) as usize
    /* See the requirements below. */
    {
        if fp < 0.0 {
            fp = -fp;
            *ascii = 45; /* '-'  PLUS 1 TOTAL 1 */
            ascii = ascii.add(1);
            size = size.wrapping_sub(1);
        }

        if fp >= f64::MIN_POSITIVE && fp <= f64::MAX {
            let mut exp_b10: c_int; /* A base 10 exponent */
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
            exp_b10 = png_frexp(fp).1; /* exponent to base 2 */

            exp_b10 = (exp_b10.wrapping_mul(77)) >> 8; /* <= exponent to base 10 */

            /* Avoid underflow here. */
            base = png_pow10(exp_b10); /* May underflow */

            while base < f64::MIN_POSITIVE || base < fp {
                /* And this may overflow. */
                let test: f64 = png_pow10(exp_b10 + 1);

                if test <= f64::MAX {
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
                    let mut d: f64;

                    fp *= 10.0;
                    /* Use modf here, not floor and subtract, so that
                     * the separation is done in one step.  At the end
                     * of the loop don't break the number into parts so
                     * that the final digit is rounded.
                     */
                    if cdigits.wrapping_add(czero).wrapping_add(1)
                        < precision.wrapping_add(clead)
                    {
                        d = fp.trunc();
                        fp = fp - d;
                    } else {
                        d = (fp + 0.5).floor();

                        if d > 9.0 {
                            /* Rounding up to 10, handle that here. */
                            if czero > 0 {
                                czero = czero.wrapping_sub(1);
                                d = 1.0;
                                if cdigits == 0 {
                                    clead = clead.wrapping_sub(1);
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

                                    cdigits = cdigits.wrapping_sub(1);
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
                        czero = czero.wrapping_add(1);
                        if cdigits == 0 {
                            clead = clead.wrapping_add(1);
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
                            czero = czero.wrapping_sub(1);
                        }

                        if exp_b10 != -1 {
                            if exp_b10 == 0 {
                                *ascii = 46;
                                ascii = ascii.add(1);
                                size = size.wrapping_sub(1); /* counted above */
                            }

                            exp_b10 -= 1;
                        }
                        *ascii = (48 + (d as c_int)) as c_char;
                        ascii = ascii.add(1);
                        cdigits = cdigits.wrapping_add(1);
                    }

                    if !(cdigits.wrapping_add(czero) < precision.wrapping_add(clead)
                        && fp > f64::MIN_POSITIVE)
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
                        cdigits = cdigits.wrapping_add(1);
                        uexp_b10 /= 10;
                    }
                }

                /* Need another size check here for the exponent digits, so
                 * this need not be considered above.
                 */
                if size > cdigits as usize {
                    while cdigits > 0 {
                        cdigits = cdigits.wrapping_sub(1);
                        *ascii = exponent[cdigits as usize];
                        ascii = ascii.add(1);
                    }

                    *ascii = 0;

                    return;
                }
            }
        } else if !(fp >= f64::MIN_POSITIVE) {
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
    png_error(png_ptr, c"ASCII conversion buffer too small".as_ptr());
}

/* Function to format a fixed point value in ASCII.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_ascii_from_fixed(
    png_ptr: png_const_structrp,
    ascii: png_charp,
    size: usize,
    fp: png_fixed_point,
) {
    let mut ascii = ascii;

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
            let mut first: c_uint = 16 /* flag value */;
            let mut digits: [c_char; 10] = [0; 10];

            while num != 0 {
                /* Split the low digit off num: */
                let tmp: c_uint = (num / 10) as c_uint;
                num = num.wrapping_sub((tmp as png_uint_32).wrapping_mul(10));
                digits[ndigits as usize] = (48 + num) as c_char;
                ndigits = ndigits.wrapping_add(1);
                /* Record the first non-zero digit, note that this is a number
                 * starting at 1, it's not actually the array index.
                 */
                if first == 16 && num > 0 {
                    first = ndigits;
                }
                num = tmp as png_uint_32;
            }

            if ndigits > 0 {
                while ndigits > 5 {
                    ndigits = ndigits.wrapping_sub(1);
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
                        i = i.wrapping_sub(1);
                    }
                    while ndigits >= first {
                        ndigits = ndigits.wrapping_sub(1);
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
    png_error(png_ptr, c"ASCII conversion buffer too small".as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_fixed(
    png_ptr: png_const_structrp,
    fp: f64,
    text: png_const_charp,
) -> png_fixed_point {
    let r: f64 = (100000.0 * fp + 0.5).floor();

    if r > 2147483647. || r < -2147483648. {
        png_fixed_error(png_ptr, text);
    }

    r as png_fixed_point
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_fixed_ITU(
    png_ptr: png_const_structrp,
    fp: f64,
    text: png_const_charp,
) -> png_uint_32 {
    let r: f64 = (10000.0 * fp + 0.5).floor();

    if r > 2147483647. || r < 0.0 {
        png_fixed_error(png_ptr, text);
    }

    r as png_uint_32
}
