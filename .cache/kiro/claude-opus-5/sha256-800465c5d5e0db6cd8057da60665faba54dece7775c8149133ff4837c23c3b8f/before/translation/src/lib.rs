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
use std::ptr;

// The C code writes via `printf`, so we go through libc's stdio as well. This
// keeps the emitted bytes and the stdout buffering behaviour identical to the
// original, including when the library is mixed with C code that also writes to
// stdout.
unsafe extern "C" {
    #[link_name = "printf"]
    fn c_printf(fmt: *const c_char, ...) -> std::ffi::c_int;
}

/// C: `void printLine(const char *line)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if line != ptr::null() {
        // printf("%s\n", line);
        unsafe {
            c_printf(c"%s\n".as_ptr(), line);
        }
    }
}

/// C: `static void helperBad()` -- defined but never called in the original.
/// Kept for fidelity with the source; it is not an exported symbol.
#[allow(dead_code, non_snake_case)]
fn helperBad() {
    unsafe {
        printLine(c"helperBad()".as_ptr());
    }
}

/// C: `void bad()`
///
/// Note: the original never calls `helperBad()` here (unlike `good()`, which
/// calls `helperGood()`). That asymmetry is preserved verbatim.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    unsafe {
        printLine(c"bad()".as_ptr());
    }
}

/// C: `static void helperGood()`
#[allow(non_snake_case)]
fn helperGood() {
    unsafe {
        printLine(c"helperGood()".as_ptr());
    }
}

/// C: `void good()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    unsafe {
        printLine(c"good()".as_ptr());
    }
    helperGood();
}

/// C: `void driver(void)`
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
