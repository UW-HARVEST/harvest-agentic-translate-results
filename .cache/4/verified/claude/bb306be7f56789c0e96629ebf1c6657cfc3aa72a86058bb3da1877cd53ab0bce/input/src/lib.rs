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

use core::ffi::{c_char, c_int};
use core::mem::MaybeUninit;

unsafe extern "C" {
    // Use the C library's printf so that formatting and stdio buffering
    // semantics are byte-for-byte identical to the original C library.
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// `void printIntPtrLine(const int *intNumber)` -> `printf("%d\n", *intNumber);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(intNumber: *const c_int) {
    // Dereference exactly as the C code does (no NULL check in the original).
    let value: c_int = unsafe { *intNumber };
    unsafe {
        c_printf(c"%d\n".as_ptr(), value);
    }
}

/// `void bad(void)`
///
/// Faithfully reproduces the original defect (CWE-457/CWE-824: use of an
/// uninitialized pointer variable). The pointer is left uninitialized and
/// then dereferenced, exactly as in the C source. This is intentional --
/// the bug must NOT be fixed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    // int *data;   /* never assigned */
    //
    // A volatile read out of the uninitialized stack slot mirrors what an
    // unoptimized C compiler does: it loads whatever bytes happen to live in
    // that stack location, instead of letting the optimizer collapse the
    // undefined value into something unrelated.
    let slot: MaybeUninit<*const c_int> = MaybeUninit::uninit();
    let data: *const c_int = unsafe { core::ptr::read_volatile(slot.as_ptr()) };
    // printIntPtrLine(data);
    unsafe {
        printIntPtrLine(data);
    }
}

/// `void good(void)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    // int data; data = 5;
    let data: c_int = 5;
    // int *data_addr; data_addr = &data;
    let data_addr: *const c_int = &data;
    // printIntPtrLine(data_addr);
    unsafe {
        printIntPtrLine(data_addr);
    }
}

/// `void driver(int useGood)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        unsafe { good() };
    } else {
        unsafe { bad() };
    }
}
