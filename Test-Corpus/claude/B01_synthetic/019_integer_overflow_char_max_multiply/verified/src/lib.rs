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

//! C-ABI surface of the translated `driver` program.
//!
//! `c_src/src/main.c` compiles to a shared object that exports exactly five
//! dynamic symbols: `printLine`, `printHexCharLine`, `bad`, `good` and `main`
//! (`goodG2B` / `goodB2G` are `static`). This module re-exports the same five
//! names with the same C signatures so the Rust `cdylib` is a drop-in
//! replacement and can be compared against the C `.so` symbol for symbol.

#[path = "imp.rs"]
mod imp;

use std::os::raw::{c_char, c_int};

/// `void printLine(const char * line)`
///
/// # Safety
/// `line` must be NULL or a valid pointer to a NUL-terminated byte string.
#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if line.is_null() {
        // The C code checks `line != NULL` and prints nothing otherwise.
        imp::print_line(None);
        return;
    }
    let bytes = std::ffi::CStr::from_ptr(line).to_bytes();
    imp::print_line(Some(bytes));
}

/// `void printHexCharLine(char charHex)`
#[no_mangle]
pub extern "C" fn printHexCharLine(char_hex: c_char) {
    imp::print_hex_char_line(char_hex as i8);
}

/// `void bad(void)`
#[no_mangle]
pub extern "C" fn bad() {
    imp::bad();
}

/// `void good(void)`
#[no_mangle]
pub extern "C" fn good() {
    imp::good();
}

/// `int main(void)` — reads one integer from stdin and dispatches.
#[no_mangle]
pub extern "C" fn main() -> c_int {
    imp::program_main() as c_int
}
