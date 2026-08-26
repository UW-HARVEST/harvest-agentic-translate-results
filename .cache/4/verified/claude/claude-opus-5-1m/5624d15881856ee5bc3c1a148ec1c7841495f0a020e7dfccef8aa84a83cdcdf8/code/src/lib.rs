// Rust translation of c_src/src/driver.c (CWE-190 style integer overflow demo).
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

// Names and control flow deliberately mirror the original C source.
#![allow(non_snake_case, unused_assignments)]

use std::ffi::{c_char, c_int};

extern "C" {
    // Variadic C printf: used so that output goes through the very same libc
    // stdout FILE stream (and buffering) as the original C library did.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `<limits.h>`'s `CHAR_MAX`. `c_char` is signed on some targets (e.g. x86_64)
/// and unsigned on others (e.g. aarch64), exactly like the C `char` type, so
/// deriving the constant from `c_char` reproduces the platform behaviour.
const CHAR_MAX: c_char = c_char::MAX;

/// `void printLine(const char * line)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(b"%s\n\0".as_ptr() as *const c_char, line);
    }
}

/// `void printHexCharLine(char charHex)`
///
/// The `char` argument undergoes the default integer promotion to `int` before
/// being consumed by `%02x`, so negative values (on targets with a signed
/// `char`) are sign-extended and print as eight hex digits.
///
/// The parameter is declared `c_int` rather than `c_char` on purpose. Under the
/// SysV AMD64 psABI the upper 24 bits of a `char` argument register are
/// unspecified, and gcc compiles this C function to narrow defensively --
/// `movsbl %dil,%esi` -- at *every* optimisation level. Declaring the Rust
/// parameter as `c_char` instead makes rustc attach LLVM's `signext` attribute,
/// and an optimised build then trusts the caller and forwards the full 32-bit
/// register (`mov %edi,%esi`), which diverges from the C for any caller that
/// leaves stale bits in the upper 24. Taking a `c_int` and narrowing here
/// reproduces the C's own instruction sequence, and is byte-for-byte identical
/// for every ABI-conforming caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printHexCharLine(charHex: c_int) {
    // Keep only the low byte, then re-apply the platform `char`'s signedness,
    // exactly as the C `char` parameter plus its integer promotion do.
    let charHex = charHex as u8 as c_char;
    printf(b"%02x\n\0".as_ptr() as *const c_char, charHex as c_int);
}

/// `void bad()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data: c_char;
    data = CHAR_MAX;
    if data > 0 {
        // char result = data * 2;  -- promotion to int, then truncation back
        // to char, which overflows for CHAR_MAX.
        let result: c_char = ((data as c_int) * 2) as c_char;
        printHexCharLine(result as c_int);
    }
}

/// `static void goodG2B()`
unsafe fn goodG2B() {
    let data: c_char;
    data = 2;
    if data > 0 {
        let result: c_char = ((data as c_int) * 2) as c_char;
        printHexCharLine(result as c_int);
    }
}

/// `static void goodB2G()`
unsafe fn goodB2G() {
    let mut data: c_char;
    data = b' ' as c_char;
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: c_char = ((data as c_int) * 2) as c_char;
            printHexCharLine(result as c_int);
        } else {
            printLine(
                b"data value is too large to perform arithmetic safely.\0".as_ptr()
                    as *const c_char,
            );
        }
    }
}

/// `void good()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    goodG2B();
    goodB2G();
}

/// `void driver(int useGood)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
