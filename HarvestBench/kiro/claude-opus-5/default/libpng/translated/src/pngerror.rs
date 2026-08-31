//! Translation of pngerror.c
//!
//! Non-local exits: the application-facing mechanism is unchanged -- the app
//! calls `setjmp(png_jmpbuf(png_ptr))` and libpng calls the supplied
//! `longjmp_fn`.  The *internal* non-local exit used by the simplified API
//! (`png_safe_error` -> `png_safe_execute`) is implemented with a Rust unwind
//! instead of a second `setjmp`, which is not expressible in Rust; the observable
//! behaviour (message recorded in `image->message`, failure return, image freed)
//! is the same.

use crate::*;
use core::ffi::c_char;

pub const PNG_MAX_ERROR_TEXT: usize = 196;

const PNG_LITERAL_LEFT_SQUARE_BRACKET: c_char = 0x5b;
const PNG_LITERAL_RIGHT_SQUARE_BRACKET: c_char = 0x5d;

/* ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) -> ! {
    unsafe {
        if !png_ptr.is_null() && (*png_ptr).error_fn.is_some() {
            ((*png_ptr).error_fn.unwrap())(png_ptr as png_structrp, error_message);
        }

        /* If the custom handler doesn't exist, or if it returns,
        use the default handler, which will not return. */
        png_default_error(png_ptr, error_message)
    }
}

