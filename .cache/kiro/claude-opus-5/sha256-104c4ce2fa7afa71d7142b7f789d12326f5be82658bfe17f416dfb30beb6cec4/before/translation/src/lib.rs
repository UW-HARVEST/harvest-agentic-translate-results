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

// The C code writes to stdout through the C runtime's `printf`. Routing output
// through the very same function guarantees byte-identical output *and*
// identical stream-buffering/interleaving behaviour when this library is linked
// against a C `main` that also writes to stdout.
unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(format: *const c_char, ...) -> c_int;
}

/// `void printLine(const char * line)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if line != ptr::null() {
        unsafe {
            c_printf(c"%s\n".as_ptr(), line);
        }
    }
}

/// `void printIntLine(int intNumber)`
#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        c_printf(c"%d\n".as_ptr(), int_number);
    }
}

/// `void bad()`
///
/// The original C allocates only 10 *bytes* with `alloca(10)` and then stores
/// ten `int`s (40 bytes on typical targets) into that region -- a stack buffer
/// overflow. Only `data[0]` is ever read back, and every element of `source`
/// is zero, so the observable behaviour (a single `0` line on stdout) is
/// reproduced here without actually corrupting the stack. The out-of-bounds
/// write itself is undefined behaviour in C and has no meaningful Rust
/// equivalent; the printed output is preserved exactly.
#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    // `data = (int *)alloca(10);` -- undersized allocation in the original.
    let mut data = [0 as c_int; 10];
    {
        let source = [0 as c_int; 10];
        let mut i: usize = 0;
        while i < 10 {
            data[i] = source[i];
            i += 1;
        }
        printIntLine(data[0]);
    }
}

/// `void good()`
#[unsafe(no_mangle)]
pub extern "C" fn good() {
    // `data = NULL;` then `data = (int *)alloca(10*sizeof(int));`
    let mut data = [0 as c_int; 10];
    {
        let source = [0 as c_int; 10];
        let mut i: usize = 0;
        while i < 10 {
            data[i] = source[i];
            i += 1;
        }
        printIntLine(data[0]);
    }
}

/// `void driver(int useGood)`
#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
