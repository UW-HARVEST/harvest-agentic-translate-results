// Rust translation of c_src/src/driver.c
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

// The C entry point names are kept verbatim so the exported linker symbols match.
#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    /// C `printf` from libc. Used directly so that formatting *and* stdout
    /// buffering behaviour are identical to the original C library (Rust's own
    /// `print!` uses a separate, line-buffered stdout handle).
    #[link_name = "printf"]
    unsafe fn c_printf(format: *const c_char, ...) -> c_int;
}

/// C: `void printHexCharLine (char charHex)`
///
/// Emits `printf("%02x\n", charHex)`.
///
/// The argument undergoes the C default argument promotion `char` -> `int`
/// before reaching `printf`. On targets where `char` is signed (e.g. x86_64
/// Linux) a value such as `0x80` therefore arrives as `-128` and `%02x`, which
/// consumes an `unsigned int`, prints `ffffff80` rather than `80`. That
/// behaviour is reproduced here rather than "fixed".
#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(charHex: c_char) {
    // `as c_int` follows the signedness of `c_char` on the target, matching the
    // C integer promotion exactly.
    let promoted: c_int = charHex as c_int;

    // b"%02x\n\0" is a NUL-terminated copy of the original C format string.
    unsafe {
        c_printf(c"%02x\n".as_ptr(), promoted);
    }
}

/// C: `void driver(char data)`
///
/// ```c
/// char result = data + 1;
/// printHexCharLine(result);
/// ```
///
/// `data + 1` is computed in `int` and then truncated back into a `char` on
/// assignment, so `0x7f` wraps to `-128` (printed as `ffffff80`) and `0xff`
/// wraps to `0` (printed as `00`). `wrapping_add` reproduces that truncation.
#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    let result: c_char = data.wrapping_add(1);
    printHexCharLine(result);
}
