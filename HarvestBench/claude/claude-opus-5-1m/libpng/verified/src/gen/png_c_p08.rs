/* png.c lines 2930..3355 */

/* A local convenience routine. */
/* png_product2 */
/* NOTE: in C this function only exists when PNG_FLOATING_ARITHMETIC_SUPPORTED
 * is *not* defined, so the inner `#ifdef PNG_FLOATING_ARITHMETIC_SUPPORTED`
 * ("Should now be unused") branch is never the one compiled; the png_muldiv
 * branch below is the faithful translation.
 */
unsafe fn png_product2(a: png_fixed_point, b: png_fixed_point) -> png_fixed_point {
    /* The required result is a * b; the following preserves accuracy. */
    let mut res: png_fixed_point = 0;

    if png_muldiv(&mut res, a, b, 100000) != 0 {
        return res;
    }

    0 /* overflow */
}

/* png_reciprocal2 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reciprocal2(
    a: png_fixed_point,
    b: png_fixed_point,
) -> png_fixed_point {
    /* The required result is 1/a * 1/b; the following preserves accuracy. */
    if a != 0 && b != 0 {
        let mut r: f64 = 1E15 / (a as f64);
        r /= b as f64;
        r = floor(r + 0.5);

        if r <= 2147483647. && r >= -2147483648. {
            return r as png_fixed_point;
        }
    }

    0 /* overflow */
}

/* Fixed point gamma.
 *
 * The code to calculate the tables used below can be found in the shell script
 * contrib/tools/intgamma.sh
 *
 * To calculate gamma this code implements fast log() and exp() calls using only
 * fixed point arithmetic.  This code has sufficient precision for either 8-bit
 * or 16-bit sample values.
 *
 * The tables used here were calculated using simple 'bc' programs, but C double
 * precision floating point arithmetic would work fine.
 *
 * 8-bit log table
 *   This is a table of -log(value/255)/log(2) for 'value' in the range 128 to
 *   255, so it's the base 2 logarithm of a normalized 8-bit floating point
 *   mantissa.  The numbers are 32-bit fractions.
 */
/* `static const png_uint_32 png_8bit_l2[128]` lives in src/gen/png_c_tables.rs */

/* png_log8bit */
unsafe fn png_log8bit(mut x: c_uint) -> png_int_32 {
    let mut lg2: c_uint = 0;
    /* Each time 'x' is multiplied by 2, 1 must be subtracted off the final log,
     * because the log is actually negate that means adding 1.  The final
     * returned value thus has the range 0 (for 255 input) to 7.994 (for 1
     * input), return -1 for the overflow (log 0) case, - so the result is
     * always at most 19 bits.
     */
    x &= 0xff;
    if x == 0 {
        return -1;
    }

    if (x & 0xf0) == 0 {
        lg2 = 4;
        x <<= 4;
    }

    if (x & 0xc0) == 0 {
        lg2 += 2;
        x <<= 2;
    }

    if (x & 0x80) == 0 {
        lg2 += 1;
        x <<= 1;
    }

    /* result is at most 19 bits, so this cast is safe: */
    ((lg2 << 16).wrapping_add(
        (png_8bit_l2[(x - 128) as usize].wrapping_add(32768)) >> 16,
    )) as png_int_32
}

/* The above gives exact (to 16 binary places) log2 values for 8-bit images,
 * for 16-bit images we use the most significant 8 bits of the 16-bit value to
 * get an approximation then multiply the approximation by a correction factor
 * determined by the remaining up to 8 bits.  This requires an additional step
 * in the 16-bit case.
 *
 * We want log2(value/65535), we have log2(v'/255), where:
 *
 *    value = v' * 256 + v''
 *          = v' * f
 *
 * So f is value/v', which is equal to (256+v''/v') since v' is in the range 128
 * to 255 and v'' is in the range 0 to 255 f will be in the range 256 to less
 * than 258.  The final factor also needs to correct for the fact that our 8-bit
 * value is scaled by 255, whereas the 16-bit values must be scaled by 65535.
 *
 * This gives a final formula using a calculated value 'x' which is value/v' and
 * scaling by 65536 to match the above table:
 *
 *   log2(x/257) * 65536
 *
 * Since these numbers are so close to '1' we can use simple linear
 * interpolation between the two end values 256/257 (result -368.61) and 258/257
 * (result 367.179).  The values used below are scaled by a further 64 to give
 * 16-bit precision in the interpolation:
 *
 * Start (256): -23591
 * Zero  (257):      0
 * End   (258):  23499
 */
