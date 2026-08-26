use crate::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_fixed(
    png_ptr: png_const_structrp,
    fp: f64,
    text: png_const_charp,
) -> png_fixed_point {
    let r: f64 = floor(100000.0 * fp + 0.5);

    if r > 2147483647.0 || r < -2147483648.0 {
        png_fixed_error(png_ptr, text);
    }

    r as png_fixed_point
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_fixed_ITU(
    png_ptr: png_const_structrp,
    fp: f64,
    text: png_const_charp,
) -> png_uint_32 {
    let r: f64 = floor(10000.0 * fp + 0.5);

    if r > 2147483647.0 || r < 0.0 {
        png_fixed_error(png_ptr, text);
    }

    r as png_uint_32
}

/* muldiv functions */
/* This API takes signed arguments and rounds the result to the nearest
 * integer (or, for a fixed point number - the standard argument - to
 * the nearest .00001).  Overflow and divide by zero are signalled in
 * the result, a boolean - true on success, false on overflow.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_muldiv(
    res: png_fixed_point_p,
    a: png_fixed_point,
    multiplied_by: png_int_32,
    divided_by: png_int_32,
) -> c_int {
    /* Return a * times / divisor, rounded. */
    if divided_by != 0 {
        if a == 0 || multiplied_by == 0 {
            *res = 0;
            return 1;
        } else {
            let mut r: f64 = a as f64;
            r *= multiplied_by as f64;
            r /= divided_by as f64;
            r = floor(r + 0.5);

            /* A png_fixed_point is a 32-bit integer. */
            if r <= 2147483647.0 && r >= -2147483648.0 {
                *res = r as png_fixed_point;
                return 1;
            }
        }
    }

    0
}

/* Calculate a reciprocal, return 0 on div-by-zero or overflow. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reciprocal(a: png_fixed_point) -> png_fixed_point {
    let r: f64 = floor(1E10 / a as f64 + 0.5);

    if r <= 2147483647.0 && r >= -2147483648.0 {
        return r as png_fixed_point;
    }

    0 /* error/overflow */
}

