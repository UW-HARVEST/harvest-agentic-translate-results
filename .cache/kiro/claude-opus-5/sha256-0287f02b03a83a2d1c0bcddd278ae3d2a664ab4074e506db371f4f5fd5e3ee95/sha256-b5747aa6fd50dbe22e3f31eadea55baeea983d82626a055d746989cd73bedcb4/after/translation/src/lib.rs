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

//! Rust translation of `src/slicing.c`.
//!
//! Output is emitted through the C runtime's `printf` so that the bytes
//! written, and the stdio buffering behaviour, are identical to the original
//! C library (important when the caller is a C program that also writes to
//! `stdout`).

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

/*
Index into a passed string
and print the substring indexed by [*start_ptr, *stop_ptr).
If there is no start, use 0.
If there is no stop, use the end of the string.
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *mut c_int,
    stop_ptr: *mut c_int,
) -> c_int {
    // size_t len = strlen(mystr);
    let len: usize = unsafe { strlen(mystr) };

    let start: c_int;
    let stop: c_int;

    if !start_ptr.is_null() {
        start = unsafe { *start_ptr };
        // `start > len` in C: the int is converted to size_t, so the
        // comparison is unsigned. A negative `start` therefore compares
        // greater than `len`. Reproduced here with the same cast.
        if (start as usize) > len {
            unsafe {
                printf(c"Error: start is off the end of the string!\n".as_ptr());
            }
            return 1;
        }
    } else {
        start = 0;
    }

    if !stop_ptr.is_null() {
        let s = unsafe { *stop_ptr };
        // Same unsigned promotion as above.
        if (s as usize) > len {
            unsafe {
                printf(c"Error: stop is off the end of the string!\n".as_ptr());
            }
            return 1;
        }
        if s <= start {
            unsafe {
                printf(c"Error: stop must come after start!\n".as_ptr());
            }
            return 1;
        }
        stop = s;
    // single-line else statement just to make style checking sad
    } else {
        stop = len as c_int;
    }

    /* char arithmetic: skip ahead `start` characters in the array */
    unsafe {
        printf(
            c"%.*s\n".as_ptr(),
            stop.wrapping_sub(start),
            mystr.offset(start as isize),
        );
    }

    0
}
