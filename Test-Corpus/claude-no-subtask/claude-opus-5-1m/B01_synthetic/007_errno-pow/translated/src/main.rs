// Translated from c_src/src/main.c
// Takes two arguments, a base and an exponent, and prints base^exponent.

use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int};
use std::ptr;

extern "C" {
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
    fn pow(x: c_double, y: c_double) -> c_double;
    fn __errno_location() -> *mut c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut libc::FILE, fmt: *const c_char, ...) -> c_int;
}

// errno value constants (Linux glibc)
const ERANGE: c_int = 34;
const EDOM: c_int = 33;

fn errno_get() -> c_int {
    unsafe { *__errno_location() }
}

fn errno_set(val: c_int) {
    unsafe { *__errno_location() = val };
}

fn main() {
    let exit_code = run();
    std::process::exit(exit_code);
}

fn run() -> i32 {
    // Collect raw argv as CStrings to preserve byte-exact bytes
    let args_os: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = args_os.len();

    // Build owned CString copies of the args. Using as_bytes via OsStrExt.
    use std::os::unix::ffi::OsStrExt;
    let arg_cstrings: Vec<CString> = args_os
        .iter()
        .map(|os| CString::new(os.as_bytes()).expect("argv contained NUL byte"))
        .collect();

    let argv0 = &arg_cstrings[0];

    if argc != 3 {
        // fprintf(stderr, "Usage: %s base exponent\n", argv[0]);
        unsafe {
            let fmt = CString::new("Usage: %s base exponent\n").unwrap();
            fprintf(libc_stderr(), fmt.as_ptr(), argv0.as_ptr());
        }
        return 1;
    }

    // Convert base
    errno_set(0);
    let mut endptr1: *mut c_char = ptr::null_mut();
    let base: c_double = unsafe { strtod(arg_cstrings[1].as_ptr(), &mut endptr1) };
    if errno_get() == ERANGE {
        unsafe {
            let fmt = CString::new("Range error while converting base '%s'\n").unwrap();
            fprintf(libc_stderr(), fmt.as_ptr(), arg_cstrings[1].as_ptr());
        }
        return 1;
    } else if unsafe { *endptr1 } != 0 {
        unsafe {
            let fmt = CString::new("Invalid numeric input for base: '%s'\n").unwrap();
            fprintf(libc_stderr(), fmt.as_ptr(), arg_cstrings[1].as_ptr());
        }
        return 1;
    }

    // Convert exponent
    errno_set(0);
    let mut endptr2: *mut c_char = ptr::null_mut();
    let exponent: c_double = unsafe { strtod(arg_cstrings[2].as_ptr(), &mut endptr2) };
    if errno_get() == ERANGE {
        unsafe {
            let fmt = CString::new("Range error while converting exponent '%s'\n").unwrap();
            fprintf(libc_stderr(), fmt.as_ptr(), arg_cstrings[2].as_ptr());
        }
        return 1;
    } else if unsafe { *endptr2 } != 0 {
        unsafe {
            let fmt = CString::new("Invalid numeric input for exponent: '%s'\n").unwrap();
            fprintf(libc_stderr(), fmt.as_ptr(), arg_cstrings[2].as_ptr());
        }
        return 1;
    }

    // Calculate power
    errno_set(0);
    let result: c_double = unsafe { pow(base, exponent) };
    if errno_get() == EDOM {
        unsafe {
            let fmt = CString::new(
                "Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n",
            )
            .unwrap();
            fprintf(libc_stderr(), fmt.as_ptr(), base, exponent);
        }
        return 1;
    } else if errno_get() == ERANGE {
        unsafe {
            let fmt =
                CString::new("Range error: pow(%.2f, %.2f) caused overflow or underflow.\n")
                    .unwrap();
            fprintf(libc_stderr(), fmt.as_ptr(), base, exponent);
        }
        return 1;
    }

    unsafe {
        let fmt = CString::new("Result: %.2f\n").unwrap();
        printf(fmt.as_ptr(), result);
    }
    0
}

fn libc_stderr() -> *mut libc::FILE {
    extern "C" {
        static mut stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}
