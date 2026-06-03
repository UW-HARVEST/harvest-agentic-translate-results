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

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

/// Counts the number of occurrences of byte `c` in the C string `in_str`.
///
/// Mirrors the behavior of the original C `foo` function which used
/// `strchr` in a loop to count occurrences of a character.
pub fn foo(input: &[u8], c: u8) -> i32 {
    let mut res: i32 = 0;
    for &b in input {
        if b == c {
            res += 1;
        }
    }
    res
}

/// Rust-native driver: prints the count of 'A' and 'x' bytes in the input.
pub fn driver_rs(input: &[u8]) {
    println!("A: {}", foo(input, b'A'));
    println!("x: {}", foo(input, b'x'));
}

/// C-compatible foo function. `in_str` must be a valid NUL-terminated C string.
///
/// # Safety
/// `in_str` must point to a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn foo_c(in_str: *const c_char, c: c_char) -> c_int {
    if in_str.is_null() {
        return 0;
    }
    let cstr = CStr::from_ptr(in_str);
    foo(cstr.to_bytes(), c as u8) as c_int
}

/// C-compatible driver function. `in_str` must be a valid NUL-terminated C string.
///
/// # Safety
/// `in_str` must point to a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn driver(in_str: *const c_char) {
    if in_str.is_null() {
        return;
    }
    let cstr = CStr::from_ptr(in_str);
    driver_rs(cstr.to_bytes());
}
