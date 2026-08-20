// Rust translation of the C library in c_src/.
//
// Original copyright notice from the C sources:
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

use std::ffi::c_char;
use std::ffi::c_double;
use std::ffi::c_int;
use std::ffi::c_ulonglong;

extern "C" {
    // int printf(const char *restrict format, ...);
    fn printf(format: *const c_char, ...) -> c_int;
}

/// C source:
/// ```c
/// typedef union {
///     uint64_t x;
///     double f;
/// } raw_double_t;
///
/// void driver(double f) {
///     raw_double_t u = {.f = f};
///     printf("%llx %a %.4f\n", u.x, f, f);
/// }
/// ```
///
/// The type-punning union read (`u.x` after storing through `u.f`) is exactly
/// the bit pattern of the `double`, i.e. `f64::to_bits`.
///
/// Formatting is delegated to the platform C library's `printf` with the very
/// same format string and arguments so that the emitted bytes (including the
/// hexadecimal-float `%a` conversion and the `%.4f` rounding behavior) as well
/// as the stdout buffering behavior are identical to the C implementation.
#[unsafe(no_mangle)]
pub extern "C" fn driver(f: c_double) {
    let x: u64 = f.to_bits();

    // "%llx %a %.4f\n" as a NUL-terminated C string literal.
    const FMT: &[u8; 14] = b"%llx %a %.4f\n\0";

    unsafe {
        printf(
            FMT.as_ptr() as *const c_char,
            x as c_ulonglong,
            f as c_double,
            f as c_double,
        );
    }
}
