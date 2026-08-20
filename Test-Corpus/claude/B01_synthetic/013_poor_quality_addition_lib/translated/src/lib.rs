// Rust translation of c_src/src/driver.c (+ c_src/include/driver.h).
//
// Original copyright notice from the C sources:
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

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int};

// The C code emits all of its output through `printf` on the C runtime's
// `stdout`. Going through libc's `printf` directly (rather than Rust's
// `println!`) keeps the produced bytes *and* the stream buffering/flushing
// behaviour byte-for-byte identical to the original library, including when a
// host program mixes its own C stdio writes with calls into this library.
extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `void printLine(const char * line)`
///
/// Prints `line` followed by a newline; a NULL pointer prints nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(c"%s\n".as_ptr(), line);
    }
}

/// `void printIntLine(int intNumber)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    printf(c"%d\n".as_ptr(), int_number);
}

/// `void bad(void)`
///
/// Faithful translation of the original: the result of `intOne + intTwo` is
/// computed and then discarded (the C source has the bug that it never assigns
/// it to `intSum`), so `intSum` stays 0 for both prints. This behaviour is
/// intentionally preserved, not "fixed".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let int_sum: c_int = 0;
    printIntLine(int_sum);
    let _ = int_one.wrapping_add(int_two); // value discarded, exactly as in C
    printIntLine(int_sum);
}

/// `void good(void)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let mut int_sum: c_int = 0;
    printIntLine(int_sum);
    int_sum = int_one.wrapping_add(int_two);
    printIntLine(int_sum);
}

/// `void driver(void)` — the library's documented entry point (driver.h).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver() {
    printLine(c"Calling good()...".as_ptr());
    good();
    printLine(c"Finished good()".as_ptr());
    printLine(c"Calling bad()...".as_ptr());
    bad();
    printLine(c"Finished bad()".as_ptr());
}
