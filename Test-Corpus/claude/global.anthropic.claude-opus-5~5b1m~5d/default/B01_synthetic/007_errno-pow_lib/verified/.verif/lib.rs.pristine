// Rust translation of the C `pow` library (c_src/src/pow.c).
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_double, c_int, c_void};

/// Opaque `FILE` type from <stdio.h>.
pub type FILE = c_void;

extern "C" {
    /// libc's per-thread `errno` location (glibc / musl).
    fn __errno_location() -> *mut c_int;

    /// `stderr` stream from <stdio.h>.
    static mut stderr: *mut FILE;

    fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;

    /// libm's `pow`, so that error semantics (including `errno` side effects)
    /// are bit-for-bit identical with the C original.
    #[link_name = "pow"]
    fn libm_pow(base: c_double, exponent: c_double) -> c_double;
}

/// <errno.h> values on Linux.
const EDOM: c_int = 33;
const ERANGE: c_int = 34;

#[inline]
unsafe fn errno_get() -> c_int {
    *__errno_location()
}

#[inline]
unsafe fn errno_set(value: c_int) {
    *__errno_location() = value;
}

const DOMAIN_ERROR_FMT: &[u8] =
    b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0";
const RANGE_ERROR_FMT: &[u8] = b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0";

/// Takes two arguments, a base and an exponent, and returns base^exponent
#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double {
    unsafe {
        // Calculate power
        errno_set(0);
        let result = libm_pow(base, exponent);
        let err = errno_get();
        if err == EDOM {
            fprintf(
                stderr,
                DOMAIN_ERROR_FMT.as_ptr() as *const c_char,
                base,
                exponent,
            );
            return -1.0;
        } else if err == ERANGE {
            fprintf(
                stderr,
                RANGE_ERROR_FMT.as_ptr() as *const c_char,
                base,
                exponent,
            );
            return -1.0;
        }

        result
    }
}
