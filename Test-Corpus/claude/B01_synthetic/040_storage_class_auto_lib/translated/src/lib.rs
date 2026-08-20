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
use std::ffi::c_int;

extern "C" {
    // C stdio `printf`, used so that output buffering / interleaving behaviour
    // matches the original C library byte for byte.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Translation of:
///
/// ```c
/// void driver(int x) {
///     auto int y = 2*x;
///     y += 300;
///     printf("%d\n", y);
/// }
/// ```
///
/// `auto` is merely the (default) automatic storage-class specifier for the
/// local `int y`, so it has no effect on the semantics.  Signed arithmetic is
/// reproduced with wrapping semantics, matching the behaviour of the compiled
/// C code on two's-complement hardware.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y: c_int = (2 as c_int).wrapping_mul(x);
    y = y.wrapping_add(300);
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, y);
    }
}
