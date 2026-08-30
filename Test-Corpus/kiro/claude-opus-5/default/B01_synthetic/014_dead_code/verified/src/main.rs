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
//!
//! Two properties of the C program are *not* free in Rust and are restored
//! explicitly below, because they are observable from outside the process:
//!
//! 1. **SIGPIPE disposition.** The C program inherits the default disposition,
//!    so a write to a pipe with no reader kills it with signal 13. The Rust
//!    standard library sets `SIGPIPE` to `SIG_IGN` before `main` runs, which
//!    would turn that into an ignored `EPIPE` and a 0 exit status. `main`
//!    restores `SIG_DFL`.
//!
//! 2. **stdout buffering discipline.** glibc gives `stdout` full buffering when
//!    it is not a terminal (so this program's output leaves as a single write
//!    at exit) and line buffering when it is. `std::io::Stdout` is always a
//!    `LineWriter`. The `Out` writer below reproduces the C discipline, which
//!    matters for how much output a partially-consuming reader observes before
//!    the pipe breaks.

use std::cell::RefCell;
use std::io::Write;

// Linked from libc, which Rust already links on this target. Declaring the two
// functions directly avoids adding a dependency for two symbols.
extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
    fn isatty(fd: i32) -> i32;
}

const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

/// Restore the signal disposition the C program runs with.
///
/// Rust's runtime ignores `SIGPIPE` so that failed writes surface as `EPIPE`.
/// The C program does not, so a broken pipe must terminate this process by
/// signal, with no exit status, exactly as it terminates the C one.
fn reset_sigpipe() {
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// A stand-in for C's `stdout` FILE stream, with glibc's buffering rules.
struct Out {
    buf: Vec<u8>,
    /// True when stdout is a terminal: glibc line-buffers, so each newline is
    /// handed to the OS immediately. Otherwise it fully buffers and this
    /// program's entire output is written once, at exit.
    line_buffered: bool,
}

impl Out {
    fn new() -> Self {
        Out {
            buf: Vec::new(),
            line_buffered: unsafe { isatty(1) } == 1,
        }
    }

    /// Append bytes, mirroring `printf` into a buffered stream.
    ///
    /// Write errors are discarded because the C program discards `printf`'s
    /// return value; a failing stdout (a closed descriptor, a full device)
    /// leaves its exit status at 0.
    fn write(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
        if self.line_buffered && self.buf.contains(&b'\n') {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(&self.buf);
        let _ = stdout.flush();
        self.buf.clear();
    }
}

thread_local! {
    static OUT: RefCell<Out> = RefCell::new(Out::new());
}

/// Mirrors `void printLine(const char *line)`.
///
/// In C the `line != NULL` check guards the `printf`. In Rust the argument is
/// an `Option<&str>` so that a missing string (the C `NULL` case) results in no
/// output at all, exactly like the original. `main` never passes `None`, just
/// as the C never passes `NULL`.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        // printf("%s\n", line)
        OUT.with(|out| {
            let mut out = out.borrow_mut();
            out.write(line.as_bytes());
            out.write(b"\n");
        });
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
    reset_sigpipe();

    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));

    // C returns 0 from main; libc flushes stdout during exit.
    OUT.with(|out| out.borrow_mut().flush());
    std::process::exit(0);
}
