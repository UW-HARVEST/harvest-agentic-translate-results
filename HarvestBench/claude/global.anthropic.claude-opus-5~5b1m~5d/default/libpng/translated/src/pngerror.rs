//! pngerror.c - error handling.
use crate::prelude::*;
use core::ffi::{c_char, c_int, c_void};

pub const PNG_MAX_ERROR_TEXT: usize = 196;

/// Payload of the panic used to emulate the `longjmp` calls that the C code
/// performs internally (`png_safe_error`, `png_free_jmpbuf`,
/// `png_create_png_struct`).
pub struct PngLongjmp(pub c_int);

/// A `png_longjmp_ptr` that raises the internal unwind used in place of
/// `longjmp` in the few places where libpng calls `setjmp` itself.
pub unsafe extern "C-unwind" fn png_internal_longjmp(_jb: *mut jmp_buf, val: c_int) -> ! {
    std::panic::panic_any(PngLongjmp(val))
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) -> ! {
    if !png_ptr.is_null() && (*png_ptr).error_fn.is_some() {
        ((*png_ptr).error_fn.unwrap())(png_ptr as png_structrp, error_message);
    }

    /* If the custom handler doesn't exist, or if it returns,
       use the default handler, which will not return. */
    png_default_error(png_ptr, error_message)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_safecat(
    buffer: png_charp,
    bufsize: usize,
    pos_in: usize,
    string_in: png_const_charp,
) -> usize {
    let mut pos = pos_in;
    let mut string = string_in;

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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_format_number(
    start: png_const_charp,
    end_in: png_charp,
    format: c_int,
    number_in: png_alloc_size_t,
) -> png_charp {
    static DIGITS: [u8; 17] = *b"0123456789ABCDEF\0";

    let mut end = end_in;
    let mut number = number_in;
    let mut count: c_int = 0; /* number of digits output */
    let mut mincount: c_int = 1; /* minimum number required */
    let mut output: c_int = 0; /* digit output (for the fixed point format) */

    end = end.sub(1);
    *end = 0;

    /* This is written so that the loop always runs at least once, even with
     * number zero.
     */
    while (end as *const c_char) > start && (number != 0 || count < mincount) {
        if format == PNG_NUMBER_FORMAT_fixed {
            /* Needs five digits (the fraction) */
            mincount = 5;
            if output != 0 || number % 10 != 0 {
                end = end.sub(1);
                *end = DIGITS[(number % 10) as usize] as c_char;
                output = 1;
            }
            number /= 10;
        } else if format == PNG_NUMBER_FORMAT_02u || format == PNG_NUMBER_FORMAT_u {
            /* PNG_NUMBER_FORMAT_02u expects at least 2 digits, then falls
             * through into the PNG_NUMBER_FORMAT_u case.
             */
            if format == PNG_NUMBER_FORMAT_02u {
                mincount = 2;
            }
            end = end.sub(1);
            *end = DIGITS[(number % 10) as usize] as c_char;
            number /= 10;
        } else if format == PNG_NUMBER_FORMAT_02x || format == PNG_NUMBER_FORMAT_x {
            /* PNG_NUMBER_FORMAT_02x expects at least two digits, then falls
             * through into the PNG_NUMBER_FORMAT_x case.
             */
            if format == PNG_NUMBER_FORMAT_02x {
                mincount = 2;
            }
            end = end.sub(1);
            *end = DIGITS[(number & 0xf) as usize] as c_char;
            number >>= 4;
        } else {
            /* an error */
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_warning(
    png_ptr: png_const_structrp,
    warning_message: png_const_charp,
) {
    let offset: usize = 0;
    if !png_ptr.is_null() && (*png_ptr).warning_fn.is_some() {
        ((*png_ptr).warning_fn.unwrap())(png_ptr as png_structrp, warning_message.add(offset));
    } else {
        png_default_warning(png_ptr, warning_message.add(offset));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_warning_parameter(
    p: *mut [c_char; PNG_WARNING_PARAMETER_SIZE],
    number: c_int,
    string: png_const_charp,
) {
    if number > 0 && number <= PNG_WARNING_PARAMETER_COUNT as c_int {
        let row = (*p.add((number - 1) as usize)).as_mut_ptr();
        png_safecat(row, PNG_WARNING_PARAMETER_SIZE, 0, string);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_warning_parameter_unsigned(
    p: *mut [c_char; PNG_WARNING_PARAMETER_SIZE],
    number: c_int,
    format: c_int,
    value: png_alloc_size_t,
) {
    let mut buffer: [c_char; PNG_NUMBER_BUFFER_SIZE] = [0; PNG_NUMBER_BUFFER_SIZE];
    let s = png_format_number(
        buffer.as_ptr(),
        buffer.as_mut_ptr().add(PNG_NUMBER_BUFFER_SIZE),
        format,
        value,
    );
    png_warning_parameter(p, number, s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_warning_parameter_signed(
    p: *mut [c_char; PNG_WARNING_PARAMETER_SIZE],
    number: c_int,
    format: c_int,
    value: png_int_32,
) {
    let mut u: png_alloc_size_t;
    let mut str_: png_charp;
    let mut buffer: [c_char; PNG_NUMBER_BUFFER_SIZE] = [0; PNG_NUMBER_BUFFER_SIZE];

    /* Avoid overflow by doing the negate in a png_alloc_size_t: */
    u = value as isize as png_alloc_size_t;
    if value < 0 {
        u = (!u).wrapping_add(1);
    }

    str_ = png_format_number(
        buffer.as_ptr(),
        buffer.as_mut_ptr().add(PNG_NUMBER_BUFFER_SIZE),
        format,
        u,
    );

    if value < 0 && str_ > buffer.as_mut_ptr() {
        str_ = str_.sub(1);
        *str_ = b'-' as c_char;
    }

    png_warning_parameter(p, number, str_);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_formatted_warning(
    png_ptr: png_const_structrp,
    p: *mut [c_char; PNG_WARNING_PARAMETER_SIZE],
    message_in: png_const_charp,
) {
    /* The internal buffer is just 192 bytes - enough for all our messages,
     * overflow doesn't happen because this code checks!
     */
    let mut i: usize = 0; /* Index in the msg[] buffer: */
    let mut msg: [c_char; 192] = [0; 192];
    let mut message = message_in;

    while i < 192 - 1 && *message != 0 {
        /* '@' at end of string is now just printed (previously it was skipped);
         * it is an error in the calling code to terminate the string with @.
         */
        if !p.is_null() && *message == b'@' as c_char && *message.add(1) != 0 {
            message = message.add(1); /* Consume the '@' */
            let parameter_char: c_int = *message as c_int;
            static VALID_PARAMETERS: [u8; 10] = *b"123456789\0";
            let mut parameter: usize = 0;

            /* Search for the parameter digit, the index in the string is the
             * parameter to use.
             */
            while VALID_PARAMETERS[parameter] as c_int != parameter_char
                && VALID_PARAMETERS[parameter] != 0
            {
                parameter += 1;
            }

            /* If the parameter digit is out of range it will just get printed. */
            if parameter < PNG_WARNING_PARAMETER_COUNT {
                /* Append this parameter */
                let mut parm: png_const_charp = (*p.add(parameter)).as_ptr();
                let pend: png_const_charp = (*p.add(parameter))
                    .as_ptr()
                    .add(PNG_WARNING_PARAMETER_SIZE);

                while i < 192 - 1 && *parm != 0 && parm < pend {
                    msg[i] = *parm;
                    i += 1;
                    parm = parm.add(1);
                }

                /* Consume the parameter digit too: */
                message = message.add(1);
                continue;
            }

            /* else not a parameter and there is a character after the @ sign;
             * just copy that.
             */
        }

        msg[i] = *message;
        i += 1;
        message = message.add(1);
    }

    /* i is always less than (sizeof msg), so: */
    msg[i] = 0;

    png_warning(png_ptr, msg.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_benign_error(
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
pub unsafe extern "C-unwind" fn png_app_warning(
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
pub unsafe extern "C-unwind" fn png_app_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) {
    if ((*png_ptr).flags & PNG_FLAG_APP_ERRORS_WARN) != 0 {
        png_warning(png_ptr, error_message);
    } else {
        png_error(png_ptr, error_message);
    }
}

#[inline]
fn isnonalpha(c: c_int) -> bool {
    c < 65 || c > 122 || (c > 90 && c < 97)
}

static PNG_DIGIT: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'A', b'B', b'C', b'D', b'E', b'F',
];

unsafe fn png_format_buffer(
    png_ptr: png_const_structrp,
    buffer: png_charp,
    error_message: png_const_charp,
) {
    let chunk_name: png_uint_32 = (*png_ptr).chunk_name;
    let mut iout: usize = 0;
    let mut ishift: c_int = 24;

    while ishift >= 0 {
        let c: c_int = ((chunk_name >> ishift) & 0xff) as c_int;

        ishift -= 8;
        if isnonalpha(c) {
            *buffer.add(iout) = PNG_LITERAL_LEFT_SQUARE_BRACKET as c_char;
            iout += 1;
            *buffer.add(iout) = PNG_DIGIT[((c & 0xf0) >> 4) as usize] as c_char;
            iout += 1;
            *buffer.add(iout) = PNG_DIGIT[(c & 0x0f) as usize] as c_char;
            iout += 1;
            *buffer.add(iout) = PNG_LITERAL_RIGHT_SQUARE_BRACKET as c_char;
            iout += 1;
        } else {
            *buffer.add(iout) = c as c_char;
            iout += 1;
        }
    }

    if error_message.is_null() {
        *buffer.add(iout) = 0;
    } else {
        let mut iin: usize = 0;

        *buffer.add(iout) = b':' as c_char;
        iout += 1;
        *buffer.add(iout) = b' ' as c_char;
        iout += 1;

        while iin < PNG_MAX_ERROR_TEXT - 1 && *error_message.add(iin) != 0 {
            *buffer.add(iout) = *error_message.add(iin);
            iout += 1;
            iin += 1;
        }

        /* iin < PNG_MAX_ERROR_TEXT, so the following is safe: */
        *buffer.add(iout) = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_chunk_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) -> ! {
    let mut msg: [c_char; 18 + PNG_MAX_ERROR_TEXT] = [0; 18 + PNG_MAX_ERROR_TEXT];
    if png_ptr.is_null() {
        png_error(png_ptr, error_message)
    } else {
        png_format_buffer(png_ptr, msg.as_mut_ptr(), error_message);
        png_error(png_ptr, msg.as_ptr())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_chunk_warning(
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
pub unsafe extern "C-unwind" fn png_chunk_benign_error(
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
pub unsafe extern "C-unwind" fn png_chunk_report(
    png_ptr: png_const_structrp,
    message: png_const_charp,
    error: c_int,
) {
    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 {
        if error < PNG_CHUNK_ERROR {
            png_chunk_warning(png_ptr, message);
        } else {
            png_chunk_benign_error(png_ptr, message);
        }
    } else {
        if error < PNG_CHUNK_WRITE_ERROR {
            png_app_warning(png_ptr, message);
        } else {
            png_app_error(png_ptr, message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_fixed_error(
    png_ptr: png_const_structrp,
    name: png_const_charp,
) -> ! {
    const FIXED_MESSAGE: &[u8] = b"fixed point overflow in ";
    const FIXED_MESSAGE_LN: usize = 24;
    let mut iin: usize;
    let mut msg: [c_char; FIXED_MESSAGE_LN + PNG_MAX_ERROR_TEXT] =
        [0; FIXED_MESSAGE_LN + PNG_MAX_ERROR_TEXT];
    memcpy(
        msg.as_mut_ptr() as *mut u8,
        FIXED_MESSAGE.as_ptr(),
        FIXED_MESSAGE_LN,
    );
    iin = 0;
    if !name.is_null() {
        while iin < PNG_MAX_ERROR_TEXT - 1 && *name.add(iin) != 0 {
            msg[FIXED_MESSAGE_LN + iin] = *name.add(iin);
            iin += 1;
        }
    }
    msg[FIXED_MESSAGE_LN + iin] = 0;
    png_error(png_ptr, msg.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_longjmp_fn(
    png_ptr: png_structrp,
    longjmp_fn: png_longjmp_ptr,
    jmp_buf_size: usize,
) -> *mut jmp_buf {
    if png_ptr.is_null() {
        return core::ptr::null_mut();
    }

    if (*png_ptr).jmp_buf_ptr.is_null() {
        (*png_ptr).jmp_buf_size = 0; /* not allocated */

        if jmp_buf_size <= core::mem::size_of::<jmp_buf>() {
            (*png_ptr).jmp_buf_ptr = core::ptr::addr_of_mut!((*png_ptr).jmp_buf_local);
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
            if (*png_ptr).jmp_buf_ptr != core::ptr::addr_of_mut!((*png_ptr).jmp_buf_local) {
                png_error(png_ptr, c"Libpng jmp_buf still allocated".as_ptr());
            }
        }

        if size != jmp_buf_size {
            png_warning(png_ptr, c"Application jmp_buf size changed".as_ptr());
            return core::ptr::null_mut(); /* caller will probably crash */
        }
    }

    /* Finally fill in the function, now we have a satisfactory buffer. */
    (*png_ptr).longjmp_fn = longjmp_fn;
    (*png_ptr).jmp_buf_ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_free_jmpbuf(png_ptr: png_structrp) {
    if !png_ptr.is_null() {
        let jb: *mut jmp_buf = (*png_ptr).jmp_buf_ptr;

        /* A size of 0 is used to indicate a local, stack, allocation of the
         * pointer; used here and in png.c
         */
        if !jb.is_null() && (*png_ptr).jmp_buf_size > 0 {
            if jb != core::ptr::addr_of_mut!((*png_ptr).jmp_buf_local) {
                /* Make an internal, libpng, "jmp_buf" to return here.  The Rust
                 * translation uses an unwind in place of setjmp/longjmp.
                 */
                let mut free_jmp_buf: jmp_buf = jmp_buf([0; 25]);

                (*png_ptr).jmp_buf_ptr = &mut free_jmp_buf; /* come back here */
                (*png_ptr).jmp_buf_size = 0; /* stack allocation */
                (*png_ptr).longjmp_fn = Some(png_internal_longjmp);
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    png_free(png_ptr, jb as png_voidp); /* Return here on error */
                }));
            }
        }

        /* *Always* cancel everything out: */
        (*png_ptr).jmp_buf_size = 0;
        (*png_ptr).jmp_buf_ptr = core::ptr::null_mut();
        (*png_ptr).longjmp_fn = None;
    }
}

unsafe fn png_default_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> ! {
    crate::cabi::fprintf(
        crate::cabi::stderr_ptr,
        c"libpng error: %s".as_ptr(),
        if !error_message.is_null() {
            error_message
        } else {
            c"undefined".as_ptr()
        },
    );
    crate::cabi::fprintf(crate::cabi::stderr_ptr, c"\n".as_ptr());
    png_longjmp(png_ptr, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_longjmp(png_ptr: png_const_structrp, val: c_int) -> ! {
    if !png_ptr.is_null()
        && (*png_ptr).longjmp_fn.is_some()
        && !(*png_ptr).jmp_buf_ptr.is_null()
    {
        ((*png_ptr).longjmp_fn.unwrap())((*png_ptr).jmp_buf_ptr, val);
    }

    /* If control reaches this point, png_longjmp() must not return. */
    crate::cabi::abort()
}

unsafe fn png_default_warning(png_ptr: png_const_structrp, warning_message: png_const_charp) {
    crate::cabi::fprintf(
        crate::cabi::stderr_ptr,
        c"libpng warning: %s".as_ptr(),
        warning_message,
    );
    crate::cabi::fprintf(crate::cabi::stderr_ptr, c"\n".as_ptr());
    let _ = png_ptr;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_error_fn(
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_error_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr.is_null() {
        return core::ptr::null_mut();
    }

    (*png_ptr).error_ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_safe_error(
    png_nonconst_ptr: png_structp,
    error_message: png_const_charp,
) {
    let png_ptr: png_const_structrp = png_nonconst_ptr;
    let image: png_imagep = (*png_ptr).error_ptr as png_imagep;

    /* An error is always logged here, overwriting anything (typically a
     * warning) that is already there:
     */
    if !image.is_null() {
        png_safecat(
            (*image).message.as_mut_ptr(),
            core::mem::size_of_val(&(*image).message),
            0,
            error_message,
        );
        (*image).warning_or_error |= PNG_IMAGE_ERROR;

        if !(*image).opaque.is_null() && !(*(*image).opaque).error_buf.is_null() {
            png_internal_longjmp(core::ptr::null_mut(), 1);
        }

        /* Missing longjmp buffer, the following is to help debugging: */
        {
            let pos = png_safecat(
                (*image).message.as_mut_ptr(),
                core::mem::size_of_val(&(*image).message),
                0,
                c"bad longjmp: ".as_ptr(),
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
    crate::cabi::abort()
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_safe_warning(
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
pub unsafe extern "C-unwind" fn png_safe_execute(
    image: png_imagep,
    function: Option<unsafe extern "C-unwind" fn(png_voidp) -> c_int>,
    arg: png_voidp,
) -> c_int {
    let saved_error_buf: png_voidp = (*(*image).opaque).error_buf;
    /* Stands in for `jmp_buf safe_jmpbuf` - only ever tested for NULL. */
    let mut safe_jmpbuf: jmp_buf = jmp_buf([0; 25]);

    /* Safely execute function(arg), with png_error returning back here. */
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        (*(*image).opaque).error_buf = (&mut safe_jmpbuf) as *mut jmp_buf as png_voidp;
        let result = (function.unwrap())(arg);
        (*(*image).opaque).error_buf = saved_error_buf;

        result
    }));

    if let Ok(result) = caught {
        if result != 0 {
            return 1; /* success */
        }
    }

    /* The function failed either because of a caught png_error and a regular
     * return of false above or because of an uncaught png_error from the
     * function itself.
     */
    (*(*image).opaque).error_buf = saved_error_buf;

    if saved_error_buf.is_null() {
        png_image_free(image);
    }

    0 /* failure */
}
