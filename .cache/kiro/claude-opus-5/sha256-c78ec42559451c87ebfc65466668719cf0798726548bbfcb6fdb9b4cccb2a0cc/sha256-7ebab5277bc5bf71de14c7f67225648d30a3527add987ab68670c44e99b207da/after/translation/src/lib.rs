// Rust translation of c_src/src/hello.c
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

use std::ffi::c_int;
use std::io::Write;

/// Mirrors `int helloworld()` from `c_src/src/hello.c`.
///
/// The C implementation calls `printf("Hello World!\n")` and returns 0. The
/// return value of `printf` is discarded there, so any write failure is
/// likewise ignored here in order to reproduce the original behaviour exactly.
#[unsafe(no_mangle)]
pub extern "C" fn helloworld() -> c_int {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    // Write the exact byte sequence produced by the C `printf` call.
    let _ = handle.write_all(b"Hello World!\n");
    // C stdio would flush this at process exit (or immediately when stdout is a
    // TTY); flushing here keeps the emitted bytes and their ordering identical
    // without depending on Rust's buffering policy.
    let _ = handle.flush();
    0
}
