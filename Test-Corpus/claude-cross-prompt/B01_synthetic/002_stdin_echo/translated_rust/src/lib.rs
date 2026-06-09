// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Behavior must match the original byte-for-byte.

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;

extern "C" {
    static stdin: *mut c_void;
    static stdout: *mut c_void;
    fn fgets(s: *mut c_char, n: c_int, stream: *mut c_void) -> *mut c_char;
    fn fputs(s: *const c_char, stream: *mut c_void) -> c_int;
}

/// interactive echo; ignores arguments, copies stdin to stdout
#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut text: [c_char; 128] = [0; 128];

    unsafe {
        while !fgets(text.as_mut_ptr(), 128, stdin).is_null() {
            fputs(text.as_ptr(), stdout);
        }
    }
    0
}
