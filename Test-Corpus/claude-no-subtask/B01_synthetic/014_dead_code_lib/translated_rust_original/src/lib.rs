// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust to produce byte-identical output.

use std::os::raw::c_char;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> std::os::raw::c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // Use libc printf to match C output exactly.
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

#[allow(dead_code)]
fn helper_bad() {
    let s = b"helperBad()\0";
    printLine(s.as_ptr() as *const c_char);
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let s = b"bad()\0";
    printLine(s.as_ptr() as *const c_char);
}

fn helper_good() {
    let s = b"helperGood()\0";
    printLine(s.as_ptr() as *const c_char);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let s = b"good()\0";
    printLine(s.as_ptr() as *const c_char);
    helper_good();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    let s1 = b"Calling good()...\0";
    printLine(s1.as_ptr() as *const c_char);
    good();
    let s2 = b"Finished good()\0";
    printLine(s2.as_ptr() as *const c_char);
    let s3 = b"Calling bad()...\0";
    printLine(s3.as_ptr() as *const c_char);
    bad();
    let s4 = b"Finished bad()\0";
    printLine(s4.as_ptr() as *const c_char);
}
