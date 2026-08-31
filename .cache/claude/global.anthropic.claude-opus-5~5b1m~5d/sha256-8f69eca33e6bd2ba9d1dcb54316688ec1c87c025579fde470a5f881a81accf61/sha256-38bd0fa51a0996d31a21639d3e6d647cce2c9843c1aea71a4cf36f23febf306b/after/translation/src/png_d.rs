//! png.c lines 2760-4044: muldiv/reciprocal, fixed point log/exp, gamma
//! correction and gamma table construction, png_set_option and the
//! png_image_free / png_image_error simplified API helpers.
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

/* muldiv functions */
/* This API takes signed arguments and rounds the result to the nearest
 * integer (or, for a fixed point number - the standard argument - to
 * the nearest .00001).  Overflow and divide by zero are signalled in
 * the result, a boolean - true on success, false on overflow.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_muldiv(
    res: png_fixed_point_p,
    a: png_fixed_point,
    times: png_int_32,
    divisor: png_int_32,
) -> c_int {
    /* Return a * times / divisor, rounded. */
    if divisor != 0 {
        if a == 0 || times == 0 {
            *res = 0;
            return 1;
        } else {
            let mut r: f64 = a as f64;
            r *= times as f64;
            r /= divisor as f64;
            r = (r + 0.5).floor();

            /* A png_fixed_point is a 32-bit integer. */
            if r <= 2147483647. && r >= -2147483648. {
                *res = r as png_fixed_point;
                return 1;
            }
        }
    }

    0
}

/* Calculate a reciprocal, return 0 on div-by-zero or overflow. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_reciprocal(a: png_fixed_point) -> png_fixed_point {
    let r: f64 = (1E10 / (a as f64) + 0.5).floor();

    if r <= 2147483647. && r >= -2147483648. {
        return r as png_fixed_point;
    }

    0 /* error/overflow */
}

/* This is the shared test on whether a gamma value is 'significant' - whether
 * it is worth doing gamma correction.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_gamma_significant(gamma_val: png_fixed_point) -> c_int {
    /* sRGB:       1/2.2 == 0.4545(45)
     * AdobeRGB:   1/(2+51/256) ~= 0.45471 5dp
     *
     * So the correction from AdobeRGB to sRGB (output) is:
     *
     *    2.2/(2+51/256) == 1.00035524
     *
     * I.e. vanishingly small (<4E-4) but still detectable in 16-bit linear (+/-
     * 23).  Note that the Adobe choice seems to be something intended to give an
     * exact number with 8 binary fractional digits - it is the closest to 2.2
     * that is possible a base 2 .8p representation.
     */
    (gamma_val < PNG_FP_1 - PNG_GAMMA_THRESHOLD_FIXED
        || gamma_val > PNG_FP_1 + PNG_GAMMA_THRESHOLD_FIXED) as c_int
}

