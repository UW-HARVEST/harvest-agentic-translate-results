// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_int;

unsafe extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
    fn puts(s: *const u8) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let result: c_int = x | !y;
    unsafe {
        printf(b"%d\0".as_ptr(), result);
        puts(b"\0".as_ptr());
    }
}
