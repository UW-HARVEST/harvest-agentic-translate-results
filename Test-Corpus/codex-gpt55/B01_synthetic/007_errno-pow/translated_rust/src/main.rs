use std::env;
use std::ffi::{CString, OsString};
use std::os::raw::{c_char, c_double, c_int};
use std::os::unix::ffi::OsStringExt;
use std::process;

const EDOM: c_int = 33;
const ERANGE: c_int = 34;

#[repr(C)]
struct File {
    _private: [u8; 0],
}

#[link(name = "m")]
extern "C" {
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
    fn pow(x: c_double, y: c_double) -> c_double;
    fn printf(format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut File, format: *const c_char, ...) -> c_int;
    static mut stderr: *mut File;
}

extern "C" {
    fn __errno_location() -> *mut c_int;
}

fn cstring(arg: OsString) -> CString {
    CString::new(arg.into_vec()).unwrap()
}

unsafe fn errno() -> c_int {
    *__errno_location()
}

unsafe fn set_errno(value: c_int) {
    *__errno_location() = value;
}

fn main() {
    let args: Vec<CString> = env::args_os().map(cstring).collect();

    unsafe {
        if args.len() != 3 {
            fprintf(
                stderr,
                b"Usage: %s base exponent\n\0".as_ptr() as *const c_char,
                args[0].as_ptr(),
            );
            process::exit(1);
        }

        let mut endptr1: *mut c_char = std::ptr::null_mut();
        let mut endptr2: *mut c_char = std::ptr::null_mut();

        set_errno(0);
        let base = strtod(args[1].as_ptr(), &mut endptr1);
        if errno() == ERANGE {
            fprintf(
                stderr,
                b"Range error while converting base '%s'\n\0".as_ptr() as *const c_char,
                args[1].as_ptr(),
            );
            process::exit(1);
        } else if *endptr1 != 0 {
            fprintf(
                stderr,
                b"Invalid numeric input for base: '%s'\n\0".as_ptr() as *const c_char,
                args[1].as_ptr(),
            );
            process::exit(1);
        }

        set_errno(0);
        let exponent = strtod(args[2].as_ptr(), &mut endptr2);
        if errno() == ERANGE {
            fprintf(
                stderr,
                b"Range error while converting exponent '%s'\n\0".as_ptr() as *const c_char,
                args[2].as_ptr(),
            );
            process::exit(1);
        } else if *endptr2 != 0 {
            fprintf(
                stderr,
                b"Invalid numeric input for exponent: '%s'\n\0".as_ptr() as *const c_char,
                args[2].as_ptr(),
            );
            process::exit(1);
        }

        set_errno(0);
        let result = pow(base, exponent);
        if errno() == EDOM {
            fprintf(
                stderr,
                b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0"
                    .as_ptr() as *const c_char,
                base,
                exponent,
            );
            process::exit(1);
        } else if errno() == ERANGE {
            fprintf(
                stderr,
                b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0".as_ptr()
                    as *const c_char,
                base,
                exponent,
            );
            process::exit(1);
        }

        printf(
            b"Result: %.2f\n\0".as_ptr() as *const c_char,
            result,
        );
    }
}
