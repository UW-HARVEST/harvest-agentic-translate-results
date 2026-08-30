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

//! Rust translation of `c_src/src/driver.c`.
//!
//! The C original is:
//!
//! ```c
//! void printHexCharLine (char charHex)
//! {
//!     printf("%02x\n", charHex);
//! }
//!
//! void driver(char data)
//! {
//!     char result = data + 1;
//!     printHexCharLine(result);
//! }
//! ```
//!
//! Two details are load-bearing for byte-identical output:
//!
//! 1. `char` is signed on the platforms this library targets (x86-64 / AArch64
//!    Linux uses signed `char` for x86-64; note AArch64 Linux uses *unsigned*
//!    `char`, so the sign-extension behaviour below matches the x86-64 ABI that
//!    the reference build uses). Passing a `char` through the variadic `...` of
//!    `printf` promotes it to `int` via the default argument promotions, so a
//!    negative value such as `-128` reaches `printf` as `0xFFFFFF80`. `%02x`
//!    then formats the full 32-bit unsigned value, printing `ffffff80` rather
//!    than `80`. The `02` width is a *minimum*, so it never truncates.
//!
//! 2. `data + 1` is computed in `int` and truncated back into a `char`, which
//!    wraps for `data == CHAR_MAX` (127 + 1 -> -128). This is implementation
//!    defined in C but wraps in practice on the reference build, so the
//!    translation reproduces the wrap rather than panicking.
//!
//! Output is emitted by calling the platform's own `printf`, so stream
//! buffering, interleaving with any C caller's output, and flush-at-exit
//! semantics are identical to the C library. Rust's `std::io::stdout` could not
//! be used safely here: this crate is a `cdylib`, so the Rust runtime shutdown
//! that flushes `stdout` never runs, and buffered bytes would be lost.

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    /// The C library's `printf`. Declared directly rather than via the `libc`
    /// crate to keep this translation dependency-free.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Format string `"%02x\n"`, NUL terminated, matching the C source exactly.
static HEX_LINE_FORMAT: [c_char; 6] = [
    b'%' as c_char,
    b'0' as c_char,
    b'2' as c_char,
    b'x' as c_char,
    b'\n' as c_char,
    0,
];

/// Translation of `void printHexCharLine(char charHex)`.
///
/// Prints `charHex` as lower-case hex followed by a newline. The argument
/// undergoes the same integer promotion to `int` that the C variadic call
/// performs, so negative `char` values print sign extended.
#[unsafe(no_mangle)]
#[allow(non_snake_case)] // Parameter name kept identical to the C source.
pub extern "C" fn printHexCharLine(charHex: c_char) {
    // Default argument promotion: char -> int. On targets where `c_char` is
    // unsigned this widens without sign extension, exactly as C would.
    let promoted: c_int = charHex as c_int;

    unsafe {
        printf(HEX_LINE_FORMAT.as_ptr(), promoted);
    }
}

/// Translation of `void driver(char data)`.
///
/// Adds one to `data` (wrapping within `char`, as the C truncation does) and
/// hands the result to [`printHexCharLine`].
#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    let result: c_char = data.wrapping_add(1);
    printHexCharLine(result);
}
