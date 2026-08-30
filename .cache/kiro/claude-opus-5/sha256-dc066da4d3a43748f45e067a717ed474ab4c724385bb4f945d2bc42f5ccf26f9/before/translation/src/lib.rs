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

// The C source uses camelCase identifiers; the exported linker symbols must
// match them exactly, so the Rust naming lint is disabled here.
#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};
use std::mem::MaybeUninit;
use std::ptr;

// `driver.h` declares no namespace/renaming macros, so the exported symbol
// names are identical to the source-level names.

unsafe extern "C" {
    // Use C's `printf` (not Rust's `println!`) so that formatting *and* stdout
    // buffering behaviour are byte-for-byte identical to the C library.
    safe fn printf(format: *const c_char, ...) -> c_int;
}

/// Format string for `printf("%d\n", ...)`, NUL-terminated.
const FMT_D_NEWLINE: &[u8; 4] = b"%d\n\0";

/// C: `void printIntPtrLine(const int *intNumber)`
///
/// Dereferences `intNumber` and prints it as a decimal integer followed by a
/// newline. No NULL/validity check is performed, exactly as in the C original.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(intNumber: *const c_int) {
    let value: c_int = unsafe { *intNumber };
    printf(FMT_D_NEWLINE.as_ptr().cast::<c_char>(), value);
}

/// C: `void bad(void)`
///
/// Reproduces CWE-457/CWE-824: `int *data;` is left uninitialised and then
/// dereferenced. This is intentionally *not* fixed. The uninitialised stack
/// slot is read through a volatile load so the compiler cannot fold the read
/// away or replace the whole function with a trap; the observable behaviour
/// therefore mirrors the compiled C (garbage pointer, typically a fault).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data: MaybeUninit<*const c_int> = MaybeUninit::uninit();
    let data_value: *const c_int = unsafe { ptr::read_volatile(data.as_ptr()) };
    unsafe { printIntPtrLine(data_value) };
}

/// C: `void good(void)`
///
/// Initialises `data` to 5, takes its address, and prints through it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let data: c_int;
    data = 5;
    let data_addr: *const c_int;
    data_addr = &raw const data;
    unsafe { printIntPtrLine(data_addr) };
}

/// C: `void driver(int useGood)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        unsafe { good() };
    } else {
        unsafe { bad() };
    }
}
