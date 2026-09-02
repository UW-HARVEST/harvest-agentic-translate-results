// Rust translation of c_src/src/slicing.c (MIT Lincoln Laboratory, 2025).
//
// The C library exports exactly one public symbol, `slice`, declared in
// include/slicing.h as:
//
//     int slice(char *mystr, int *start_ptr, int *stop_ptr);
//
// There are no namespace/renaming macros in the public header, so the final
// linker symbol is plain `slice`.
//
// Output is produced through libc's `printf` so that formatting, stdout
// buffering and flush ordering are byte-for-byte identical to the C library.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn strlen(s: *const c_char) -> usize;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

const ERR_START: &[u8] = b"Error: start is off the end of the string!\n\0";
const ERR_STOP_OFF_END: &[u8] = b"Error: stop is off the end of the string!\n\0";
const ERR_STOP_ORDER: &[u8] = b"Error: stop must come after start!\n\0";
const FMT_SLICE: &[u8] = b"%.*s\n\0";

/// Index into a passed string and print the substring indexed by
/// `[*start_ptr, *stop_ptr)`.
/// If there is no start, use 0. If there is no stop, use the end of the string.
///
/// Faithful translation notes (C behaviour reproduced verbatim, bugs included):
///
/// * `len` is a `size_t`. The C comparisons `start > len` and `stop > len`
///   therefore undergo the usual arithmetic conversions and promote the *signed*
///   `int` to `size_t`. A negative index wraps to a huge unsigned value and
///   trips the "off the end of the string" branch. `x as usize` in Rust performs
///   the same sign-extending reinterpretation.
/// * `stop <= start` is a plain signed `int` comparison, done *after* the
///   `stop > len` check. Order of the checks is preserved.
/// * When `stop_ptr` is NULL, `stop = len` truncates `size_t` to `int`.
/// * `stop - start` is computed in `int` and passed as the `%.*s` precision.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *mut c_int,
    stop_ptr: *mut c_int,
) -> c_int {
    unsafe {
        let len: usize = strlen(mystr);

        let start: c_int;
        let stop: c_int;

        if !start_ptr.is_null() {
            start = *start_ptr;
            // C: `start > len` -> int promoted to size_t.
            if (start as usize) > len {
                printf(ERR_START.as_ptr() as *const c_char);
                return 1;
            }
        } else {
            start = 0;
        }

        if !stop_ptr.is_null() {
            stop = *stop_ptr;
            // C: `stop > len` -> int promoted to size_t.
            if (stop as usize) > len {
                printf(ERR_STOP_OFF_END.as_ptr() as *const c_char);
                return 1;
            }
            // C: signed comparison.
            if stop <= start {
                printf(ERR_STOP_ORDER.as_ptr() as *const c_char);
                return 1;
            }
        } else {
            // C: `stop = len` truncates size_t -> int.
            stop = len as c_int;
        }

        // char arithmetic: skip ahead `start` characters in the array
        printf(
            FMT_SLICE.as_ptr() as *const c_char,
            stop.wrapping_sub(start),
            mystr.offset(start as isize),
        );

        0
    }
}
