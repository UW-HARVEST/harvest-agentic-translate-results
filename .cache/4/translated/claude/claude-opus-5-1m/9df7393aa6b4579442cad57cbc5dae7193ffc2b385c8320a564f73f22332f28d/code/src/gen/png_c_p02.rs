/* png.c lines 691..1136 */

/* This function returns a pointer to the io_ptr associated with the user
 * functions.  The application should free any memory associated with this
 * pointer before png_write_destroy() or png_read_destroy() are called.
 */
/* png_get_io_ptr */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_io_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr == core::ptr::null_mut() {
        return core::ptr::null_mut();
    }

    (*png_ptr).io_ptr
}

/* Initialize the default input/output functions for the PNG file.  If you
 * use your own read or write routines, you can call either png_set_read_fn()
 * or png_set_write_fn() instead of png_init_io().  If you have defined
 * PNG_NO_STDIO or otherwise disabled PNG_STDIO_SUPPORTED, you must use a
 * function of your own because "FILE *" isn't necessarily available.
 */
/* png_init_io */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_init_io(png_ptr: png_structrp, fp: *mut FILE) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).io_ptr = fp as png_voidp;
}

/* PNG signed integers are saved in 32-bit 2's complement format.  ANSI C-90
 * defines a cast of a signed integer to an unsigned integer either to preserve
 * the value, if it is positive, or to calculate:
 *
 *     (UNSIGNED_MAX+1) + integer
 *
 * Where UNSIGNED_MAX is the appropriate maximum unsigned value, so when the
 * negative integral value is added the result will be an unsigned value
 * corresponding to the 2's complement representation.
 */
/* png_save_int_32 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_save_int_32(buf: png_bytep, i: png_int_32) {
    png_save_uint_32(buf, i as png_uint_32);
}

/* `static const char short_months[12][4]` from png_convert_to_rfc1123_buffer */
static short_months: [[c_char; 4]; 12] = [
    [b'J' as c_char, b'a' as c_char, b'n' as c_char, 0],
    [b'F' as c_char, b'e' as c_char, b'b' as c_char, 0],
    [b'M' as c_char, b'a' as c_char, b'r' as c_char, 0],
    [b'A' as c_char, b'p' as c_char, b'r' as c_char, 0],
    [b'M' as c_char, b'a' as c_char, b'y' as c_char, 0],
    [b'J' as c_char, b'u' as c_char, b'n' as c_char, 0],
    [b'J' as c_char, b'u' as c_char, b'l' as c_char, 0],
    [b'A' as c_char, b'u' as c_char, b'g' as c_char, 0],
    [b'S' as c_char, b'e' as c_char, b'p' as c_char, 0],
    [b'O' as c_char, b'c' as c_char, b't' as c_char, 0],
    [b'N' as c_char, b'o' as c_char, b'v' as c_char, 0],
    [b'D' as c_char, b'e' as c_char, b'c' as c_char, 0],
];

/* Convert the supplied time into an RFC 1123 string suitable for use in
 * a "Creation Time" or other text-based time string.
 */
