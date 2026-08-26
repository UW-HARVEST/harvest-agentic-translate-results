// Rust translation of the C library in c_src/.
//
// Original copyright header from the C sources:
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

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    // int printf(const char *restrict format, ...);
    #[link_name = "printf"]
    unsafe fn c_printf(format: *const c_char, ...) -> c_int;
}

/// Read a `char` parameter out of the raw argument register the way gcc does.
///
/// Both C entry points are declared `void f(char)`. On x86-64 SysV a narrow
/// integer argument occupies the low 8 bits of the argument register and the
/// upper 24 bits are *unspecified*; gcc consequently never trusts them and
/// emits an explicit `movsbl %dil, ...` (verified at `-O0`, `-O1`, `-O2`, `-O3`
/// and `-Os`), i.e. it truncates to the low byte and then sign-extends.
///
/// The exported wrappers below therefore take a `c_int` and truncate here,
/// rather than taking a `c_char`. Declaring the parameter as `c_char` makes
/// rustc attach LLVM's `signext i8` attribute, which *promises* that the caller
/// already sign-extended the value; with optimisations enabled LLVM then folds
/// `sext(trunc(edi))` down to a bare `edi` and the caller's upper bits leak into
/// `printf`. That diverges from the C for any caller that passes a wider value
/// through the same register (an unprototyped/K&R-style call, or an FFI binding
/// that declares the parameter as `int`), which is exactly what happens for a C
/// `enum` parameter handed a value with no valid variant. Taking the argument as
/// a full `c_int` and masking makes the low-byte-only behaviour explicit and
/// optimisation-independent, while remaining bit-for-bit compatible with every
/// conforming caller that passes a real `char`.
#[inline]
fn char_arg(raw: c_int) -> c_char {
    (raw as u32 & 0xff) as u8 as c_char
}

/// `void printHexCharLine(char charHex)`
///
/// C body:
/// ```c
/// printf("%02x\n", charHex);
/// ```
///
/// `charHex` undergoes the default integer promotions to `int` before being
/// passed to `printf`. On this target `char` is signed, so negative values are
/// sign-extended and then printed by `%x` as an `unsigned int` (e.g. `-1`
/// prints as `ffffffff`). The C `printf` is used directly so that both the
/// formatted bytes and the stdio buffering behaviour match the C library
/// exactly.
///
/// `#[inline(never)]` keeps this a real, callable function body so that the
/// exported symbol is what `driver` actually invokes, mirroring gcc's
/// `jmp printHexCharLine@plt`.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
#[inline(never)]
pub unsafe extern "C" fn printHexCharLine(charHex: c_int) {
    // Only the low 8 bits of the argument register are the `char`; gcc reads it
    // with `movsbl %dil, %esi`.
    let char_hex: c_char = char_arg(charHex);
    // Default argument promotion: char -> int (sign-extending on this target).
    let promoted: c_int = char_hex as c_int;
    unsafe {
        c_printf(c"%02x\n".as_ptr(), promoted);
    }
}

/// `void driver(char data)`
///
/// C body:
/// ```c
/// char result = data + 1;
/// printHexCharLine(result);
/// ```
///
/// `data + 1` is computed in `int` and then converted back to `char`, which
/// wraps modulo 256 (implementation-defined conversion as performed by gcc),
/// e.g. `127 + 1 == -128`. gcc emits `add $0x1,%edi; movsbl %dil,%edi`, so only
/// the low byte of the incoming register can affect the result — adding 1 to the
/// full register and then truncating is the same value as adding 1 to the
/// truncated byte, modulo 256.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: c_int) {
    let data: c_char = char_arg(data);
    let result: c_char = data.wrapping_add(1);
    unsafe {
        printHexCharLine(result as c_int);
    }
}
