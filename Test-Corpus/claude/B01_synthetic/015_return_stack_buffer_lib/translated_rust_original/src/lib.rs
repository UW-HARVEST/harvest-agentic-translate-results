// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust - byte-identical output required.

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Prints the given C string followed by a newline, if not NULL.
#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // printf("%s\n", line);
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

/// Reproduces the C helperBad() bug: returns a pointer to a stack-allocated
/// buffer that is no longer valid after the function returns. This is
/// undefined behavior in the original C code; we mirror its structure here.
fn helper_bad() -> *mut c_char {
    let mut char_string: [c_char; 17] = [
        b'h' as c_char,
        b'e' as c_char,
        b'l' as c_char,
        b'p' as c_char,
        b'e' as c_char,
        b'r' as c_char,
        b'B' as c_char,
        b'a' as c_char,
        b'd' as c_char,
        b' ' as c_char,
        b's' as c_char,
        b't' as c_char,
        b'r' as c_char,
        b'i' as c_char,
        b'n' as c_char,
        b'g' as c_char,
        0,
    ];
    char_string.as_mut_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    printLine(helper_bad());
}

/// Returns a pointer to a static (program-lifetime) C string.
fn helper_good1() -> *mut c_char {
    static CHAR_STRING: [u8; 19] = *b"helperGood1 string\0";
    CHAR_STRING.as_ptr() as *mut c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    printLine(helper_good1());
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
