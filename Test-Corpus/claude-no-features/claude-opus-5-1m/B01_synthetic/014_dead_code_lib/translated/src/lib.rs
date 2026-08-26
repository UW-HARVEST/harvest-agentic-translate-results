// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_char;

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            // Use libc printf to match C output exactly (including buffering).
            libc::printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

fn print_line(line: *const c_char) {
    printLine(line);
}

#[allow(dead_code)]
fn helper_bad() {
    print_line(b"helperBad()\0".as_ptr() as *const c_char);
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    print_line(b"bad()\0".as_ptr() as *const c_char);
}

fn helper_good() {
    print_line(b"helperGood()\0".as_ptr() as *const c_char);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    print_line(b"good()\0".as_ptr() as *const c_char);
    helper_good();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    print_line(b"Calling good()...\0".as_ptr() as *const c_char);
    good();
    print_line(b"Finished good()\0".as_ptr() as *const c_char);
    print_line(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad();
    print_line(b"Finished bad()\0".as_ptr() as *const c_char);
}
