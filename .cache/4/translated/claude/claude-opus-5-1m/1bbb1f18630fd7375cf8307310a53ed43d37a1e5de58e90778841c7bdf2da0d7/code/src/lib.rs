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

//! Public ABI of this cdylib (matches `nm -D` of the C shared library):
//!
//! * `driver`  -- from `c_src/src/driver.c` / `c_src/include/driver.h`

use std::ffi::{c_char, c_int};

extern "C" {
    /// The C implementation writes through the C runtime's `printf`, i.e. onto
    /// the process-wide C `stdout` stream.  Calling the very same function here
    /// keeps the emitted bytes *and* the stream buffering / flush-at-exit
    /// behaviour identical to the original library.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Translation of:
///
/// ```c
/// void driver(int x) {
///     for (int i = 0, j = 0; i < x; i++, j += 2) {
///         printf("%d %d\n", i, j);
///     }
/// }
/// ```
///
/// `i` and `j` are C `int`s, so the increments are performed with wrapping
/// arithmetic to mirror what the C compiler emits on the target platform
/// instead of panicking.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_int) {
    let fmt = c"%d %d\n";

    let mut i: c_int = 0;
    let mut j: c_int = 0;
    while i < x {
        printf(fmt.as_ptr(), i, j);

        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}
