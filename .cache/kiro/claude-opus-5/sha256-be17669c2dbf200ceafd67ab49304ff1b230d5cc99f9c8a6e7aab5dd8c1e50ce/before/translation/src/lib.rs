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

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    /// C `printf` from libc. Used directly (rather than Rust's `std::io`) so that
    /// stdout buffering, flush-at-exit behaviour and interleaving with any C
    /// caller's own output remain byte-for-byte identical to the original.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `printf("%s\n", line)` — the format string as a NUL-terminated C literal.
const FMT_STRING_NEWLINE: &[u8; 4] = b"%s\n\0";

/// void printLine(const char *line)
///
/// Prints `line` followed by a newline, but only when the pointer is non-NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(FMT_STRING_NEWLINE.as_ptr() as *const c_char, line);
        }
    }
}

/// static char *helperBad()
/// {
///     char charString[] = "helperBad string";
///     return charString;
/// }
///
/// The original returns the address of a function-local automatic array, which
/// is undefined behaviour (CWE-562: Return of Stack Variable Address). Every
/// optimisation level of the reference compiler resolves this by emitting a
/// literal NULL return (`movl $0, %eax`) rather than a real stack address, so
/// the observable result is a NULL pointer. That behaviour is reproduced here
/// verbatim — the bug is preserved, not fixed.
fn helperBad() -> *mut c_char {
    // The local `charString` buffer is constructed and immediately discarded in
    // the C original; it has no observable effect on the returned value.
    std::ptr::null_mut()
}

/// void bad()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    unsafe {
        printLine(helperBad());
    }
}

/// The `static char charString[]` inside `helperGood1`: a mutable, statically
/// allocated, NUL-terminated buffer whose lifetime outlives the call.
static mut HELPER_GOOD1_STRING: [c_char; 19] = {
    let mut buf = [0 as c_char; 19];
    let src = b"helperGood1 string";
    let mut i = 0;
    while i < src.len() {
        buf[i] = src[i] as c_char;
        i += 1;
    }
    // buf[18] stays 0: the terminating NUL.
    buf
};

/// static char *helperGood1()
/// {
///     static char charString[] = "helperGood1 string";
///     return charString;
/// }
fn helperGood1() -> *mut c_char {
    (&raw mut HELPER_GOOD1_STRING) as *mut c_char
}

/// void good()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    unsafe {
        printLine(helperGood1());
    }
}

/// void driver(int useGood)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    unsafe {
        if useGood != 0 {
            good();
        } else {
            bad();
        }
    }
}
