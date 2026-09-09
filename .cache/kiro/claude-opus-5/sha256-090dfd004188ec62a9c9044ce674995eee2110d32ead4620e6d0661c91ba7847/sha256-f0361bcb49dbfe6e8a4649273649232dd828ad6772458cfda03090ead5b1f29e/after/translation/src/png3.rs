//! Translation of c_src/src/png.c lines 2726..4044 (to end of file).
//!
//! Build configuration note: PNG_FLOATING_ARITHMETIC_SUPPORTED,
//! PNG_FLOATING_POINT_SUPPORTED and PNG_16BIT_SUPPORTED are all defined in
//! pnglibconf.h, so the floating-point arithmetic branches are compiled and the
//! `#ifndef PNG_FLOATING_ARITHMETIC_SUPPORTED` fixed-point helpers
//! (png_8bit_l2, png_log8bit, png_log16bit, png_32bit_exp, png_exp,
//! png_exp8bit, png_exp16bit, png_product2) are NOT part of this build and are
//! therefore not translated here.
//!
//! The three big sRGB data tables (png_sRGB_table, png_sRGB_base,
//! png_sRGB_delta) that appear in this line range are translated in
//! translation/src/srgb.rs and are intentionally not duplicated here.
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use crate::prelude::*;

/* png.c: png_fixed()
 *
 * #if defined(PNG_FLOATING_POINT_SUPPORTED) &&
 *    !defined(PNG_FIXED_POINT_MACRO_SUPPORTED) && (... several features ...)
 * All those features are enabled in this build, so the function is present.
 */
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

/* png.c: png_fixed_ITU()
 *
 * #if defined(PNG_FLOATING_POINT_SUPPORTED) &&
 *    !defined(PNG_FIXED_POINT_MACRO_SUPPORTED) &&
 *    (defined(PNG_cLLI_SUPPORTED) || defined(PNG_mDCV_SUPPORTED))
 * cLLI and mDCV are both enabled, so the function is present.
 */
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

/* png.c: png_muldiv()
 *
 * Return a * times / divisor, rounded.  PNG_FLOATING_ARITHMETIC_SUPPORTED is
 * defined, so only the floating-point branch is compiled.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_muldiv(
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

/* png.c: png_reciprocal()
 *
 * Calculate a reciprocal, return 0 on div-by-zero or overflow.
 * PNG_FLOATING_ARITHMETIC_SUPPORTED path only.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reciprocal(a: png_fixed_point) -> png_fixed_point {
    let r: f64 = floor(1E10 / a as f64 + 0.5);

    if r <= 2147483647.0 && r >= -2147483648.0 {
        return r as png_fixed_point;
    }

    0 /* error/overflow */
}

/* png.c: png_gamma_significant()
 *
 * Shared test on whether a gamma value is 'significant'.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_significant(gamma_val: png_fixed_point) -> c_int {
    (gamma_val < PNG_FP_1 - PNG_GAMMA_THRESHOLD_FIXED
        || gamma_val > PNG_FP_1 + PNG_GAMMA_THRESHOLD_FIXED) as c_int
}

/* png.c: png_reciprocal2()
 *
 * The required result is 1/a * 1/b; the following preserves accuracy.
 * PNG_FLOATING_ARITHMETIC_SUPPORTED path only.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reciprocal2(
    a: png_fixed_point,
    b: png_fixed_point,
) -> png_fixed_point {
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

/* png.c: png_gamma_8bit_correct()
 *
 * PNG_FLOATING_ARITHMETIC_SUPPORTED path only.
 *
 *   double r = floor(255*pow((int)value/255.,gamma_val*.00001)+.5);
 *   return (png_byte)r;
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_8bit_correct(
    value: c_uint,
    gamma_val: png_fixed_point,
) -> png_byte {
    if value > 0 && value < 255 {
        let r: f64 =
            floor(255.0 * pow(value as c_int as f64 / 255.0, gamma_val as f64 * 0.00001) + 0.5);
        return r as png_byte;
    }

    (value & 0xff) as png_byte
}

/* png.c: png_gamma_16bit_correct()
 *
 * PNG_16BIT_SUPPORTED and PNG_FLOATING_ARITHMETIC_SUPPORTED path only.
 *
 *   double r = floor(65535*pow((png_int_32)value/65535.,gamma_val*.00001)+.5);
 *   return (png_uint_16)r;
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_16bit_correct(
    value: c_uint,
    gamma_val: png_fixed_point,
) -> png_uint_16 {
    if value > 0 && value < 65535 {
        let r: f64 = floor(
            65535.0
                * pow(
                    value as png_int_32 as f64 / 65535.0,
                    gamma_val as f64 * 0.00001,
                )
                + 0.5,
        );
        return r as png_uint_16;
    }

    value as png_uint_16
}

/* png.c: png_gamma_correct()
 *
 * Interprets values as 8-bit or 16-bit based on png_ptr->bit_depth.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_correct(
    png_ptr: png_structrp,
    value: c_uint,
    gamma_val: png_fixed_point,
) -> png_uint_16 {
    if (*png_ptr).bit_depth == 8 {
        png_gamma_8bit_correct(value, gamma_val) as png_uint_16
    } else {
        png_gamma_16bit_correct(value, gamma_val)
    }
}

/* png.c: png_build_16bit_table() (C `static`)
 *
 * PNG_16BIT_SUPPORTED table.  PNG_FLOATING_ARITHMETIC_SUPPORTED path compiled.
 */
