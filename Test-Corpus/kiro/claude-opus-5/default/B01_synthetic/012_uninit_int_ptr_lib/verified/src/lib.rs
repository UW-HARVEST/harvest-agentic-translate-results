// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/driver.c`.
//!
//! This is a faithful translation, not a fix. The original C demonstrates a
//! CWE-457/CWE-824 defect: `bad()` declares an uninitialized `int *data` and
//! dereferences it. That behaviour is reproduced here rather than corrected,
//! per the translation requirements.
//!
//! Output is emitted through the C library's `printf` (rather than Rust's
//! `std::io::stdout`) so that the bytes written, and their interleaving with
//! output produced by any C caller, are identical to the original: same
//! `%d\n` formatting and the same shared C `stdout` buffer.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};
use std::mem::MaybeUninit;

unsafe extern "C" {
    /// `int printf(const char *restrict format, ...);` from the C library.
    ///
    /// Linking against the platform libc directly keeps the exact `stdio`
    /// buffering semantics of the original translation unit.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `"%d\n"` as a NUL-terminated C string literal.
const FMT_INT_LINE: &[u8; 4] = b"%d\n\0";

/// Translation of:
///
/// ```c
/// void printIntPtrLine(const int *intNumber)
/// {
///     printf("%d\n", *intNumber);
/// }
/// ```
///
/// # Safety
///
/// `intNumber` must be a valid, aligned, dereferenceable pointer to an
/// initialized `int`. The original C performs no validation, so none is
/// added here.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(intNumber: *const c_int) {
    unsafe {
        // This is `*intNumber` in the C, but read through `ptr::read` rather
        // than a `*` deref. The two compile to the same load, and in a release
        // build they are indistinguishable; a `*` deref, however, additionally
        // carries rustc's debug-mode null/alignment UB checks. Those would turn
        // the C's undefined-but-observable behaviour - a fault on `NULL`, a
        // plain load from a misaligned address - into a Rust panic and abort
        // whenever `debug-assertions` are on, so the library would behave
        // differently from the C depending on how it was compiled.
        let value: c_int = std::ptr::read(intNumber);
        printf(FMT_INT_LINE.as_ptr() as *const c_char, value);
    }
}

/// Translation of:
///
/// ```c
/// void bad()
/// {
///     int *data;
///     printIntPtrLine(data);
/// }
/// ```
///
/// The pointer `data` is deliberately left uninitialized before being passed
/// to `printIntPtrLine`, exactly as in the C. This is undefined behaviour in
/// both languages; it is preserved because the defect is the point of this
/// test case.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    // `int *data;` -- a stack slot is reserved and never assigned.
    let slot: MaybeUninit<*const c_int> = MaybeUninit::uninit();
    // Reading it volatile keeps the compiler from folding the `undef` away, so
    // what reaches `printIntPtrLine` is the actual garbage left on the stack --
    // the same thing the unoptimized C passes.
    let data: *const c_int = unsafe { std::ptr::read_volatile(slot.as_ptr()) };
    unsafe {
        printIntPtrLine(data);
    }
}

/// Translation of:
///
/// ```c
/// void good()
/// {
///     int data;
///     data = 5;
///     int *data_addr;
///     data_addr = &data;
///     printIntPtrLine(data_addr);
/// }
/// ```
///
/// Prints `5\n`.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let data: c_int;
    data = 5;
    let data_addr: *const c_int;
    data_addr = &raw const data;
    unsafe {
        printIntPtrLine(data_addr);
    }
}

/// Translation of:
///
/// ```c
/// void driver(int useGood)
/// {
///     if (useGood) { good(); } else { bad(); }
/// }
/// ```
///
/// Any nonzero `useGood` selects the `good()` path, matching C truthiness.
#[inline(never)]
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
