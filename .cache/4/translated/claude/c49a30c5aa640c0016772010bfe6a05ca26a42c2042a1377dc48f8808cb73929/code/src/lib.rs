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

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    // <stdio.h> printf; used directly so that stdout buffering / flushing
    // semantics (and interleaving with any other C output) match the C library
    // byte-for-byte.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `void printLine(const char *line)`
///
/// Prints `line` followed by a newline, but only when `line` is non-NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(c"%s\n".as_ptr(), line);
    }
}

/// `static void helperBad()` — internal (not exported), and, as in the original
/// C source, never actually called by `bad()`.
#[allow(dead_code)]
unsafe fn helper_bad() {
    printLine(c"helperBad()".as_ptr());
}

/// `void bad()`
///
/// Note: faithfully reproduces the original C behavior — `bad()` prints its own
/// banner and does *not* call `helperBad()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    printLine(c"bad()".as_ptr());
}

/// `static void helperGood()` — internal (not exported).
unsafe fn helper_good() {
    printLine(c"helperGood()".as_ptr());
}

/// `void good()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    printLine(c"good()".as_ptr());
    helper_good();
}

/// `void driver(void)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver() {
    printLine(c"Calling good()...".as_ptr());
    good();
    printLine(c"Finished good()".as_ptr());
    printLine(c"Calling bad()...".as_ptr());
    bad();
    printLine(c"Finished bad()".as_ptr());
}
