// Rust translation of c_src/src/hello.c
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
    /// C library `printf`, used instead of Rust's `std::io::stdout` so that the
    /// bytes land in the very same libc `stdout` FILE stream (and buffer) the C
    /// implementation used. This keeps output byte-identical and correctly
    /// interleaved with any other C-side stdio writes.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Translation of:
///
/// ```c
/// int helloworld() {
///     printf("Hello World!\n");
///     return 0;
/// }
/// ```
///
/// The header declares `int helloworld();` (no prototype / no parameters), so
/// the exported symbol is plain `helloworld` — there are no namespace-renaming
/// macros in `hello.h`.
#[unsafe(no_mangle)]
pub extern "C" fn helloworld() -> c_int {
    // The C source passes a plain string literal to printf; the return value of
    // printf is discarded there, so it is discarded here too.
    const MESSAGE: &[u8; 14] = b"Hello World!\n\0";
    unsafe {
        printf(MESSAGE.as_ptr() as *const c_char);
    }
    0
}
