// Rust translation of c_src/src/main.c — C ABI surface.
//
// The C source compiles to a shared library that exports exactly two dynamic
// symbols (`run` and `main`, everything else being `static`). This crate root
// re-exports the same two symbols with the same C ABI so that an external
// consumer — and the differential tests — can drive the Rust translation
// exactly like the C library.
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

#[path = "imp.rs"]
// Under `cfg(test)` the exported `main` below is compiled out, which would make
// the parsing helpers look unused.
#[cfg_attr(test, allow(dead_code))]
mod imp;

/// C: `void run(int extra_bedrooms)`
#[no_mangle]
pub extern "C" fn run(extra_bedrooms: std::os::raw::c_int) {
    imp::run(extra_bedrooms as i32);
}

/// C: `int main()`
///
/// The C translation unit is built as a shared object as well as an executable,
/// so `main` is part of its dynamic symbol table. Keep the same symbol here.
///
/// Hidden from `cfg(test)` builds only because libtest supplies its own `main`.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> std::os::raw::c_int {
    imp::c_main() as std::os::raw::c_int
}
