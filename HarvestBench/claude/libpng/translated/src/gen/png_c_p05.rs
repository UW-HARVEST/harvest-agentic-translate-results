/* png.c lines 1961..2279 */

/* png_check_IHDR */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_check_IHDR(
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
        png_warning(
            png_ptr,
            b"Image width is zero in IHDR\0".as_ptr() as png_const_charp,
        );
        error = 1;
    }

    if width > PNG_UINT_31_MAX {
        png_warning(
            png_ptr,
            b"Invalid image width in IHDR\0".as_ptr() as png_const_charp,
        );
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
            - 48 /* big_row_buf hack */
            - 1) /* filter byte */
            / 8) /* 8-byte RGBA pixels */
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
            b"Image width is too large for this architecture\0".as_ptr() as png_const_charp,
        );
        error = 1;
    }

    if width > (*png_ptr).user_width_max {
        png_warning(
            png_ptr,
            b"Image width exceeds user limit in IHDR\0".as_ptr() as png_const_charp,
        );
        error = 1;
    }

    if height == 0 {
        png_warning(
            png_ptr,
            b"Image height is zero in IHDR\0".as_ptr() as png_const_charp,
        );
        error = 1;
    }

    if height > PNG_UINT_31_MAX {
        png_warning(
            png_ptr,
            b"Invalid image height in IHDR\0".as_ptr() as png_const_charp,
        );
        error = 1;
    }

    if height > (*png_ptr).user_height_max {
        png_warning(
            png_ptr,
            b"Image height exceeds user limit in IHDR\0".as_ptr() as png_const_charp,
        );
        error = 1;
    }

    /* Check other values */
    if bit_depth != 1 && bit_depth != 2 && bit_depth != 4 && bit_depth != 8 && bit_depth != 16 {
        png_warning(
            png_ptr,
            b"Invalid bit depth in IHDR\0".as_ptr() as png_const_charp,
        );
        error = 1;
    }

    if color_type < 0 || color_type == 1 || color_type == 5 || color_type > 6 {
        png_warning(
            png_ptr,
            b"Invalid color type in IHDR\0".as_ptr() as png_const_charp,
        );
        error = 1;
    }

    if (color_type == PNG_COLOR_TYPE_PALETTE && bit_depth > 8)
        || ((color_type == PNG_COLOR_TYPE_RGB
            || color_type == PNG_COLOR_TYPE_GRAY_ALPHA
            || color_type == PNG_COLOR_TYPE_RGB_ALPHA)
            && bit_depth < 8)
    {
        png_warning(
            png_ptr,
            b"Invalid color type/bit depth combination in IHDR\0".as_ptr() as png_const_charp,
        );
        error = 1;
    }

    if interlace_type >= PNG_INTERLACE_LAST {
        png_warning(
            png_ptr,
            b"Unknown interlace method in IHDR\0".as_ptr() as png_const_charp,
        );
        error = 1;
    }

    if compression_type != PNG_COMPRESSION_TYPE_BASE {
        png_warning(
            png_ptr,
            b"Unknown compression method in IHDR\0".as_ptr() as png_const_charp,
        );
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
            b"MNG features are not allowed in a PNG datastream\0".as_ptr() as png_const_charp,
        );
    }

    if filter_type != PNG_FILTER_TYPE_BASE {
        if !(((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
            && (filter_type == PNG_INTRAPIXEL_DIFFERENCING)
            && (((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) == 0)
            && (color_type == PNG_COLOR_TYPE_RGB || color_type == PNG_COLOR_TYPE_RGB_ALPHA))
        {
            png_warning(
                png_ptr,
                b"Unknown filter method in IHDR\0".as_ptr() as png_const_charp,
            );
            error = 1;
        }

        if ((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) != 0 {
            png_warning(
                png_ptr,
                b"Invalid filter method in IHDR\0".as_ptr() as png_const_charp,
            );
            error = 1;
        }
    }

    if error == 1 {
        png_error(
            png_ptr,
            b"Invalid IHDR data\0".as_ptr() as png_const_charp,
        );
    }
}

/* ASCII to fp functions */
/* Check an ASCII formatted floating point value, see the more detailed
 * comments in pngpriv.h
 */
/* The `case` labels of the second switch in png_check_fp_number are sums of
 * two constants; Rust requires a named constant for a match pattern.
 */
const PNG_FP_C_INTEGER_SAW_SIGN: c_int = PNG_FP_INTEGER + PNG_FP_SAW_SIGN;
const PNG_FP_C_INTEGER_SAW_DOT: c_int = PNG_FP_INTEGER + PNG_FP_SAW_DOT;
const PNG_FP_C_INTEGER_SAW_DIGIT: c_int = PNG_FP_INTEGER + PNG_FP_SAW_DIGIT;
const PNG_FP_C_INTEGER_SAW_E: c_int = PNG_FP_INTEGER + PNG_FP_SAW_E;
const PNG_FP_C_FRACTION_SAW_DIGIT: c_int = PNG_FP_FRACTION + PNG_FP_SAW_DIGIT;
const PNG_FP_C_FRACTION_SAW_E: c_int = PNG_FP_FRACTION + PNG_FP_SAW_E;
const PNG_FP_C_EXPONENT_SAW_SIGN: c_int = PNG_FP_EXPONENT + PNG_FP_SAW_SIGN;
const PNG_FP_C_EXPONENT_SAW_DIGIT: c_int = PNG_FP_EXPONENT + PNG_FP_SAW_DIGIT;

/* png_check_fp_number */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_check_fp_number(
    string: png_const_charp,
    size: usize,
    statep: *mut c_int,
    whereami: *mut usize,
) -> c_int {
    let mut state: c_int = *statep;
    let mut i: usize = *whereami;

    'PNG_FP_End: {
        while i < size {
            let type_: c_int;
            /* First find the type of the next character */
            match *string.add(i) {
                43 => {
                    type_ = PNG_FP_SAW_SIGN;
                }
                45 => {
                    type_ = PNG_FP_SAW_SIGN + PNG_FP_NEGATIVE;
                }
                46 => {
                    type_ = PNG_FP_SAW_DOT;
                }
                48 => {
                    type_ = PNG_FP_SAW_DIGIT;
                }
                49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 => {
                    type_ = PNG_FP_SAW_DIGIT + PNG_FP_NONZERO;
                }
                69 | 101 => {
                    type_ = PNG_FP_SAW_E;
                }
                _ => break 'PNG_FP_End,
            }

            /* Now deal with this type according to the current
             * state, the type is arranged to not overlap the
             * bits of the PNG_FP_STATE.
             */
            match (state & PNG_FP_STATE) + (type_ & PNG_FP_SAW_ANY) {
                PNG_FP_C_INTEGER_SAW_SIGN => {
                    if (state & PNG_FP_SAW_ANY) != 0 {
                        break 'PNG_FP_End; /* not a part of the number */
                    }

                    png_fp_add_state(&mut state, type_);
                }

                PNG_FP_C_INTEGER_SAW_DOT => {
                    /* Ok as trailer, ok as lead of fraction. */
                    if (state & PNG_FP_SAW_DOT) != 0 {
                        /* two dots */
                        break 'PNG_FP_End;
                    } else if (state & PNG_FP_SAW_DIGIT) != 0 {
                        /* trailing dot? */
                        png_fp_add_state(&mut state, type_);
                    } else {
                        png_fp_set_state(&mut state, PNG_FP_FRACTION | type_);
                    }
                }

                PNG_FP_C_INTEGER_SAW_DIGIT => {
                    if (state & PNG_FP_SAW_DOT) != 0 {
                        /* delayed fraction */
                        png_fp_set_state(&mut state, PNG_FP_FRACTION | PNG_FP_SAW_DOT);
                    }

                    png_fp_add_state(&mut state, type_ | PNG_FP_WAS_VALID);
                }

                PNG_FP_C_INTEGER_SAW_E => {
                    if (state & PNG_FP_SAW_DIGIT) == 0 {
                        break 'PNG_FP_End;
                    }

                    png_fp_set_state(&mut state, PNG_FP_EXPONENT);
                }

                /* case PNG_FP_FRACTION + PNG_FP_SAW_SIGN:
                      goto PNG_FP_End; ** no sign in fraction */

                /* case PNG_FP_FRACTION + PNG_FP_SAW_DOT:
                      goto PNG_FP_End; ** Because SAW_DOT is always set */
                PNG_FP_C_FRACTION_SAW_DIGIT => {
                    png_fp_add_state(&mut state, type_ | PNG_FP_WAS_VALID);
                }

                PNG_FP_C_FRACTION_SAW_E => {
                    /* This is correct because the trailing '.' on an
                     * integer is handled above - so we can only get here
                     * with the sequence ".E" (with no preceding digits).
                     */
                    if (state & PNG_FP_SAW_DIGIT) == 0 {
                        break 'PNG_FP_End;
                    }

                    png_fp_set_state(&mut state, PNG_FP_EXPONENT);
                }

                PNG_FP_C_EXPONENT_SAW_SIGN => {
                    if (state & PNG_FP_SAW_ANY) != 0 {
                        break 'PNG_FP_End; /* not a part of the number */
                    }

                    png_fp_add_state(&mut state, PNG_FP_SAW_SIGN);
                }

                /* case PNG_FP_EXPONENT + PNG_FP_SAW_DOT:
                      goto PNG_FP_End; */
                PNG_FP_C_EXPONENT_SAW_DIGIT => {
                    png_fp_add_state(&mut state, PNG_FP_SAW_DIGIT | PNG_FP_WAS_VALID);
                }

                /* case PNG_FP_EXPONEXT + PNG_FP_SAW_E:
                      goto PNG_FP_End; */
                _ => break 'PNG_FP_End, /* I.e. break 2 */
            }

            /* The character seems ok, continue. */
            i += 1;
        }
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
/* png_check_fp_string */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_check_fp_string(string: png_const_charp, size: usize) -> c_int {
    let mut state: c_int = 0;
    let mut char_index: usize = 0;

    if png_check_fp_number(string, size, &mut state, &mut char_index) != 0
        && (char_index == size || *string.add(char_index) == 0)
    {
        return state; /* must be non-zero - see above */
    }

    0 /* i.e. fail */
}
