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

// Link against the platform C library's printf so that stdout buffering,
// formatting and flushing behaviour is byte-for-byte identical to the C
// original (including interleaving with any other libc stdio output).
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `"%d\n\0"` — the exact format string used by the C implementation.
const FMT_D_NL: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];

// void printIntPtrLine(const int *intNumber)
// {
//     printf("%d\n", *intNumber);
// }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(intNumber: *const c_int) {
    unsafe {
        printf(FMT_D_NL.as_ptr(), *intNumber);
    }
}

// void bad()
// {
//     int *data;
//     printIntPtrLine(data);
// }
//
// CWE-457: `data` is never initialised, so an indeterminate pointer value is
// read off the stack and handed to printIntPtrLine, which dereferences it.
// This bug is reproduced faithfully (NOT fixed): the uninitialised stack slot
// is read through a volatile load so the compiler cannot fold the read away.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data: MaybeUninit<*const c_int> = MaybeUninit::uninit();
    let data_val: *const c_int = unsafe { core::ptr::read_volatile(data.as_ptr()) };
    unsafe {
        printIntPtrLine(data_val);
    }
}

// void good()
// {
//     int data;
//     data = 5;
//     int *data_addr;
//     data_addr = &data;
//     printIntPtrLine(data_addr);
// }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let mut data: MaybeUninit<c_int> = MaybeUninit::uninit();
    data.write(5);
    let data_addr: *const c_int = data.as_ptr();
    unsafe {
        printIntPtrLine(data_addr);
    }
}

// void driver(int useGood)
// {
//     if (useGood) { good(); } else { bad(); }
// }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        unsafe { good() };
    } else {
        unsafe { bad() };
    }
}
