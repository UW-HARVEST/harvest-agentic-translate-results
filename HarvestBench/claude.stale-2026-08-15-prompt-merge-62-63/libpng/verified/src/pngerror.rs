//! Translation of pngerror.c - error and warning handling.
use crate::prelude::*;

const PNG_MAX_ERROR_TEXT: usize = 196;

// This function is called whenever there is a fatal error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) -> ! {
    if !png_ptr.is_null() && (*png_ptr).error_fn.is_some() {
        ((*png_ptr).error_fn.unwrap())(png_ptr as png_structrp, error_message);
    }
    // If the custom handler doesn't exist, or if it returns, use the default
    // handler, which will not return.
    png_default_error(png_ptr, error_message)
}

/// Utility to safely append strings to a buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safecat(
    buffer: png_charp,
    bufsize: size_t,
    mut pos: size_t,
    mut string: png_const_charp,
) -> size_t {
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

/// Dump an unsigned value into a buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_format_number(
    start: png_const_charp,
    mut end: png_charp,
    format: c_int,
    mut number: png_alloc_size_t,
) -> png_charp {
    let mut count = 0;
    let mut mincount = 1;
    let mut output = 0;

    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";

    end = end.offset(-1);
    *end = 0;

    while end as *const c_char > start && (number != 0 || count < mincount) {
        match format {
            x if x == PNG_NUMBER_FORMAT_fixed => {
                mincount = 5;
                if output != 0 || number % 10 != 0 {
                    end = end.offset(-1);
                    *end = DIGITS[(number % 10) as usize] as c_char;
                    output = 1;
                }
                number /= 10;
            }
            x if x == PNG_NUMBER_FORMAT_02u => {
                mincount = 2;
                end = end.offset(-1);
                *end = DIGITS[(number % 10) as usize] as c_char;
                number /= 10;
            }
            x if x == PNG_NUMBER_FORMAT_u => {
                end = end.offset(-1);
                *end = DIGITS[(number % 10) as usize] as c_char;
                number /= 10;
            }
            x if x == PNG_NUMBER_FORMAT_02x => {
                mincount = 2;
                end = end.offset(-1);
                *end = DIGITS[(number & 0xf) as usize] as c_char;
                number >>= 4;
            }
            x if x == PNG_NUMBER_FORMAT_x => {
                end = end.offset(-1);
                *end = DIGITS[(number & 0xf) as usize] as c_char;
                number >>= 4;
            }
            _ => {
                number = 0;
            }
        }

        count += 1;

        if format == PNG_NUMBER_FORMAT_fixed && count == 5 && (end as *const c_char > start) {
            if output != 0 {
                end = end.offset(-1);
                *end = b'.' as c_char;
            } else if number == 0 {
                end = end.offset(-1);
                *end = b'0' as c_char;
            }
        }
    }

    end
}

