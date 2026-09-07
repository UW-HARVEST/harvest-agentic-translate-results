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

//! Rust translation of `c_src/src/driver.c`.
//!
//! Public ABI surface (matches `nm -D` of the C `libdriver.so`):
//!   * `printLine`
//!   * `bad`
//!   * `good`
//!   * `driver`
//!
//! `helperBad` and `helperGood1` are `static` in the C translation unit and
//! therefore are *not* exported; they stay private here as well.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};

// The C code performs its output through C stdio (`printf`). We call straight
// into libc so that buffering, flushing and interleaving with any C code in the
// same process are bit-for-bit identical to the original library rather than
// going through Rust's own `std::io::Stdout` buffer.
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `void printLine(const char *line)`
///
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
        unsafe {
            printf(c"%s\n".as_ptr(), line);
        }
    }
}

/// `static char *helperBad()`
///
/// ```c
/// static char *helperBad()
/// {
///     char charString[] = "helperBad string";
///     return charString;
/// }
/// ```
///
/// This returns the address of an automatic (stack) array whose lifetime ends
/// when the function returns — CWE-562, "Return of Stack Variable Address", and
/// undefined behaviour in C. It is *not* a bug to be fixed here: the reference
/// library's observable behaviour must be reproduced exactly.
///
/// GCC (verified for `-O0` through `-O3` and `-Os`, which covers the flags the
/// reference CMake build uses) diagnoses the dead reference and materialises a
/// null pointer for the return value — the emitted body is literally
/// `mov $0x0,%eax; ret`. Consequently `bad()` calls `printLine(NULL)` and the
/// library prints nothing at all on this path. Returning a genuinely dangling
/// pointer here instead would make `printLine` emit whatever bytes happened to
/// be left on the stack, which is *not* what the C library does.
fn helperBad() -> *mut c_char {
    core::ptr::null_mut()
}

/// `void bad()`
///
/// ```c
/// void bad()
/// {
///     printLine(helperBad());
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    unsafe {
        printLine(helperBad());
    }
}

/// `static char *helperGood1()`
///
/// ```c
/// static char *helperGood1()
/// {
///     static char charString[] = "helperGood1 string";
///     return charString;
/// }
/// ```
///
/// The array has static storage duration, so returning it is well defined. The
/// C version hands back a writable `char *` into a single shared object; the
/// `static mut` below reproduces that storage class and mutability faithfully.
fn helperGood1() -> *mut c_char {
    static mut CHAR_STRING: [c_char; 19] = [
        b'h' as c_char,
        b'e' as c_char,
        b'l' as c_char,
        b'p' as c_char,
        b'e' as c_char,
        b'r' as c_char,
        b'G' as c_char,
        b'o' as c_char,
        b'o' as c_char,
        b'd' as c_char,
        b'1' as c_char,
        b' ' as c_char,
        b's' as c_char,
        b't' as c_char,
        b'r' as c_char,
        b'i' as c_char,
        b'n' as c_char,
        b'g' as c_char,
        0,
    ];

    // Address-of on a `static mut` does not read or write the value, so taking
    // the pointer is sound; any aliasing concerns belong to the C caller, just
    // as in the original.
    (&raw mut CHAR_STRING) as *mut c_char
}

/// `void good()`
///
/// ```c
/// void good()
/// {
///     printLine(helperGood1());
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    unsafe {
        printLine(helperGood1());
    }
}

/// `void driver(int useGood)`
///
/// ```c
/// void driver(int useGood)
/// {
///     if (useGood)
///     {
///         good();
///     }
///     else
///     {
///         bad();
///     }
/// }
/// ```
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
