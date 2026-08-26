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
    /// Same `printf` the C library links against, so that output encoding,
    /// stream and buffering behaviour are byte-for-byte identical.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Faithful reimplementation of C `strcspn`.
///
/// Returns the length of the maximal initial segment of `s1` that consists
/// entirely of bytes *not* present in `s2`. As in C, both strings are
/// NUL-terminated and the terminating NUL of `s2` is not part of the
/// reject set.
unsafe fn strcspn(s1: *const c_char, s2: *const c_char) -> usize {
    // Build the reject byte set from s2.
    let mut reject = [false; 256];
    let mut p = s2;
    loop {
        let b = *p as u8;
        if b == 0 {
            break;
        }
        reject[b as usize] = true;
        p = p.add(1);
    }

    // Scan s1 until a rejected byte (or the terminating NUL) is found.
    let mut n: usize = 0;
    let mut q = s1;
    loop {
        let b = *q as u8;
        if b == 0 || reject[b as usize] {
            break;
        }
        n += 1;
        q = q.add(1);
    }
    n
}

/// void driver(const char *s1, const char *s2);
///
/// Prints `strcspn(s1, s2)` followed by a newline, exactly as the C original
/// does with `printf("%zu\n", strcspn(s1, s2))`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let n: usize = strcspn(s1, s2);
    printf(b"%zu\n\0".as_ptr() as *const c_char, n);
}
