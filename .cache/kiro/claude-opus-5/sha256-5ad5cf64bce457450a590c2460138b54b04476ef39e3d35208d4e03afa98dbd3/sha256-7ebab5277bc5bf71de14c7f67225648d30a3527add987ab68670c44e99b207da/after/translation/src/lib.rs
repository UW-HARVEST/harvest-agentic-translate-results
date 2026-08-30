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
//! Output is written through C `printf` so that buffering and interleaving with
//! any other C stdio output in the hosting process match the original library
//! byte for byte.

#![allow(non_snake_case)]

use std::ffi::c_char;
use std::ffi::c_int;
use std::ptr;

unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(format: *const c_char, ...) -> c_int;
}

/// Format string equivalent to the C source's `"%s\n"`.
const LINE_FORMAT: &[u8; 4] = b"%s\n\0";

/// `void printLine(const char *line)`
///
/// Prints `line` followed by a newline; a NULL pointer prints nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if line != ptr::null() {
        unsafe {
            c_printf(LINE_FORMAT.as_ptr() as *const c_char, line);
        }
    }
}

/// `static void helperBad()` — defined but never called in the C source.
///
/// Retained (unused) to mirror the original translation unit exactly.
#[allow(dead_code)]
fn helperBad() {
    unsafe {
        printLine(c"helperBad()".as_ptr());
    }
}

/// `void bad()`
///
/// Note: the C implementation never calls `helperBad()`; that behavior is
/// preserved here rather than "fixed".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    unsafe {
        printLine(c"bad()".as_ptr());
    }
}

/// `static void helperGood()`
fn helperGood() {
    unsafe {
        printLine(c"helperGood()".as_ptr());
    }
}

/// `void good()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    unsafe {
        printLine(c"good()".as_ptr());
    }
    helperGood();
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
