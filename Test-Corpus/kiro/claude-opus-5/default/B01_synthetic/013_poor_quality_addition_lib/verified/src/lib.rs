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

use std::ffi::{c_char, c_int};
use std::ptr;

// Write through C's `printf` so that stdout buffering, flushing and
// interleaving with any other C code in the process behave exactly as they
// do in the original library.
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `void printLine(const char * line)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if line != ptr::null() {
        unsafe {
            printf(c"%s\n".as_ptr(), line);
        }
    }
}

/// `void printIntLine(int intNumber)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(c"%d\n".as_ptr(), int_number);
    }
}

/// `void bad(void)`
///
/// Faithfully reproduces the original: the result of `intOne + intTwo` is
/// discarded instead of being assigned to `intSum`, so both printed values
/// are `0`. This behaviour is intentionally preserved.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let int_sum: c_int = 0;
    unsafe {
        printIntLine(int_sum);
    }
    let _ = int_one + int_two; // result discarded, exactly as in the C source
    unsafe {
        printIntLine(int_sum);
    }
}

/// `void good(void)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let mut int_sum: c_int = 0;
    unsafe {
        printIntLine(int_sum);
    }
    int_sum = int_one + int_two;
    unsafe {
        printIntLine(int_sum);
    }
}

/// `void driver(void)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver() {
    unsafe {
        printLine(c"Calling good()...".as_ptr());
        good();
        printLine(c"Finished good()".as_ptr());
        printLine(c"Calling bad()...".as_ptr());
        bad();
        printLine(c"Finished bad()".as_ptr());
    }
}
