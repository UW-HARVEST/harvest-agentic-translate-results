// Rust translation of c_src/src/driver.c
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

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};

extern "C" {
    /// C `strchr` from libc. Used directly so that the search semantics
    /// (including the `c == 0` case, where the terminating NUL is matched)
    /// are byte-for-byte identical to the original C code.
    fn strchr(s: *const c_char, c: c_int) -> *const c_char;

    /// C `printf` from libc. Used directly so that stdout buffering,
    /// flushing order and formatting exactly match the original C library.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `int foo(const char *in, char c)`
///
/// Counts how many times `c` occurs in the NUL-terminated string `in`,
/// mirroring the original loop:
///
/// ```c
/// int res = 0;
/// for (const char *s = in; s = strchr(s, c); s++) { res++; }
/// return res;
/// ```
///
/// # Safety
/// `in_` must be a valid pointer to a NUL-terminated C string (same
/// requirement as the original C function).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo(in_: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;

    // The C code passes a `char` to strchr, which is promoted to `int`.
    // On the reference platform `char` is signed, so a negative value is
    // passed through as-is; strchr compares after converting to `char`.
    let needle = c as c_int;

    let mut s: *const c_char = in_;
    loop {
        s = strchr(s, needle);
        if s.is_null() {
            break;
        }
        res = res.wrapping_add(1);
        // Loop increment `s++` happens after the body.
        s = s.add(1);
    }

    res
}

/// `void driver(const char *in)`
///
/// # Safety
/// `in_` must be a valid pointer to a NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    printf(b"A: %d\n\0".as_ptr() as *const c_char, foo(in_, b'A' as c_char));
    printf(b"x: %d\n\0".as_ptr() as *const c_char, foo(in_, b'x' as c_char));
}
