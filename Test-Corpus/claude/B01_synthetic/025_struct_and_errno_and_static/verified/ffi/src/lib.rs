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

//! Shared-object mirror of `c_src/src/main.c`.
//!
//! `gcc -shared -fPIC c_src/src/main.c` yields a `.so` whose dynamic symbol
//! table contains exactly two defined application symbols, `main` and `run`
//! (everything else in the translation unit is `static`).  This crate is built
//! as a `cdylib` and exports the very same two symbols with the same C ABI, so
//! the differential tests can `dlopen` both objects and compare them
//! function-for-function.

use std::os::raw::c_int;

/// `int main()` from `c_src/src/main.c`.
///
/// Reads one line from `stdin` and either runs the house twice or prints the
/// error message; always returns `0`.
///
/// Excluded under `cfg(test)` because libtest links its own entry point, and two
/// `main`s in one binary is a hard link error ("entry symbol `main` declared
/// multiple times"). Only the `cdylib` is ever built without `cfg(test)`, and
/// nothing links this crate as an rlib, so the exported symbol is unaffected.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    driver::c_main_with(run)
}

/// `void run(int extra_bedrooms)` from `c_src/src/main.c`.
///
/// Operates on this object's own copy of the process-global `the_house`, exactly
/// like the C `.so` operates on its own `static house_t the_house`.
#[no_mangle]
pub extern "C" fn run(extra_bedrooms: c_int) {
    driver::run_global(extra_bedrooms);
}
