#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

extern "C" {
    fn strlen(s: *const c_char) -> usize;
    fn printf(format: *const c_char, ...) -> c_int;
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
    let len: usize = strlen(mystr);

    let start: c_int;
    let stop: c_int;

    if !start_ptr.is_null() {
        start = *start_ptr;
        // In C, `start > len` promotes int to size_t, so negative ints
        // become huge unsigned values and fail this check. In Rust,
        // `c_int as usize` sign-extends, matching C semantics.
        if (start as usize) > len {
            printf(b"Error: start is off the end of the string!\n\0".as_ptr() as *const c_char);
            return 1;
        }
    } else {
        start = 0;
    }

    if !stop_ptr.is_null() {
        stop = *stop_ptr;
        if (stop as usize) > len {
            printf(b"Error: stop is off the end of the string!\n\0".as_ptr() as *const c_char);
            return 1;
        }
        if stop <= start {
            printf(b"Error: stop must come after start!\n\0".as_ptr() as *const c_char);
            return 1;
        }
    } else {
        // Implicit conversion from size_t to int as in C.
        stop = len as c_int;
    }

    /* char arithmetic: skip ahead `start` characters in the array */
    printf(
        b"%.*s\n\0".as_ptr() as *const c_char,
        stop.wrapping_sub(start),
        mystr.offset(start as isize),
    );

    0
}
