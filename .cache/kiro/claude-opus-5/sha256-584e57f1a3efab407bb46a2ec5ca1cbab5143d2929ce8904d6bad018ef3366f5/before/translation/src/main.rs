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

//! Rust translation of `c_src/src/main.c`.
//!
//! The C program is:
//!
//! ```c
//! int main() {
//!     printf("Hello World!\n");
//!     return 0;
//! }
//! ```
//!
//! It reads no input and writes the fixed string `Hello World!\n` to stdout,
//! then exits with status 0.

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    // `printf("Hello World!\n")` — write the exact bytes, no extra newline.
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    // Ignore write errors to mirror C, which does not check printf's return value.
    let _ = out.write_all(b"Hello World!\n");
    // C's exit-from-main flushes stdio streams; do the same before returning.
    let _ = out.flush();

    // `return 0;`
    ExitCode::from(0)
}
