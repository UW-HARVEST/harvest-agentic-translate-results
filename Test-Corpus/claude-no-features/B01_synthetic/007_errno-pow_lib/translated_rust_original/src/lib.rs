// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_double;

extern "C" {
    fn pow(base: c_double, exponent: c_double) -> c_double;
    fn fprintf(stream: *mut libc::FILE, format: *const libc::c_char, ...) -> libc::c_int;
    // The C global variable `stderr`.
    #[link_name = "stderr"]
    static mut STDERR_FP: *mut libc::FILE;
}

#[inline]
fn errno_get() -> libc::c_int {
    unsafe { *libc::__errno_location() }
}

#[inline]
fn errno_set(val: libc::c_int) {
    unsafe { *libc::__errno_location() = val };
}

/// Takes two arguments, a base and an exponent, and returns base^exponent
#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double {
    // Calculate power
    errno_set(0);
    let result = unsafe { pow(base, exponent) };
    let err = errno_get();
    if err == libc::EDOM {
        let fmt = b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0";
        unsafe {
            fprintf(
                STDERR_FP,
                fmt.as_ptr() as *const libc::c_char,
                base,
                exponent,
            );
        }
        return -1.0;
    } else if err == libc::ERANGE {
        let fmt = b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0";
        unsafe {
            fprintf(
                STDERR_FP,
                fmt.as_ptr() as *const libc::c_char,
                base,
                exponent,
            );
        }
        return -1.0;
    }

    result
}
