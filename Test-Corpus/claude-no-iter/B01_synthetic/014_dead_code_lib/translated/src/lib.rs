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

use std::ffi::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // Mirror C's `printf("%s\n", line);` exactly.
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            libc::printf(fmt, line);
        }
    }
}

fn helperBad() {
    let s = b"helperBad()\0".as_ptr() as *const c_char;
    printLine(s);
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let s = b"bad()\0".as_ptr() as *const c_char;
    printLine(s);
}

fn helperGood() {
    let s = b"helperGood()\0".as_ptr() as *const c_char;
    printLine(s);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let s = b"good()\0".as_ptr() as *const c_char;
    printLine(s);
    helperGood();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    let s1 = b"Calling good()...\0".as_ptr() as *const c_char;
    printLine(s1);
    good();
    let s2 = b"Finished good()\0".as_ptr() as *const c_char;
    printLine(s2);
    let s3 = b"Calling bad()...\0".as_ptr() as *const c_char;
    printLine(s3);
    bad();
    let s4 = b"Finished bad()\0".as_ptr() as *const c_char;
    printLine(s4);
}

// Reference unused helpers so they aren't dead-code-eliminated
// (mirrors C's `static` helpers existing in the translation unit).
#[allow(dead_code)]
const _UNUSED_HELPER_BAD: fn() = helperBad;