/* png_convert_to_rfc1123_buffer */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_to_rfc1123_buffer(
    out: *mut c_char,
    ptime: png_const_timep,
) -> c_int {
    if out == core::ptr::null_mut() {
        return 0;
    }

    if (*ptime).year > 9999 /* RFC1123 limitation */
        || (*ptime).month == 0
        || (*ptime).month > 12
        || (*ptime).day == 0
        || (*ptime).day > 31
        || (*ptime).hour > 23
        || (*ptime).minute > 59
        || (*ptime).second > 60
    {
        return 0;
    }

    {
        let mut pos: usize = 0;
        let mut number_buf: [c_char; 5] = [0, 0, 0, 0, 0]; /* enough for a four-digit year */

        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_u, (unsigned)ptime->day); */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_u,
                (*ptime).day as c_uint as png_alloc_size_t,
            ),
        );
        /* APPEND(' '); */
        if pos < 28 {
            *out.add(pos) = b' ' as c_char;
            pos += 1;
        }
        /* APPEND_STRING(short_months[(ptime->month - 1)]); */
        pos = png_safecat(
            out,
            29,
            pos,
            short_months[((*ptime).month as c_int - 1) as usize].as_ptr() as png_const_charp,
        );
        /* APPEND(' '); */
        if pos < 28 {
            *out.add(pos) = b' ' as c_char;
            pos += 1;
        }
        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_u, ptime->year); */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_u,
                (*ptime).year as png_alloc_size_t,
            ),
        );
        /* APPEND(' '); */
        if pos < 28 {
            *out.add(pos) = b' ' as c_char;
            pos += 1;
        }
        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_02u, (unsigned)ptime->hour); */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_02u,
                (*ptime).hour as c_uint as png_alloc_size_t,
            ),
        );
        /* APPEND(':'); */
        if pos < 28 {
            *out.add(pos) = b':' as c_char;
            pos += 1;
        }
        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_02u, (unsigned)ptime->minute); */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_02u,
                (*ptime).minute as c_uint as png_alloc_size_t,
            ),
        );
        /* APPEND(':'); */
        if pos < 28 {
            *out.add(pos) = b':' as c_char;
            pos += 1;
        }
        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_02u, (unsigned)ptime->second); */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_02u,
                (*ptime).second as c_uint as png_alloc_size_t,
            ),
        );
        /* APPEND_STRING(" +0000"); This reliably terminates the buffer */
        pos = png_safecat(out, 29, pos, b" +0000\0".as_ptr() as png_const_charp);
    }

    1
}

/* To do: remove the following from libpng-1.7 */
/* Original API that uses a private buffer in png_struct.
 * Deprecated because it causes png_struct to carry a spurious temporary
 * buffer (png_struct::time_buffer), better to have the caller pass this in.
 */
/* png_convert_to_rfc1123 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_to_rfc1123(
    png_ptr: png_structrp,
    ptime: png_const_timep,
) -> png_const_charp {
    if png_ptr != core::ptr::null_mut() {
        /* The only failure above if png_ptr != NULL is from an invalid ptime */
        if png_convert_to_rfc1123_buffer((*png_ptr).time_buffer.as_mut_ptr(), ptime) == 0 {
            png_warning(
                png_ptr,
                b"Ignoring invalid time value\0".as_ptr() as png_const_charp,
            );
        } else {
            return (*png_ptr).time_buffer.as_ptr() as png_const_charp;
        }
    }

    core::ptr::null()
}

/* png_get_copyright */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_copyright(png_ptr: png_const_structrp) -> png_const_charp {
    /* PNG_STRING_COPYRIGHT is not defined, so the #else branch applies.  The
     * literal below is the concatenation of the C source's fragments with
     * PNG_STRING_NEWLINE == "\n".
     */
    concat!(
        "\n",
        "libpng version 1.6.59.git", "\n",
        "Copyright (c) 2018-2026 Cosmin Truta", "\n",
        "Copyright (c) 1998-2002,2004,2006-2018 Glenn Randers-Pehrson", "\n",
        "Copyright (c) 1996-1997 Andreas Dilger", "\n",
        "Copyright (c) 1995-1996 Guy Eric Schalnat, Group 42, Inc.", "\n",
        "\0"
    )
    .as_ptr() as png_const_charp
}

/* The following return the library version as a short string in the
 * format 1.0.0 through 99.99.99zz.  To get the version of *.h files
 * used with your application, print out PNG_LIBPNG_VER_STRING, which
 * is defined in png.h.
 * Note: now there is no difference between png_get_libpng_ver() and
 * png_get_header_ver().  Due to the version_nn_nn_nn typedef guard,
 * it is guaranteed that png.c uses the correct version of png.h.
 */
/* png_get_libpng_ver */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_libpng_ver(png_ptr: png_const_structrp) -> png_const_charp {
    /* Version of *.c files used when building libpng */
    png_get_header_ver(png_ptr)
}

/* png_get_header_ver */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_header_ver(png_ptr: png_const_structrp) -> png_const_charp {
    /* Version of *.h files used when building libpng */
    PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp
}

/* png_get_header_version */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_header_version(png_ptr: png_const_structrp) -> png_const_charp {
    /* Returns longer string containing both version and date */
    /* __STDC__ is defined, PNG_READ_SUPPORTED is defined, so the result is
     * PNG_HEADER_VERSION_STRING PNG_STRING_NEWLINE.
     */
    b" libpng version 1.6.59.git\n\n\0".as_ptr() as png_const_charp
}

