// Rust translation of the C library in c_src/.
//
// Original copyright header from the C sources:
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

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    // int printf(const char *restrict format, ...);
    #[link_name = "printf"]
    unsafe fn c_printf(format: *const c_char, ...) -> c_int;
}

/// `void printHexCharLine(char charHex)`
///
/// C body:
/// ```c
/// printf("%02x\n", charHex);
/// ```
///
/// `charHex` undergoes the default integer promotions to `int` before being
/// passed to `printf`. On this target `char` is signed, so negative values are
/// sign-extended and then printed by `%x` as an `unsigned int` (e.g. `-1`
/// prints as `ffffffff`). The C `printf` is used directly so that both the
/// formatted bytes and the stdio buffering behaviour match the C library
/// exactly.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn printHexCharLine(charHex: c_char) {
    // Default argument promotion: char -> int (sign-extending on this target).
    let promoted: c_int = charHex as c_int;
    unsafe {
        c_printf(c"%02x\n".as_ptr(), promoted);
    }
}

/// `void driver(char data)`
///
/// C body:
/// ```c
/// char result = data + 1;
/// printHexCharLine(result);
/// ```
///
/// `data + 1` is computed in `int` and then converted back to `char`, which
/// wraps modulo 256 (implementation-defined conversion as performed by gcc),
/// e.g. `127 + 1 == -128`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: c_char) {
    let result: c_char = data.wrapping_add(1);
    unsafe {
        printHexCharLine(result);
    }
}
