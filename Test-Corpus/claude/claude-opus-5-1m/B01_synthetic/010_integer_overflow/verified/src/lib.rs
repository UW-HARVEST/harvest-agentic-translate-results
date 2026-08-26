// Translated from c_src/src/main.c
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
//
//! FFI surface of the translation.
//!
//! `c_src/src/main.c` compiled as a shared library (`gcc -shared -fPIC`)
//! exports exactly two dynamic symbols:
//!
//! ```text
//! T main
//! T printHexCharLine
//! ```
//!
//! This `cdylib` exports the same two symbols, with the same C signatures, so
//! that an external caller (`dlopen` + `dlsym`) cannot tell the two libraries
//! apart.  Both wrappers delegate to [`driver_core`], which is the same code
//! the `driver` executable runs.

#![allow(non_snake_case)]

mod driver_core;

use std::os::raw::c_int;

/// `void printHexCharLine (char charHex)`
///
/// The parameter is declared as `c_int` rather than `c_char` on purpose: on the
/// x86-64 SysV ABI a `char` argument occupies a full 32-bit register whose
/// upper 24 bits are *undefined*, and gcc's callee re-derives the value from
/// the low byte only (`movsbl %dil,%esi`).  Taking a `c_int` and truncating
/// with `as i8` reproduces that byte-exact behaviour for every possible
/// register content, and is ABI-compatible with a `char` parameter.
#[no_mangle]
pub extern "C" fn printHexCharLine(charHex: c_int) {
    let char_hex = charHex as i8; // gcc: movsbl %dil,%esi
    driver_core::print_hex_char_line_stdout(char_hex);
}

/// `int main()`
#[no_mangle]
pub extern "C" fn main() -> c_int {
    driver_core::c_main() as c_int
}