/// Utility to safely append strings to a buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safecat(
    buffer: png_charp,
    bufsize: usize,
    mut pos: usize,
    mut string: png_const_charp,
) -> usize {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_format_number(
    start: png_const_charp,
    end_in: png_charp,
    format: c_int,
    mut number: png_alloc_size_t,
) -> png_charp {
    unsafe {
        let mut end = end_in;
        let mut count: c_int = 0; /* number of digits output */
        let mut mincount: c_int = 1; /* minimum number required */
        let mut output: c_int = 0; /* digit output (for the fixed point format) */

        end = end.sub(1);
        *end = 0;

        /* This is written so that the loop always runs at least once, even with
         * number zero.
         */
        while (end as usize) > (start as usize) && (number != 0 || count < mincount) {
            const DIGITS: &[u8; 17] = b"0123456789ABCDEF\0";

            if format == PNG_NUMBER_FORMAT_fixed {
                /* Needs five digits (the fraction) */
                mincount = 5;
                if output != 0 || number % 10 != 0 {
                    end = end.sub(1);
                    *end = DIGITS[(number % 10) as usize] as c_char;
                    output = 1;
                }
                number /= 10;
            } else if format == PNG_NUMBER_FORMAT_02u {
                /* PNG_NUMBER_FORMAT_02u == 2, expects at least 2 digits, falls
                 * through to PNG_NUMBER_FORMAT_u.
                 */
                mincount = 2;
                end = end.sub(1);
                *end = DIGITS[(number % 10) as usize] as c_char;
                number /= 10;
            } else if format == PNG_NUMBER_FORMAT_u {
                /* == PNG_NUMBER_FORMAT_d == 1 */
                end = end.sub(1);
                *end = DIGITS[(number % 10) as usize] as c_char;
                number /= 10;
            } else if format == PNG_NUMBER_FORMAT_02x {
                mincount = 2;
                end = end.sub(1);
                *end = DIGITS[(number & 0xf) as usize] as c_char;
                number >>= 4;
            } else if format == PNG_NUMBER_FORMAT_x {
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
            if format == PNG_NUMBER_FORMAT_fixed
                && count == 5
                && (end as usize) > (start as usize)
            {
                /* End of the fraction, but maybe nothing was output?  In that
                 * case drop the decimal point.  If the number is a true zero
                 * handle that here.
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
}

/// `PNG_FORMAT_NUMBER(buffer, format, number)`
#[inline]
unsafe fn PNG_FORMAT_NUMBER(
    buffer: *mut [c_char; PNG_NUMBER_BUFFER_SIZE],
    format: c_int,
    number: png_alloc_size_t,
) -> png_charp {
    unsafe {
        let start = buffer as png_charp;
        png_format_number(start, start.add(PNG_NUMBER_BUFFER_SIZE), format, number)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_warning(
    png_ptr: png_const_structrp,
    warning_message: png_const_charp,
) {
    unsafe {
        let offset: isize = 0;
        if !png_ptr.is_null() && (*png_ptr).warning_fn.is_some() {
            ((*png_ptr).warning_fn.unwrap())(
                png_ptr as png_structrp,
                warning_message.offset(offset),
            );
        } else {
            png_default_warning(png_ptr, warning_message.offset(offset));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter(
    p: *mut png_warning_parameters,
    number: c_int,
    string: png_const_charp,
) {
    unsafe {
        if number > 0 && number <= PNG_WARNING_PARAMETER_COUNT as c_int {
            let slot = (*p).as_mut_ptr().add((number - 1) as usize);
            png_safecat(
                slot as png_charp,
                PNG_WARNING_PARAMETER_SIZE,
                0,
                string,
            );
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter_unsigned(
    p: *mut png_warning_parameters,
    number: c_int,
    format: c_int,
    value: png_alloc_size_t,
) {
    unsafe {
        let mut buffer: [c_char; PNG_NUMBER_BUFFER_SIZE] = [0; PNG_NUMBER_BUFFER_SIZE];
        let s = PNG_FORMAT_NUMBER(&mut buffer, format, value);
        png_warning_parameter(p, number, s);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter_signed(
    p: *mut png_warning_parameters,
    number: c_int,
    format: c_int,
    value: png_int_32,
) {
    unsafe {
        let mut buffer: [c_char; PNG_NUMBER_BUFFER_SIZE] = [0; PNG_NUMBER_BUFFER_SIZE];

        /* Avoid overflow by doing the negate in a png_alloc_size_t: */
        let mut u: png_alloc_size_t = value as isize as png_alloc_size_t;
        if value < 0 {
            u = (!u).wrapping_add(1);
        }

        let mut str_: png_charp = PNG_FORMAT_NUMBER(&mut buffer, format, u);

        if value < 0 && (str_ as usize) > (buffer.as_mut_ptr() as usize) {
            str_ = str_.sub(1);
            *str_ = b'-' as c_char;
        }

        png_warning_parameter(p, number, str_);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_formatted_warning(
    png_ptr: png_const_structrp,
    p: *mut png_warning_parameters,
    mut message: png_const_charp,
) {
    unsafe {
        /* The internal buffer is just 192 bytes. */
        let mut i: usize = 0;
        let mut msg: [c_char; 192] = [0; 192];

        while i < 192 - 1 && *message != 0 {
            /* '@' at end of string is now just printed. */
            if !p.is_null() && *message == b'@' as c_char && *message.add(1) != 0 {
                message = message.add(1); /* Consume the '@' */
                let parameter_char: c_int = *message as c_int;
                const VALID_PARAMETERS: &[u8; 10] = b"123456789\0";
                let mut parameter: usize = 0;

                /* Search for the parameter digit. */
                while VALID_PARAMETERS[parameter] as c_char as c_int != parameter_char
                    && VALID_PARAMETERS[parameter] != 0
                {
                    parameter += 1;
                }

                /* If the parameter digit is out of range it will just get
                 * printed.
                 */
                if parameter < PNG_WARNING_PARAMETER_COUNT {
                    /* Append this parameter */
                    let base = (*p).as_ptr().add(parameter) as *const c_char;
                    let mut parm: *const c_char = base;
                    let pend: *const c_char = base.add(PNG_WARNING_PARAMETER_SIZE);

                    while i < 192 - 1 && *parm != 0 && (parm as usize) < (pend as usize) {
                        msg[i] = *parm;
                        i += 1;
                        parm = parm.add(1);
                    }

                    /* Consume the parameter digit too: */
                    message = message.add(1);
                    continue;
                }

                /* else not a parameter; just copy the character after '@'. */
            }

            msg[i] = *message;
            i += 1;
            message = message.add(1);
        }

        /* i is always less than (sizeof msg), so: */
        msg[i] = 0;

        png_warning(png_ptr, msg.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_benign_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_app_warning(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) {
    unsafe {
        if ((*png_ptr).flags & PNG_FLAG_APP_WARNINGS_WARN) != 0 {
            png_warning(png_ptr, error_message);
        } else {
            png_error(png_ptr, error_message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_app_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) {
    unsafe {
        if ((*png_ptr).flags & PNG_FLAG_APP_ERRORS_WARN) != 0 {
            png_warning(png_ptr, error_message);
        } else {
            png_error(png_ptr, error_message);
        }
    }
}

#[inline]
fn isnonalpha(c: c_int) -> bool {
    c < 65 || c > 122 || (c > 90 && c < 97)
}

const PNG_DIGIT: [c_char; 16] = [
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
    unsafe {
        let chunk_name: png_uint_32 = (*png_ptr).chunk_name;
        let mut iout: isize = 0;
        let mut ishift: c_int = 24;

        while ishift >= 0 {
            let c: c_int = ((chunk_name >> ishift) & 0xff) as c_int;

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
                *buffer.offset(iout) = c as u8 as c_char;
                iout += 1;
            }
        }

        if error_message.is_null() {
            *buffer.offset(iout) = 0;
        } else {
            let mut iin: isize = 0;

            *buffer.offset(iout) = b':' as c_char;
            iout += 1;
            *buffer.offset(iout) = b' ' as c_char;
            iout += 1;

            while iin < (PNG_MAX_ERROR_TEXT as isize) - 1 && *error_message.offset(iin) != 0 {
                *buffer.offset(iout) = *error_message.offset(iin);
                iout += 1;
                iin += 1;
            }

            *buffer.offset(iout) = 0;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_chunk_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) -> ! {
    unsafe {
        let mut msg: [c_char; 18 + PNG_MAX_ERROR_TEXT] = [0; 18 + PNG_MAX_ERROR_TEXT];
        if png_ptr.is_null() {
            png_error(png_ptr, error_message)
        } else {
            png_format_buffer(png_ptr, msg.as_mut_ptr(), error_message);
            png_error(png_ptr, msg.as_ptr())
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_chunk_warning(
    png_ptr: png_const_structrp,
    warning_message: png_const_charp,
) {
    unsafe {
        let mut msg: [c_char; 18 + PNG_MAX_ERROR_TEXT] = [0; 18 + PNG_MAX_ERROR_TEXT];
        if png_ptr.is_null() {
            png_warning(png_ptr, warning_message);
        } else {
            png_format_buffer(png_ptr, msg.as_mut_ptr(), warning_message);
            png_warning(png_ptr, msg.as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_chunk_benign_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) {
    unsafe {
        if ((*png_ptr).flags & PNG_FLAG_BENIGN_ERRORS_WARN) != 0 {
            png_chunk_warning(png_ptr, error_message);
        } else {
            png_chunk_error(png_ptr, error_message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_chunk_report(
    png_ptr: png_const_structrp,
    message: png_const_charp,
    error: c_int,
) {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_fixed_error(
    png_ptr: png_const_structrp,
    name: png_const_charp,
) -> ! {
    unsafe {
        const FIXED_MESSAGE: &[u8; 23] = b"fixed point overflow in";
        /* "fixed point overflow in " -- 23 characters plus the trailing space */
        const FIXED_MESSAGE_FULL: &[u8; 24] = b"fixed point overflow in ";
        const FIXED_MESSAGE_LN: usize = 23;
        let _ = FIXED_MESSAGE;

        let mut msg: [c_char; FIXED_MESSAGE_LN + PNG_MAX_ERROR_TEXT] =
            [0; FIXED_MESSAGE_LN + PNG_MAX_ERROR_TEXT];
        memcpy(
            msg.as_mut_ptr() as *mut c_void,
            FIXED_MESSAGE_FULL.as_ptr() as *const c_void,
            FIXED_MESSAGE_LN,
        );
        let mut iin: usize = 0;
        if !name.is_null() {
            while iin < PNG_MAX_ERROR_TEXT - 1 && *name.add(iin) != 0 {
                msg[FIXED_MESSAGE_LN + iin] = *name.add(iin);
                iin += 1;
            }
        }
        msg[FIXED_MESSAGE_LN + iin] = 0;
        png_error(png_ptr, msg.as_ptr())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_longjmp_fn(
    png_ptr: png_structrp,
    longjmp_fn: png_longjmp_ptr,
    jmp_buf_size: usize,
) -> *mut jmp_buf {
    unsafe {
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
                    png_error(png_ptr, c"Libpng jmp_buf still allocated".as_ptr());
                }
            }

            if size != jmp_buf_size {
                png_warning(png_ptr, c"Application jmp_buf size changed".as_ptr());
                return core::ptr::null_mut();
            }
        }

        (*png_ptr).longjmp_fn = longjmp_fn;
        (*png_ptr).jmp_buf_ptr
    }
}

/// Internal `longjmp_fn` replacement used while `png_free_jmpbuf` releases the
/// application's buffer; it unwinds instead of calling `longjmp`.
unsafe extern "C-unwind" fn png_internal_longjmp(_jb: *mut jmp_buf, _val: c_int) -> ! {
    std::panic::panic_any(PngUnwind);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_jmpbuf(png_ptr: png_structrp) {
    unsafe {
        if !png_ptr.is_null() {
            let jb: *mut jmp_buf = (*png_ptr).jmp_buf_ptr;

            /* A size of 0 is used to indicate a local, stack, allocation of the
             * pointer; used here and in png.c
             */
            if !jb.is_null() && (*png_ptr).jmp_buf_size > 0 {
                /* This stuff is so that a failure to free the error control
                 * structure does not leave libpng in a state with no valid error
                 * handling: the free always succeeds, if there is an error it
                 * gets ignored.
                 */
                if jb != &raw mut (*png_ptr).jmp_buf_local {
                    let mut free_jmp_buf: jmp_buf = jmp_buf([0u8; 200]);

                    (*png_ptr).jmp_buf_ptr = &raw mut free_jmp_buf; /* come back here */
                    (*png_ptr).jmp_buf_size = 0; /* stack allocation */
                    (*png_ptr).longjmp_fn = Some(png_internal_longjmp);

                    let _ = catch_png_unwind(|| {
                        png_free(png_ptr, jb as png_voidp);
                    });
                }
            }

            /* *Always* cancel everything out: */
            (*png_ptr).jmp_buf_size = 0;
            (*png_ptr).jmp_buf_ptr = core::ptr::null_mut();
            (*png_ptr).longjmp_fn = None;
        }
    }
}

unsafe extern "C-unwind" fn png_default_error(
    png_ptr: png_const_structrp,
    error_message: png_const_charp,
) -> ! {
    unsafe {
        let undefined = c"undefined";
        fprintf(
            c_stderr,
            c"libpng error: %s".as_ptr(),
            if !error_message.is_null() {
                error_message
            } else {
                undefined.as_ptr()
            },
        );
        fprintf(c_stderr, c"\n".as_ptr());
        png_longjmp(png_ptr, 1)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_longjmp(png_ptr: png_const_structrp, val: c_int) -> ! {
    unsafe {
        if !png_ptr.is_null()
            && (*png_ptr).longjmp_fn.is_some()
            && !(*png_ptr).jmp_buf_ptr.is_null()
        {
            ((*png_ptr).longjmp_fn.unwrap())((*png_ptr).jmp_buf_ptr, val);
        }

        /* If control reaches this point, png_longjmp() must not return. */
        abort()
    }
}

unsafe extern "C" fn png_default_warning(
    png_ptr: png_const_structrp,
    warning_message: png_const_charp,
) {
    unsafe {
        fprintf(c_stderr, c"libpng warning: %s".as_ptr(), warning_message);
        fprintf(c_stderr, c"\n".as_ptr());
        PNG_UNUSED(png_ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_error_fn(
    png_ptr: png_structrp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warning_fn: png_error_ptr,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        (*png_ptr).error_ptr = error_ptr;
        (*png_ptr).error_fn = error_fn;
        (*png_ptr).warning_fn = warning_fn;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_error_ptr(png_ptr: png_const_structrp) -> png_voidp {
    unsafe {
        if png_ptr.is_null() {
            return core::ptr::null_mut();
        }

        (*png_ptr).error_ptr
    }
}

/* ------------------------------------------------------------------------- *
 *  Simplified API error handling
 * ------------------------------------------------------------------------- */

/// Marker payload for the internal unwind used in place of the second
/// `setjmp`/`longjmp` pair.
pub struct PngUnwind;

/// Runs `f`, trapping the internal unwind used by `png_safe_error`.
/// Returns `false` if the unwind happened.
pub fn catch_png_unwind<F: FnOnce()>(f: F) -> bool {
    install_silent_hook();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    match r {
        Ok(()) => true,
        Err(payload) => {
            if payload.downcast_ref::<PngUnwind>().is_some() {
                false
            } else {
                std::panic::resume_unwind(payload)
            }
        }
    }
}

static HOOK: std::sync::OnceLock<()> = std::sync::OnceLock::new();

/// libpng writes nothing to stderr when the simplified API traps an error, so
/// the default Rust panic message must be suppressed for our own payload.
fn install_silent_hook() {
    HOOK.get_or_init(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if info.payload().downcast_ref::<PngUnwind>().is_none() {
                previous(info);
            }
        }));
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_safe_error(
    png_nonconst_ptr: png_structp,
    error_message: png_const_charp,
) {
    unsafe {
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
                std::panic::panic_any(PngUnwind);
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
        abort()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_safe_warning(
    png_nonconst_ptr: png_structp,
    warning_message: png_const_charp,
) {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safe_execute(
    image: png_imagep,
    function: Option<unsafe extern "C-unwind" fn(png_voidp) -> c_int>,
    arg: png_voidp,
) -> c_int {
    unsafe {
        let saved_error_buf: png_voidp = (*(*image).opaque).error_buf;
        /* Stands in for the `jmp_buf safe_jmpbuf` of the C code: a non-NULL
         * marker telling png_safe_error that a trap is installed.
         */
        let mut safe_jmpbuf: jmp_buf = jmp_buf([0u8; 200]);

        let mut result: c_int = 0;
        let ok = catch_png_unwind(|| {
            (*(*image).opaque).error_buf = (&raw mut safe_jmpbuf) as png_voidp;
            result = (function.unwrap())(arg);
            (*(*image).opaque).error_buf = saved_error_buf;
        });

        if ok && result != 0 {
            return 1; /* success */
        }

        /* Ensure that the error_buf is always set back to the value saved
         * above:
         */
        (*(*image).opaque).error_buf = saved_error_buf;

        if saved_error_buf.is_null() {
            png_image_free(image);
        }

        0 /* failure */
    }
}
