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

extern "C" {
    /// libc `printf`. Used instead of Rust's own `stdout` so that output
    /// interleaving and buffering behaviour match the C library exactly.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `void driver(int x)`
///
/// ```c
/// void driver(int x) {
///     register int y = 2*x;
///     y += 300;
///     printf("%d\n", y);
/// }
/// ```
///
/// `register` is only a storage-class hint in C and has no observable effect,
/// so it is dropped. The arithmetic uses wrapping semantics, which is what
/// the C compiler emits in practice for `int` on the target platforms
/// (signed overflow is UB in C, so any behaviour is permitted; wrapping
/// reproduces the generated code).
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y: c_int = x.wrapping_mul(2);
    y = y.wrapping_add(300);

    // "%d\n\0"
    const FMT: &[u8] = b"%d\n\0";
    unsafe {
        printf(FMT.as_ptr() as *const c_char, y);
    }
}