/* NOTE: this routine is not used internally! */
/* Build a grayscale palette.  Palette is assumed to be 1 << bit_depth
 * large of png_color.  This lets grayscale images be treated as
 * paletted.  Most useful for gamma correction and simplification
 * of code.  This API is not used internally.
 */
/* png_build_grayscale_palette */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_build_grayscale_palette(bit_depth: c_int, palette: png_colorp) {
    let num_palette: c_int;
    let color_inc: c_int;
    let mut i: c_int;
    let mut v: c_int;

    if palette == core::ptr::null_mut() {
        return;
    }

    match bit_depth {
        1 => {
            num_palette = 2;
            color_inc = 0xff;
        }

        2 => {
            num_palette = 4;
            color_inc = 0x55;
        }

        4 => {
            num_palette = 16;
            color_inc = 0x11;
        }

        8 => {
            num_palette = 256;
            color_inc = 1;
        }

        _ => {
            num_palette = 0;
            color_inc = 0;
        }
    }

    i = 0;
    v = 0;
    while i < num_palette {
        (*palette.offset(i as isize)).red = (v & 0xff) as png_byte;
        (*palette.offset(i as isize)).green = (v & 0xff) as png_byte;
        (*palette.offset(i as isize)).blue = (v & 0xff) as png_byte;
        i += 1;
        v += color_inc;
    }
}

/* png_handle_as_unknown */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_handle_as_unknown(
    png_ptr: png_const_structrp,
    chunk_name: png_const_bytep,
) -> c_int {
    /* Check chunk_name and return "keep" value if it's on the list, else 0 */
    let mut p: png_const_bytep;
    let p_end: png_const_bytep;

    if png_ptr == core::ptr::null_mut()
        || chunk_name == core::ptr::null()
        || (*png_ptr).num_chunk_list == 0
    {
        return PNG_HANDLE_CHUNK_AS_DEFAULT;
    }

    p_end = (*png_ptr).chunk_list as png_const_bytep;
    p = p_end.add(((*png_ptr).num_chunk_list * 5) as usize); /* beyond end */

    /* The code is the fifth byte after each four byte string.  Historically this
     * code was always searched from the end of the list, this is no longer
     * necessary because the 'set' routine handles duplicate entries correctly.
     */
    loop
    /* num_chunk_list > 0, so at least one */
    {
        p = p.sub(5);

        if memcmp(chunk_name as *const c_void, p as *const c_void, 4) == 0 {
            return *p.add(4) as c_int;
        }

        if !(p > p_end) {
            break;
        }
    }

    /* This means that known chunks should be processed and unknown chunks should
     * be handled according to the value of png_ptr->unknown_default; this can be
     * confusing because, as a result, there are two levels of defaulting for
     * unknown chunks.
     */
    PNG_HANDLE_CHUNK_AS_DEFAULT
}

/* png_chunk_unknown_handling */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_unknown_handling(
    png_ptr: png_const_structrp,
    chunk_name: png_uint_32,
) -> c_int {
    let mut chunk_string: [png_byte; 5] = [0; 5];

    PNG_CSTRING_FROM_CHUNK(chunk_string.as_mut_ptr(), chunk_name);
    png_handle_as_unknown(png_ptr, chunk_string.as_ptr())
}

/* This function, added to libpng-1.0.6g, is untested. */
/* png_reset_zstream */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reset_zstream(png_ptr: png_structrp) -> c_int {
    if png_ptr == core::ptr::null_mut() {
        return Z_STREAM_ERROR;
    }

    /* WARNING: this resets the window bits to the maximum! */
    inflateReset(core::ptr::addr_of_mut!((*png_ptr).zstream))
}

/* This function was added to libpng-1.0.7 */
/* png_access_version_number */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_access_version_number() -> png_uint_32 {
    /* Version of *.c files used when building libpng */
    PNG_LIBPNG_VER as png_uint_32
}

/* Ensure that png_ptr->zstream.msg holds some appropriate error message string.
 * If it doesn't 'ret' is used to set it to something appropriate, even in cases
 * like Z_OK or Z_STREAM_END where the error code is apparently a success code.
 */
