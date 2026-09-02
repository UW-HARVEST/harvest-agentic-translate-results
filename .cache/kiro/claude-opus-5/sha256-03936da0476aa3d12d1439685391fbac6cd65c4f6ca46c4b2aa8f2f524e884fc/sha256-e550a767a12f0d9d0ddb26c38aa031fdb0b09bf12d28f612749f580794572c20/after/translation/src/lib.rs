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

//! Rust translation of the `driver` C library (c_src/src/driver.c).
//!
//! Public ABI (matches `nm -D libdriver.so` of the C build):
//!   * `printHexCharLine`
//!   * `driver`
//!
//! Byte-identical output is guaranteed by delegating to the platform's
//! `printf` from libc, exactly as the C source does. This also preserves
//! stdout stream/buffering semantics so output interleaves with any C
//! caller's own `stdio` writes identically.

// The C library's identifiers are camelCase; keep them verbatim so the exported
// linker symbols match the C ABI exactly.
#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};

unsafe extern "C" {
    /// `int printf(const char *restrict format, ...);` from the C runtime.
    ///
    /// Linking against libc's `printf` (rather than reimplementing formatting
    /// with Rust's `std::io`) is what makes the output byte-identical and keeps
    /// the same stdout buffering behaviour as the original library.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// The exact format string used by the C implementation: `"%02x\n"`.
///
/// Stored as a NUL-terminated byte string so it can be handed straight to
/// `printf`.
const HEX_CHAR_LINE_FMT: &[u8; 6] = b"%02x\n\0";

/// Translation of:
///
/// ```c
/// void printHexCharLine (char charHex)
/// {
///     printf("%02x\n", charHex);
/// }
/// ```
///
/// Note the C semantics that must be preserved verbatim: `charHex` has type
/// `char`, which on the target ABI is **signed**. Passing it to the variadic
/// `printf` applies the integer promotions, sign-extending it to `int`. The
/// `%02x` conversion then reinterprets that `int` as `unsigned int`. So a
/// negative `char` such as `-1` prints as `ffffffff`, not `ff`. This is
/// arguably a bug in the original library, but it is reproduced exactly rather
/// than fixed.
#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(charHex: c_char) {
    // Integer promotion of the signed `char` argument to `int`.
    let promoted: c_int = charHex as c_int;

    unsafe {
        printf(HEX_CHAR_LINE_FMT.as_ptr() as *const c_char, promoted);
    }
}

/// Translation of:
///
/// ```c
/// void driver(char data)
/// {
///     char result = data + 1;
///     printHexCharLine(result);
/// }
/// ```
///
/// `data + 1` is computed in `int` after promotion and then converted back to
/// `char`. That conversion is a wrapping truncation on this ABI, so
/// `driver(0x7f)` yields `result == -128` (printed as `ffffff80`) and
/// `driver(0xff /* -1 */)` yields `result == 0` (printed as `00`).
#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    let result: c_char = (data as c_int).wrapping_add(1) as c_char;
    printHexCharLine(result);
}
