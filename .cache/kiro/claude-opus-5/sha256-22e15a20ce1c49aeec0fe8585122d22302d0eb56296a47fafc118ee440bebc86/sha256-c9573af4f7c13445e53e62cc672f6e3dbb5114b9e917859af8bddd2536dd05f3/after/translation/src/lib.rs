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

//! Rust translation of the `driver` C library (c_src/src/driver.c).
//!
//! Public ABI (matches `nm -D libdriver.so` of the C build):
//!   * `printHexCharLine`
//!   * `driver`
//!
//! Byte-identical output is guaranteed by delegating to the platform's
//! `printf` from libc, exactly as the C source does. This also preserves
//! stdout stream/buffering semantics so output interleaves with any C
//! caller's own `stdio` writes identically.

// The C library's identifiers are camelCase; keep them verbatim so the exported
// linker symbols match the C ABI exactly.
#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};

unsafe extern "C" {
    /// `int printf(const char *restrict format, ...);` from the C runtime.
    ///
    /// Linking against libc's `printf` (rather than reimplementing formatting
    /// with Rust's `std::io`) is what makes the output byte-identical and keeps
    /// the same stdout buffering behaviour as the original library.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// The exact format string used by the C implementation: `"%02x\n"`.
///
/// Stored as a NUL-terminated byte string so it can be handed straight to
/// `printf`.
const HEX_CHAR_LINE_FMT: &[u8; 6] = b"%02x\n\0";

/// Translation of:
///
/// ```c
/// void printHexCharLine (char charHex)
/// {
///     printf("%02x\n", charHex);
/// }
/// ```
///
/// Note the C semantics that must be preserved verbatim: `charHex` has type
/// `char`, which on the target ABI is **signed**. Passing it to the variadic
/// `printf` applies the integer promotions, sign-extending it to `int`. The
/// `%02x` conversion then reinterprets that `int` as `unsigned int`. So a
/// negative `char` such as `-1` prints as `ffffffff`, not `ff`. This is
/// arguably a bug in the original library, but it is reproduced exactly rather
/// than fixed.
///
/// ## Why the parameter is declared `c_int` and not `c_char`
///
/// The exported symbol is ABI-identical either way — on the SysV AMD64 ABI a
/// `char` argument travels in the low byte of `%edi` — but the *observable
/// behaviour for a non-narrowed argument register differs*, and the C's
/// behaviour is the ground truth.
///
/// GCC compiles the C function to
///
/// ```text
/// mov    %edi,%eax
/// mov    %al,-0x4(%rbp)      ; spill only the LOW BYTE  -> truncation
/// movsbl -0x4(%rbp),%eax     ; reload sign-extended     -> promotion
/// ```
///
/// i.e. it *re-narrows* the incoming register to 8 bits before promoting. A
/// Rust `extern "C" fn(c_char)` instead compiles to a bare `mov %edi,%esi`,
/// because the ABI lets the callee assume the caller already narrowed. Any
/// caller that passes a value with non-zero high bytes — which is exactly what
/// happens when an `int` reaches a narrow C parameter, the same situation as an
/// out-of-range value reaching a C `enum` parameter — would then see
/// `printHexCharLine(0x100)` print `100` in Rust but `00` in C.
///
/// Declaring the parameter as `c_int` and truncating explicitly reproduces
/// GCC's `mov %al` + `movsbl` pair, making the two libraries agree for every
/// 32-bit argument value, not just the properly narrowed ones. For a
/// well-behaved caller passing a real `char` the behaviour is unchanged.
#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(charHex: c_int) {
    // `mov %al, ...`: the C callee only ever looks at the low byte of the
    // argument register.
    let narrowed: c_char = charHex as c_char;
    // `movsbl`: integer promotion of the signed `char` argument back to `int`.
    let promoted: c_int = narrowed as c_int;

    unsafe {
        printf(HEX_CHAR_LINE_FMT.as_ptr() as *const c_char, promoted);
    }
}

/// Translation of:
///
/// ```c
/// void driver(char data)
/// {
///     char result = data + 1;
///     printHexCharLine(result);
/// }
/// ```
///
/// `data + 1` is computed in `int` after promotion and then converted back to
/// `char`. That conversion is a wrapping truncation on this ABI, so
/// `driver(0x7f)` yields `result == -128` (printed as `ffffff80`) and
/// `driver(0xff /* -1 */)` yields `result == 0` (printed as `00`).
///
/// The parameter is declared `c_int` for the same reason as
/// [`printHexCharLine`]: GCC emits `mov %al` / `movzbl` here too, so only the
/// low byte of the argument register may influence the result.
#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_int) {
    // Truncate the argument register to a byte first, exactly as the C does,
    // then add 1 in the promoted `int` domain and truncate back to `char`.
    let narrowed: c_char = data as c_char;
    let result: c_char = (narrowed as c_int).wrapping_add(1) as c_char;
    printHexCharLine(result as c_int);
}
