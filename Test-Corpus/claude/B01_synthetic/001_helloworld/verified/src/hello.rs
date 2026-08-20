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

//! Faithful translation of the body of `int main()` in `c_src/src/main.c`.
//!
//! This is the single source of truth for the translated logic; it is shared by
//! the executable (`src/main.rs`) and by the exported `main` symbol of the
//! shared library (`src/lib.rs`) so the two can never drift apart.

use std::io::Write;

/// Translation of:
///
/// ```c
/// int main() {
///     printf("Hello World!\n");
///     return 0;
/// }
/// ```
///
/// Notes on exact-behavior preservation:
/// * `printf`'s return value is discarded by the C code, so write/flush errors
///   are deliberately ignored here as well (`let _ = ...`). A failing write must
///   NOT change the returned status.
/// * The C function returns 0 unconditionally.
pub fn c_main() -> i32 {
    // C: printf("Hello World!\n");
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"Hello World!\n");
    let _ = out.flush();

    // C: return 0;
    0
}