/* This is the shared test on whether a gamma value is 'significant' - whether
 * it is worth doing gamma correction.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_significant(gamma_value: png_fixed_point) -> c_int {
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
    (gamma_value < PNG_FP_1 - PNG_GAMMA_THRESHOLD_FIXED
        || gamma_value > PNG_FP_1 + PNG_GAMMA_THRESHOLD_FIXED) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reciprocal2(a: png_fixed_point, b: png_fixed_point) -> png_fixed_point {
    /* The required result is 1/a * 1/b; the following preserves accuracy. */

    if a != 0 && b != 0 {
        let mut r: f64 = 1E15 / a as f64;
        r /= b as f64;
        r = floor(r + 0.5);

        if r <= 2147483647.0 && r >= -2147483648.0 {
            return r as png_fixed_point;
        }
    }

    0 /* overflow */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_8bit_correct(
    value: c_uint,
    gamma_value: png_fixed_point,
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
            255.0 * pow(value as c_int as f64 / 255.0, gamma_value as f64 * 0.00001) + 0.5,
        );
        return r as png_byte;
    }

    (value & 0xff) as png_byte
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_16bit_correct(
    value: c_uint,
    gamma_value: png_fixed_point,
) -> png_uint_16 {
    if value > 0 && value < 65535 {
        /* The same (unsigned int)->(double) constraints apply here as above,
         * however in this case the (unsigned int) to (int) conversion can
         * overflow on an ANSI-C90 compliant system so the cast needs to ensure
         * that this is not possible.
         */
        let r: f64 = floor(
            65535.0 * pow(value as png_int_32 as f64 / 65535.0, gamma_value as f64 * 0.00001) + 0.5,
        );
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
pub unsafe extern "C" fn png_gamma_correct(
    png_ptr: png_structrp,
    value: c_uint,
    gamma_value: png_fixed_point,
) -> png_uint_16 {
    if (*png_ptr).bit_depth == 8 {
        png_gamma_8bit_correct(value, gamma_value) as png_uint_16
    } else {
        png_gamma_16bit_correct(value, gamma_value)
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
unsafe fn png_build_16bit_table(
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

    let table: png_uint_16pp = png_calloc(
        png_ptr as png_const_structrp,
        num as usize * core::mem::size_of::<png_uint_16p>(),
    ) as png_uint_16pp;
    *ptable = table;

    i = 0;
    while i < num {
        let sub_table: png_uint_16p = png_malloc(
            png_ptr as png_const_structrp,
            256 * core::mem::size_of::<png_uint_16>(),
        ) as png_uint_16p;
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
                let ig: png_uint_32 = ((j << (8u32 - shift)) + i) as png_uint_32;

                /* Inline the 'max' scaling operation: */
                /* See png_gamma_8bit_correct for why the cast to (int) is
                 * required here.
                 */
                let d: f64 =
                    floor(65535.0 * pow(ig as f64 * fmax, gamma_val as f64 * 0.00001) + 0.5);
                *sub_table.add(j as usize) = d as png_uint_16;

                j += 1;
            }
        } else {
            /* We must still build a table, but do it the fast way. */
            let mut j: c_uint = 0;

            while j < 256 {
                let mut ig: png_uint_32 = ((j << (8u32 - shift)) + i) as png_uint_32;

                if shift != 0 {
                    ig = (ig * 65535u32 + max_by_2) / max;
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
unsafe fn png_build_16to8_table(
    png_ptr: png_structrp,
    ptable: *mut png_uint_16pp,
    shift: c_uint,
    gamma_val: png_fixed_point,
) {
    let num: c_uint = 1u32 << (8u32 - shift);
    let max: c_uint = (1u32 << (16u32 - shift)) - 1u32;
    let mut i: c_uint;
    let mut last: png_uint_32;

    let table: png_uint_16pp = png_calloc(
        png_ptr as png_const_structrp,
        num as usize * core::mem::size_of::<png_uint_16p>(),
    ) as png_uint_16pp;
    *ptable = table;

    /* 'num' is the number of tables and also the number of low bits of low
     * bits of the input 16-bit value used to select a table.  Each table is
     * itself indexed by the high 8 bits of the value.
     */
    i = 0;
    while i < num {
        *table.add(i as usize) = png_malloc(
            png_ptr as png_const_structrp,
            256 * core::mem::size_of::<png_uint_16>(),
        ) as png_uint_16p;
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
    while i < 255
    /* 8-bit output value */
    {
        /* Find the corresponding maximum input value */
        let out: png_uint_16 = (i * 257u32) as png_uint_16; /* 16-bit output value */

        /* Find the boundary value in 16 bits: */
        let mut bound: png_uint_32 =
            png_gamma_16bit_correct(out as c_uint + 128u32, gamma_val) as png_uint_32;

        /* Adjust (round) to (16-shift) bits: */
        bound = (bound * max + 32768u32) / 65535u32 + 1u32;

        while last < bound {
            *(*table.add((last & (0xffu32 >> shift)) as usize)).add((last >> (8u32 - shift)) as usize) =
                out;
            last += 1;
        }

        i += 1;
    }

    /* And fill in the final entries. */
    while last < (num << 8) {
        *(*table.add((last & (0xffu32 >> shift)) as usize)).add((last >> (8u32 - shift)) as usize) =
            65535u32 as png_uint_16;
        last += 1;
    }
}

/* Build a single 8-bit table: same as the 16-bit case but much simpler (and
 * typically much faster).  Note that libpng currently does no sBIT processing
 * (apparently contrary to the spec) so a 256-entry table is always generated.
 */
unsafe fn png_build_8bit_table(
    png_ptr: png_structrp,
    ptable: png_bytepp,
    gamma_val: png_fixed_point,
) {
    let mut i: c_uint;
    let table: png_bytep =
        png_malloc(png_ptr as png_const_structrp, 256 as png_alloc_size_t) as png_bytep;
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
pub unsafe extern "C" fn png_destroy_gamma_table(png_ptr: png_structrp) {
    png_free(png_ptr as png_const_structrp, (*png_ptr).gamma_table as png_voidp);
    (*png_ptr).gamma_table = core::ptr::null_mut();

    if !(*png_ptr).gamma_16_table.is_null() {
        let mut i: c_int;
        let istop: c_int = 1 << (8 - (*png_ptr).gamma_shift);
        i = 0;
        while i < istop {
            png_free(
                png_ptr as png_const_structrp,
                *(*png_ptr).gamma_16_table.add(i as usize) as png_voidp,
            );
            i += 1;
        }
        png_free(
            png_ptr as png_const_structrp,
            (*png_ptr).gamma_16_table as png_voidp,
        );
        (*png_ptr).gamma_16_table = core::ptr::null_mut();
    }

    png_free(png_ptr as png_const_structrp, (*png_ptr).gamma_from_1 as png_voidp);
    (*png_ptr).gamma_from_1 = core::ptr::null_mut();
    png_free(png_ptr as png_const_structrp, (*png_ptr).gamma_to_1 as png_voidp);
    (*png_ptr).gamma_to_1 = core::ptr::null_mut();

    if !(*png_ptr).gamma_16_from_1.is_null() {
        let mut i: c_int;
        let istop: c_int = 1 << (8 - (*png_ptr).gamma_shift);
        i = 0;
        while i < istop {
            png_free(
                png_ptr as png_const_structrp,
                *(*png_ptr).gamma_16_from_1.add(i as usize) as png_voidp,
            );
            i += 1;
        }
        png_free(
            png_ptr as png_const_structrp,
            (*png_ptr).gamma_16_from_1 as png_voidp,
        );
        (*png_ptr).gamma_16_from_1 = core::ptr::null_mut();
    }
    if !(*png_ptr).gamma_16_to_1.is_null() {
        let mut i: c_int;
        let istop: c_int = 1 << (8 - (*png_ptr).gamma_shift);
        i = 0;
        while i < istop {
            png_free(
                png_ptr as png_const_structrp,
                *(*png_ptr).gamma_16_to_1.add(i as usize) as png_voidp,
            );
            i += 1;
        }
        png_free(
            png_ptr as png_const_structrp,
            (*png_ptr).gamma_16_to_1 as png_voidp,
        );
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_build_gamma_table(png_ptr: png_structrp, bit_depth: c_int) {
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
        png_warning(png_ptr as png_const_structrp, cstr!("gamma table being rebuilt"));
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
            shift = ((16u32 - sig_bit as c_uint) & 0xff) as png_byte;
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

        if shift as c_uint > 8u32 {
            shift = 8; /* Guarantees at least one table! */
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
pub unsafe extern "C" fn png_set_option(png_ptr: png_structrp, option: c_int, onoff: c_int) -> c_int {
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

unsafe fn png_image_free_function(argument: png_voidp) -> c_int {
    let image: png_imagep = argument as png_imagep;
    let cp: png_controlp = (*image).opaque;

    /* Double check that we have a png_ptr - it should be impossible to get here
     * without one.
     */
    if (*cp).png_ptr.is_null() {
        return 0;
    }

    /* First free any data held in the control structure. */

    if (*cp).owned_file() != 0 {
        let fp: *mut FILE = (*(*cp).png_ptr).io_ptr as *mut FILE;
        (*cp).set_owned_file(0);

        /* Ignore errors here. */
        if !fp.is_null() {
            (*(*cp).png_ptr).io_ptr = core::ptr::null_mut();
            fclose(fp);
        }
    }

    /* Copy the control structure so that the original, allocated, version can be
     * safely freed.  Notice that a png_error here stops the remainder of the
     * cleanup, but this is probably fine because that would indicate bad memory
     * problems anyway.
     */
    let mut c = *cp;
    (*image).opaque = core::ptr::addr_of_mut!(c);
    png_free(c.png_ptr as png_const_structrp, cp as png_voidp);

    /* Then the structures, calling the correct API. */
    if c.for_write() != 0 {
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
pub unsafe extern "C" fn png_image_free(image: png_imagep) {
    /* Safely call the real function, but only if doing so is safe at this point
     * (if not inside an error handling context).  Otherwise assume
     * png_safe_execute will call this API after the return.
     */
    if !image.is_null() && !(*image).opaque.is_null() && (*(*image).opaque).error_buf.is_null() {
        png_image_free_function(image as png_voidp);
        (*image).opaque = core::ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_error(image: png_imagep, error_message: png_const_charp) -> c_int {
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
