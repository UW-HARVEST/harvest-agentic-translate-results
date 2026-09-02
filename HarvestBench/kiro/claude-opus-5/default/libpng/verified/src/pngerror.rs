//! Translation of c_src/src/pngerror.c lines 1..850
use crate::prelude::*;

/* Local helpers/macros used by this module (pngpriv.h / pngdebug.h). */

/// `PNG_LITERAL_LEFT_SQUARE_BRACKET` (pngdebug.h)
const PNG_LITERAL_LEFT_SQUARE_BRACKET: c_char = 0x5b;
/// `PNG_LITERAL_RIGHT_SQUARE_BRACKET` (pngdebug.h)
const PNG_LITERAL_RIGHT_SQUARE_BRACKET: c_char = 0x5d;
/// `PNG_STRING_NEWLINE` (pngdebug.h)
const PNG_STRING_NEWLINE: &[u8] = b"\n\0";

/// `PNG_MAX_ERROR_TEXT` (pngerror.c local #define)
const PNG_MAX_ERROR_TEXT: usize = 196;

/// `png_warning_parameters` (pngpriv.h): `char [COUNT][SIZE]`.  A C parameter of
/// this type decays to a pointer to the first row (`char (*)[SIZE]`).
pub type png_warning_parameters_row = [c_char; PNG_WARNING_PARAMETER_SIZE];
pub type png_warning_parameters = *mut png_warning_parameters_row;

/* This function is called whenever there is a fatal error.  This function
 * should not be changed.  If there is a need to handle errors differently,
 * you should supply a replacement error function and use png_set_error_fn()
 * to replace the error function at run-time.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) -> ! {
    if png_ptr != core::ptr::null() && (*png_ptr).error_fn.is_some() {
        if let Some(f) = (*png_ptr).error_fn {
            f(png_ptr as png_structrp, error_message);
        }
    }

    /* If the custom handler doesn't exist, or if it returns,
    use the default handler, which will not return. */
    png_default_error(png_ptr, error_message);
}

/* Utility to safely appends strings to a buffer.  This never errors out so
 * error checking is not required in the caller.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safecat(
    buffer: png_charp,
    bufsize: usize,
    mut pos: usize,
    mut string: png_const_charp,
) -> usize {
    if buffer != core::ptr::null_mut() && pos < bufsize {
        if string != core::ptr::null() {
            while *string != 0 && pos < bufsize - 1 {
                *buffer.add(pos) = *string;
                pos += 1;
                string = string.add(1);
            }
        }

        *buffer.add(pos) = 0;
    }

    pos
}

/* Utility to dump an unsigned value into a buffer, given a start pointer and
 * and end pointer (which should point just *beyond* the end of the buffer!)
 * Returns the pointer to the start of the formatted string.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_format_number(
    start: png_const_charp,
    mut end: png_charp,
    format: c_int,
    mut number: png_alloc_size_t,
) -> png_charp {
    let mut count: c_int = 0; /* number of digits output */
    let mut mincount: c_int = 1; /* minimum number required */
    let mut output: c_int = 0; /* digit output (for the fixed point format) */

    end = end.sub(1);
    *end = 0;

    /* This is written so that the loop always runs at least once, even with
     * number zero.
     */
    while (end as *const c_char) > start && (number != 0 || count < mincount) {
        static digits: [c_char; 17] = [
            b'0' as c_char,
            b'1' as c_char,
            b'2' as c_char,
            b'3' as c_char,
            b'4' as c_char,
            b'5' as c_char,
            b'6' as c_char,
            b'7' as c_char,
            b'8' as c_char,
            b'9' as c_char,
            b'A' as c_char,
            b'B' as c_char,
            b'C' as c_char,
            b'D' as c_char,
            b'E' as c_char,
            b'F' as c_char,
            0,
        ];

        if format == PNG_NUMBER_FORMAT_fixed {
            /* Needs five digits (the fraction) */
            mincount = 5;
            if output != 0 || number % 10 != 0 {
                end = end.sub(1);
                *end = digits[(number % 10) as usize];
                output = 1;
            }
            number /= 10;
        } else if format == PNG_NUMBER_FORMAT_02u {
            /* Expects at least 2 digits. */
            mincount = 2;
            /* FALLTHROUGH */
            end = end.sub(1);
            *end = digits[(number % 10) as usize];
            number /= 10;
        } else if format == PNG_NUMBER_FORMAT_u {
            end = end.sub(1);
            *end = digits[(number % 10) as usize];
            number /= 10;
        } else if format == PNG_NUMBER_FORMAT_02x {
            /* This format expects at least two digits */
            mincount = 2;
            /* FALLTHROUGH */
            end = end.sub(1);
            *end = digits[(number & 0xf) as usize];
            number >>= 4;
        } else if format == PNG_NUMBER_FORMAT_x {
            end = end.sub(1);
            *end = digits[(number & 0xf) as usize];
            number >>= 4;
        } else {
            /* default: an error */
            number = 0;
        }

        /* Keep track of the number of digits added */
        count += 1;

        /* Float a fixed number here: */
        if format == PNG_NUMBER_FORMAT_fixed && count == 5 && (end as *const c_char) > start {
            /* End of the fraction, but maybe nothing was output?  In that case
             * drop the decimal point.  If the number is a true zero handle that
             * here.
             */
            if output != 0 {
                end = end.sub(1);
                *end = b'.' as c_char;
            } else if number == 0 {
                /* and !output */
                end = end.sub(1);
                *end = b'0' as c_char;
            }
        }
    }

    end
}

