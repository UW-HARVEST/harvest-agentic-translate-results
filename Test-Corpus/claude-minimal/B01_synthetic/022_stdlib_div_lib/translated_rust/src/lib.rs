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

use std::os::raw::c_int;

/// Equivalent to C standard library's `div_t` struct returned by `div()`.
#[repr(C)]
pub struct DivT {
    pub quot: c_int,
    pub rem: c_int,
}

/// Equivalent to the C standard library's `div()` function.
///
/// Computes the quotient and remainder of `x / y` simultaneously.
fn div(x: c_int, y: c_int) -> DivT {
    DivT {
        quot: x / y,
        rem: x % y,
    }
}

/// Equivalent to the C `driver(int x, int y)` function.
///
/// Prints the quotient and remainder of `x / y`.
#[no_mangle]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let result = div(x, y);
    println!("quotient: {}, remainder: {}", result.quot, result.rem);
}
