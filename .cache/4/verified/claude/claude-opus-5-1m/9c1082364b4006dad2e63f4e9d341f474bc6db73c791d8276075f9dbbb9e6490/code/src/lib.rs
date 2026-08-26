// Rust translation of c_src/src/main.c -- C ABI exports.
//
// Every symbol the C shared library exports (`fma_array`, `call_fma`, `main`)
// is re-exported here with the exact same name so that an external consumer
// cannot tell the two libraries apart.
//
// Original copyright notice from the C source:
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

pub mod fma;

use std::os::raw::c_int;

/// `void fma_array(int *restrict out, const int *mul1, const int *mul2,
///                 const int *add, int len)`
///
/// # Safety
///
/// Same contract as the C function: for `len > 0`, `out` must be valid for
/// `len` writes and the three input pointers valid for `len` reads.
#[no_mangle]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    fma::fma_array_raw(out, mul1, mul2, add, len)
}

/// `int call_fma(const int *data, int len)`
///
/// # Safety
///
/// Same contract as the C function: for `len > 0`, `data` must be valid for
/// `len` reads.
#[no_mangle]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    fma::call_fma_raw(data, len)
}

/// `int main(void)` -- reads up to 100 integers from stdin with `scanf("%d")`
/// semantics and prints `call_fma(data, i)` followed by a newline.
#[no_mangle]
pub extern "C" fn main() -> c_int {
    fma::main_stdio()
}
