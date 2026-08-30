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
    // C stdio is used directly so that output bytes, and any interleaving with
    // output produced by a C caller, match the original exactly.
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// `printf("%s\n", line)` for non-NULL `line`, otherwise nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe { c_printf(c"%s\n".as_ptr(), line) };
    }
}

/// `printf("%d\n", intNumber)`
#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(intNumber: c_int) {
    unsafe { c_printf(c"%d\n".as_ptr(), intNumber) };
}

/// The original C allocates only `alloca(10)` bytes yet stores ten `int`s into
/// it, overrunning the region. The bug is not fixed in the sense that the
/// visible behaviour is preserved verbatim: `source` is zero-initialised, all
/// ten stores are performed in the same order, and `data[0]` (always 0) is
/// printed. The backing storage here is a properly sized stack buffer so the
/// translation does not rely on out-of-bounds stack writes.
#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    // `data = (int *)alloca(10);` -- 10 bytes in the original.
    let mut data = [0i32; 10];
    {
        let source = [0i32; 10];
        let mut i: usize = 0;
        while i < 10 {
            data[i] = source[i];
            i += 1;
        }
        printIntLine(data[0] as c_int);
    }
}

/// `alloca(10 * sizeof(int))`, copy ten zeroed `int`s, print `data[0]`.
#[unsafe(no_mangle)]
pub extern "C" fn good() {
    // `data = NULL;` followed by the correctly sized allocation.
    let mut data = [0i32; 10];
    {
        let source = [0i32; 10];
        let mut i: usize = 0;
        while i < 10 {
            data[i] = source[i];
            i += 1;
        }
        printIntLine(data[0] as c_int);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
