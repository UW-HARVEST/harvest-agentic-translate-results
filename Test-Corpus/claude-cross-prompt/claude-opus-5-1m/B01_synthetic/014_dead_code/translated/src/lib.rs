// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust preserving byte-identical output.

use std::ffi::c_char;
use std::os::raw::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // printf("%s\n", line)
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

#[allow(dead_code)]
fn helper_bad() {
    let s = b"helperBad()\0".as_ptr() as *const c_char;
    unsafe { printLine(s) };
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let s = b"bad()\0".as_ptr() as *const c_char;
    unsafe { printLine(s) };
}

fn helper_good() {
    let s = b"helperGood()\0".as_ptr() as *const c_char;
    unsafe { printLine(s) };
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let s = b"good()\0".as_ptr() as *const c_char;
    unsafe { printLine(s) };
    helper_good();
}

#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    let s1 = b"Calling good()...\0".as_ptr() as *const c_char;
    let s2 = b"Finished good()\0".as_ptr() as *const c_char;
    let s3 = b"Calling bad()...\0".as_ptr() as *const c_char;
    let s4 = b"Finished bad()\0".as_ptr() as *const c_char;

    unsafe {
        printLine(s1);
        good();
        printLine(s2);
        printLine(s3);
        bad();
        printLine(s4);
    }

    0
}
