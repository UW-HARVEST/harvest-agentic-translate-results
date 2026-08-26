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
//
//! Core translation of `c_src/src/main.c`.
//!
//! This module holds the behaviour shared by the executable (`src/main.rs`)
//! and the C-ABI shared library (`src/lib.rs`). Output is byte-oriented
//! (`&[u8]`, not `&str`) because C's `printf("%s\n", line)` copies raw bytes
//! up to the terminating NUL and never validates UTF-8.

use std::fs::File;
use std::io::Write;
use std::mem::ManuallyDrop;
use std::os::unix::io::FromRawFd;

/// Writes raw bytes to file descriptor 1 (stdout).
///
/// C's `printf`/`puts` write to the `stdout` `FILE*`, i.e. fd 1, and the
/// return value is discarded, so write errors are ignored here as well.
/// The descriptor is wrapped in `ManuallyDrop` so that fd 1 is never closed.
fn write_stdout(bytes: &[u8]) {
    // SAFETY: fd 1 is owned by the process for its whole lifetime; the
    // `ManuallyDrop` guarantees the `File` wrapper never closes it.
    let mut out = ManuallyDrop::new(unsafe { File::from_raw_fd(1) });
    let _ = out.write_all(bytes);
}

/// Mirrors `void printLine(const char *line)`.
///
/// `None` models a NULL pointer (the C function prints nothing);
/// `Some(bytes)` models a non-NULL pointer to a NUL-terminated string, whose
/// bytes (excluding the NUL) are printed followed by a single `\n`.
pub fn print_line(line: Option<&[u8]>) {
    if let Some(line) = line {
        // printf("%s\n", line): the string bytes then one newline.
        let mut buf = Vec::with_capacity(line.len() + 1);
        buf.extend_from_slice(line);
        buf.push(b'\n');
        write_stdout(&buf);
    }
}

/// Convenience wrapper for the literal call sites in the C source.
fn print_literal(line: &[u8]) {
    print_line(Some(line));
}

/// Mirrors `static void helperBad(void)`.
///
/// The C function is `static` and never called (so it is neither exported nor
/// reachable). It is reproduced here for fidelity and is likewise never
/// called; `bad()` deliberately does not invoke it.
#[allow(dead_code)]
pub fn helper_bad() {
    print_literal(b"helperBad()");
}

/// Mirrors `void bad(void)`.
pub fn bad() {
    print_literal(b"bad()");
}

/// Mirrors `static void helperGood(void)`.
fn helper_good() {
    print_literal(b"helperGood()");
}

/// Mirrors `void good(void)`.
pub fn good() {
    print_literal(b"good()");
    helper_good();
}

/// Mirrors `int main(int argc, char *argv[])`.
///
/// `argc`/`argv` are ignored by the C code, so they are not taken here.
/// Always returns 0, exactly like the C `return 0;`.
#[allow(dead_code)]
pub fn c_main() -> i32 {
    print_literal(b"Calling good()...");
    good();
    print_literal(b"Finished good()");
    print_literal(b"Calling bad()...");
    bad();
    print_literal(b"Finished bad()");

    0
}
