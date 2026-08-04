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

use std::os::raw::c_char;

/// Equivalent of C's `strcspn`: returns the length of the initial segment of
/// `s1` which consists entirely of bytes not present in `s2`.
///
/// # Safety
/// `s1` and `s2` must be valid pointers to NUL-terminated C strings.
unsafe fn strcspn(s1: *const c_char, s2: *const c_char) -> usize {
    // Build a 256-bit table of bytes that appear in s2.
    let mut table = [false; 256];
    let mut p = s2;
    while *p != 0 {
        table[*p as u8 as usize] = true;
        p = p.add(1);
    }

    let mut count: usize = 0;
    let mut q = s1;
    while *q != 0 {
        if table[*q as u8 as usize] {
            break;
        }
        count += 1;
        q = q.add(1);
    }
    count
}

/// C-compatible entry point matching `void driver(const char *s1, const char *s2)`.
///
/// # Safety
/// `s1` and `s2` must be valid pointers to NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    println!("{}", strcspn(s1, s2));
}
