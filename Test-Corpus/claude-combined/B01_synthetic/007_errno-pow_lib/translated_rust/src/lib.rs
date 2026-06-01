// Copyright 2025 MIT Lincoln Laboratory
// Rust translation of pow.c — byte-identical behavior.

use std::ffi::{c_int, c_void};

// Linux errno values
const EDOM: c_int = 33;
const ERANGE: c_int = 34;

extern "C" {
    fn pow(base: f64, exponent: f64) -> f64;
    fn __errno_location() -> *mut c_int;
    fn fprintf(stream: *mut c_void, format: *const u8, ...) -> c_int;

    static stderr: *mut c_void;
}

#[inline]
fn errno_get() -> c_int {
    unsafe { *__errno_location() }
}

#[inline]
fn errno_set(v: c_int) {
    unsafe {
        *__errno_location() = v;
    }
}

/// Takes two arguments, a base and an exponent, and returns base^exponent
#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: f64, exponent: f64) -> f64 {
    // Calculate power
    errno_set(0);
    let result = unsafe { pow(base, exponent) };
    let e = errno_get();
    if e == EDOM {
        unsafe {
            fprintf(
                stderr,
                b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0".as_ptr(),
                base,
                exponent,
            );
        }
        return -1.0;
    } else if e == ERANGE {
        unsafe {
            fprintf(
                stderr,
                b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0".as_ptr(),
                base,
                exponent,
            );
        }
        return -1.0;
    }

    result
}
