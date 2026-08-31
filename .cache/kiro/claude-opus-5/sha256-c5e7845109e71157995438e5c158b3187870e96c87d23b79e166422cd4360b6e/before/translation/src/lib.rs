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

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    /// The platform `printf`. Calling it (rather than Rust's `println!`) keeps
    /// stdout buffering semantics and interleaving with a C caller's own
    /// output identical to the original library.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Format string equivalent to the C literal `"%d %d\n"` (NUL-terminated).
const FMT: &[u8; 7] = b"%d %d\n\0";

/// void driver(int x);
///
/// for (int i = 0, j = 0; i < x; i++, j += 2) printf("%d %d\n", i, j);
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut i: c_int = 0;
    let mut j: c_int = 0;
    while i < x {
        unsafe {
            printf(FMT.as_ptr() as *const c_char, i, j);
        }
        // `i++, j += 2` — wrapping mirrors the machine behaviour of the C code
        // without introducing a panic that the original would not have.
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}
