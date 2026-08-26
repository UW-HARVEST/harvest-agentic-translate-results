// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Reproduces byte-identical output.

use std::ffi::c_char;
use std::ffi::c_double;
use std::ffi::c_int;

extern "C" {
    fn fprintf(stream: *mut libc::FILE, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
    fn pow(x: c_double, y: c_double) -> c_double;
    fn __errno_location() -> *mut c_int;
}

const ERANGE: c_int = 34;
const EDOM: c_int = 33;

#[inline]
unsafe fn get_errno() -> c_int {
    unsafe { *__errno_location() }
}

#[inline]
unsafe fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v };
}

#[inline]
fn stderr_stream() -> *mut libc::FILE {
    // glibc's `stderr` is a `FILE *` global. Match that here.
    extern "C" {
        static mut stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}

/// Takes two arguments, a base and an exponent, and prints base^exponent
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    unsafe {
        let stderr_p = stderr_stream();

        if argc != 3 {
            fprintf(
                stderr_p,
                b"Usage: %s base exponent\n\0".as_ptr() as *const c_char,
                *argv.offset(0),
            );
            return 1;
        }

        let mut endptr1: *mut c_char = std::ptr::null_mut();
        let mut endptr2: *mut c_char = std::ptr::null_mut();

        // Convert base
        set_errno(0);
        let base = strtod(*argv.offset(1), &mut endptr1);
        if get_errno() == ERANGE {
            fprintf(
                stderr_p,
                b"Range error while converting base '%s'\n\0".as_ptr() as *const c_char,
                *argv.offset(1),
            );
            return 1;
        } else if *endptr1 != 0 {
            fprintf(
                stderr_p,
                b"Invalid numeric input for base: '%s'\n\0".as_ptr() as *const c_char,
                *argv.offset(1),
            );
            return 1;
        }

        // Convert exponent
        set_errno(0);
        let exponent = strtod(*argv.offset(2), &mut endptr2);
        if get_errno() == ERANGE {
            fprintf(
                stderr_p,
                b"Range error while converting exponent '%s'\n\0".as_ptr() as *const c_char,
                *argv.offset(2),
            );
            return 1;
        } else if *endptr2 != 0 {
            fprintf(
                stderr_p,
                b"Invalid numeric input for exponent: '%s'\n\0".as_ptr() as *const c_char,
                *argv.offset(2),
            );
            return 1;
        }

        // Calculate power
        set_errno(0);
        let result = pow(base, exponent);
        if get_errno() == EDOM {
            fprintf(
                stderr_p,
                b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0"
                    .as_ptr() as *const c_char,
                base,
                exponent,
            );
            return 1;
        } else if get_errno() == ERANGE {
            fprintf(
                stderr_p,
                b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0".as_ptr()
                    as *const c_char,
                base,
                exponent,
            );
            return 1;
        }

        printf(b"Result: %.2f\n\0".as_ptr() as *const c_char, result);
        0
    }
}
