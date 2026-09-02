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

//! Translation of `c_src/src/sillymain.c` (declared by `c_src/src/sillymain.h`).

use std::io::Write;

/// Translation of:
///
/// ```c
/// int helloworld() {
///     printf("Hello World!\n");
///     return 0;
/// }
/// ```
///
/// Writes exactly the 13 bytes `Hello World!\n` to stdout and returns 0. As in
/// the C original, the return value of the print is discarded and never checked.
pub fn helloworld() -> i32 {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    // `printf("Hello World!\n")` — the trailing newline is part of the format
    // string, so the output is `Hello World!` followed by a single '\n'.
    let _ = out.write_all(b"Hello World!\n");

    // C's stdio flushes at normal process exit; `std::process::exit` in `main`
    // bypasses Rust's flush-at-exit, so flush here to guarantee the bytes land.
    let _ = out.flush();

    0
}
