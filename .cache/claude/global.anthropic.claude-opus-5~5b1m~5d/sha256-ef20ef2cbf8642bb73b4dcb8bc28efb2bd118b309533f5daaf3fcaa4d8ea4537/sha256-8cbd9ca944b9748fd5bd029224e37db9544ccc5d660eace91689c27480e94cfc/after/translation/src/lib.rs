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

//! Rust translation of `c_src/src/driver.c`.
//!
//! The C library exports exactly one public symbol: `driver`.
//!
//! ```c
//! void driver(int x, int y) {
//!     div_t result = div(x, y);
//!     printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
//! }
//! ```
//!
//! To remain bit-for-bit faithful (including the platform behaviour for the
//! degenerate inputs `y == 0` and `x == INT_MIN, y == -1`, which raise SIGFPE
//! on x86-64 rather than returning a value), this translation defers to the
//! very same libc routines the C code used: `div(3)` and `printf(3)`.
//! Using libc's `printf` also guarantees identical stdout buffering behaviour,
//! so output bytes and their ordering match the C library exactly.

#![allow(non_camel_case_types)]

use std::ffi::c_int;

/// Mirror of glibc's `div_t`:
///
/// ```c
/// typedef struct { int quot; int rem; } div_t;
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
struct div_t {
    quot: c_int,
    rem: c_int,
}

extern "C" {
    /// `div_t div(int numer, int denom);` from `<stdlib.h>`.
    fn div(numer: c_int, denom: c_int) -> div_t;

    /// `int printf(const char *restrict format, ...);` from `<stdio.h>`.
    fn printf(format: *const std::ffi::c_char, ...) -> c_int;
}

/// `void driver(int x, int y);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_int, y: c_int) {
    // div_t result = div(x, y);
    let result = div(x, y);

    // printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
    printf(
        b"quotient: %d, remainder: %d\n\0".as_ptr() as *const std::ffi::c_char,
        result.quot,
        result.rem,
    );
}
