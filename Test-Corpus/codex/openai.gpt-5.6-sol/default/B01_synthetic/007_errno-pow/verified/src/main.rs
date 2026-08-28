use std::env;
use std::ffi::{c_char, c_double, c_int, c_void, CString};
use std::os::unix::ffi::OsStringExt;
use std::process;
use std::ptr;

const EDOM: c_int = 33;
const ERANGE: c_int = 34;

extern "C" {
    fn strtod(input: *const c_char, end: *mut *mut c_char) -> c_double;
    fn __errno_location() -> *mut c_int;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    static mut stderr: *mut c_void;
}

#[link(name = "m")]
extern "C" {
    fn pow(base: c_double, exponent: c_double) -> c_double;
}

fn main() {
    let args: Vec<CString> = env::args_os()
        .map(|arg| CString::new(arg.into_vec()).expect("argument contains a NUL byte"))
        .collect();

    unsafe {
        if args.len() != 3 {
            fprintf(
                stderr,
                b"Usage: %s base exponent\n\0".as_ptr().cast(),
                args[0].as_ptr(),
            );
            process::exit(1);
        }

        let mut endptr1 = ptr::null_mut();
        *__errno_location() = 0;
        let base = strtod(args[1].as_ptr(), &mut endptr1);
        if *__errno_location() == ERANGE {
            fprintf(
                stderr,
                b"Range error while converting base '%s'\n\0"
                    .as_ptr()
                    .cast(),
                args[1].as_ptr(),
            );
            process::exit(1);
        } else if *endptr1 != 0 {
            fprintf(
                stderr,
                b"Invalid numeric input for base: '%s'\n\0".as_ptr().cast(),
                args[1].as_ptr(),
            );
            process::exit(1);
        }

        let mut endptr2 = ptr::null_mut();
        *__errno_location() = 0;
        let exponent = strtod(args[2].as_ptr(), &mut endptr2);
        if *__errno_location() == ERANGE {
            fprintf(
                stderr,
                b"Range error while converting exponent '%s'\n\0"
                    .as_ptr()
                    .cast(),
                args[2].as_ptr(),
            );
            process::exit(1);
        } else if *endptr2 != 0 {
            fprintf(
                stderr,
                b"Invalid numeric input for exponent: '%s'\n\0"
                    .as_ptr()
                    .cast(),
                args[2].as_ptr(),
            );
            process::exit(1);
        }

        *__errno_location() = 0;
        let result = pow(base, exponent);
        if *__errno_location() == EDOM {
            fprintf(
                stderr,
                b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0"
                    .as_ptr()
                    .cast(),
                base,
                exponent,
            );
            process::exit(1);
        } else if *__errno_location() == ERANGE {
            fprintf(
                stderr,
                b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0"
                    .as_ptr()
                    .cast(),
                base,
                exponent,
            );
            process::exit(1);
        }

        printf(b"Result: %.2f\n\0".as_ptr().cast(), result);
    }
}
