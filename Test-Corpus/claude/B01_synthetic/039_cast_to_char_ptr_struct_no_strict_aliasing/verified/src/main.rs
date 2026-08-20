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

//! Executable target.
//!
//! `#![no_main]` is used so that the `#[no_mangle] extern "C" fn main()` defined
//! in [`imp`] is the real ELF entry point that the C runtime calls — exactly
//! like the `int main()` of `c_src/src/main.c`. This keeps a single copy of the
//! translation shared with the `cdylib` target (`src/lib.rs`), and keeps the
//! `main` symbol byte-compatible with the C build.
//! (`cfg(test)` builds — `cargo build --all-targets` — get libtest's own entry
//! point instead, so `no_main` is applied only outside of them.)
#![cfg_attr(not(test), no_main)]

#[path = "imp.rs"]
mod imp;
