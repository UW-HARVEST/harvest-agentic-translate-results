// Rust translation of the MIT Lincoln Laboratory `driver` C library.
//
// Original copyright notice from c_src (reproduced verbatim):
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

//! Faithful translation of `c_src/src/driver.c` / `c_src/include/driver.h`.
//!
//! Public ABI (as reported by `nm -D` on the C shared object):
//!   * `printHexCharLine`
//!   * `driver`
//!
//! Both are `void f(char)`.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};

// The C implementation writes through `printf` from `<stdio.h>`.  We call the
// very same function so that the emitted bytes -- and the stdio buffering /
// flush-at-exit behaviour that decides *when* those bytes appear -- are
// bit-for-bit identical to the original library.
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// ABI note: why the exported wrappers take `c_int` and narrow explicitly
// ---------------------------------------------------------------------------
//
// Both C functions are declared `void f(char)`.  In the SysV x86-64 ABI a
// `char` argument travels in the low byte of `%edi`, and GCC's prologue for
// these functions *re-narrows* it before use:
//
//     mov   %edi,%eax
//     mov   %al,-0x4(%rbp)     ; keep only the low 8 bits
//     movsbl -0x4(%rbp),%eax   ; sign-extend them back to int
//
// So the C callee ignores whatever the caller left in bits 8..31.  Rust's
// `extern "C" fn(c_char)` instead lowers the parameter with LLVM's `signext i8`
// attribute, which lets an optimised build *assume* the caller already
// sign-extended and forward `%edi` unchanged.  At `-O` that made
// `printHexCharLine(128i32)` print `80` where the C library prints `ffffff80`.
//
// Taking `c_int` and doing `as u8 as c_char` reproduces GCC's narrowing
// verbatim for every possible register value.  It is ABI-identical for
// conforming `char` callers (the argument register is the same, and truncating
// a properly extended `char` is the identity), while additionally matching the
// C library for the non-conforming/over-wide values a real C caller can also
// produce.

/// C: `printf("%02x\n", charHex);`
///
/// Note the (deliberately preserved) quirk of the original: `charHex` has type
/// `char`, so it undergoes the default argument promotion to `int` before being
/// consumed by `%x`, which reinterprets it as `unsigned int`.  On targets where
/// `char` is signed (e.g. x86-64 Linux) a negative value therefore prints as
/// eight hex digits -- `printHexCharLine(0x80)` yields `ffffff80`, not `80`.
/// This is not a bug we fix here; it is reproduced exactly.
#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(charHex: c_int) {
    // `mov %al, ...` -- the C callee only ever looks at the low 8 bits.
    let charHex: c_char = charHex as u8 as c_char;
    // b"%02x\n\0" -- identical format string to the C source.
    const FORMAT: &[u8; 6] = b"%02x\n\0";
    unsafe {
        // `movsbl` -- the default argument promotion sign-extends `char`.
        printf(FORMAT.as_ptr() as *const c_char, charHex as c_int);
    }
}

/// C:
/// ```c
/// void driver(char data)
/// {
///     char result = data + 1;
///     printHexCharLine(result);
/// }
/// ```
///
/// `data + 1` is evaluated in `int` and then converted back to `char`; GCC/Clang
/// implement that narrowing conversion as a two's-complement truncation, which
/// `wrapping_add` reproduces (so `driver(0x7f)` produces `result == -128`).
#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_int) {
    // `mov %al, ...` -- as above, only the low 8 bits of the incoming register
    // participate.
    let data: c_char = data as u8 as c_char;
    let result: c_char = data.wrapping_add(1);
    printHexCharLine(result as c_int);
}
