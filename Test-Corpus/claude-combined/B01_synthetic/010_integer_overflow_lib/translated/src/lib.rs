// Copyright 2025 MIT Lincoln Laboratory
// Translation of c_src/src/driver.c to Rust.
//
// This is a library exporting the same C ABI as the original driver.so.

use std::ffi::c_char;
use std::os::raw::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Print a `char` (signed in the System V ABI on x86_64) using the C
/// format string `"%02x\n"`. The `char` is promoted to `int` for the
/// variadic call; this matches the behavior of the original C code,
/// including sign-extension for negative chars (e.g. `-1` -> `"ffffffff"`).
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "C" fn printHexCharLine(charHex: c_char) {
    // The C source does:  printf("%02x\n", charHex);
    // `charHex` (a `signed char`) is promoted to `int` for the variadic
    // call, sign-extending negative values. Replicate that promotion
    // explicitly so this works regardless of the host's `c_char` signedness.
    let promoted: c_int = charHex as c_int;
    let fmt = b"%02x\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, promoted);
    }
}

/// `driver` adds 1 to the input byte (with C `char` truncation/wrap)
/// and prints the resulting byte in hex via `printHexCharLine`.
#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    // C semantics: `char result = data + 1;`
    // The addition is performed in `int`, then truncated back to `char`
    // on assignment. Doing the wrapping arithmetic on `u8` reproduces the
    // truncation, regardless of whether `c_char` is signed or unsigned on
    // this platform.
    let result: c_char = (data as u8).wrapping_add(1) as c_char;
    printHexCharLine(result);
}
