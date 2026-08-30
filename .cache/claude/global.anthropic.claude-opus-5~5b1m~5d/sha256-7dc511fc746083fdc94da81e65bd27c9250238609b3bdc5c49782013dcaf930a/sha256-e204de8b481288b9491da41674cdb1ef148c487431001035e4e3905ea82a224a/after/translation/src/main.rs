// Rust translation of c_src/src/main.c
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

use std::cell::RefCell;
use std::io::{self, Write};

// `isatty` decides C's stdio buffering mode for stdout.
extern "C" {
    fn isatty(fd: i32) -> i32;
}

/// Replicates C stdio's stdout buffering discipline:
/// - stdout connected to a terminal  -> line buffered (flush on each newline)
/// - anything else (pipe, file, ...) -> fully buffered (flush at exit)
///
/// This matters only in exotic cases (e.g. a pipe whose reader disappears
/// mid-run), but matching it keeps the syscall pattern -- and therefore the
/// exact point at which a write error can occur -- the same as the C program's.
struct CStdout {
    buf: Vec<u8>,
    line_buffered: bool,
}

impl CStdout {
    fn new() -> Self {
        CStdout {
            buf: Vec::with_capacity(4096),
            line_buffered: unsafe { isatty(1) } == 1,
        }
    }

    /// Append bytes, flushing on newline when line buffered.
    fn write(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
        if self.line_buffered && bytes.contains(&b'\n') {
            self.flush();
        }
    }

    /// Write the buffer out. Like C's printf/fflush, a failure is ignored: the
    /// C program never checks the return value and still returns 0.
    fn flush(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(&self.buf);
        let _ = out.flush();
        self.buf.clear();
    }
}

thread_local! {
    static STDOUT: RefCell<CStdout> = RefCell::new(CStdout::new());
}

fn flush_stdout() {
    STDOUT.with(|s| s.borrow_mut().flush());
}

/// Mirrors `void printLine(const char *line)`.
///
/// The C function guards against a NULL pointer, so the Rust equivalent takes an
/// `Option<&str>` and prints nothing when it is `None`. Every call site in the
/// C program passes a string literal, so the `None` arm is unreachable from the
/// executable; it is kept to mirror the source faithfully.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        // printf("%s\n", line);
        STDOUT.with(|s| {
            let mut out = s.borrow_mut();
            out.write(line.as_bytes());
            out.write(b"\n");
        });
    }
}

/// Mirrors `static void helperBad()`.
///
/// Never called in the original C program (it is a `static` function that is
/// only defined, never referenced); kept here for fidelity.
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

// The Rust runtime sets SIGPIPE to SIG_IGN before `main` runs, but a C program
// inherits the default disposition (terminate). Without restoring SIG_DFL the
// translation exits 0 on a broken pipe where the C program dies from SIGPIPE
// (shell status 141). `signal` comes from libc, which is already linked.
extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

/// Value of `SIGPIPE` on Linux; `SIG_DFL` is the null handler.
const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

fn restore_default_sigpipe() {
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_default_sigpipe();

    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));

    // return 0; -- C flushes stdio streams during exit.
    flush_stdout();
    std::process::exit(0);
}
