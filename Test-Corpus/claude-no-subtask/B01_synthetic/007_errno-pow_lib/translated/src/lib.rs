// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT

use std::os::raw::{c_char, c_double, c_int};

unsafe extern "C" {
    fn pow(base: c_double, exponent: c_double) -> c_double;
    fn fprintf(stream: *mut libc::FILE, format: *const c_char, ...) -> c_int;
}

fn errno_ptr() -> *mut c_int {
    unsafe { libc::__errno_location() }
}

fn stderr_handle() -> *mut libc::FILE {
    // On Linux/glibc, stderr is a macro for stderr (an extern variable).
    // libc exposes it via the `stderr` static or function. The `libc` crate
    // exposes `stderr` as an `unsafe extern "C" { static mut stderr: *mut FILE; }`.
    unsafe extern "C" {
        static mut stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}

// Takes two arguments, a base and an exponent, and returns base^exponent
#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double {
    unsafe {
        // Calculate power
        let errno = errno_ptr();
        *errno = 0;
        let result = pow(base, exponent);

        let domain_fmt =
            b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0";
        let range_fmt = b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0";

        if *errno == libc::EDOM {
            fprintf(
                stderr_handle(),
                domain_fmt.as_ptr() as *const c_char,
                base,
                exponent,
            );
            return -1.0;
        } else if *errno == libc::ERANGE {
            fprintf(
                stderr_handle(),
                range_fmt.as_ptr() as *const c_char,
                base,
                exponent,
            );
            return -1.0;
        }

        result
    }
}
