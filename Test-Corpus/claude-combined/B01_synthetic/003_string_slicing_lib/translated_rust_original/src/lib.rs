// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// Rust translation of c_src/src/slicing.c
// Produces byte-identical stdout output to the original C library.

#![allow(non_snake_case)]

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn strlen(s: *const c_char) -> usize;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `int slice(char *mystr, int *start_ptr, int *stop_ptr);`
///
/// Index into a passed string and print the substring indexed by
/// `[*start_ptr, *stop_ptr)`. If `start_ptr` is NULL, use 0.
/// If `stop_ptr` is NULL, use the end of the string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *mut c_int,
    stop_ptr: *mut c_int,
) -> c_int {
    let len: usize = unsafe { strlen(mystr) };

    let start: c_int;
    let stop: c_int;

    if !start_ptr.is_null() {
        start = unsafe { *start_ptr };
        // C: `if (start > len)` — `start` (int) is promoted to size_t for the
        // comparison via the usual arithmetic conversions. We replicate that
        // here by casting `start` to `usize` (matching the C semantics: a
        // negative `start` becomes a huge unsigned value and triggers the
        // error branch).
        if (start as usize) > len {
            unsafe {
                printf(b"Error: start is off the end of the string!\n\0".as_ptr() as *const c_char);
            }
            return 1;
        }
    } else {
        start = 0;
    }

    if !stop_ptr.is_null() {
        stop = unsafe { *stop_ptr };
        if (stop as usize) > len {
            unsafe {
                printf(b"Error: stop is off the end of the string!\n\0".as_ptr() as *const c_char);
            }
            return 1;
        }
        if stop <= start {
            unsafe {
                printf(b"Error: stop must come after start!\n\0".as_ptr() as *const c_char);
            }
            return 1;
        }
    } else {
        // C: `stop = len;` — narrowing size_t -> int. Replicate exactly.
        stop = len as c_int;
    }

    // C: `printf("%.*s\n", stop - start, mystr + start);`
    // char arithmetic: skip ahead `start` characters in the array.
    unsafe {
        printf(
            b"%.*s\n\0".as_ptr() as *const c_char,
            stop - start,
            mystr.offset(start as isize),
        );
    }

    0
}