pub unsafe extern "C" fn png_build_16bit_table(
    png_ptr: png_structrp,
    ptable: *mut png_uint_16pp,
    shift: c_uint,
    gamma_val: png_fixed_point,
) {
    /* Various values derived from 'shift': */
    let num: c_uint = 1u32 << (8u32 - shift);
    /* CSE the division and work round wacky GCC warnings. */
    let fmax: f64 = 1.0 / ((1i32 << (16u32 - shift)) - 1) as f64;
    let max: c_uint = (1u32 << (16u32 - shift)) - 1u32;
    let max_by_2: c_uint = 1u32 << (15u32 - shift);
    let mut i: c_uint;

    let table: png_uint_16pp =
        png_calloc(png_ptr, num as usize * core::mem::size_of::<png_uint_16p>()) as png_uint_16pp;
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
            let mut j: c_uint = 0;
            while j < 256 {
                let ig: png_uint_32 = (j << (8 - shift)) + i;
                /* Inline the 'max' scaling operation. */
                let d: f64 =
                    floor(65535.0 * pow(ig as f64 * fmax, gamma_val as f64 * 0.00001) + 0.5);
                *sub_table.add(j as usize) = d as png_uint_16;
                j += 1;
            }
        } else {
            /* We must still build a table, but do it the fast way. */
            let mut j: c_uint = 0;
            while j < 256 {
                let mut ig: png_uint_32 = (j << (8 - shift)) + i;

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

/* png.c: png_build_16to8_table() (C `static`)
 *
 * NOTE: expects the *inverse* of the overall gamma transformation.
 * PNG_16BIT_SUPPORTED.
 */
pub unsafe extern "C" fn png_build_16to8_table(
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
        png_calloc(png_ptr, num as usize * core::mem::size_of::<png_uint_16p>()) as png_uint_16pp;
    *ptable = table;

    i = 0;
    while i < num {
        *table.add(i as usize) =
            png_malloc(png_ptr, 256 * core::mem::size_of::<png_uint_16>()) as png_uint_16p;
        i += 1;
    }

    last = 0;
    i = 0;
    while i < 255 {
        /* 8-bit output value */
        /* Find the corresponding maximum input value */
        let out: png_uint_16 = (i * 257u32) as png_uint_16; /* 16-bit output value */

        /* Find the boundary value in 16 bits: */
        let mut bound: png_uint_32 =
            png_gamma_16bit_correct(out as c_uint + 128u32, gamma_val) as png_uint_32;

        /* Adjust (round) to (16-shift) bits: */
        bound = (bound * max + 32768u32) / 65535u32 + 1u32;

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

/* png.c: png_build_8bit_table() (C `static`)
 *
 * Build a single 8-bit table.
 */
pub unsafe extern "C" fn png_build_8bit_table(
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

/* png.c: png_destroy_gamma_table()
 *
 * Release the memory used by the gamma tables.  PNG_16BIT_SUPPORTED and
 * (READ_BACKGROUND || READ_ALPHA_MODE || RGB_TO_GRAY) are all enabled.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_gamma_table(png_ptr: png_structrp) {
    png_free(png_ptr, (*png_ptr).gamma_table as png_voidp);
    (*png_ptr).gamma_table = core::ptr::null_mut();

    if (*png_ptr).gamma_16_table != core::ptr::null_mut() {
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

    /* READ_BACKGROUND || READ_ALPHA_MODE || RGB_TO_GRAY */
    png_free(png_ptr, (*png_ptr).gamma_from_1 as png_voidp);
    (*png_ptr).gamma_from_1 = core::ptr::null_mut();
    png_free(png_ptr, (*png_ptr).gamma_to_1 as png_voidp);
    (*png_ptr).gamma_to_1 = core::ptr::null_mut();

    if (*png_ptr).gamma_16_from_1 != core::ptr::null_mut() {
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
    if (*png_ptr).gamma_16_to_1 != core::ptr::null_mut() {
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

/* png.c: png_build_gamma_table()
 *
 * GAMMA_TRANSFORMS == 1 (READ_BACKGROUND || READ_ALPHA_MODE || RGB_TO_GRAY all
 * enabled) and PNG_16BIT_SUPPORTED enabled.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_build_gamma_table(png_ptr: png_structrp, bit_depth: c_int) {
    let file_gamma: png_fixed_point;
    let screen_gamma: png_fixed_point;
    let correction: png_fixed_point;
    let file_to_linear: png_fixed_point;
    let linear_to_screen: png_fixed_point;

    /* Remove any existing table; this copes with multiple calls to
     * png_read_update_info.
     */
    if (*png_ptr).gamma_table != core::ptr::null_mut()
        || (*png_ptr).gamma_16_table != core::ptr::null_mut()
    {
        png_warning(png_ptr, cstr(b"gamma table being rebuilt\0"));
        png_destroy_gamma_table(png_ptr);
    }

    file_gamma = (*png_ptr).file_gamma;
    screen_gamma = (*png_ptr).screen_gamma;
    file_to_linear = png_reciprocal(file_gamma);

    if screen_gamma > 0 {
        linear_to_screen = png_reciprocal(screen_gamma);
        correction = png_reciprocal2(screen_gamma, file_gamma);
    } else {
        /* screen gamma unknown */
        linear_to_screen = file_gamma;
        correction = PNG_FP_1;
    }

    if bit_depth <= 8 {
        png_build_8bit_table(png_ptr, &mut (*png_ptr).gamma_table, correction);

        if ((*png_ptr).transformations & (PNG_COMPOSE | PNG_RGB_TO_GRAY)) != 0 {
            png_build_8bit_table(png_ptr, &mut (*png_ptr).gamma_to_1, file_to_linear);

            png_build_8bit_table(png_ptr, &mut (*png_ptr).gamma_from_1, linear_to_screen);
        }
    } else {
        let mut shift: png_byte;
        let sig_bit: png_byte;

        if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
            let mut sb: png_byte = (*png_ptr).sig_bit.red;

            if (*png_ptr).sig_bit.green > sb {
                sb = (*png_ptr).sig_bit.green;
            }

            if (*png_ptr).sig_bit.blue > sb {
                sb = (*png_ptr).sig_bit.blue;
            }

            sig_bit = sb;
        } else {
            sig_bit = (*png_ptr).sig_bit.gray;
        }

        if sig_bit as c_uint > 0 && (sig_bit as c_uint) < 16u32 {
            /* shift == insignificant bits */
            shift = ((16u32 - sig_bit as c_uint) & 0xff) as png_byte;
        } else {
            shift = 0; /* keep all 16 bits */
        }

        if ((*png_ptr).transformations & (PNG_16_TO_8 | PNG_SCALE_16_TO_8)) != 0 {
            /* PNG_MAX_GAMMA_8 is the number of bits to keep. */
            if (shift as c_uint) < (16u32 - PNG_MAX_GAMMA_8 as c_uint) {
                shift = (16u32 - PNG_MAX_GAMMA_8 as c_uint) as png_byte;
            }
        }

        if shift as c_uint > 8u32 {
            shift = 8u32 as png_byte; /* Guarantees at least one table! */
        }

        (*png_ptr).gamma_shift = shift as c_int;

        if ((*png_ptr).transformations & (PNG_16_TO_8 | PNG_SCALE_16_TO_8)) != 0 {
            png_build_16to8_table(
                png_ptr,
                &mut (*png_ptr).gamma_16_table,
                shift as c_uint,
                png_reciprocal(correction),
            );
        } else {
            png_build_16bit_table(
                png_ptr,
                &mut (*png_ptr).gamma_16_table,
                shift as c_uint,
                correction,
            );
        }

        if ((*png_ptr).transformations & (PNG_COMPOSE | PNG_RGB_TO_GRAY)) != 0 {
            png_build_16bit_table(
                png_ptr,
                &mut (*png_ptr).gamma_16_to_1,
                shift as c_uint,
                file_to_linear,
            );

            /* The '16 from 1' table should be full precision, however the
             * lookup on this table still uses gamma_shift, so it can't be.
             */
            png_build_16bit_table(
                png_ptr,
                &mut (*png_ptr).gamma_16_from_1,
                shift as c_uint,
                linear_to_screen,
            );
        }
    }
}

/* png.c: png_set_option()  (PNG_SET_OPTION_SUPPORTED) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_option(
    png_ptr: png_structrp,
    option: c_int,
    onoff: c_int,
) -> c_int {
    if png_ptr != core::ptr::null_mut()
        && option >= 0
        && option < PNG_OPTION_NEXT
        && (option & 1) == 0
    {
        let mask: png_uint_32 = 3u32 << option;
        let setting: png_uint_32 = (2u32 + (onoff != 0) as png_uint_32) << option;
        let current: png_uint_32 = (*png_ptr).options;

        (*png_ptr).options = ((current & !mask) | setting) as png_uint_32;

        return ((current & mask) as c_int) >> option;
    }

    PNG_OPTION_INVALID
}

/* png.c: png_image_free_function() (C `static`)
 *
 * PNG_SIMPLIFIED_READ_SUPPORTED, PNG_SIMPLIFIED_WRITE_SUPPORTED and
 * PNG_STDIO_SUPPORTED are enabled.
 */
pub unsafe extern "C" fn png_image_free_function(argument: png_voidp) -> c_int {
    let image: png_imagep = argument as png_imagep;
    let cp: png_controlp = (*image).opaque;
    let mut c: png_control;

    /* Double check that we have a png_ptr - it should be impossible to get here
     * without one.
     */
    if (*cp).png_ptr == core::ptr::null_mut() {
        return 0;
    }

    /* First free any data held in the control structure. */
    if (*cp).owned_file != 0 {
        let fp: *mut FILE = (*(*cp).png_ptr).io_ptr as *mut FILE;
        (*cp).owned_file = 0;

        /* Ignore errors here. */
        if fp != core::ptr::null_mut() {
            (*(*cp).png_ptr).io_ptr = core::ptr::null_mut();
            fclose(fp);
        }
    }

    /* Copy the control structure so that the original, allocated, version can
     * be safely freed.
     */
    c = core::ptr::read(cp);
    (*image).opaque = &mut c as *mut png_control as png_controlp;
    png_free(c.png_ptr, cp as png_voidp);

    /* Then the structures, calling the correct API. */
    if c.for_write != 0 {
        png_destroy_write_struct(
            &mut c.png_ptr as *mut png_structp as png_structpp,
            &mut c.info_ptr as *mut png_infop as png_infopp,
        );
    } else {
        png_destroy_read_struct(
            &mut c.png_ptr as *mut png_structp as png_structpp,
            &mut c.info_ptr as *mut png_infop as png_infopp,
            core::ptr::null_mut(),
        );
    }

    /* Success. */
    1
}

/* png.c: png_image_free() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_free(image: png_imagep) {
    /* Safely call the real function, but only if doing so is safe at this point
     * (if not inside an error handling context).
     */
    if image != core::ptr::null_mut()
        && (*image).opaque != core::ptr::null_mut()
        && (*(*image).opaque).error_buf == core::ptr::null_mut()
    {
        png_image_free_function(image as png_voidp);
        (*image).opaque = core::ptr::null_mut();
    }
}

/* png.c: png_image_error() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_error(
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