/* png_zstream_error */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_zstream_error(png_ptr: png_structrp, ret: c_int) {
    /* Translate 'ret' into an appropriate error string, priority is given to the
     * one in zstream if set.  This always returns a string, even in cases like
     * Z_OK or Z_STREAM_END where the error code is a success code.
     */
    if (*png_ptr).zstream.msg == core::ptr::null() {
        match ret {
            Z_STREAM_END => {
                /* Normal exit */
                (*png_ptr).zstream.msg =
                    b"unexpected end of LZ stream\0".as_ptr() as *const c_char;
            }

            Z_NEED_DICT => {
                /* This means the deflate stream did not have a dictionary; this
                 * indicates a bogus PNG.
                 */
                (*png_ptr).zstream.msg = b"missing LZ dictionary\0".as_ptr() as *const c_char;
            }

            Z_ERRNO => {
                /* gz APIs only: should not happen */
                (*png_ptr).zstream.msg = b"zlib IO error\0".as_ptr() as *const c_char;
            }

            Z_STREAM_ERROR => {
                /* internal libpng error */
                (*png_ptr).zstream.msg = b"bad parameters to zlib\0".as_ptr() as *const c_char;
            }

            Z_DATA_ERROR => {
                (*png_ptr).zstream.msg = b"damaged LZ stream\0".as_ptr() as *const c_char;
            }

            Z_MEM_ERROR => {
                (*png_ptr).zstream.msg = b"insufficient memory\0".as_ptr() as *const c_char;
            }

            Z_BUF_ERROR => {
                /* End of input or output; not a problem if the caller is doing
                 * incremental read or write.
                 */
                (*png_ptr).zstream.msg = b"truncated\0".as_ptr() as *const c_char;
            }

            Z_VERSION_ERROR => {
                (*png_ptr).zstream.msg = b"unsupported zlib version\0".as_ptr() as *const c_char;
            }

            PNG_UNEXPECTED_ZLIB_RETURN => {
                /* Compile errors here mean that zlib now uses the value co-opted in
                 * pngpriv.h for PNG_UNEXPECTED_ZLIB_RETURN; update the switch above
                 * and change pngpriv.h.  Note that this message is "... return",
                 * whereas the default/Z_OK one is "... return code".
                 */
                (*png_ptr).zstream.msg = b"unexpected zlib return\0".as_ptr() as *const c_char;
            }

            /* default: and case Z_OK: */
            _ => {
                (*png_ptr).zstream.msg =
                    b"unexpected zlib return code\0".as_ptr() as *const c_char;
            }
        }
    }
}

/* png_fp_add */
unsafe fn png_fp_add(addend0: png_int_32, addend1: png_int_32, error: *mut c_int) -> png_int_32 {
    /* Safely add two fixed point values setting an error flag and returning 0.5
     * on overflow.
     * IMPLEMENTATION NOTE: ANSI requires signed overflow not to occur, therefore
     * relying on addition of two positive values producing a negative one is not
     * safe.
     */
    if addend0 > 0 {
        if 0x7fffffff - addend0 >= addend1 {
            return addend0 + addend1;
        }
    } else if addend0 < 0 {
        if -0x7fffffff - addend0 <= addend1 {
            return addend0 + addend1;
        }
    } else {
        return addend1;
    }

    *error = 1;
    PNG_FP_1 / 2
}

/* png_fp_sub */
unsafe fn png_fp_sub(addend0: png_int_32, addend1: png_int_32, error: *mut c_int) -> png_int_32 {
    /* As above but calculate addend0-addend1. */
    if addend1 > 0 {
        if -0x7fffffff + addend1 <= addend0 {
            return addend0 - addend1;
        }
    } else if addend1 < 0 {
        if 0x7fffffff + addend1 >= addend0 {
            return addend0 - addend1;
        }
    } else {
        return addend0;
    }

    *error = 1;
    PNG_FP_1 / 2
}

/* png_safe_add */
unsafe fn png_safe_add(
    addend0_and_result: *mut png_int_32,
    addend1: png_int_32,
    addend2: png_int_32,
) -> c_int {
    /* Safely add three integers.  Returns 0 on success, 1 on overflow.  Does not
     * set the result on overflow.
     */
    let mut error: c_int = 0;
    let result: c_int = png_fp_add(
        *addend0_and_result,
        png_fp_add(addend1, addend2, core::ptr::addr_of_mut!(error)),
        core::ptr::addr_of_mut!(error),
    );
    if error == 0 {
        *addend0_and_result = result;
    }
    error
}
