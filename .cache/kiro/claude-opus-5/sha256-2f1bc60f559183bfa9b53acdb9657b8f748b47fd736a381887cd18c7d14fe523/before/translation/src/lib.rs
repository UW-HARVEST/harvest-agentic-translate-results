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

// Keep the C identifiers verbatim so the exported symbols match the original.
#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};
use std::mem::MaybeUninit;
use std::ptr;

// The C code prints via <stdio.h> printf. Call the platform's printf directly so
// that stream buffering, flushing and interleaving with any C-side output are
// identical to the original library.
extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `printf("%s\n", line)` format string, NUL terminated.
const FMT_LINE: &[u8; 4] = b"%s\n\0";

/// Translation of:
/// ```c
/// void printLine(const char *line)
/// {
///     if (line != NULL)
///     {
///         printf("%s\n", line);
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(FMT_LINE.as_ptr() as *const c_char, line);
    }
}

/// Translation of:
/// ```c
/// void bad()
/// {
///     char *data;
///     printLine(data);
/// }
/// ```
///
/// NOTE: this is the intentional defect of the original code (CWE-457, use of
/// an uninitialized variable). It is reproduced verbatim and NOT fixed: `data`
/// is never assigned, so whatever value happens to occupy that stack slot is
/// passed to `printLine`. The volatile read models the unoptimized C load so
/// the read is actually performed instead of being folded away.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data: MaybeUninit<*const c_char> = MaybeUninit::uninit();
    let data: *const c_char = ptr::read_volatile(data.as_ptr());
    printLine(data);
}

/// Translation of:
/// ```c
/// void good()
/// {
///     char *data;
///     data = "string";
///     printLine(data);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let data: *const c_char = b"string\0".as_ptr() as *const c_char;
    printLine(data);
}

/// Translation of:
/// ```c
/// void driver(int useGood)
/// {
///     if (useGood) { good(); } else { bad(); }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
