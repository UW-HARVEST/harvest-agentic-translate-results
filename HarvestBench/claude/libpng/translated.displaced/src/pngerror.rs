// pngerror.c - stub functions for i/o and memory allocation
//
// This file provides a location for all error handling.  Users who
// need special error handling are expected to write replacement functions
// and use png_set_error_fn() to use those functions.  See the instructions
// at each function.

use crate::*;

/* This function is called whenever there is a fatal error.  This function
 * should not be changed.  If there is a need to handle errors differently,
 * you should supply a replacement error function and use png_set_error_fn()
 * to replace the error function at run-time.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> ! {
    if !png_ptr.is_null() && (*png_ptr).error_fn.is_some() {
        ((*png_ptr).error_fn.unwrap())(png_ptr as png_structrp, error_message);
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
    pos: usize,
    string: png_const_charp,
) -> usize {
    let mut pos = pos;
    let mut string = string;

    if !buffer.is_null() && pos < bufsize {
        if !string.is_null() {
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
    end: png_charp,
    format: c_int,
    number: png_alloc_size_t,
) -> png_charp {
    let mut end = end;
    let mut number = number;
    let mut count: c_int = 0; /* number of digits output */
    let mut mincount: c_int = 1; /* minimum number required */
    let mut output: c_int = 0; /* digit output (for the fixed point format) */

    end = end.sub(1);
    *end = 0;

    /* This is written so that the loop always runs at least once, even with
     * number zero.
     */
    while (end as png_const_charp) > start && (number != 0 || count < mincount) {
        static digits: [u8; 17] = *b"0123456789ABCDEF\0";

        match format {
            PNG_NUMBER_FORMAT_fixed => {
                /* Needs five digits (the fraction) */
                mincount = 5;
                if output != 0 || number % 10 != 0 {
                    end = end.sub(1);
                    *end = digits[(number % 10) as usize] as c_char;
                    output = 1;
                }
                number /= 10;
            }

            PNG_NUMBER_FORMAT_02u | PNG_NUMBER_FORMAT_u => {
                /* PNG_NUMBER_FORMAT_02u expects at least 2 digits, then falls
                 * through into the PNG_NUMBER_FORMAT_u code.
                 */
                if format == PNG_NUMBER_FORMAT_02u {
                    mincount = 2;
                }

                end = end.sub(1);
                *end = digits[(number % 10) as usize] as c_char;
                number /= 10;
            }

            PNG_NUMBER_FORMAT_02x | PNG_NUMBER_FORMAT_x => {
                /* PNG_NUMBER_FORMAT_02x expects at least two digits, then falls
                 * through into the PNG_NUMBER_FORMAT_x code.
                 */
                if format == PNG_NUMBER_FORMAT_02x {
                    mincount = 2;
                }

                end = end.sub(1);
                *end = digits[(number & 0xf) as usize] as c_char;
                number >>= 4;
            }

            _ => {
                /* an error */
                number = 0;
            }
        }

        /* Keep track of the number of digits added */
        count += 1;

        /* Float a fixed number here: */
        if (format == PNG_NUMBER_FORMAT_fixed)
            && (count == 5)
            && ((end as png_const_charp) > start)
        {
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
pub unsafe extern "C" fn png_warning(png_ptr: png_const_structrp, warning_message: png_const_charp) {
    let offset: c_int = 0;
    if !png_ptr.is_null() && (*png_ptr).warning_fn.is_some() {
        ((*png_ptr).warning_fn.unwrap())(
            png_ptr as png_structrp,
            warning_message.offset(offset as isize),
        );
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
    if number > 0 && (number as usize) <= PNG_WARNING_PARAMETER_COUNT {
        png_safecat(
            (*p.add((number - 1) as usize)).as_mut_ptr(),
            PNG_WARNING_PARAMETER_SIZE, /* (sizeof p[number-1]) */
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
    let bufp: png_charp = buffer.as_mut_ptr();

    /* PNG_FORMAT_NUMBER(buffer, format, value) */
    png_warning_parameter(
        p,
        number,
        png_format_number(
            bufp as png_const_charp,
            bufp.add(PNG_NUMBER_BUFFER_SIZE),
            format,
            value,
        ),
    );
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
    let bufp: png_charp = buffer.as_mut_ptr();

    /* Avoid overflow by doing the negate in a png_alloc_size_t: */
    u = value as png_alloc_size_t;
    if value < 0 {
        u = (!u).wrapping_add(1);
    }

    /* PNG_FORMAT_NUMBER(buffer, format, u) */
    str = png_format_number(
        bufp as png_const_charp,
        bufp.add(PNG_NUMBER_BUFFER_SIZE),
        format,
        u,
    );

    if value < 0 && str > bufp {
        str = str.sub(1);
        *str = b'-' as c_char;
    }

    png_warning_parameter(p, number, str as png_const_charp);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_formatted_warning(
    png_ptr: png_const_structrp,
    p: png_warning_parameters,
    message: png_const_charp,
) {
    /* The internal buffer is just 192 bytes - enough for all our messages,
     * overflow doesn't happen because this code checks!  If someone figures
     * out how to send us a message longer than 192 bytes, all that will
     * happen is that the message will be truncated appropriately.
     */
    let mut message = message;
    let mut i: usize = 0; /* Index in the msg[] buffer: */
    let mut msg: [c_char; 192] = [0; 192];

    /* Each iteration through the following loop writes at most one character
     * to msg[i++] then returns here to validate that there is still space for
     * the trailing '\0'.  It may (in the case of a parameter) read more than
     * one character from message[]; it must check for '\0' and continue to the
     * test if it finds the end of string.
     */
    while i < 192 - 1 && *message != 0 {
        /* '@' at end of string is now just printed (previously it was skipped);
         * it is an error in the calling code to terminate the string with @.
         */
        if !p.is_null() && *message == b'@' as c_char && *message.add(1) != 0 {
            message = message.add(1); /* Consume the '@' */
            let parameter_char: c_int = *message as c_int;
            static valid_parameters: [u8; 10] = *b"123456789\0";
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
            if (parameter as usize) < PNG_WARNING_PARAMETER_COUNT {
                /* Append this parameter */
                let mut parm: png_const_charp = (*p.add(parameter as usize)).as_ptr();
                let pend: png_const_charp = (*p.add(parameter as usize))
                    .as_ptr()
                    .add(PNG_WARNING_PARAMETER_SIZE);

                /* No need to copy the trailing '\0' here, but there is no guarantee
                 * that parm[] has been initialized, so there is no guarantee of a
                 * trailing '\0':
                 */
                while i < 192 - 1 && *parm != 0 && parm < pend {
                    msg[i] = *parm;
                    i += 1;
                    parm = parm.add(1);
                }

                /* Consume the parameter digit too: */
                message = message.add(1);
                continue;
            }

            /* else not a parameter and there is a character after the @ sign; just
             * copy that.  This is known not to be '\0' because of the test above.
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

    /* And this is the formatted message. It may be larger than
     * PNG_MAX_ERROR_TEXT, but that is only used for 'chunk' errors and these
     * are not (currently) formatted.
     */
    png_warning(png_ptr, msg.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_benign_error(
    png_ptr: png_const_structrp,
    warning_message: png_const_charp,
) {
    if ((*png_ptr).flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0 {
        if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 && (*png_ptr).chunk_name != 0 {
            png_chunk_warning(png_ptr, warning_message);
        } else {
            png_warning(png_ptr, warning_message);
        }
    } else {
        if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 && (*png_ptr).chunk_name != 0 {
            png_chunk_error(png_ptr, warning_message);
        } else {
            png_error(png_ptr, warning_message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_app_warning(png_ptr: png_const_structrp, message: png_const_charp) {
    if ((*png_ptr).flags & PNG_FLAG_APP_WARNINGS_WARN) != 0 {
        png_warning(png_ptr, message);
    } else {
        png_error(png_ptr, message);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_app_error(png_ptr: png_const_structrp, message: png_const_charp) {
    if ((*png_ptr).flags & PNG_FLAG_APP_ERRORS_WARN) != 0 {
        png_warning(png_ptr, message);
    } else {
        png_error(png_ptr, message);
    }
}

const PNG_MAX_ERROR_TEXT: usize = 196;

/* These utilities are used internally to build an error message that relates
 * to the current chunk.  The chunk name comes from png_ptr->chunk_name,
 * which is used to prefix the message.  The message is limited in length
 * to 63 bytes. The name characters are output as hex digits wrapped in []
 * if the character is invalid.
 */
/* #define isnonalpha(c) ((c) < 65 || (c) > 122 || ((c) > 90 && (c) < 97)) */

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

unsafe fn png_format_buffer(
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
        if c < 65 || c > 122 || (c > 90 && c < 97) {
            /* isnonalpha(c) != 0 */
            *buffer.offset(iout as isize) = 0x5b as c_char; /* PNG_LITERAL_LEFT_SQUARE_BRACKET */
            iout += 1;
            *buffer.offset(iout as isize) = png_digit[((c & 0xf0) >> 4) as usize];
            iout += 1;
            *buffer.offset(iout as isize) = png_digit[(c & 0x0f) as usize];
            iout += 1;
            *buffer.offset(iout as isize) = 0x5d as c_char; /* PNG_LITERAL_RIGHT_SQUARE_BRACKET */
            iout += 1;
        } else {
            *buffer.offset(iout as isize) = c as c_char;
            iout += 1;
        }
    }

    if error_message.is_null() {
        *buffer.offset(iout as isize) = 0;
    } else {
        let mut iin: c_int = 0;

        *buffer.offset(iout as isize) = b':' as c_char;
        iout += 1;
        *buffer.offset(iout as isize) = b' ' as c_char;
        iout += 1;

        while iin < (PNG_MAX_ERROR_TEXT as c_int) - 1 && *error_message.offset(iin as isize) != 0 {
            *buffer.offset(iout as isize) = *error_message.offset(iin as isize);
            iout += 1;
            iin += 1;
        }

        /* iin < PNG_MAX_ERROR_TEXT, so the following is safe: */
        *buffer.offset(iout as isize) = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) -> ! {
    let mut msg: [c_char; 18 + PNG_MAX_ERROR_TEXT] = [0; 18 + PNG_MAX_ERROR_TEXT];
    if png_ptr.is_null() {
        png_error(png_ptr, error_message);
    } else {
        png_format_buffer(png_ptr, msg.as_mut_ptr(), error_message);
        png_error(png_ptr, msg.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_warning(
    png_ptr: png_const_structrp,
    warning_message: png_const_charp,
) {
    let mut msg: [c_char; 18 + PNG_MAX_ERROR_TEXT] = [0; 18 + PNG_MAX_ERROR_TEXT];
    if png_ptr.is_null() {
        png_warning(png_ptr, warning_message);
    } else {
        png_format_buffer(png_ptr, msg.as_mut_ptr(), warning_message);
        png_warning(png_ptr, msg.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_benign_error(
    png_ptr: png_const_structrp,
    warning_message: png_const_charp,
) {
    if ((*png_ptr).flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0 {
        png_chunk_warning(png_ptr, warning_message);
    } else {
        png_chunk_error(png_ptr, warning_message);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_fixed_error(png_ptr: png_const_structrp, name: png_const_charp) -> ! {
    /* #define fixed_message "fixed point overflow in " */
    static fixed_message: [u8; 25] = *b"fixed point overflow in \0";
    /* #define fixed_message_ln ((sizeof fixed_message)-1) */
    const fixed_message_ln: usize = 24;

    let mut iin: c_uint;
    let mut msg: [c_char; fixed_message_ln + PNG_MAX_ERROR_TEXT] =
        [0; fixed_message_ln + PNG_MAX_ERROR_TEXT];

    memcpy(
        msg.as_mut_ptr() as *mut c_void,
        fixed_message.as_ptr() as *const c_void,
        fixed_message_ln,
    );
    iin = 0;
    if !name.is_null() {
        while (iin as usize) < PNG_MAX_ERROR_TEXT - 1 && *name.add(iin as usize) != 0 {
            msg[fixed_message_ln + iin as usize] = *name.add(iin as usize);
            iin += 1;
        }
    }
    msg[fixed_message_ln + iin as usize] = 0;
    png_error(png_ptr, msg.as_ptr());
}

/* This API only exists if ANSI-C style error handling is used,
 * otherwise it is necessary for png_default_error to be overridden.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_longjmp_fn(
    png_ptr: png_structrp,
    longjmp_fn: png_longjmp_ptr,
    jmp_buf_size: usize,
) -> *mut jmp_buf {
    /* From libpng 1.6.0 the app gets one chance to set a 'jmpbuf_size' value
     * and it must not change after that.  Libpng doesn't care how big the
     * buffer is, just that it doesn't change.
     *
     * If the buffer size is no *larger* than the size of jmp_buf when libpng is
     * compiled a built in jmp_buf is returned; this preserves the pre-1.6.0
     * semantics that this call will not fail.  If the size is larger, however,
     * the buffer is allocated and this may fail, causing the function to return
     * NULL.
     */
    if png_ptr.is_null() {
        return core::ptr::null_mut();
    }

    if (*png_ptr).jmp_buf_ptr.is_null() {
        (*png_ptr).jmp_buf_size = 0; /* not allocated */

        if jmp_buf_size <= core::mem::size_of::<jmp_buf>() {
            (*png_ptr).jmp_buf_ptr = &raw mut (*png_ptr).jmp_buf_local;
        } else {
            (*png_ptr).jmp_buf_ptr = png_malloc_warn(png_ptr, jmp_buf_size) as *mut jmp_buf;

            if (*png_ptr).jmp_buf_ptr.is_null() {
                return core::ptr::null_mut(); /* new NULL return on OOM */
            }

            (*png_ptr).jmp_buf_size = jmp_buf_size;
        }
    } else {
        /* Already allocated: check the size */
        let mut size: usize = (*png_ptr).jmp_buf_size;

        if size == 0 {
            size = core::mem::size_of::<jmp_buf>();
            if (*png_ptr).jmp_buf_ptr != &raw mut (*png_ptr).jmp_buf_local {
                /* This is an internal error in libpng: somehow we have been left
                 * with a stack allocated jmp_buf when the application regained
                 * control.  It's always possible to fix this up, but for the moment
                 * this is a png_error because that makes it easy to detect.
                 */
                png_error(png_ptr, cstr!("Libpng jmp_buf still allocated"));
                /* png_ptr->jmp_buf_ptr = &png_ptr->jmp_buf_local; */
            }
        }

        if size != jmp_buf_size {
            png_warning(png_ptr, cstr!("Application jmp_buf size changed"));
            return core::ptr::null_mut(); /* caller will probably crash: no choice here */
        }
    }

    /* Finally fill in the function, now we have a satisfactory buffer. It is
     * valid to change the function on every call.
     */
    (*png_ptr).longjmp_fn = longjmp_fn;
    (*png_ptr).jmp_buf_ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_jmpbuf(png_ptr: png_structrp) {
    if !png_ptr.is_null() {
        let jb: *mut jmp_buf = (*png_ptr).jmp_buf_ptr;

        /* A size of 0 is used to indicate a local, stack, allocation of the
         * pointer; used here and in png.c
         */
        if !jb.is_null() && (*png_ptr).jmp_buf_size > 0 {
            /* This stuff is so that a failure to free the error control structure
             * does not leave libpng in a state with no valid error handling: the
             * free always succeeds, if there is an error it gets ignored.
             */
            if jb != &raw mut (*png_ptr).jmp_buf_local {
                /* Make an internal, libpng, jmp_buf to return here */
                let mut free_jmp_buf: jmp_buf = core::mem::zeroed();

                if png_private_setjmp(free_jmp_buf.as_mut_ptr()) == 0 {
                    (*png_ptr).jmp_buf_ptr = &raw mut free_jmp_buf; /* come back here */
                    (*png_ptr).jmp_buf_size = 0; /* stack allocation */
                    /* png_ptr->longjmp_fn = longjmp; i.e. the counterpart of the
                     * setjmp() used just above, which for this internal buffer is
                     * png_private_setjmp()/png_private_longjmp().  The cast only
                     * discards the 'noreturn' property of the pointer.
                     */
                    let private_longjmp: unsafe extern "C" fn(*mut __jmp_buf_tag, c_int) -> ! =
                        png_private_longjmp;
                    (*png_ptr).longjmp_fn = Some(core::mem::transmute::<
                        unsafe extern "C" fn(*mut __jmp_buf_tag, c_int) -> !,
                        unsafe extern "C" fn(*mut __jmp_buf_tag, c_int),
                    >(private_longjmp));
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
 * this function MUST NOT RETURN, or the program will likely crash.  This
 * function is used by default, or if the program supplies NULL for the
 * error function pointer in png_set_error_fn().
 */
unsafe fn png_default_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> ! {
    fprintf(
        stderr,
        cstr!("libpng error: %s"),
        if !error_message.is_null() {
            error_message
        } else {
            cstr!("undefined")
        },
    );
    fprintf(stderr, cstr!("\n")); /* PNG_STRING_NEWLINE */

    png_longjmp(png_ptr, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_longjmp(png_ptr: png_const_structrp, val: c_int) -> ! {
    if !png_ptr.is_null()
        && (*png_ptr).longjmp_fn.is_some()
        && !(*png_ptr).jmp_buf_ptr.is_null()
    {
        ((*png_ptr).longjmp_fn.unwrap())((*(*png_ptr).jmp_buf_ptr).as_mut_ptr(), val);
    }

    /* If control reaches this point, png_longjmp() must not return. The only
     * choice is to terminate the whole process (or maybe the thread); to do
     * this the ANSI-C abort() function is used unless a different method is
     * implemented by overriding the default configuration setting for
     * PNG_ABORT().
     */
    abort(); /* PNG_ABORT() */
}

/* This function is called when there is a warning, but the library thinks
 * it can continue anyway.  Replacement functions don't have to do anything
 * here if you don't want them to.  In the default configuration, png_ptr is
 * not used, but it is passed in case it may be useful.
 */
unsafe fn png_default_warning(png_ptr: png_const_structrp, warning_message: png_const_charp) {
    fprintf(stderr, cstr!("libpng warning: %s"), warning_message);
    fprintf(stderr, cstr!("\n")); /* PNG_STRING_NEWLINE */
}

/* This function is called when the application wants to use another method
 * of handling errors and warnings.  Note that the error function MUST NOT
 * return to the calling routine or serious problems will occur.  The return
 * method used in the default routine calls longjmp(png_ptr->jmp_buf_ptr, 1)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_error_fn(
    png_ptr: png_structrp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warning_fn: png_error_ptr,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).error_ptr = error_ptr;
    (*png_ptr).error_fn = error_fn;

    (*png_ptr).warning_fn = warning_fn;
}

/* This function returns a pointer to the error_ptr associated with the user
 * functions.  The application should free any memory associated with this
 * pointer before png_write_destroy and png_read_destroy are called.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_error_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr.is_null() {
        return core::ptr::null_mut();
    }

    (*png_ptr).error_ptr as png_voidp
}

/* Currently the above both depend on SETJMP_SUPPORTED, however it would be
 * possible to implement without setjmp support just so long as there is some
 * way to handle the error return here:
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safe_error(png_ptr: png_structp, error_message: png_const_charp) {
    let png_ptr: png_const_structrp = png_ptr;
    let image: png_imagep = (*png_ptr).error_ptr as png_imagep;

    /* An error is always logged here, overwriting anything (typically a warning)
     * that is already there:
     */
    if !image.is_null() {
        png_safecat(
            (*image).message.as_mut_ptr(),
            64, /* (sizeof image->message) */
            0,
            error_message,
        );
        (*image).warning_or_error |= PNG_IMAGE_ERROR;

        /* Retrieve the jmp_buf from within the png_control, making this work for
         * C++ compilation too is pretty tricky: C++ wants a pointer to the first
         * element of a jmp_buf, but C doesn't tell us the type of that.
         */
        if !(*image).opaque.is_null() && !(*(*image).opaque).error_buf.is_null() {
            png_private_longjmp((*(*image).opaque).error_buf as *mut __jmp_buf_tag, 1);
        }

        /* Missing longjmp buffer, the following is to help debugging: */
        {
            let pos: usize = png_safecat(
                (*image).message.as_mut_ptr(),
                64, /* (sizeof image->message) */
                0,
                cstr!("bad longjmp: "),
            );
            png_safecat(
                (*image).message.as_mut_ptr(),
                64, /* (sizeof image->message) */
                pos,
                error_message,
            );
        }
    }

    /* Here on an internal programming error. */
    abort();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safe_warning(png_ptr: png_structp, warning_message: png_const_charp) {
    let png_ptr: png_const_structrp = png_ptr;
    let image: png_imagep = (*png_ptr).error_ptr as png_imagep;

    /* A warning is only logged if there is no prior warning or error. */
    if (*image).warning_or_error == 0 {
        png_safecat(
            (*image).message.as_mut_ptr(),
            64, /* (sizeof image->message) */
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
    let mut safe_jmpbuf: jmp_buf = core::mem::zeroed();

    /* Safely execute function(arg), with png_error returning back here. */
    if png_private_setjmp(safe_jmpbuf.as_mut_ptr()) == 0 {
        let result: c_int;

        (*(*image).opaque).error_buf = safe_jmpbuf.as_mut_ptr() as png_voidp;
        result = (function.unwrap())(arg);
        (*(*image).opaque).error_buf = saved_error_buf;

        if result != 0 {
            return 1; /* success */
        }
    }

    /* The function failed either because of a caught png_error and a regular
     * return of false above or because of an uncaught png_error from the
     * function itself.  Ensure that the error_buf is always set back to the
     * value saved above:
     */
    (*(*image).opaque).error_buf = saved_error_buf;

    /* On the final false return, when about to return control to the caller, the
     * image is freed (png_image_free does this check but it is duplicated here
     * for clarity:
     */
    if saved_error_buf.is_null() {
        png_image_free(image);
    }

    0 /* failure */
}
