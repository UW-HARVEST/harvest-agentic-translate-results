// Rust translation of c_src/ (MIT Lincoln Laboratory `driver` library).
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
    /// libc `printf`; used directly so that stdout buffering / interleaving
    /// behaviour is byte-for-byte identical to the original C library.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `void driver(int x)`
///
/// Original C:
/// ```c
/// void driver(int x) {
///     register int y = 2*x;
///     y += 300;
///     printf("%d\n", y);
/// }
/// ```
///
/// The `register` storage-class specifier has no observable effect. Signed
/// overflow in `2*x` / `y += 300` is undefined behaviour in C but wraps in
/// practice on the platforms targeted by the C build, so wrapping arithmetic
/// is used here to reproduce that behaviour exactly.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y: c_int = (x as i32).wrapping_mul(2) as c_int;
    y = y.wrapping_add(300);

    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, y);
    }
}
