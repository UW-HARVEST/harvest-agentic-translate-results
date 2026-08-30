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

//! The pieces of `<stdio.h>` / `<stdlib.h>` that the C sources use.
//!
//! The C programs write their results with `printf`/`fprintf` and parse their
//! arguments with `atoi`. Re-implementing those in Rust would *not* be
//! byte-identical: `printf` on `stdout` is fully buffered when stdout is not a
//! terminal (Rust's `println!` is line buffered, so the interleaving with a C
//! caller's own output would differ), `%s` with a null pointer prints `(null)`
//! under glibc, and `atoi` has its own saturation/truncation behavior for
//! out-of-range text. The faithful translation therefore keeps calling the same
//! C library routines.

use core::ffi::{c_char, c_int, c_void};

extern "C" {
    /// glibc's `FILE *stderr`.
    pub static mut stderr: *mut c_void;

    pub fn printf(fmt: *const c_char, ...) -> c_int;
    pub fn fprintf(stream: *mut c_void, fmt: *const c_char, ...) -> c_int;
    pub fn atoi(s: *const c_char) -> c_int;
}
