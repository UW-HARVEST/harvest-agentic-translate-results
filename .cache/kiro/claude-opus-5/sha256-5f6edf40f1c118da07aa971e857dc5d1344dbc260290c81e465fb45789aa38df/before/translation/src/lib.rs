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
//! Output is emitted through the platform C library's `printf` so that
//! buffering and formatting behaviour (and therefore the exact byte stream on
//! stdout) match the original C shared library.

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(format: *const c_char, ...) -> c_int;
}

/// `printf("%s\n", line)` when `line` is non-NULL, matching `printLine` in the C source.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            c_printf(c"%s\n".as_ptr(), line);
        }
    }
}

/// `printf("%d\n", intNumber)`, matching `printIntLine` in the C source.
#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        c_printf(c"%d\n".as_ptr(), int_number);
    }
}

/// Faithful translation of the C `bad()`: the sum is computed but discarded,
/// so `intSum` stays 0 and `0` is printed twice. The bug is preserved on purpose.
#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let int_sum: c_int = 0;
    printIntLine(int_sum);
    // C source has the statement `intOne + intTwo;` whose value is unused.
    let _ = int_one.wrapping_add(int_two);
    printIntLine(int_sum);
}

/// Faithful translation of the C `good()`: prints `0` then `2`.
#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let mut int_sum: c_int = 0;
    printIntLine(int_sum);
    int_sum = int_one.wrapping_add(int_two);
    printIntLine(int_sum);
}

/// Entry point declared in `include/driver.h`.
#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    unsafe {
        printLine(c"Calling good()...".as_ptr());
        good();
        printLine(c"Finished good()".as_ptr());
        printLine(c"Calling bad()...".as_ptr());
        bad();
        printLine(c"Finished bad()".as_ptr());
    }
}
