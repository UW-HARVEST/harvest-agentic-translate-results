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

//! Rust translation of `c_src/src/main.c`.
//!
//! The C original models a `const char *` that may be NULL; that is represented
//! here as `Option<&str>` so the NULL guard in `printLine` is preserved rather
//! than optimized away. `helperBad` is defined but never called in the C source,
//! and that dead-code shape is reproduced faithfully instead of being "fixed".

use std::io::Write;

/// Equivalent of the C `printLine(const char *line)`:
/// `printf("%s\n", line)` guarded by a NULL check.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        // `printf("%s\n", line)` — stdout, no extra spacing.
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(line.as_bytes());
        let _ = out.write_all(b"\n");
    }
}

/// Equivalent of the C `static void helperBad()`. Never called, exactly as in
/// the original source.
#[allow(dead_code)]
fn helper_bad() {
    print_line(Some("helperBad()"));
}

/// Equivalent of the C `void bad()`. Note that it does *not* call
/// `helperBad()`; this mirrors the original behavior.
fn bad() {
    print_line(Some("bad()"));
}

/// Equivalent of the C `static void helperGood()`.
fn helper_good() {
    print_line(Some("helperGood()"));
}

/// Equivalent of the C `void good()`.
fn good() {
    print_line(Some("good()"));
    helper_good();
}

fn main() {
    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));

    // C `main` returns 0.
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}
