// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// Rust translation of c_src/src/driver.c — produces byte-identical output
// to the original C library.

use std::ffi::c_char;

extern "C" {
    fn printf(format: *const c_char, ...) -> i32;
}

/// Mirrors the C function `void printHexCharLine(char charHex)`.
///
/// The C version calls `printf("%02x\n", charHex)`. Because `charHex` is a
/// `char` (which on most Linux/x86_64 toolchains is `signed char`), C's default
/// argument promotion sign-extends it to `int` before passing it to the
/// variadic `printf`. We replicate that exactly by sign-extending the
/// `c_char` argument to `i32` ourselves and forwarding to libc `printf`.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "C" fn printHexCharLine(charHex: c_char) {
    // Sign-extend `c_char` to `i32` to match C's default argument promotion
    // rules for a signed char passed to a variadic function.
    let promoted: i32 = charHex as i32;
    let fmt = b"%02x\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, promoted);
    }
}

/// Mirrors the C function `void driver(char data)`.
#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    // Replicates `char result = data + 1;` in C. With signed `char`,
    // overflow from 0x7f -> 0x80 is technically undefined behavior, but in
    // practice every mainstream C compiler on x86_64 wraps using two's
    // complement. We reproduce that with `wrapping_add`.
    let result: c_char = data.wrapping_add(1);
    printHexCharLine(result);
}
