// Rust translation of the C library in c_src/ (String_Slice).
//
// Original C source: c_src/src/slicing.c, public header: c_src/include/slicing.h
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

use std::ffi::{c_char, c_int};

// The C code writes its output with printf(3) and measures strings with
// strlen(3).  We call the very same libc entry points so that the emitted bytes
// -- and the stdio buffering behaviour observed by any C caller sharing this
// process' stdout -- are identical to the original library.
extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

/// Index into a passed string and print the substring indexed by
/// `[*start_ptr, *stop_ptr)`.
/// If there is no start, use 0.
/// If there is no stop, use the end of the string.
///
/// Faithful translation of:
///     int slice(char *mystr, int *start_ptr, int *stop_ptr)
///
/// Note on fidelity: in the original C, `start`/`stop` are `int` while `len` is
/// `size_t`.  The comparisons `start > len` and `stop > len` therefore undergo
/// the usual arithmetic conversions, converting the (possibly negative) `int`
/// to `size_t`.  A negative index consequently becomes a huge unsigned value
/// and is rejected by the "off the end of the string" check.  That behaviour is
/// reproduced here exactly (via `as isize as usize`) rather than "fixed".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *mut c_int,
    stop_ptr: *mut c_int,
) -> c_int {
    // size_t len = strlen(mystr);
    let len: usize = strlen(mystr);

    let start: c_int;
    let stop: c_int;

    if !start_ptr.is_null() {
        start = *start_ptr;
        // if (start > len)  --  int promoted to size_t
        if (start as isize as usize) > len {
            printf(b"Error: start is off the end of the string!\n\0".as_ptr() as *const c_char);
            return 1;
        }
    } else {
        start = 0;
    }

    if !stop_ptr.is_null() {
        let s = *stop_ptr;
        // if (stop > len)  --  int promoted to size_t
        if (s as isize as usize) > len {
            printf(b"Error: stop is off the end of the string!\n\0".as_ptr() as *const c_char);
            return 1;
        }
        // if (stop <= start)  --  signed int comparison
        if s <= start {
            printf(b"Error: stop must come after start!\n\0".as_ptr() as *const c_char);
            return 1;
        }
        stop = s;
    // single-line else statement just to make style checking sad
    } else {
        // stop = len;  --  size_t truncated to int
        stop = len as c_int;
    }

    /* char arithmetic: skip ahead `start` characters in the array */
    // printf("%.*s\n", stop - start, mystr + start);
    printf(
        b"%.*s\n\0".as_ptr() as *const c_char,
        stop.wrapping_sub(start),
        mystr.wrapping_offset(start as isize) as *const c_char,
    );

    0
}
