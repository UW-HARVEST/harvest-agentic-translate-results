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
    fn printf(format: *const c_char, ...) -> c_int;
}

// Format strings used by the C code (with NUL terminator).
const FMT_STR: &[u8] = b"%s\n\0";
const FMT_INT: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(FMT_STR.as_ptr() as *const c_char, line);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(FMT_INT.as_ptr() as *const c_char, int_number);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    // C code: int *data = (int *)alloca(10);  // only 10 BYTES
    // then writes 10 ints into it (undefined behavior).
    // We replicate the observable output: it prints data[0] which is 0.
    // To preserve UB-style behavior for byte-identical output, we still
    // do the same loop and call printIntLine.
    let mut data: [c_int; 10] = [0; 10];
    let source: [c_int; 10] = [0; 10];
    let mut i: usize = 0;
    while i < 10 {
        data[i] = source[i];
        i += 1;
    }
    unsafe {
        printIntLine(data[0]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    // C code: int *data = (int *)alloca(10 * sizeof(int));
    let mut data: [c_int; 10] = [0; 10];
    let source: [c_int; 10] = [0; 10];
    let mut i: usize = 0;
    while i < 10 {
        data[i] = source[i];
        i += 1;
    }
    unsafe {
        printIntLine(data[0]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        unsafe {
            good();
        }
    } else {
        unsafe {
            bad();
        }
    }
}
