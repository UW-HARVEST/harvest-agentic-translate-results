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

// We route all output through the C library's `printf` so that stdout
// buffering, ordering and formatting are byte-for-byte identical to the
// original C library (including interleaving with any C caller's own output).
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `limits.h`'s `CHAR_MAX` for the platforms where `char` is signed
/// (the value the original C library was compiled with).
const CHAR_MAX: c_int = c_char::MAX as c_int;

/// void printLine (const char * line)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(b"%s\n\0".as_ptr() as *const c_char, line);
    }
}

/// void printHexCharLine (char charHex)
///
/// NOTE: in C, `charHex` undergoes the default argument promotion to `int`
/// before being consumed by `%02x`, which then reinterprets it as `unsigned
/// int`. For negative values this therefore prints eight hex digits
/// (e.g. -2 => "fffffffe"). That behaviour is reproduced faithfully here.
unsafe fn print_hex_char_line_impl(charHex: c_char) {
    let promoted: c_int = charHex as c_int;
    printf(b"%02x\n\0".as_ptr() as *const c_char, promoted);
}

/// Exported ABI wrapper for `void printHexCharLine(char charHex)`.
///
/// The parameter is declared as `c_int` rather than `c_char` **on purpose**.
/// The x86-64 psABI leaves the upper 24 bits of an argument register holding a
/// `char` unspecified, and the two toolchains resolve that differently:
///
/// * GCC's callee ignores them — it emits `mov %edi,%eax; mov %al,…;
///   movsbl …,%eax`, i.e. it truncates the register to 8 bits and sign-extends,
///   so the C library's observable behaviour is a pure function of the LOW BYTE.
/// * Rust/LLVM tags an `extern "C" fn(c_char)` parameter `signext` and therefore
///   *assumes* the caller already extended it. At `-O` the truncation is elided
///   (`mov %edi,%esi`), so garbage in the upper bits leaks into the `%02x`
///   output and diverges from C.
///
/// Taking the full register and truncating explicitly reproduces GCC's codegen
/// byte-for-byte for all 2^32 possible register values, and is indistinguishable
/// from `fn(c_char)` for any caller that does extend correctly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printHexCharLine(charHex: c_int) {
    print_hex_char_line_impl(charHex as c_char)
}

/// void bad()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data: c_char;
    data = CHAR_MAX as c_char;
    if data as c_int > 0 {
        // char result = data * 2;  (int arithmetic, truncated back to char)
        let result: c_char = ((data as c_int) * 2) as c_char;
        printHexCharLine(result as c_int);
    }
}

/// static void goodG2B()
unsafe fn goodG2B() {
    let data: c_char;
    data = 2;
    if data as c_int > 0 {
        let result: c_char = ((data as c_int) * 2) as c_char;
        printHexCharLine(result as c_int);
    }
}

/// static void goodB2G()
unsafe fn goodB2G() {
    let mut data: c_char;
    data = b' ' as c_char;
    data = CHAR_MAX as c_char;
    if data as c_int > 0 {
        if (data as c_int) < (CHAR_MAX / 2) {
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

/// void good()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    goodG2B();
    goodB2G();
}

/// void driver(int useGood)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