/* A local convenience routine. */
pub unsafe fn png_product2(a: png_fixed_point, b: png_fixed_point) -> png_fixed_point {
    /* The required result is a * b; the following preserves accuracy. */
    let mut res: png_fixed_point = 0;

    if png_muldiv(&mut res, a, b, 100000) != 0 {
        return res;
    }

    0 /* overflow */
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_reciprocal2(
    a: png_fixed_point,
    b: png_fixed_point,
) -> png_fixed_point {
    /* The required result is 1/a * 1/b; the following preserves accuracy. */
    if a != 0 && b != 0 {
        let mut r: f64 = 1E15 / (a as f64);
        r /= b as f64;
        r = (r + 0.5).floor();

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
pub static png_8bit_l2: [png_uint_32; 128] = [
    4270715492, 4222494797, 4174646467, 4127164793, 4080044201, 4033279239,
    3986864580, 3940795015, 3895065449, 3849670902, 3804606499, 3759867474,
    3715449162, 3671346997, 3627556511, 3584073329, 3540893168, 3498011834,
    3455425220, 3413129301, 3371120137, 3329393864, 3287946700, 3246774933,
    3205874930, 3165243125, 3124876025, 3084770202, 3044922296, 3005329011,
    2965987113, 2926893432, 2888044853, 2849438323, 2811070844, 2772939474,
    2735041326, 2697373562, 2659933400, 2622718104, 2585724991, 2548951424,
    2512394810, 2476052606, 2439922311, 2404001468, 2368287663, 2332778523,
    2297471715, 2262364947, 2227455964, 2192742551, 2158222529, 2123893754,
    2089754119, 2055801552, 2022034013, 1988449497, 1955046031, 1921821672,
    1888774511, 1855902668, 1823204291, 1790677560, 1758320682, 1726131893,
    1694109454, 1662251657, 1630556815, 1599023271, 1567649391, 1536433567,
    1505374214, 1474469770, 1443718700, 1413119487, 1382670639, 1352370686,
    1322218179, 1292211689, 1262349810, 1232631153, 1203054352, 1173618059,
    1144320946, 1115161701, 1086139034, 1057251672, 1028498358, 999877854,
    971388940, 943030410, 914801076, 886699767, 858725327, 830876614,
    803152505, 775551890, 748073672, 720716771, 693480120, 666362667,
    639363374, 612481215, 585715177, 559064263, 532527486, 506103872,
    479792461, 453592303, 427502463, 401522014, 375650043, 349885648,
    324227938, 298676034, 273229066, 247886176, 222646516, 197509248,
    172473545, 147538590, 122703574, 97967701, 73330182, 48790236,
    24347096, 0,
];

pub unsafe fn png_log8bit(x_in: c_uint) -> png_int_32 {
    let mut x: c_uint = x_in;
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
    ((lg2 << 16).wrapping_add((png_8bit_l2[(x - 128) as usize].wrapping_add(32768)) >> 16))
        as png_int_32
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
pub unsafe fn png_log16bit(x_in: png_uint_32) -> png_int_32 {
    let mut x: png_uint_32 = x_in;
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

    if x <= 65536u32 {
        /* <= '257' */
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
    ((lg2.wrapping_add(2048)) >> 12) as png_int_32
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
pub static png_32bit_exp: [png_uint_32; 16] = [
    /* NOTE: the first entry is deliberately set to the maximum 32-bit value. */
    4294967295, 4112874773, 3938502376, 3771522796, 3611622603, 3458501653,
    3311872529, 3171459999, 3037000500, 2908241642, 2784941738, 2666869345,
    2553802834, 2445529972, 2341847524, 2242560872,
];

/* Adjustment table; provided to explain the numbers in the code below. */
/* (see the #if 0 block in png.c:
 * for (i=11;i>=0;--i){ print i, " ", (1 - e(-(2^i)/65536*l(2))) * 2^(32-i), "\n"}
 *    11 44937.64284865548751208448
 *    10 45180.98734845585101160448
 *     9 45303.31936980687359311872
 *     8 45364.65110595323018870784
 *     7 45395.35850361789624614912
 *     6 45410.72259715102037508096
 *     5 45418.40724413220722311168
 *     4 45422.25021786898173001728
 *     3 45424.17186732298419044352
 *     2 45425.13273269940811464704
 *     1 45425.61317555035558641664
 *     0 45425.85339951654943850496
 */

pub unsafe fn png_exp(x: png_fixed_point) -> png_uint_32 {
    if x > 0 && x <= 0xfffff {
        /* Else overflow or zero (underflow) */
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

pub unsafe fn png_exp8bit(lg2: png_fixed_point) -> png_byte {
    /* Get a 32-bit value: */
    let mut x: png_uint_32 = png_exp(lg2);

    /* Convert the 32-bit value to 0..255 by multiplying by 256-1. Note that the
     * second, rounding, step can't overflow because of the first, subtraction,
     * step.
     */
    x = x.wrapping_sub(x >> 8);
    ((x.wrapping_add(0x7fffff) >> 24) & 0xff) as png_byte
}

pub unsafe fn png_exp16bit(lg2: png_fixed_point) -> png_uint_16 {
    /* Get a 32-bit value: */
    let mut x: png_uint_32 = png_exp(lg2);

    /* Convert the 32-bit value to 0..65535 by multiplying by 65536-1: */
    x = x.wrapping_sub(x >> 16);
    (x.wrapping_add(32767) >> 16) as png_uint_16
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_gamma_8bit_correct(
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
        let r: f64 = (255.0
            * (((value as c_int) as f64) / 255.).powf((gamma_val as f64) * 0.00001)
            + 0.5)
            .floor();
        return r as png_byte;
    }

    (value & 0xff) as png_byte
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_gamma_16bit_correct(
    value: c_uint,
    gamma_val: png_fixed_point,
) -> png_uint_16 {
    if value > 0 && value < 65535 {
        /* The same (unsigned int)->(double) constraints apply here as above,
         * however in this case the (unsigned int) to (int) conversion can
         * overflow on an ANSI-C90 compliant system so the cast needs to ensure
         * that this is not possible.
         */
        let r: f64 = (65535.0
            * (((value as png_int_32) as f64) / 65535.).powf((gamma_val as f64) * 0.00001)
            + 0.5)
            .floor();
        return r as png_uint_16;
    }

    value as png_uint_16
}

/* This does the right thing based on the bit_depth field of the
 * png_struct, interpreting values as 8-bit or 16-bit.  While the result
 * is nominally a 16-bit value if bit depth is 8 then the result is
 * 8-bit (as are the arguments.)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_gamma_correct(
    png_ptr: png_structrp,
    value: c_uint,
    gamma_val: png_fixed_point,
) -> png_uint_16 {
    if (*png_ptr).bit_depth == 8 {
        return png_gamma_8bit_correct(value, gamma_val) as png_uint_16;
    } else {
        return png_gamma_16bit_correct(value, gamma_val);
    }
}

/* Internal function to build a single 16-bit table - the table consists of
 * 'num' 256 entry subtables, where 'num' is determined by 'shift' - the amount
 * to shift the input values right (or 16-number_of_signifiant_bits).
 *
 * The caller is responsible for ensuring that the table gets cleaned up on
 * png_error (i.e. if one of the mallocs below fails) - i.e. the *table argument
 * should be somewhere that will be cleaned.
 */
pub unsafe fn png_build_16bit_table(
    png_ptr: png_structrp,
    ptable: *mut png_uint_16pp,
    shift: c_uint,
    gamma_val: png_fixed_point,
) {
    /* Various values derived from 'shift': */
    let num: c_uint = 1u32 << (8u32 - shift);
    /* CSE the division and work round wacky GCC warnings (see the comments
     * in png_gamma_8bit_correct for where these come from.)
     */
    let fmax: f64 = 1.0 / ((((1 as png_int_32) << (16u32 - shift)) - 1) as f64);
    let max: c_uint = (1u32 << (16u32 - shift)) - 1u32;
    let max_by_2: c_uint = 1u32 << (15u32 - shift);
    let mut i: c_uint;

    let table: png_uint_16pp =
        png_calloc(png_ptr, (num as usize) * core::mem::size_of::<png_uint_16p>())
            as png_uint_16pp;
    *ptable = table;

    i = 0;
    while i < num {
        let sub_table: png_uint_16p =
            png_malloc(png_ptr, 256 * core::mem::size_of::<png_uint_16>()) as png_uint_16p;
        *table.add(i as usize) = sub_table;

        /* The 'threshold' test is repeated here because it can arise for one of
         * the 16-bit tables even if the others don't hit it.
         */
        if png_gamma_significant(gamma_val) != 0 {
            /* The old code would overflow at the end and this would cause the
             * 'pow' function to return a result >1, resulting in an
             * arithmetic error.  This code follows the spec exactly; ig is
             * the recovered input sample, it always has 8-16 bits.
             *
             * We want input * 65535/max, rounded, the arithmetic fits in 32
             * bits (unsigned) so long as max <= 32767.
             */
            let mut j: c_uint = 0;
            while j < 256 {
                let ig: png_uint_32 = (j << (8u32 - shift)).wrapping_add(i);
                /* Inline the 'max' scaling operation: */
                /* See png_gamma_8bit_correct for why the cast to (int) is
                 * required here.
                 */
                let d: f64 = (65535.
                    * ((ig as f64) * fmax).powf((gamma_val as f64) * 0.00001)
                    + 0.5)
                    .floor();
                *sub_table.add(j as usize) = d as png_uint_16;
                j += 1;
            }
        } else {
            /* We must still build a table, but do it the fast way. */
            let mut j: c_uint = 0;

            while j < 256 {
                let mut ig: png_uint_32 = (j << (8u32 - shift)).wrapping_add(i);

                if shift != 0 {
                    ig = (ig.wrapping_mul(65535u32).wrapping_add(max_by_2)) / max;
                }

                *sub_table.add(j as usize) = ig as png_uint_16;
                j += 1;
            }
        }

        i += 1;
    }
}

/* NOTE: this function expects the *inverse* of the overall gamma transformation
 * required.
 */
pub unsafe fn png_build_16to8_table(
    png_ptr: png_structrp,
    ptable: *mut png_uint_16pp,
    shift: c_uint,
    gamma_val: png_fixed_point,
) {
    let num: c_uint = 1u32 << (8u32 - shift);
    let max: c_uint = (1u32 << (16u32 - shift)) - 1u32;
    let mut i: c_uint;
    let mut last: png_uint_32;

    let table: png_uint_16pp =
        png_calloc(png_ptr, (num as usize) * core::mem::size_of::<png_uint_16p>())
            as png_uint_16pp;
    *ptable = table;

    /* 'num' is the number of tables and also the number of low bits of low
     * bits of the input 16-bit value used to select a table.  Each table is
     * itself indexed by the high 8 bits of the value.
     */
    i = 0;
    while i < num {
        *table.add(i as usize) =
            png_malloc(png_ptr, 256 * core::mem::size_of::<png_uint_16>()) as png_uint_16p;
        i += 1;
    }

    /* 'gamma_val' is set to the reciprocal of the value calculated above, so
     * pow(out,g) is an *input* value.  'last' is the last input value set.
     *
     * In the loop 'i' is used to find output values.  Since the output is
     * 8-bit there are only 256 possible values.  The tables are set up to
     * select the closest possible output value for each input by finding
     * the input value at the boundary between each pair of output values
     * and filling the table up to that boundary with the lower output
     * value.
     *
     * The boundary values are 0.5,1.5..253.5,254.5.  Since these are 9-bit
     * values the code below uses a 16-bit value in i; the values start at
     * 128.5 (for 0.5) and step by 257, for a total of 254 values (the last
     * entries are filled with 255).  Start i at 128 and fill all 'last'
     * table entries <= 'max'
     */
    last = 0;
    i = 0;
    while i < 255 {
        /* 8-bit output value */
        /* Find the corresponding maximum input value */
        let out: png_uint_16 = i.wrapping_mul(257u32) as png_uint_16; /* 16-bit output value */

        /* Find the boundary value in 16 bits: */
        let mut bound: png_uint_32 =
            png_gamma_16bit_correct((out as c_uint).wrapping_add(128u32), gamma_val)
                as png_uint_32;

        /* Adjust (round) to (16-shift) bits: */
        bound = (bound.wrapping_mul(max).wrapping_add(32768u32)) / 65535u32 + 1u32;

        while last < bound {
            *(*table.add((last & (0xffu32 >> shift)) as usize))
                .add((last >> (8u32 - shift)) as usize) = out;
            last += 1;
        }

        i += 1;
    }

    /* And fill in the final entries. */
    while last < (num << 8) {
        *(*table.add((last & (0xffu32 >> shift)) as usize))
            .add((last >> (8u32 - shift)) as usize) = 65535u32 as png_uint_16;
        last += 1;
    }
}

/* Build a single 8-bit table: same as the 16-bit case but much simpler (and
 * typically much faster).  Note that libpng currently does no sBIT processing
 * (apparently contrary to the spec) so a 256-entry table is always generated.
 */
pub unsafe fn png_build_8bit_table(
    png_ptr: png_structrp,
    ptable: png_bytepp,
    gamma_val: png_fixed_point,
) {
    let mut i: c_uint;
    let table: png_bytep = png_malloc(png_ptr, 256) as png_bytep;
    *ptable = table;

    if png_gamma_significant(gamma_val) != 0 {
        i = 0;
        while i < 256 {
            *table.add(i as usize) = png_gamma_8bit_correct(i, gamma_val);
            i += 1;
        }
    } else {
        i = 0;
        while i < 256 {
            *table.add(i as usize) = (i & 0xff) as png_byte;
            i += 1;
        }
    }
}

/* Used from png_read_destroy and below to release the memory used by the gamma
 * tables.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_destroy_gamma_table(png_ptr: png_structrp) {
    png_free(png_ptr, (*png_ptr).gamma_table as png_voidp);
    (*png_ptr).gamma_table = core::ptr::null_mut();

    if !(*png_ptr).gamma_16_table.is_null() {
        let mut i: c_int;
        let istop: c_int = 1 << (8 - (*png_ptr).gamma_shift);
        i = 0;
        while i < istop {
            png_free(
                png_ptr,
                *(*png_ptr).gamma_16_table.add(i as usize) as png_voidp,
            );
            i += 1;
        }
        png_free(png_ptr, (*png_ptr).gamma_16_table as png_voidp);
        (*png_ptr).gamma_16_table = core::ptr::null_mut();
    }

    png_free(png_ptr, (*png_ptr).gamma_from_1 as png_voidp);
    (*png_ptr).gamma_from_1 = core::ptr::null_mut();
    png_free(png_ptr, (*png_ptr).gamma_to_1 as png_voidp);
    (*png_ptr).gamma_to_1 = core::ptr::null_mut();

    if !(*png_ptr).gamma_16_from_1.is_null() {
        let mut i: c_int;
        let istop: c_int = 1 << (8 - (*png_ptr).gamma_shift);
        i = 0;
        while i < istop {
            png_free(
                png_ptr,
                *(*png_ptr).gamma_16_from_1.add(i as usize) as png_voidp,
            );
            i += 1;
        }
        png_free(png_ptr, (*png_ptr).gamma_16_from_1 as png_voidp);
        (*png_ptr).gamma_16_from_1 = core::ptr::null_mut();
    }
    if !(*png_ptr).gamma_16_to_1.is_null() {
        let mut i: c_int;
        let istop: c_int = 1 << (8 - (*png_ptr).gamma_shift);
        i = 0;
        while i < istop {
            png_free(
                png_ptr,
                *(*png_ptr).gamma_16_to_1.add(i as usize) as png_voidp,
            );
            i += 1;
        }
        png_free(png_ptr, (*png_ptr).gamma_16_to_1 as png_voidp);
        (*png_ptr).gamma_16_to_1 = core::ptr::null_mut();
    }
}

/* We build the 8- or 16-bit gamma tables here.  Note that for 16-bit
 * tables, we don't make a full table if we are reducing to 8-bit in
 * the future.  Note also how the gamma_16 tables are segmented so that
 * we don't need to allocate > 64K chunks for a full 16-bit table.
 *
 * TODO: move this to pngrtran.c and make it static.  Better yet create
 * pngcolor.c and put all the PNG_COLORSPACE stuff in there.
 */
/* GAMMA_TRANSFORMS is 1 in this build (READ_BACKGROUND || READ_ALPHA_MODE ||
 * READ_RGB_TO_GRAY are all supported).
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_build_gamma_table(png_ptr: png_structrp, bit_depth: c_int) {
    let file_gamma: png_fixed_point;
    let screen_gamma: png_fixed_point;
    let correction: png_fixed_point;
    let file_to_linear: png_fixed_point;
    let linear_to_screen: png_fixed_point;

    /* Remove any existing table; this copes with multiple calls to
     * png_read_update_info. The warning is because building the gamma tables
     * multiple times is a performance hit - it's harmless but the ability to
     * call png_read_update_info() multiple times is new in 1.5.6 so it seems
     * sensible to warn if the app introduces such a hit.
     */
    if !(*png_ptr).gamma_table.is_null() || !(*png_ptr).gamma_16_table.is_null() {
        png_warning(png_ptr, c"gamma table being rebuilt".as_ptr());
        png_destroy_gamma_table(png_ptr);
    }

    /* The following fields are set, finally, in png_init_read_transformations.
     * If file_gamma is 0 (unset) nothing can be done otherwise if screen_gamma
     * is 0 (unset) there is no gamma correction but to/from linear is possible.
     */
    file_gamma = (*png_ptr).file_gamma;
    screen_gamma = (*png_ptr).screen_gamma;
    file_to_linear = png_reciprocal(file_gamma);

    if screen_gamma > 0 {
        linear_to_screen = png_reciprocal(screen_gamma);
        correction = png_reciprocal2(screen_gamma, file_gamma);
    } else
    /* screen gamma unknown */
    {
        linear_to_screen = file_gamma;
        correction = PNG_FP_1;
    }

    if bit_depth <= 8 {
        png_build_8bit_table(
            png_ptr,
            core::ptr::addr_of_mut!((*png_ptr).gamma_table),
            correction,
        );

        if ((*png_ptr).transformations & (PNG_COMPOSE | PNG_RGB_TO_GRAY)) != 0 {
            png_build_8bit_table(
                png_ptr,
                core::ptr::addr_of_mut!((*png_ptr).gamma_to_1),
                file_to_linear,
            );

            png_build_8bit_table(
                png_ptr,
                core::ptr::addr_of_mut!((*png_ptr).gamma_from_1),
                linear_to_screen,
            );
        }
    } else {
        let mut shift: png_byte;
        let mut sig_bit: png_byte;

        if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
            sig_bit = (*png_ptr).sig_bit.red;

            if (*png_ptr).sig_bit.green > sig_bit {
                sig_bit = (*png_ptr).sig_bit.green;
            }

            if (*png_ptr).sig_bit.blue > sig_bit {
                sig_bit = (*png_ptr).sig_bit.blue;
            }
        } else {
            sig_bit = (*png_ptr).sig_bit.gray;
        }

        /* 16-bit gamma code uses this equation:
         *
         *   ov = table[(iv & 0xff) >> gamma_shift][iv >> 8]
         *
         * Where 'iv' is the input color value and 'ov' is the output value -
         * pow(iv, gamma).
         *
         * Thus the gamma table consists of up to 256 256-entry tables.  The table
         * is selected by the (8-gamma_shift) most significant of the low 8 bits
         * of the color value then indexed by the upper 8 bits:
         *
         *   table[low bits][high 8 bits]
         *
         * So the table 'n' corresponds to all those 'iv' of:
         *
         *   <all high 8-bit values><n << gamma_shift>..<(n+1 << gamma_shift)-1>
         *
         */
        if sig_bit > 0 && (sig_bit as c_uint) < 16u32 {
            /* shift == insignificant bits */
            shift = ((16u32 - (sig_bit as c_uint)) & 0xff) as png_byte;
        } else {
            shift = 0; /* keep all 16 bits */
        }

        if ((*png_ptr).transformations & (PNG_16_TO_8 | PNG_SCALE_16_TO_8)) != 0 {
            /* PNG_MAX_GAMMA_8 is the number of bits to keep - effectively
             * the significant bits in the *input* when the output will
             * eventually be 8 bits.  By default it is 11.
             */
            if (shift as c_uint) < (16u32 - PNG_MAX_GAMMA_8 as c_uint) {
                shift = (16u32 - PNG_MAX_GAMMA_8 as c_uint) as png_byte;
            }
        }

        if (shift as c_uint) > 8u32 {
            shift = 8u32 as png_byte; /* Guarantees at least one table! */
        }

        (*png_ptr).gamma_shift = shift as c_int;

        /* NOTE: prior to 1.5.4 this test used to include PNG_BACKGROUND (now
         * PNG_COMPOSE).  This effectively smashed the background calculation for
         * 16-bit output because the 8-bit table assumes the result will be
         * reduced to 8 bits.
         */
        if ((*png_ptr).transformations & (PNG_16_TO_8 | PNG_SCALE_16_TO_8)) != 0 {
            png_build_16to8_table(
                png_ptr,
                core::ptr::addr_of_mut!((*png_ptr).gamma_16_table),
                shift as c_uint,
                png_reciprocal(correction),
            );
        } else {
            png_build_16bit_table(
                png_ptr,
                core::ptr::addr_of_mut!((*png_ptr).gamma_16_table),
                shift as c_uint,
                correction,
            );
        }

        if ((*png_ptr).transformations & (PNG_COMPOSE | PNG_RGB_TO_GRAY)) != 0 {
            png_build_16bit_table(
                png_ptr,
                core::ptr::addr_of_mut!((*png_ptr).gamma_16_to_1),
                shift as c_uint,
                file_to_linear,
            );

            /* Notice that the '16 from 1' table should be full precision, however
             * the lookup on this table still uses gamma_shift, so it can't be.
             * TODO: fix this.
             */
            png_build_16bit_table(
                png_ptr,
                core::ptr::addr_of_mut!((*png_ptr).gamma_16_from_1),
                shift as c_uint,
                linear_to_screen,
            );
        }
    }
}

/* HARDWARE OR SOFTWARE OPTION SUPPORT */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_option(
    png_ptr: png_structrp,
    option: c_int,
    onoff: c_int,
) -> c_int {
    if !png_ptr.is_null() && option >= 0 && option < PNG_OPTION_NEXT && (option & 1) == 0 {
        let mask: png_uint_32 = 3u32 << option;
        let setting: png_uint_32 = (2u32 + ((onoff != 0) as png_uint_32)) << option;
        let current: png_uint_32 = (*png_ptr).options;

        (*png_ptr).options = ((current & !mask) | setting) as png_uint_32;

        return ((current & mask) as c_int) >> option;
    }

    PNG_OPTION_INVALID
}

/* SIMPLIFIED READ/WRITE SUPPORT */
pub unsafe extern "C-unwind" fn png_image_free_function(argument: png_voidp) -> c_int {
    let image: png_imagep = argument as png_imagep;
    let cp: png_controlp = (*image).opaque;
    let mut c: png_control;

    /* Double check that we have a png_ptr - it should be impossible to get here
     * without one.
     */
    if (*cp).png_ptr.is_null() {
        return 0;
    }

    /* First free any data held in the control structure. */
    if (*cp).owned_file() {
        let fp: *mut c_void = (*(*cp).png_ptr).io_ptr as *mut c_void;
        (*cp).set_owned_file(false);

        /* Ignore errors here. */
        if !fp.is_null() {
            (*(*cp).png_ptr).io_ptr = core::ptr::null_mut();
            crate::cabi::fclose(fp);
        }
    }

    /* Copy the control structure so that the original, allocated, version can be
     * safely freed.  Notice that a png_error here stops the remainder of the
     * cleanup, but this is probably fine because that would indicate bad memory
     * problems anyway.
     */
    c = core::ptr::read(cp);
    (*image).opaque = core::ptr::addr_of_mut!(c);
    png_free(c.png_ptr, cp as png_voidp);

    /* Then the structures, calling the correct API. */
    if c.for_write() {
        png_destroy_write_struct(
            core::ptr::addr_of_mut!(c.png_ptr),
            core::ptr::addr_of_mut!(c.info_ptr),
        );
    } else {
        png_destroy_read_struct(
            core::ptr::addr_of_mut!(c.png_ptr),
            core::ptr::addr_of_mut!(c.info_ptr),
            core::ptr::null_mut(),
        );
    }

    /* Success. */
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_image_free(image: png_imagep) {
    /* Safely call the real function, but only if doing so is safe at this point
     * (if not inside an error handling context).  Otherwise assume
     * png_safe_execute will call this API after the return.
     */
    if !image.is_null()
        && !(*image).opaque.is_null()
        && (*(*image).opaque).error_buf.is_null()
    {
        png_image_free_function(image as png_voidp);
        (*image).opaque = core::ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_image_error(
    image: png_imagep,
    error_message: png_const_charp,
) -> c_int {
    /* Utility to log an error. */
    png_safecat(
        (*image).message.as_mut_ptr(),
        core::mem::size_of_val(&(*image).message),
        0,
        error_message,
    );
    (*image).warning_or_error |= PNG_IMAGE_ERROR;
    png_image_free(image);
    0
}
