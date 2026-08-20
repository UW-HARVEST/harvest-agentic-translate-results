// Translated from c_src/src/main.c
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

//! C-ABI surface of the translation. Mirrors the exported symbols of the
//! shared library built from `c_src/src/main.c` (`driver` and `main`).

// When this crate is compiled as a test harness the exported `main` below is
// cfg-ed out, which leaves parts of the translation unreferenced.
#[cfg_attr(test, allow(dead_code))]
mod driver_impl;

use std::os::raw::c_int;

/// `void driver(int x)`
#[no_mangle]
pub extern "C" fn driver(x: c_int) {
    driver_impl::driver(x as i32);
}

/// `int main(void)`
///
/// Only present in the real shared library / binary artifacts; when this crate
/// is compiled as a test harness the symbol would collide with the harness's
/// own `main`.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    driver_impl::run() as c_int
}
