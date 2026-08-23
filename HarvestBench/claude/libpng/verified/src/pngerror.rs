//! Translation of `c_src/src/pngerror.c`

use crate::*;

const PNG_MAX_ERROR_TEXT: usize = 196;

const PNG_LITERAL_LEFT_SQUARE_BRACKET: c_char = 0x5b;
const PNG_LITERAL_RIGHT_SQUARE_BRACKET: c_char = 0x5d;

#[inline]
fn isnonalpha(c: c_int) -> bool {
    c < 65 || c > 122 || (c > 90 && c < 97)
}

/* png_error */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> ! {
    if png_ptr != core::ptr::null_mut() && (*png_ptr).error_fn.is_some() {
        ((*png_ptr).error_fn.unwrap())(png_ptr, error_message);
    }

    /* If the custom handler doesn't exist, or if it returns,
     * use the default handler, which will not return. */
    png_default_error(png_ptr, error_message);
}

/* png_safecat */
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

/* png_format_number */
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
        const digits: &[u8; 17] = b"0123456789ABCDEF\0";

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

            PNG_NUMBER_FORMAT_02u => {
                /* Expects at least 2 digits. */
                mincount = 2;
                /* FALLTHROUGH */
                end = end.sub(1);
                *end = digits[(number % 10) as usize] as c_char;
                number /= 10;
            }

            /* PNG_NUMBER_FORMAT_u == PNG_NUMBER_FORMAT_d == 1 */
            PNG_NUMBER_FORMAT_u => {
                end = end.sub(1);
                *end = digits[(number % 10) as usize] as c_char;
                number /= 10;
            }

            PNG_NUMBER_FORMAT_x => {
                end = end.sub(1);
                *end = digits[(number & 0xf) as usize] as c_char;
                number >>= 4;
            }

            PNG_NUMBER_FORMAT_02x => {
                /* This format expects at least two digits */
                mincount = 2;
                /* FALLTHROUGH */
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
            && ((end as *const c_char) > start)
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

/// `PNG_FORMAT_NUMBER(buffer, format, number)` for a `[c_char; N]` buffer.
#[inline]
unsafe fn PNG_FORMAT_NUMBER<const N: usize>(
    buffer: &mut [c_char; N],
    format: c_int,
    number: png_alloc_size_t,
) -> png_charp {
    png_format_number(
        buffer.as_ptr(),
        buffer.as_mut_ptr().add(N),
        format,
        number,
    )
}

/* png_warning */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning(png_ptr: png_const_structrp, warning_message: png_const_charp) {
    let offset: c_int = 0;
    if png_ptr != core::ptr::null_mut() && (*png_ptr).warning_fn.is_some() {
        ((*png_ptr).warning_fn.unwrap())(png_ptr, warning_message.offset(offset as isize));
    } else {
        png_default_warning(png_ptr, warning_message.offset(offset as isize));
    }
}

/* png_warning_parameter */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter(
    p: *mut [c_char; PNG_WARNING_PARAMETER_SIZE],
    number: c_int,
    string: png_const_charp,
) {
    if number > 0 && number <= PNG_WARNING_PARAMETER_COUNT as c_int {
        png_safecat(
            (*p.add((number - 1) as usize)).as_mut_ptr(),
            PNG_WARNING_PARAMETER_SIZE,
            0,
            string,
        );
    }
}

/* png_warning_parameter_unsigned */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter_unsigned(
    p: *mut [c_char; PNG_WARNING_PARAMETER_SIZE],
    number: c_int,
    format: c_int,
    value: png_alloc_size_t,
) {
    let mut buffer: [c_char; PNG_NUMBER_BUFFER_SIZE] = [0; PNG_NUMBER_BUFFER_SIZE];
    let s = PNG_FORMAT_NUMBER(&mut buffer, format, value);
    png_warning_parameter(p, number, s);
}

/* png_warning_parameter_signed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter_signed(
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

    str_ = PNG_FORMAT_NUMBER(&mut buffer, format, u);

    if value < 0 && str_ > buffer.as_mut_ptr() {
        str_ = str_.sub(1);
        *str_ = b'-' as c_char;
    }

    png_warning_parameter(p, number, str_);
}

/* png_formatted_warning */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_formatted_warning(
    png_ptr: png_const_structrp,
    p: *mut [c_char; PNG_WARNING_PARAMETER_SIZE],
    mut message: png_const_charp,
) {
    let mut i: usize = 0;
    let mut msg: [c_char; 192] = [0; 192];

    while i < 192 - 1 && *message != 0 {
        if p != core::ptr::null_mut() && *message == b'@' as c_char && *message.add(1) != 0 {
            message = message.add(1); /* Consume the '@' */
            let parameter_char: c_int = *message as c_int;
            const valid_parameters: &[u8; 10] = b"123456789\0";
            let mut parameter: c_int = 0;

            while valid_parameters[parameter as usize] as c_int != parameter_char
                && valid_parameters[parameter as usize] != 0
            {
                parameter += 1;
            }

            if parameter < PNG_WARNING_PARAMETER_COUNT as c_int {
                /* Append this parameter */
                let mut parm: png_const_charp = (*p.add(parameter as usize)).as_ptr();
                let pend: png_const_charp =
                    (*p.add(parameter as usize)).as_ptr().add(PNG_WARNING_PARAMETER_SIZE);

                while i < 192 - 1 && *parm != 0 && parm < pend {
                    msg[i] = *parm;
                    i += 1;
                    parm = parm.add(1);
                }

                /* Consume the parameter digit too: */
                message = message.add(1);
                continue;
            }
        }

        msg[i] = *message;
        i += 1;
        message = message.add(1);
    }

    msg[i] = 0;

    png_warning(png_ptr, msg.as_ptr());
}

/* png_benign_error */
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

/* png_app_warning */
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

/* png_app_error */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_app_error(png_ptr: png_const_structrp, error_message: png_const_charp) {
    if ((*png_ptr).flags & PNG_FLAG_APP_ERRORS_WARN) != 0 {
        png_warning(png_ptr, error_message);
    } else {
        png_error(png_ptr, error_message);
    }
}

const png_digit: [c_char; 16] = [
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

/* png_format_buffer */
unsafe fn png_format_buffer(
    png_ptr: png_const_structrp,
    buffer: png_charp,
    error_message: png_const_charp,
) {
    let chunk_name: png_uint_32 = (*png_ptr).chunk_name;
    let mut iout: c_int = 0;
    let mut ishift: c_int = 24;

    while ishift >= 0 {
        let c: c_int = ((chunk_name >> ishift) & 0xff) as c_int;

        ishift -= 8;
        if isnonalpha(c) {
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

        *buffer.offset(iout as isize) = 0;
    }
}

/* png_chunk_error */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) -> ! {
    let mut msg: [c_char; 18 + PNG_MAX_ERROR_TEXT] = [0; 18 + PNG_MAX_ERROR_TEXT];
    if png_ptr == core::ptr::null_mut() {
        png_error(png_ptr, error_message);
    } else {
        png_format_buffer(png_ptr, msg.as_mut_ptr(), error_message);
        png_error(png_ptr, msg.as_ptr());
    }
}

/* png_chunk_warning */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_warning(
    png_ptr: png_const_structrp,
    warning_message: png_const_charp,
) {
    let mut msg: [c_char; 18 + PNG_MAX_ERROR_TEXT] = [0; 18 + PNG_MAX_ERROR_TEXT];
    if png_ptr == core::ptr::null_mut() {
        png_warning(png_ptr, warning_message);
    } else {
        png_format_buffer(png_ptr, msg.as_mut_ptr(), warning_message);
        png_warning(png_ptr, msg.as_ptr());
    }
}

/* png_chunk_benign_error */
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

/* png_chunk_report */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_report(
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
    } else if ((*png_ptr).mode & PNG_IS_READ_STRUCT) == 0 {
        if error < PNG_CHUNK_WRITE_ERROR {
            png_app_warning(png_ptr, message);
        } else {
            png_app_error(png_ptr, message);
        }
    }
}

/* png_fixed_error */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_fixed_error(png_ptr: png_const_structrp, name: png_const_charp) -> ! {
    const fixed_message: &[u8; 25] = b"fixed point overflow in \0";
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
    if name != core::ptr::null() {
        while iin < (PNG_MAX_ERROR_TEXT as c_uint - 1) && *name.add(iin as usize) != 0 {
            msg[fixed_message_ln + iin as usize] = *name.add(iin as usize);
            iin += 1;
        }
    }
    msg[fixed_message_ln + iin as usize] = 0;
    png_error(png_ptr, msg.as_ptr());
}

/* png_set_longjmp_fn */
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

        if jmp_buf_size <= core::mem::size_of::<jmp_buf>() {
            (*png_ptr).jmp_buf_ptr = core::ptr::addr_of_mut!((*png_ptr).jmp_buf_local);
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
            size = core::mem::size_of::<jmp_buf>();
            if (*png_ptr).jmp_buf_ptr != core::ptr::addr_of_mut!((*png_ptr).jmp_buf_local) {
                png_error(png_ptr, b"Libpng jmp_buf still allocated\0".as_ptr() as png_const_charp);
            }
        }

        if size != jmp_buf_size {
            png_warning(png_ptr, b"Application jmp_buf size changed\0".as_ptr() as png_const_charp);
            return core::ptr::null_mut(); /* caller will probably crash: no choice here */
        }
    }

    (*png_ptr).longjmp_fn = longjmp_fn;
    (*png_ptr).jmp_buf_ptr
}

