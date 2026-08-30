// Rust translation of c_src/src/main.c
//
// Behavior is preserved exactly, including the intentional defect in `bad()`
// where the result of `intOne + intTwo` is computed and discarded instead of
// being stored in `intSum` (CWE-482: Comparing/using an unused value).
//
// Original C copyright notice:
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

use std::io::{self, BufWriter, LineWriter, Write};

/// Restore the default disposition of `SIGPIPE`.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, whereas a C
/// program started from a shell inherits `SIG_DFL`. Without this, writing to a
/// pipe whose reader has gone away kills the C program with signal 13 while the
/// Rust program would merely see `EPIPE`, ignore it (neither program checks
/// `printf`'s return value) and exit 0 — a difference in exit status.
fn reset_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    // SAFETY: `signal` is a plain libc call; restoring the default handler for
    // SIGPIPE is exactly the state a C program starts in.
    unsafe {
        extern "C" {
            fn signal(signum: i32, handler: usize) -> usize;
        }
        let _ = signal(SIGPIPE, SIG_DFL);
    }
}

/// True if the given file descriptor refers to a terminal.
fn is_a_tty(fd: i32) -> bool {
    // SAFETY: `isatty` only inspects the descriptor and has no side effects.
    unsafe {
        extern "C" {
            fn isatty(fd: i32) -> i32;
        }
        isatty(fd) == 1
    }
}

/// Emulates C stdio's buffering choice for `stdout`: line buffered when it is a
/// terminal, fully buffered otherwise. Rust's `io::stdout()` is always line
/// buffered, which would split the output into one `write(2)` per line instead
/// of C's single flush at exit. That granularity is observable when a reader
/// closes the pipe partway through.
enum COut {
    Line(LineWriter<io::Stdout>),
    Full(BufWriter<io::Stdout>),
}

impl Write for COut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            COut::Line(w) => w.write(buf),
            COut::Full(w) => w.write(buf),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            COut::Line(w) => w.flush(),
            COut::Full(w) => w.flush(),
        }
    }
}

impl COut {
    fn new() -> Self {
        let stdout = io::stdout();
        if is_a_tty(1) {
            COut::Line(LineWriter::new(stdout))
        } else {
            // BUFSIZ on glibc; the whole output is far smaller, so it becomes a
            // single write at exit, matching the C program.
            COut::Full(BufWriter::with_capacity(8192, stdout))
        }
    }
}

/// Mirrors `void printLine(const char *line)`.
///
/// The C version guards against a NULL pointer, so the Rust version takes an
/// `Option<&str>` and prints nothing for `None`.
fn print_line<W: Write>(out: &mut W, line: Option<&str>) {
    if let Some(line) = line {
        // printf("%s\n", line);
        let _ = writeln!(out, "{}", line);
    }
}

/// Mirrors `void printIntLine(int intNumber)`.
fn print_int_line<W: Write>(out: &mut W, int_number: i32) {
    // printf("%d\n", intNumber);
    let _ = writeln!(out, "{}", int_number);
}

/// Mirrors `void bad()`.
///
/// The addition result is intentionally discarded (as in the original C), so
/// `int_sum` stays 0 and is printed as 0 both times.
fn bad<W: Write>(out: &mut W) {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = 0;
    print_int_line(out, int_sum);
    // `intOne + intTwo;` — value computed, never assigned. Reproduced verbatim.
    // wrapping_add keeps the semantics of C `int` arithmetic without panicking.
    let _ = int_one.wrapping_add(int_two);
    print_int_line(out, int_sum);
}

/// Mirrors `void good()`.
fn good<W: Write>(out: &mut W) {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
    print_int_line(out, int_sum);
    int_sum = int_one.wrapping_add(int_two);
    print_int_line(out, int_sum);
}

fn main() {
    // Match the signal disposition a C program is started with.
    reset_sigpipe();

    let mut out = COut::new();

    print_line(&mut out, Some("Calling good()..."));
    good(&mut out);
    print_line(&mut out, Some("Finished good()"));
    print_line(&mut out, Some("Calling bad()..."));
    bad(&mut out);
    print_line(&mut out, Some("Finished bad()"));

    // Flush explicitly; C's exit from main flushes stdout. Errors are discarded
    // because the C never inspects printf's or exit's flush result either.
    let _ = out.flush();

    // return 0;
    std::process::exit(0);
}
