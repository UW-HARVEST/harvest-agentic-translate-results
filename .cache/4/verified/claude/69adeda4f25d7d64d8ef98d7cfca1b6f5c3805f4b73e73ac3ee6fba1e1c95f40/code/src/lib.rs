// Rust translation of the C library in c_src/ (pow).
//
// Original copyright notice from the C sources:
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

use std::ffi::{CStr, c_char, c_double, c_int, c_void};

// <errno.h> values on Linux/glibc (as used by the original C code).
const EDOM: c_int = 33;
const ERANGE: c_int = 34;

unsafe extern "C" {
    /// glibc's thread-local `errno` accessor; `errno` is a macro expanding to
    /// `*__errno_location()`.
    fn __errno_location() -> *mut c_int;

    /// The C standard library `stderr` stream, used so that output goes through
    /// exactly the same FILE* / buffering path as the C implementation.
    static stderr: *mut c_void;

    /// Variadic `fprintf`, used to reproduce C's `%.2f` formatting byte for byte
    /// (including `inf` / `nan` spellings and locale handling).
    #[link_name = "fprintf"]
    fn c_fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
}

#[cfg_attr(target_os = "linux", link(name = "m"))]
unsafe extern "C" {
    /// libm's `pow`, called directly (rather than `f64::powf`) so that `errno`
    /// is set exactly as in the C implementation.
    #[link_name = "pow"]
    fn c_pow(base: c_double, exponent: c_double) -> c_double;
}

const DOMAIN_ERROR_FMT: &CStr =
    c"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n";
const RANGE_ERROR_FMT: &CStr = c"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n";

// Takes two arguments, a base and an exponent, and returns base^exponent
#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double {
    // Calculate power
    unsafe {
        let errno_ptr = __errno_location();
        *errno_ptr = 0;
        let result = c_pow(base, exponent);
        let err = *errno_ptr;
        if err == EDOM {
            c_fprintf(
                stderr,
                DOMAIN_ERROR_FMT.as_ptr(),
                base,
                exponent,
            );
            return -1.0;
        } else if err == ERANGE {
            c_fprintf(
                stderr,
                RANGE_ERROR_FMT.as_ptr(),
                base,
                exponent,
            );
            return -1.0;
        }

        result
    }
}