/* png_log16bit */
unsafe fn png_log16bit(mut x: png_uint_32) -> png_int_32 {
    let mut lg2: c_uint = 0;

    /* As above, but now the input has 16 bits. */
    x &= 0xffff;
    if x == 0 {
        return -1;
    }

    if (x & 0xff00) == 0 {
        lg2 = 8;
        x <<= 8;
    }

    if (x & 0xf000) == 0 {
        lg2 += 4;
        x <<= 4;
    }

    if (x & 0xc000) == 0 {
        lg2 += 2;
        x <<= 2;
    }

    if (x & 0x8000) == 0 {
        lg2 += 1;
        x <<= 1;
    }

    /* Calculate the base logarithm from the top 8 bits as a 28-bit fractional
     * value.
     */
    lg2 <<= 28;
    lg2 = lg2.wrapping_add((png_8bit_l2[((x >> 8) - 128) as usize].wrapping_add(8)) >> 4);

    /* Now we need to interpolate the factor, this requires a division by the top
     * 8 bits.  Do this with maximum precision.
     */
    x = ((x << 16).wrapping_add(x >> 9)) / (x >> 8);

    /* Since we divided by the top 8 bits of 'x' there will be a '1' at 1<<24,
     * the value at 1<<16 (ignoring this) will be 0 or 1; this gives us exactly
     * 16 bits to interpolate to get the low bits of the result.  Round the
     * answer.  Note that the end point values are scaled by 64 to retain overall
     * precision and that 'lg2' is current scaled by an extra 12 bits, so adjust
     * the overall scaling by 6-12.  Round at every step.
     */
    x = x.wrapping_sub(1u32 << 24);

    if x <= 65536u32
    /* <= '257' */
    {
        lg2 = lg2.wrapping_add(
            ((23591u32.wrapping_mul(65536u32.wrapping_sub(x))).wrapping_add(1u32 << (16 + 6 - 12 - 1)))
                >> (16 + 6 - 12),
        );
    } else {
        lg2 = lg2.wrapping_sub(
            ((23499u32.wrapping_mul(x.wrapping_sub(65536u32))).wrapping_add(1u32 << (16 + 6 - 12 - 1)))
                >> (16 + 6 - 12),
        );
    }

    /* Safe, because the result can't have more than 20 bits: */
    (lg2.wrapping_add(2048) >> 12) as png_int_32
}

/* The 'exp()' case must invert the above, taking a 20-bit fixed point
 * logarithmic value and returning a 16 or 8-bit number as appropriate.  In
 * each case only the low 16 bits are relevant - the fraction - since the
 * integer bits (the top 4) simply determine a shift.
 *
 * The worst case is the 16-bit distinction between 65535 and 65534. This
 * requires perhaps spurious accuracy in the decoding of the logarithm to
 * distinguish log2(65535/65534.5) - 10^-5 or 17 bits.  There is little chance
 * of getting this accuracy in practice.
 *
 * To deal with this the following exp() function works out the exponent of the
 * fractional part of the logarithm by using an accurate 32-bit value from the
 * top four fractional bits then multiplying in the remaining bits.
 */
/* `static const png_uint_32 png_32bit_exp[16]` lives in src/gen/png_c_tables.rs */

