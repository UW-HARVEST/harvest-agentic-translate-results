// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Shared-library surface, mirroring the one exported by the C shared library.
//!
//! `nm -D` on the C `.so` built from `c_src/src/main.c` defines exactly one
//! symbol: `main`. This crate exports the same symbol with the same C ABI
//! signature (`int main(void)`), so a dlopen()-ing consumer sees an identical
//! surface. See `SYMBOLS.md`.

pub mod hello;

/// `int main(void)` — the only symbol exported by the C shared library.
///
/// This is a plain translation of the C function: it writes `Hello World!\n` to
/// stdout and returns 0. Like the C version it does not touch signal
/// dispositions, does not read `argc`/`argv`, and returns 0 even if the write
/// fails.
#[no_mangle]
pub extern "C" fn main() -> std::ffi::c_int {
    hello::c_main() as std::ffi::c_int
}