// This function is called whenever there is a non-fatal error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning(
    png_ptr: png_const_structrp,
    warning_message: png_const_charp,
) {
    let offset = 0isize;
    if !png_ptr.is_null() && (*png_ptr).warning_fn.is_some() {
        ((*png_ptr).warning_fn.unwrap())(png_ptr as png_structrp, warning_message.offset(offset));
    } else {
        png_default_warning(png_ptr, warning_message.offset(offset));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter(
    p: *mut c_char,
    number: c_int,
    string: png_const_charp,
) {
    if number > 0 && number <= PNG_WARNING_PARAMETER_COUNT as c_int {
        let slot = p.add((number as usize - 1) * PNG_WARNING_PARAMETER_SIZE);
        png_safecat(slot, PNG_WARNING_PARAMETER_SIZE, 0, string);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter_unsigned(
    p: *mut c_char,
    number: c_int,
    format: c_int,
    value: png_alloc_size_t,
) {
    let mut buffer = [0 as c_char; PNG_NUMBER_BUFFER_SIZE];
    let s = png_format_number(
        buffer.as_ptr(),
        buffer.as_mut_ptr().add(PNG_NUMBER_BUFFER_SIZE),
        format,
        value,
    );
    png_warning_parameter(p, number, s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter_signed(
    p: *mut c_char,
    number: c_int,
    format: c_int,
    value: png_int_32,
) {
    let mut u = value as png_alloc_size_t;
    if value < 0 {
        u = (!u).wrapping_add(1);
    }

    let mut buffer = [0 as c_char; PNG_NUMBER_BUFFER_SIZE];
    let mut str_ = png_format_number(
        buffer.as_ptr(),
        buffer.as_mut_ptr().add(PNG_NUMBER_BUFFER_SIZE),
        format,
        u,
    );

    if value < 0 && str_ > buffer.as_mut_ptr() {
        str_ = str_.offset(-1);
        *str_ = b'-' as c_char;
    }

    png_warning_parameter(p, number, str_);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_formatted_warning(
    png_ptr: png_const_structrp,
    p: *mut c_char,
    mut message: png_const_charp,
) {
    let mut i: usize = 0;
    let mut msg = [0 as c_char; 192];

    while i < 192 - 1 && *message != 0 {
        if !p.is_null() && *message == b'@' as c_char && *message.offset(1) != 0 {
            message = message.add(1);
            let parameter_char = *message;
            const VALID_PARAMETERS: &[u8; 9] = b"123456789";
            let mut parameter = 0usize;

            while parameter < 9
                && VALID_PARAMETERS[parameter] as c_char != parameter_char
                && VALID_PARAMETERS[parameter] != 0
            {
                parameter += 1;
            }

            if parameter < PNG_WARNING_PARAMETER_COUNT {
                let mut parm = p.add(parameter * PNG_WARNING_PARAMETER_SIZE) as png_const_charp;
                let pend = p.add(parameter * PNG_WARNING_PARAMETER_SIZE + PNG_WARNING_PARAMETER_SIZE)
                    as png_const_charp;

                while i < 192 - 1 && *parm != 0 && parm < pend {
                    msg[i] = *parm;
                    i += 1;
                    parm = parm.add(1);
                }

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
pub unsafe extern "C" fn png_app_error(png_ptr: png_const_structrp, error_message: png_const_charp) {
    if ((*png_ptr).flags & PNG_FLAG_APP_ERRORS_WARN) != 0 {
        png_warning(png_ptr, error_message);
    } else {
        png_error(png_ptr, error_message);
    }
}

const PNG_LITERAL_LEFT_SQUARE_BRACKET: c_char = 0x5b;
const PNG_LITERAL_RIGHT_SQUARE_BRACKET: c_char = 0x5d;

const PNG_DIGIT: [c_char; 16] = [
    b'0' as c_char, b'1' as c_char, b'2' as c_char, b'3' as c_char, b'4' as c_char, b'5' as c_char,
    b'6' as c_char, b'7' as c_char, b'8' as c_char, b'9' as c_char, b'A' as c_char, b'B' as c_char,
    b'C' as c_char, b'D' as c_char, b'E' as c_char, b'F' as c_char,
];

#[inline]
fn isnonalpha(c: c_int) -> bool {
    c < 65 || c > 122 || (c > 90 && c < 97)
}

unsafe fn png_format_buffer(
    png_ptr: png_const_structrp,
    buffer: png_charp,
    error_message: png_const_charp,
) {
    let chunk_name = (*png_ptr).chunk_name;
    let mut iout = 0isize;
    let mut ishift = 24i32;

    while ishift >= 0 {
        let c = ((chunk_name >> ishift) & 0xff) as c_int;
        ishift -= 8;
        if isnonalpha(c) {
            *buffer.offset(iout) = PNG_LITERAL_LEFT_SQUARE_BRACKET;
            iout += 1;
            *buffer.offset(iout) = PNG_DIGIT[((c & 0xf0) >> 4) as usize];
            iout += 1;
            *buffer.offset(iout) = PNG_DIGIT[(c & 0x0f) as usize];
            iout += 1;
            *buffer.offset(iout) = PNG_LITERAL_RIGHT_SQUARE_BRACKET;
            iout += 1;
        } else {
            *buffer.offset(iout) = c as c_char;
            iout += 1;
        }
    }

    if error_message.is_null() {
        *buffer.offset(iout) = 0;
    } else {
        let mut iin = 0isize;
        *buffer.offset(iout) = b':' as c_char;
        iout += 1;
        *buffer.offset(iout) = b' ' as c_char;
        iout += 1;

        while iin < (PNG_MAX_ERROR_TEXT - 1) as isize && *error_message.offset(iin) != 0 {
            *buffer.offset(iout) = *error_message.offset(iin);
            iout += 1;
            iin += 1;
        }
        *buffer.offset(iout) = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) -> ! {
    let mut msg = [0 as c_char; 18 + PNG_MAX_ERROR_TEXT];
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
    let mut msg = [0 as c_char; 18 + PNG_MAX_ERROR_TEXT];
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
    const FIXED_MESSAGE: &[u8] = b"fixed point overflow in ";
    let fixed_message_ln = FIXED_MESSAGE.len();
    let mut msg = [0 as c_char; 24 + PNG_MAX_ERROR_TEXT];
    memcpy(
        msg.as_mut_ptr() as *mut c_void,
        FIXED_MESSAGE.as_ptr() as *const c_void,
        fixed_message_ln,
    );
    let mut iin = 0usize;
    if !name.is_null() {
        while iin < PNG_MAX_ERROR_TEXT - 1 && *name.add(iin) != 0 {
            msg[fixed_message_ln + iin] = *name.add(iin);
            iin += 1;
        }
    }
    msg[fixed_message_ln + iin] = 0;
    png_error(png_ptr, msg.as_ptr());
}

// setjmp support
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_longjmp_fn(
    png_ptr: png_structrp,
    longjmp_fn: png_longjmp_ptr,
    jmp_buf_size: size_t,
) -> *mut c_void {
    if png_ptr.is_null() {
        return ptr::null_mut();
    }

    if (*png_ptr).jmp_buf_ptr.is_null() {
        (*png_ptr).jmp_buf_size = 0;

        if jmp_buf_size <= core::mem::size_of::<jmp_buf>() {
            (*png_ptr).jmp_buf_ptr = &mut (*png_ptr).jmp_buf_local;
        } else {
            (*png_ptr).jmp_buf_ptr =
                png_malloc_warn(png_ptr, jmp_buf_size as png_alloc_size_t) as *mut jmp_buf;

            if (*png_ptr).jmp_buf_ptr.is_null() {
                return ptr::null_mut();
            }

            (*png_ptr).jmp_buf_size = jmp_buf_size;
        }
    } else {
        let mut size = (*png_ptr).jmp_buf_size;

        if size == 0 {
            size = core::mem::size_of::<jmp_buf>();
            if (*png_ptr).jmp_buf_ptr != &mut (*png_ptr).jmp_buf_local {
                png_error(png_ptr, c"Libpng jmp_buf still allocated".as_ptr());
            }
        }

        if size != jmp_buf_size {
            png_warning(png_ptr, c"Application jmp_buf size changed".as_ptr());
            return ptr::null_mut();
        }
    }

    (*png_ptr).longjmp_fn = longjmp_fn;
    (*png_ptr).jmp_buf_ptr as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_jmpbuf(png_ptr: png_structrp) {
    if !png_ptr.is_null() {
        let jb = (*png_ptr).jmp_buf_ptr;

        if !jb.is_null() && (*png_ptr).jmp_buf_size > 0 {
            if jb != &mut (*png_ptr).jmp_buf_local {
                // Free the allocated jmp_buf.  The C code sets up a temporary
                // setjmp to recover from an error during free; here png_free
                // does not error, so we free directly.
                png_free(png_ptr, jb as png_voidp);
            }
        }

        (*png_ptr).jmp_buf_size = 0;
        (*png_ptr).jmp_buf_ptr = ptr::null_mut();
        (*png_ptr).longjmp_fn = None;
    }
}

unsafe fn png_default_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> ! {
    let m = if error_message.is_null() {
        c"undefined".as_ptr()
    } else {
        error_message
    };
    fprintf(stderr(), c"libpng error: %s".as_ptr(), m);
    fprintf(stderr(), c"\n".as_ptr());
    png_longjmp(png_ptr, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_longjmp(png_ptr: png_const_structrp, val: c_int) -> ! {
    if !png_ptr.is_null()
        && (*png_ptr).longjmp_fn.is_some()
        && !(*png_ptr).jmp_buf_ptr.is_null()
    {
        ((*png_ptr).longjmp_fn.unwrap())((*png_ptr).jmp_buf_ptr as *mut c_void, val);
    }
    // png_longjmp must not return.
    abort()
}

unsafe fn png_default_warning(png_ptr: png_const_structrp, warning_message: png_const_charp) {
    fprintf(stderr(), c"libpng warning: %s".as_ptr(), warning_message);
    fprintf(stderr(), c"\n".as_ptr());
    let _ = png_ptr;
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_error_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr.is_null() {
        return ptr::null_mut();
    }
    (*png_ptr).error_ptr
}

// Simplified API error handling (png_image based)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safe_error(
    png_nonconst_ptr: png_structp,
    error_message: png_const_charp,
) -> ! {
    let png_ptr = png_nonconst_ptr;
    let image = (*png_ptr).error_ptr as png_imagep;

    if !image.is_null() {
        png_safecat(
            (*image).message.as_mut_ptr(),
            core::mem::size_of_val(&(*image).message),
            0,
            error_message,
        );
        (*image).warning_or_error |= PNG_IMAGE_ERROR;

        if !(*image).opaque.is_null() && !(*(*image).opaque).error_buf.is_null() {
            longjmp((*(*image).opaque).error_buf, 1);
        }

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

    abort()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safe_warning(
    png_nonconst_ptr: png_structp,
    warning_message: png_const_charp,
) {
    let png_ptr = png_nonconst_ptr;
    let image = (*png_ptr).error_ptr as png_imagep;

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
    let saved_error_buf = (*(*image).opaque).error_buf;

    // Safely execute function(arg), with png_error returning back here.  The
    // setjmp landing pad lives in the frame of png_rust_protect, which is alive
    // for exactly the duration of function(arg) - the same lifetime the C code
    // gives to its stack-allocated `safe_jmpbuf`.  png_rust_protect stores the
    // address of the pad in image->opaque->error_buf before calling the
    // function, and returns 0 (the `on_longjmp` value below) if a png_error
    // longjmp'd back to the pad.
    let result = png_rust_protect(
        core::ptr::addr_of_mut!((*(*image).opaque).error_buf),
        function,
        arg,
        0, /* on longjmp: failure */
    );

    // The function may have failed either because of a caught png_error and a
    // regular return of false or because of an uncaught png_error from the
    // function itself.  Ensure that the error_buf is always set back to the
    // value saved above:
    (*(*image).opaque).error_buf = saved_error_buf;

    if result != 0 {
        return 1; /* success */
    }

    if saved_error_buf.is_null() {
        png_image_free(image);
    }

    0
}
