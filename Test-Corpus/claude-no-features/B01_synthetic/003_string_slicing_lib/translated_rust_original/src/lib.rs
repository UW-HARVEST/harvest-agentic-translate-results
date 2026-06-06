#![allow(non_snake_case)]

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn strlen(s: *const c_char) -> usize;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Index into a passed string
/// and print the substring indexed by [*start_ptr, *stop_ptr).
/// If there is no start, use 0.
/// If there is no stop, use the end of the string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *const c_int,
    stop_ptr: *const c_int,
) -> c_int {
    let len: usize = unsafe { strlen(mystr) };

    let start: c_int;
    let stop: c_int;

    if !start_ptr.is_null() {
        start = unsafe { *start_ptr };
        if (start as i64) > (len as i64) {
            let fmt = b"Error: start is off the end of the string!\n\0".as_ptr() as *const c_char;
            unsafe { printf(fmt) };
            return 1;
        }
    } else {
        start = 0;
    }

    if !stop_ptr.is_null() {
        stop = unsafe { *stop_ptr };
        if (stop as i64) > (len as i64) {
            let fmt = b"Error: stop is off the end of the string!\n\0".as_ptr() as *const c_char;
            unsafe { printf(fmt) };
            return 1;
        }
        if stop <= start {
            let fmt = b"Error: stop must come after start!\n\0".as_ptr() as *const c_char;
            unsafe { printf(fmt) };
            return 1;
        }
    } else {
        stop = len as c_int;
    }

    // char arithmetic: skip ahead `start` characters in the array
    let fmt = b"%.*s\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(
            fmt,
            (stop - start) as c_int,
            mystr.offset(start as isize) as *const c_char,
        )
    };

    0
}
