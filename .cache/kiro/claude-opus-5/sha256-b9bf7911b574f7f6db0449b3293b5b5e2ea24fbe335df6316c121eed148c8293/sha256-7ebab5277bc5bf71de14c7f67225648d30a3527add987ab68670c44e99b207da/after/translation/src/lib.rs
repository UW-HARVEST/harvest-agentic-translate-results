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
//! The C translation unit exports four symbols with external linkage:
//! `printLine`, `bad`, `good` and `driver`. `driver.h` declares only `driver`
//! and contains no namespace/renaming macros, so the linker symbols are the
//! source-level names verbatim.
//!
//! Output is emitted through libc's `printf` so that formatting *and* stdio
//! buffering semantics match the C library byte for byte.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};
use std::ptr;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Compile-time conversion of a byte literal (including its NUL) into a
/// `[c_char; N]`, mirroring C's `char x[] = "...";` initialization.
const fn to_c_array<const N: usize>(bytes: &[u8; N]) -> [c_char; N] {
    let mut out = [0 as c_char; N];
    let mut i = 0;
    while i < N {
        out[i] = bytes[i] as c_char;
        i += 1;
    }
    out
}

/// C: `void printLine(const char *line)`
#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // printf("%s\n", line);
        unsafe {
            printf(c"%s\n".as_ptr(), line);
        }
    }
}

/// C: `static char *helperBad()` — returns the address of an automatic array.
///
/// This is the CWE-562 defect of the original program and it is preserved, not
/// fixed. GCC diagnoses `-Wreturn-local-addr` here and, at every optimization
/// level (`-O0` through `-O3`, `-Os`), substitutes a null pointer for the
/// return value (`mov $0x0,%eax`). The observable behaviour of the compiled C
/// library is therefore `printLine(NULL)`, which prints nothing at all, and
/// that is what this translation reproduces.
fn helperBad() -> *mut c_char {
    // char charString[] = "helperBad string";
    let mut charString: [c_char; 17] = to_c_array(b"helperBad string\0");

    // The local is materialized (and then discarded) exactly as in C; its
    // address never escapes, because the compiled C does not return it either.
    let _ = std::hint::black_box(&mut charString);

    // return charString;  /* -> null after the compiler's substitution */
    ptr::null_mut()
}

/// C: `void bad()`
#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    printLine(helperBad());
}

/// Storage backing `helperGood1`'s `static char charString[]`.
///
/// The C object lives in static storage and is never written by this library,
/// so an immutable Rust `static` is behaviourally equivalent while keeping the
/// internals free of `static mut`.
static HELPER_GOOD1_STRING: [c_char; 19] = to_c_array(b"helperGood1 string\0");

/// C: `static char *helperGood1()`
fn helperGood1() -> *mut c_char {
    // static char charString[] = "helperGood1 string"; return charString;
    HELPER_GOOD1_STRING.as_ptr() as *mut c_char
}

/// C: `void good()`
#[unsafe(no_mangle)]
pub extern "C" fn good() {
    printLine(helperGood1());
}

/// C: `void driver(int useGood)`
#[unsafe(no_mangle)]
pub extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
