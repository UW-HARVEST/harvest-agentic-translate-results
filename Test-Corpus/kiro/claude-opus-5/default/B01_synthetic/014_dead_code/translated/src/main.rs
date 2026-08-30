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

//! Rust translation of c_src/src/main.c.
//!
//! Behavior is preserved exactly, including the fact that `bad()` never calls
//! `helper_bad()` (the C original defines `helperBad()` as a static function
//! that is never invoked).

use std::io::{self, Write};

/// Mirrors `void printLine(const char *line)`.
///
/// In C the `line != NULL` check guards the `printf`. In Rust the argument is
/// an `Option<&str>` so that a missing string (the C `NULL` case) results in no
/// output at all, exactly like the original.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        // printf("%s\n", line)
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(line.as_bytes());
        let _ = out.write_all(b"\n");
    }
}

/// Mirrors `static void helperBad()`. Defined but never called, as in the C.
#[allow(dead_code)]
fn helper_bad() {
    print_line(Some("helperBad()"));
}

/// Mirrors `void bad()`.
fn bad() {
    print_line(Some("bad()"));
}

/// Mirrors `static void helperGood()`.
fn helper_good() {
    print_line(Some("helperGood()"));
}

/// Mirrors `void good()`.
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

    // C returns 0 from main; flush stdout the way libc does at exit.
    let _ = io::stdout().flush();
    std::process::exit(0);
}
