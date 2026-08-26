use std::ffi::{c_char, c_double, c_int, OsStr};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;
use std::{env, ptr};

const EDOM: c_int = 33;
const ERANGE: c_int = 34;

#[repr(C)]
struct CFile {
    _private: [u8; 0],
}

unsafe extern "C" {
    static mut stderr: *mut CFile;

    fn __errno_location() -> *mut c_int;
    fn fprintf(stream: *mut CFile, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn strtod(input: *const c_char, end: *mut *mut c_char) -> c_double;
}

#[link(name = "m")]
unsafe extern "C" {
    fn pow(base: c_double, exponent: c_double) -> c_double;
}

fn as_c_argument(argument: &OsStr) -> Vec<u8> {
    let mut bytes = argument.as_bytes().to_vec();
    bytes.push(0);
    bytes
}

fn errno() -> c_int {
    // SAFETY: libc provides a valid thread-local errno pointer.
    unsafe { *__errno_location() }
}

fn set_errno(value: c_int) {
    // SAFETY: libc provides a valid thread-local errno pointer.
    unsafe {
        *__errno_location() = value;
    }
}

fn convert(argument: &[u8]) -> (c_double, c_int, bool) {
    let mut end = ptr::null_mut();
    set_errno(0);

    // SAFETY: argument is NUL-terminated and end points to writable storage.
    let value = unsafe { strtod(argument.as_ptr().cast(), &mut end) };
    let conversion_errno = errno();
    // SAFETY: strtod sets end to a location within the live argument buffer.
    let has_trailing_input = unsafe { *end != 0 };

    (value, conversion_errno, has_trailing_input)
}

fn run() -> u8 {
    let arguments: Vec<Vec<u8>> = env::args_os()
        .map(|argument| as_c_argument(&argument))
        .collect();

    if arguments.len() != 3 {
        let program: *const c_char = arguments
            .first()
            .map_or(ptr::null(), |argument| argument.as_ptr().cast());
        // SAFETY: the format is NUL-terminated and program is a C string or NULL.
        unsafe {
            fprintf(stderr, c"Usage: %s base exponent\n".as_ptr(), program);
        }
        return 1;
    }

    let (base, base_errno, base_has_trailing_input) = convert(&arguments[1]);
    if base_errno == ERANGE {
        // SAFETY: the format and argument are NUL-terminated C strings.
        unsafe {
            fprintf(
                stderr,
                c"Range error while converting base '%s'\n".as_ptr(),
                arguments[1].as_ptr(),
            );
        }
        return 1;
    } else if base_has_trailing_input {
        // SAFETY: the format and argument are NUL-terminated C strings.
        unsafe {
            fprintf(
                stderr,
                c"Invalid numeric input for base: '%s'\n".as_ptr(),
                arguments[1].as_ptr(),
            );
        }
        return 1;
    }

    let (exponent, exponent_errno, exponent_has_trailing_input) = convert(&arguments[2]);
    if exponent_errno == ERANGE {
        // SAFETY: the format and argument are NUL-terminated C strings.
        unsafe {
            fprintf(
                stderr,
                c"Range error while converting exponent '%s'\n".as_ptr(),
                arguments[2].as_ptr(),
            );
        }
        return 1;
    } else if exponent_has_trailing_input {
        // SAFETY: the format and argument are NUL-terminated C strings.
        unsafe {
            fprintf(
                stderr,
                c"Invalid numeric input for exponent: '%s'\n".as_ptr(),
                arguments[2].as_ptr(),
            );
        }
        return 1;
    }

    set_errno(0);
    // SAFETY: pow accepts every pair of double values.
    let result = unsafe { pow(base, exponent) };
    let power_errno = errno();
    if power_errno == EDOM {
        // SAFETY: the format is NUL-terminated and both values are doubles.
        unsafe {
            fprintf(
                stderr,
                c"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n".as_ptr(),
                base,
                exponent,
            );
        }
        return 1;
    } else if power_errno == ERANGE {
        // SAFETY: the format is NUL-terminated and both values are doubles.
        unsafe {
            fprintf(
                stderr,
                c"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n".as_ptr(),
                base,
                exponent,
            );
        }
        return 1;
    }

    // SAFETY: the format is NUL-terminated and result is a double.
    unsafe {
        printf(c"Result: %.2f\n".as_ptr(), result);
    }
    0
}

fn main() -> ExitCode {
    ExitCode::from(run())
}