/* png_exp */
unsafe fn png_exp(x: png_fixed_point) -> png_uint_32 {
    if x > 0 && x <= 0xfffff
    /* Else overflow or zero (underflow) */
    {
        /* Obtain a 4-bit approximation */
        let mut e: png_uint_32 = png_32bit_exp[((x >> 12) & 0x0f) as usize];

        /* Incorporate the low 12 bits - these decrease the returned value by
         * multiplying by a number less than 1 if the bit is set.  The multiplier
         * is determined by the above table and the shift. Notice that the values
         * converge on 45426 and this is used to allow linear interpolation of the
         * low bits.
         */
        if (x & 0x800) != 0 {
            e = e.wrapping_sub((((e >> 16).wrapping_mul(44938u32)).wrapping_add(16u32)) >> 5);
        }

        if (x & 0x400) != 0 {
            e = e.wrapping_sub((((e >> 16).wrapping_mul(45181u32)).wrapping_add(32u32)) >> 6);
        }

        if (x & 0x200) != 0 {
            e = e.wrapping_sub((((e >> 16).wrapping_mul(45303u32)).wrapping_add(64u32)) >> 7);
        }

        if (x & 0x100) != 0 {
            e = e.wrapping_sub((((e >> 16).wrapping_mul(45365u32)).wrapping_add(128u32)) >> 8);
        }

        if (x & 0x080) != 0 {
            e = e.wrapping_sub((((e >> 16).wrapping_mul(45395u32)).wrapping_add(256u32)) >> 9);
        }

        if (x & 0x040) != 0 {
            e = e.wrapping_sub((((e >> 16).wrapping_mul(45410u32)).wrapping_add(512u32)) >> 10);
        }

        /* And handle the low 6 bits in a single block. */
        e = e.wrapping_sub(
            (((e >> 16)
                .wrapping_mul(355u32)
                .wrapping_mul((x as png_uint_32) & 0x3fu32))
            .wrapping_add(256u32))
                >> 9,
        );

        /* Handle the upper bits of x. */
        e >>= (x >> 16) as png_uint_32;
        return e;
    }

    /* Check for overflow */
    if x <= 0 {
        return png_32bit_exp[0];
    }

    /* Else underflow */
    0
}

/* png_exp8bit */
unsafe fn png_exp8bit(lg2: png_fixed_point) -> png_byte {
    /* Get a 32-bit value: */
    let mut x: png_uint_32 = png_exp(lg2);

    /* Convert the 32-bit value to 0..255 by multiplying by 256-1. Note that the
     * second, rounding, step can't overflow because of the first, subtraction,
     * step.
     */
    x = x.wrapping_sub(x >> 8);
    (((x.wrapping_add(0x7fffffu32)) >> 24) & 0xff) as png_byte
}

/* png_exp16bit */
unsafe fn png_exp16bit(lg2: png_fixed_point) -> png_uint_16 {
    /* Get a 32-bit value: */
    let mut x: png_uint_32 = png_exp(lg2);

    /* Convert the 32-bit value to 0..65535 by multiplying by 65536-1: */
    x = x.wrapping_sub(x >> 16);
    ((x.wrapping_add(32767u32)) >> 16) as png_uint_16
}

/* png_gamma_8bit_correct */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_8bit_correct(
    value: c_uint,
    gamma_val: png_fixed_point,
) -> png_byte {
    if value > 0 && value < 255 {
        /* 'value' is unsigned, ANSI-C90 requires the compiler to correctly
         * convert this to a floating point value.  This includes values that
         * would overflow if 'value' were to be converted to 'int'.
         *
         * Apparently GCC, however, does an intermediate conversion to (int)
         * on some (ARM) but not all (x86) platforms, possibly because of
         * hardware FP limitations.  (E.g. if the hardware conversion always
         * assumes the integer register contains a signed value.)  This results
         * in ANSI-C undefined behavior for large values.
         *
         * Other implementations on the same machine might actually be ANSI-C90
         * conformant and therefore compile spurious extra code for the large
         * values.
         *
         * We can be reasonably sure that an unsigned to float conversion
         * won't be faster than an int to float one.  Therefore this code
         * assumes responsibility for the undefined behavior, which it knows
         * can't happen because of the check above.
         *
         * Note the argument to this routine is an (unsigned int) because, on
         * 16-bit platforms, it is assigned a value which might be out of
         * range for an (int); that would result in undefined behavior in the
         * caller if the *argument* ('value') were to be declared (int).
         */
        let r: f64 = floor(
            255.0
                * pow(
                    (value as c_int) as f64 / 255.,
                    (gamma_val as f64) * 0.00001,
                )
                + 0.5,
        );
        return r as png_byte;
    }

    (value & 0xff) as png_byte
}

/* png_gamma_16bit_correct */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_16bit_correct(
    value: c_uint,
    gamma_val: png_fixed_point,
) -> png_uint_16 {
    if value > 0 && value < 65535 {
        /* The same (unsigned int)->(double) constraints apply here as above,
         * however in this case the (unsigned int) to (int) conversion can
         * overflow on an ANSI-C90 compliant system so the cast needs to ensure
         * that this is not possible.
         */
        let r: f64 = floor(
            65535.0
                * pow(
                    (value as png_int_32) as f64 / 65535.,
                    (gamma_val as f64) * 0.00001,
                )
                + 0.5,
        );
        return r as png_uint_16;
    }

    value as png_uint_16
}
