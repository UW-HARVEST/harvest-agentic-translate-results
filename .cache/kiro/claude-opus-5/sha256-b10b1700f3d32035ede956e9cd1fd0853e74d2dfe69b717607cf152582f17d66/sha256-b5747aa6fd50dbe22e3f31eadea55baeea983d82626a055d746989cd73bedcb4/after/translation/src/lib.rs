// Rust translation of c_src/src/driver.c
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
use std::ffi::c_int;

unsafe extern "C" {
    /// C standard library `printf`. Used directly so that output goes through
    /// the same `stdout` `FILE` stream (and therefore the same buffering and
    /// interleaving behavior) as the original C implementation.
    #[link_name = "printf"]
    safe fn c_printf(format: *const c_char, ...) -> c_int;
}

/// Translation of:
///
/// ```c
/// void driver(int x) {
///     register int y = 2*x;
///     y += 300;
///     printf("%d\n", y);
/// }
/// ```
///
/// `wrapping_*` arithmetic is used to reproduce the two's-complement wrap-around
/// that the C code exhibits in practice on overflow (signed overflow is UB in C,
/// but the emitted code wraps).
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y: c_int = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    c_printf(c"%d\n".as_ptr(), y);
}