/* This function is called whenever there is a non-fatal error.  This function
 * should not be changed.  If there is a need to handle warnings differently,
 * you should supply a replacement warning function and use
 * png_set_error_fn() to replace the warning function at run-time.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning(
    png_ptr: png_const_structrp,
    warning_message: png_const_charp,
) {
    let offset: c_int = 0;
    if png_ptr != core::ptr::null() && (*png_ptr).warning_fn.is_some() {
        if let Some(f) = (*png_ptr).warning_fn {
            f(
                png_ptr as png_structrp,
                warning_message.offset(offset as isize),
            );
        }
    } else {
        png_default_warning(png_ptr, warning_message.offset(offset as isize));
    }
}

/* These functions support 'formatted' warning messages with up to
 * PNG_WARNING_PARAMETER_COUNT parameters.  In the format string the parameter
 * is introduced by @<number>, where 'number' starts at 1.  This follows the
 * standard established by X/Open for internationalizable error messages.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter(
    p: png_warning_parameters,
    number: c_int,
    string: png_const_charp,
) {
    if number > 0 && number <= PNG_WARNING_PARAMETER_COUNT as c_int {
        let row = p.add((number - 1) as usize);
        png_safecat(
            (*row).as_mut_ptr(),
            core::mem::size_of::<png_warning_parameters_row>(),
            0,
            string,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter_unsigned(
    p: png_warning_parameters,
    number: c_int,
    format: c_int,
    value: png_alloc_size_t,
) {
    let mut buffer: [c_char; PNG_NUMBER_BUFFER_SIZE] = [0; PNG_NUMBER_BUFFER_SIZE];
    /* PNG_FORMAT_NUMBER(buffer, format, value) */
    let s = png_format_number(
        buffer.as_ptr(),
        buffer.as_mut_ptr().add(buffer.len()),
        format,
        value,
    );
    png_warning_parameter(p, number, s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter_signed(
    p: png_warning_parameters,
    number: c_int,
    format: c_int,
    value: png_int_32,
) {
    let mut u: png_alloc_size_t;
    let mut str: png_charp;
    let mut buffer: [c_char; PNG_NUMBER_BUFFER_SIZE] = [0; PNG_NUMBER_BUFFER_SIZE];

    /* Avoid overflow by doing the negate in a png_alloc_size_t: */
    u = value as png_alloc_size_t;
    if value < 0 {
        u = (!u).wrapping_add(1);
    }

    /* PNG_FORMAT_NUMBER(buffer, format, u) */
    str = png_format_number(
        buffer.as_ptr(),
        buffer.as_mut_ptr().add(buffer.len()),
        format,
        u,
    );

    if value < 0 && str > buffer.as_mut_ptr() {
        str = str.sub(1);
        *str = b'-' as c_char;
    }

    png_warning_parameter(p, number, str);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_formatted_warning(
    png_ptr: png_const_structrp,
    p: png_warning_parameters,
    mut message: png_const_charp,
) {
    /* The internal buffer is just 192 bytes - enough for all our messages,
     * overflow doesn't happen because this code checks!  If someone figures
     * out how to send us a message longer than 192 bytes, all that will
     * happen is that the message will be truncated appropriately.
     */
    let mut i: usize = 0; /* Index in the msg[] buffer: */
    let mut msg: [c_char; 192] = [0; 192];

    /* Each iteration through the following loop writes at most one character
     * to msg[i++] then returns here to validate that there is still space for
     * the trailing '\0'.
     */
    while i < core::mem::size_of_val(&msg) - 1 && *message != 0 {
        /* '@' at end of string is now just printed (previously it was skipped);
         * it is an error in the calling code to terminate the string with @.
         */
        if p != core::ptr::null_mut() && *message == b'@' as c_char && *message.add(1) != 0 {
            message = message.add(1); /* Consume the '@' */
            let parameter_char: c_int = *message as c_int;
            static valid_parameters: [c_char; 10] = [
                b'1' as c_char,
                b'2' as c_char,
                b'3' as c_char,
                b'4' as c_char,
                b'5' as c_char,
                b'6' as c_char,
                b'7' as c_char,
                b'8' as c_char,
                b'9' as c_char,
                0,
            ];
            let mut parameter: c_int = 0;

            /* Search for the parameter digit, the index in the string is the
             * parameter to use.
             */
            while valid_parameters[parameter as usize] as c_int != parameter_char
                && valid_parameters[parameter as usize] != 0
            {
                parameter += 1;
            }

            /* If the parameter digit is out of range it will just get printed. */
            if parameter < PNG_WARNING_PARAMETER_COUNT as c_int {
                /* Append this parameter */
                let row = p.add(parameter as usize);
                let mut parm: png_const_charp = (*row).as_ptr();
                let pend: png_const_charp = (*row)
                    .as_ptr()
                    .add(core::mem::size_of::<png_warning_parameters_row>());

                /* No need to copy the trailing '\0' here, but there is no
                 * guarantee that parm[] has been initialized, so there is no
                 * guarantee of a trailing '\0':
                 */
                while i < core::mem::size_of_val(&msg) - 1 && *parm != 0 && parm < pend {
                    msg[i] = *parm;
                    i += 1;
                    parm = parm.add(1);
                }

                /* Consume the parameter digit too: */
                message = message.add(1);
                continue;
            }

            /* else not a parameter and there is a character after the @ sign;
             * just copy that.  This is known not to be '\0' because of the test
             * above.
             */
        }

        /* At this point *message can't be '\0', even in the bad parameter case
         * above where there is a lone '@' at the end of the message string.
         */
        msg[i] = *message;
        i += 1;
        message = message.add(1);
    }

    /* i is always less than (sizeof msg), so: */
    msg[i] = 0;

    /* And this is the formatted message. */
    png_warning(png_ptr, msg.as_ptr() as png_const_charp);
}

/* PNG_BENIGN_ERRORS_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_benign_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) {
    if ((*png_ptr).flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0 {
        if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 && (*png_ptr).chunk_name != 0 {
            png_chunk_warning(png_ptr, error_message);
        } else {
            png_warning(png_ptr, error_message);
        }
    } else {
        if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 && (*png_ptr).chunk_name != 0 {
            png_chunk_error(png_ptr, error_message);
        } else {
            png_error(png_ptr, error_message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_app_warning(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) {
    if ((*png_ptr).flags & PNG_FLAG_APP_WARNINGS_WARN) != 0 {
        png_warning(png_ptr, error_message);
    } else {
        png_error(png_ptr, error_message);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_app_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) {
    if ((*png_ptr).flags & PNG_FLAG_APP_ERRORS_WARN) != 0 {
        png_warning(png_ptr, error_message);
    } else {
        png_error(png_ptr, error_message);
    }
}

/* isnonalpha(c) ((c) < 65 || (c) > 122 || ((c) > 90 && (c) < 97)) */
#[inline]
fn isnonalpha(c: c_int) -> c_int {
    (c < 65 || c > 122 || (c > 90 && c < 97)) as c_int
}

static png_digit: [c_char; 16] = [
    b'0' as c_char,
    b'1' as c_char,
    b'2' as c_char,
    b'3' as c_char,
    b'4' as c_char,
    b'5' as c_char,
    b'6' as c_char,
    b'7' as c_char,
    b'8' as c_char,
    b'9' as c_char,
    b'A' as c_char,
    b'B' as c_char,
    b'C' as c_char,
    b'D' as c_char,
    b'E' as c_char,
    b'F' as c_char,
];

/* These utilities are used internally to build an error message that relates
 * to the current chunk.
 */
pub unsafe extern "C" fn png_format_buffer(
    png_ptr: png_const_structrp,
    buffer: png_charp,
    error_message: png_const_charp,
) {
    let chunk_name: png_uint_32 = (*png_ptr).chunk_name;
    let mut iout: c_int = 0;
    let mut ishift: c_int = 24;

    while ishift >= 0 {
        let c: c_int = ((chunk_name >> ishift) as c_int) & 0xff;

        ishift -= 8;
        if isnonalpha(c) != 0 {
            *buffer.offset(iout as isize) = PNG_LITERAL_LEFT_SQUARE_BRACKET;
            iout += 1;
            *buffer.offset(iout as isize) = png_digit[((c & 0xf0) >> 4) as usize];
            iout += 1;
            *buffer.offset(iout as isize) = png_digit[(c & 0x0f) as usize];
            iout += 1;
            *buffer.offset(iout as isize) = PNG_LITERAL_RIGHT_SQUARE_BRACKET;
            iout += 1;
        } else {
            *buffer.offset(iout as isize) = c as c_char;
            iout += 1;
        }
    }

    if error_message == core::ptr::null() {
        *buffer.offset(iout as isize) = 0;
    } else {
        let mut iin: c_int = 0;

        *buffer.offset(iout as isize) = b':' as c_char;
        iout += 1;
        *buffer.offset(iout as isize) = b' ' as c_char;
        iout += 1;

        while iin < PNG_MAX_ERROR_TEXT as c_int - 1 && *error_message.offset(iin as isize) != 0 {
            *buffer.offset(iout as isize) = *error_message.offset(iin as isize);
            iout += 1;
            iin += 1;
        }

        /* iin < PNG_MAX_ERROR_TEXT, so the following is safe: */
        *buffer.offset(iout as isize) = 0;
    }
}

/* READ && ERROR_TEXT */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) -> ! {
    let mut msg: [c_char; 18 + PNG_MAX_ERROR_TEXT] = [0; 18 + PNG_MAX_ERROR_TEXT];
    if png_ptr == core::ptr::null() {
        png_error(png_ptr, error_message);
    } else {
        png_format_buffer(png_ptr, msg.as_mut_ptr(), error_message);
        png_error(png_ptr, msg.as_ptr() as png_const_charp);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_warning(
    png_ptr: png_const_structrp,
    warning_message: png_const_charp,
) {
    let mut msg: [c_char; 18 + PNG_MAX_ERROR_TEXT] = [0; 18 + PNG_MAX_ERROR_TEXT];
    if png_ptr == core::ptr::null() {
        png_warning(png_ptr, warning_message);
    } else {
        png_format_buffer(png_ptr, msg.as_mut_ptr(), warning_message);
        png_warning(png_ptr, msg.as_ptr() as png_const_charp);
    }
}

/* READ && BENIGN_ERRORS */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_benign_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) {
    if ((*png_ptr).flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0 {
        png_chunk_warning(png_ptr, error_message);
    } else {
        png_chunk_error(png_ptr, error_message);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_report(
    png_ptr: png_const_structrp,
    message: png_const_charp,
    error: c_int,
) {
    /* This is always supported, but for just read or just write it
     * unconditionally does the right thing.
     */
    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 {
        if error < PNG_CHUNK_ERROR {
            png_chunk_warning(png_ptr, message);
        } else {
            png_chunk_benign_error(png_ptr, message);
        }
    } else if ((*png_ptr).mode & PNG_IS_READ_STRUCT) == 0 {
        if error < PNG_CHUNK_WRITE_ERROR {
            png_app_warning(png_ptr, message);
        } else {
            png_app_error(png_ptr, message);
        }
    }
}

/* ERROR_TEXT && FLOATING_POINT */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_fixed_error(png_ptr: png_const_structrp, name: png_const_charp) -> ! {
    const fixed_message: &[u8] = b"fixed point overflow in ";
    const fixed_message_ln: usize = fixed_message.len();
    let mut iin: c_uint;
    let mut msg: [c_char; fixed_message_ln + PNG_MAX_ERROR_TEXT] =
        [0; fixed_message_ln + PNG_MAX_ERROR_TEXT];
    memcpy(
        msg.as_mut_ptr() as *mut c_void,
        fixed_message.as_ptr() as *const c_void,
        fixed_message_ln,
    );
    iin = 0;
    if name != core::ptr::null() {
        while iin < (PNG_MAX_ERROR_TEXT as c_uint - 1) && *name.add(iin as usize) != 0 {
            msg[fixed_message_ln + iin as usize] = *name.add(iin as usize);
            iin += 1;
        }
    }
    msg[fixed_message_ln + iin as usize] = 0;
    png_error(png_ptr, msg.as_ptr() as png_const_charp);
}

/* PNG_SETJMP_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_longjmp_fn(
    png_ptr: png_structrp,
    longjmp_fn: png_longjmp_ptr,
    jmp_buf_size: usize,
) -> *mut jmp_buf {
    if png_ptr == core::ptr::null_mut() {
        return core::ptr::null_mut();
    }

    if (*png_ptr).jmp_buf_ptr == core::ptr::null_mut() {
        (*png_ptr).jmp_buf_size = 0; /* not allocated */

        if jmp_buf_size <= core::mem::size_of_val(&(*png_ptr).jmp_buf_local) {
            (*png_ptr).jmp_buf_ptr = &mut (*png_ptr).jmp_buf_local;
        } else {
            (*png_ptr).jmp_buf_ptr = png_malloc_warn(png_ptr, jmp_buf_size) as *mut jmp_buf;

            if (*png_ptr).jmp_buf_ptr == core::ptr::null_mut() {
                return core::ptr::null_mut(); /* new NULL return on OOM */
            }

            (*png_ptr).jmp_buf_size = jmp_buf_size;
        }
    } else {
        /* Already allocated: check the size */
        let mut size: usize = (*png_ptr).jmp_buf_size;

        if size == 0 {
            size = core::mem::size_of_val(&(*png_ptr).jmp_buf_local);
            if (*png_ptr).jmp_buf_ptr != &mut (*png_ptr).jmp_buf_local {
                /* This is an internal error in libpng. */
                png_error(png_ptr, cstr(b"Libpng jmp_buf still allocated\0"));
            }
        }

        if size != jmp_buf_size {
            png_warning(png_ptr, cstr(b"Application jmp_buf size changed\0"));
            return core::ptr::null_mut(); /* caller will probably crash: no choice here */
        }
    }

    /* Finally fill in the function, now we have a satisfactory buffer. */
    (*png_ptr).longjmp_fn = longjmp_fn;
    (*png_ptr).jmp_buf_ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_jmpbuf(png_ptr: png_structrp) {
    if png_ptr != core::ptr::null_mut() {
        let jb: *mut jmp_buf = (*png_ptr).jmp_buf_ptr;

        /* A size of 0 is used to indicate a local, stack, allocation of the
         * pointer; used here and in png.c
         */
        if jb != core::ptr::null_mut() && (*png_ptr).jmp_buf_size > 0 {
            /* This stuff is so that a failure to free the error control
             * structure does not leave libpng in a state with no valid error
             * handling: the free always succeeds, if there is an error it gets
             * ignored.
             */
            if jb != &mut (*png_ptr).jmp_buf_local {
                /* Make an internal, libpng, jmp_buf to return here */
                let mut free_jmp_buf: jmp_buf = jmp_buf::new();

                if setjmp(&mut free_jmp_buf) == 0 {
                    (*png_ptr).jmp_buf_ptr = &mut free_jmp_buf; /* come back here */
                    (*png_ptr).jmp_buf_size = 0; /* stack allocation */
                    (*png_ptr).longjmp_fn = Some(longjmp);
                    png_free(png_ptr, jb as png_voidp); /* Return to setjmp on error */
                }
            }
        }

        /* *Always* cancel everything out: */
        (*png_ptr).jmp_buf_size = 0;
        (*png_ptr).jmp_buf_ptr = core::ptr::null_mut();
        (*png_ptr).longjmp_fn = None;
    }
}

/* This is the default error handling function.  Note that replacements for
 * this function MUST NOT RETURN, or the program will likely crash.
 */
pub unsafe extern "C" fn png_default_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) -> ! {
    fprintf(
        stderr,
        cstr(b"libpng error: %s\0"),
        if error_message != core::ptr::null() {
            error_message
        } else {
            cstr(b"undefined\0")
        },
    );
    fprintf(stderr, PNG_STRING_NEWLINE.as_ptr() as *const c_char);
    png_longjmp(png_ptr, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_longjmp(png_ptr: png_const_structrp, val: c_int) -> ! {
    if png_ptr != core::ptr::null()
        && (*png_ptr).longjmp_fn.is_some()
        && (*png_ptr).jmp_buf_ptr != core::ptr::null_mut()
    {
        if let Some(f) = (*png_ptr).longjmp_fn {
            f((*png_ptr).jmp_buf_ptr, val);
        }
    }

    /* If control reaches this point, png_longjmp() must not return. */
    abort();
}

/* This function is called when there is a warning, but the library thinks
 * it can continue anyway.
 */
pub unsafe extern "C" fn png_default_warning(
    _png_ptr: png_const_structrp,
    warning_message: png_const_charp,
) {
    fprintf(stderr, cstr(b"libpng warning: %s\0"), warning_message);
    fprintf(stderr, PNG_STRING_NEWLINE.as_ptr() as *const c_char);
}

/* This function is called when the application wants to use another method
 * of handling errors and warnings.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_error_fn(
    png_ptr: png_structrp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warning_fn: png_error_ptr,
) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).error_ptr = error_ptr;
    (*png_ptr).error_fn = error_fn;
    (*png_ptr).warning_fn = warning_fn;
}

/* This function returns a pointer to the error_ptr associated with the user
 * functions.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_error_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr == core::ptr::null() {
        return core::ptr::null_mut();
    }

    (*png_ptr).error_ptr
}

/* SIMPLIFIED_READ || SIMPLIFIED_WRITE */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safe_error(
    png_nonconst_ptr: png_structp,
    error_message: png_const_charp,
) -> ! {
    let png_ptr: png_const_structrp = png_nonconst_ptr;
    let image: png_imagep = (*png_ptr).error_ptr as png_imagep;

    /* An error is always logged here, overwriting anything (typically a
     * warning) that is already there:
     */
    if image != core::ptr::null_mut() {
        png_safecat(
            (*image).message.as_mut_ptr(),
            core::mem::size_of_val(&(*image).message),
            0,
            error_message,
        );
        (*image).warning_or_error |= PNG_IMAGE_ERROR;

        /* Retrieve the jmp_buf from within the png_control. */
        if (*image).opaque != core::ptr::null_mut()
            && (*(*image).opaque).error_buf != core::ptr::null_mut()
        {
            /* png_control_jmp_buf(image->opaque) == image->opaque->error_buf */
            longjmp((*(*image).opaque).error_buf as *mut jmp_buf, 1);
        }

        /* Missing longjmp buffer, the following is to help debugging: */
        {
            let pos: usize = png_safecat(
                (*image).message.as_mut_ptr(),
                core::mem::size_of_val(&(*image).message),
                0,
                cstr(b"bad longjmp: \0"),
            );
            png_safecat(
                (*image).message.as_mut_ptr(),
                core::mem::size_of_val(&(*image).message),
                pos,
                error_message,
            );
        }
    }

    /* Here on an internal programming error. */
    abort();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safe_warning(
    png_nonconst_ptr: png_structp,
    warning_message: png_const_charp,
) {
    let png_ptr: png_const_structrp = png_nonconst_ptr;
    let image: png_imagep = (*png_ptr).error_ptr as png_imagep;

    /* A warning is only logged if there is no prior warning or error. */
    if (*image).warning_or_error == 0 {
        png_safecat(
            (*image).message.as_mut_ptr(),
            core::mem::size_of_val(&(*image).message),
            0,
            warning_message,
        );
        (*image).warning_or_error |= PNG_IMAGE_WARNING;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safe_execute(
    image: png_imagep,
    function: Option<unsafe extern "C" fn(png_voidp) -> c_int>,
    arg: png_voidp,
) -> c_int {
    let saved_error_buf: png_voidp = (*(*image).opaque).error_buf;
    let mut safe_jmpbuf: jmp_buf = jmp_buf::new();

    /* Safely execute function(arg), with png_error returning back here. */
    if setjmp(&mut safe_jmpbuf) == 0 {
        (*(*image).opaque).error_buf = &mut safe_jmpbuf as *mut jmp_buf as png_voidp;
        let result: c_int =
            (core::mem::transmute::<_, unsafe extern "C" fn(png_voidp) -> c_int>(function))(arg);
        (*(*image).opaque).error_buf = saved_error_buf;

        if result != 0 {
            return 1; /* success */
        }
    }

    /* The function failed either because of a caught png_error and a regular
     * return of false above or because of an uncaught png_error from the
     * function itself.
     */
    (*(*image).opaque).error_buf = saved_error_buf;

    /* On the final false return, when about to return control to the caller,
     * the image is freed.
     */
    if saved_error_buf == core::ptr::null_mut() {
        png_image_free(image);
    }

    0 /* failure */
}
