// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Reproduces byte-identical output of the original.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Translation of: void printLine(const char *line)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // Mirrors `printf("%s\n", line);`
        printf(b"%s\n\0".as_ptr() as *const c_char, line);
    }
}

/// Translation of: static char *helperBad()
///
/// The original C implementation has a bug: it returns a pointer to a
/// stack-allocated local array, which is undefined behavior. In practice,
/// the immediate observable behavior is that the calling site reads back
/// the string that was just written into the stack frame (because nothing
/// has overwritten it yet). To reproduce the observed byte-identical
/// output, we return a pointer to data containing the same bytes.
fn helperBad() -> *mut c_char {
    static BAD_STRING: [u8; 17] = *b"helperBad string\0";
    BAD_STRING.as_ptr() as *mut c_char
}

/// Translation of: void bad()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    printLine(helperBad() as *const c_char);
}

/// Translation of: static char *helperGood1()
fn helperGood1() -> *mut c_char {
    static GOOD_STRING: [u8; 19] = *b"helperGood1 string\0";
    GOOD_STRING.as_ptr() as *mut c_char
}

/// Translation of: void good()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    printLine(helperGood1() as *const c_char);
}

/// Translation of: void driver(int useGood)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