/* png_free_jmpbuf */
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn png_free_jmpbuf(png_ptr: png_structrp) {
    if png_ptr != core::ptr::null_mut() {
        let jb: *mut jmp_buf = (*png_ptr).jmp_buf_ptr;

        if jb != core::ptr::null_mut() && (*png_ptr).jmp_buf_size > 0 {
            if jb != core::ptr::addr_of_mut!((*png_ptr).jmp_buf_local) {
                /* Make an internal, libpng, jmp_buf to return here */
                let mut free_jmp_buf: jmp_buf = [0; 25];

                if setjmp(&mut free_jmp_buf) == 0 {
                    (*png_ptr).jmp_buf_ptr = &mut free_jmp_buf; /* come back here */
                    (*png_ptr).jmp_buf_size = 0; /* stack allocation */
                    (*png_ptr).longjmp_fn = Some(png_longjmp_shim);
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

/// The C code assigns `longjmp` directly to `png_ptr->longjmp_fn`; a shim is
/// used here so the Rust type of the function pointer matches.
unsafe extern "C" fn png_longjmp_shim(env: *mut jmp_buf, val: c_int) {
    longjmp(env, val)
}

/* png_default_error */
unsafe fn png_default_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> ! {
    png_stderr_message(
        b"libpng error: \0".as_ptr() as *const c_char,
        if error_message != core::ptr::null() {
            error_message
        } else {
            b"undefined\0".as_ptr() as *const c_char
        },
    );
    png_longjmp(png_ptr, 1);
}

/* png_longjmp */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_longjmp(png_ptr: png_const_structrp, val: c_int) -> ! {
    if png_ptr != core::ptr::null_mut()
        && (*png_ptr).longjmp_fn.is_some()
        && (*png_ptr).jmp_buf_ptr != core::ptr::null_mut()
    {
        ((*png_ptr).longjmp_fn.unwrap())((*png_ptr).jmp_buf_ptr, val);
    }

    PNG_ABORT();
}

/* png_default_warning */
unsafe fn png_default_warning(png_ptr: png_const_structrp, warning_message: png_const_charp) {
    png_stderr_message(
        b"libpng warning: \0".as_ptr() as *const c_char,
        warning_message,
    );
}

/* png_set_error_fn */
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

/* png_get_error_ptr */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_error_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr == core::ptr::null_mut() {
        return core::ptr::null_mut();
    }

    (*png_ptr).error_ptr
}

/* png_safe_error */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safe_error(
    png_nonconst_ptr: png_structp,
    error_message: png_const_charp,
) {
    let png_ptr: png_const_structrp = png_nonconst_ptr;
    let image: png_imagep = (*png_ptr).error_ptr as png_imagep;

    if image != core::ptr::null_mut() {
        png_safecat(
            (*image).message.as_mut_ptr(),
            core::mem::size_of::<[c_char; 64]>(),
            0,
            error_message,
        );
        (*image).warning_or_error |= PNG_IMAGE_ERROR;

        if (*image).opaque != core::ptr::null_mut()
            && (*(*image).opaque).error_buf != core::ptr::null_mut()
        {
            longjmp((*(*image).opaque).error_buf as *mut jmp_buf, 1);
        }

        /* Missing longjmp buffer, the following is to help debugging: */
        {
            let pos: usize = png_safecat(
                (*image).message.as_mut_ptr(),
                core::mem::size_of::<[c_char; 64]>(),
                0,
                b"bad longjmp: \0".as_ptr() as png_const_charp,
            );
            png_safecat(
                (*image).message.as_mut_ptr(),
                core::mem::size_of::<[c_char; 64]>(),
                pos,
                error_message,
            );
        }
    }

    /* Here on an internal programming error. */
    abort();
}

/* png_safe_warning */
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
            core::mem::size_of::<[c_char; 64]>(),
            0,
            warning_message,
        );
        (*image).warning_or_error |= PNG_IMAGE_WARNING;
    }
}

/* png_safe_execute */
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn png_safe_execute(
    image: png_imagep,
    function: Option<unsafe extern "C" fn(png_voidp) -> c_int>,
    arg: png_voidp,
) -> c_int {
    let saved_error_buf: png_voidp = (*(*image).opaque).error_buf;
    let mut safe_jmpbuf: jmp_buf = [0; 25];

    /* Safely execute function(arg), with png_error returning back here. */
    if setjmp(&mut safe_jmpbuf) == 0 {
        let result: c_int;

        (*(*image).opaque).error_buf = (&mut safe_jmpbuf) as *mut jmp_buf as png_voidp;
        result = (function.unwrap())(arg);
        (*(*image).opaque).error_buf = saved_error_buf;

        if result != 0 {
            return 1; /* success */
        }
    }

    (*(*image).opaque).error_buf = saved_error_buf;

    if saved_error_buf == core::ptr::null_mut() {
        png_image_free(image);
    }

    0 /* failure */
}
