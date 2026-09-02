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
use std::mem::ManuallyDrop;
use std::os::unix::io::FromRawFd;
use std::sync::Mutex;

extern "C" {
    /// libc `signal(2)`. Declared directly so the crate keeps its
    /// zero-dependency `Cargo.toml`.
    fn signal(signum: i32, handler: usize) -> usize;
    /// libc `isatty(3)`, used to pick the same buffering mode C stdio picks.
    fn isatty(fd: i32) -> i32;
}

/// `SIGPIPE` on Linux.
const SIGPIPE: i32 = 13;
/// `SIG_DFL` — the default disposition.
const SIG_DFL: usize = 0;
/// File descriptor 1.
const STDOUT_FD: i32 = 1;
/// glibc sizes a fully buffered stream from the fd's `st_blksize`, which is
/// 4096 for the pipes and regular files this program is run against.
const STDIO_BUF_SIZE: usize = 4096;

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, which a C
/// program does not do. Left alone, a failed write to a broken pipe would make
/// this program ignore `EPIPE` and exit 0, while the C original is killed by
/// signal 13 (shell status 141). Restoring the default disposition keeps the
/// observable exit status identical to the C program's.
fn restore_default_sigpipe() {
    // Safety: `signal` is a libc function; setting SIGPIPE back to SIG_DFL is
    // exactly the state a C program starts in.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// Pending `printf` output, held so this program buffers stdout the way C stdio
/// does rather than the way Rust's line-buffered `Stdout` does.
static PENDING: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// True when stdout is a terminal. C stdio is line buffered to a tty and fully
/// buffered to anything else; matching that keeps the number and timing of the
/// underlying `write` calls the same as the C program's.
fn stdout_is_line_buffered() -> bool {
    // Safety: `isatty` only inspects the descriptor.
    unsafe { isatty(STDOUT_FD) == 1 }
}

/// Writes the buffered bytes straight to descriptor 1 in a single `write`, the
/// way a C stdio flush does. Write errors are discarded because the C code
/// ignores `printf`'s return value.
fn flush_pending(buf: &mut Vec<u8>) {
    if buf.is_empty() {
        return;
    }
    // Safety: fd 1 is owned by the process; `ManuallyDrop` keeps it from being
    // closed when the `File` goes out of scope.
    let mut out = ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(STDOUT_FD) });
    let _ = out.write_all(buf);
    let _ = out.flush();
    buf.clear();
}

/// Equivalent of the C `printLine(const char *line)`:
/// `printf("%s\n", line)` guarded by a NULL check.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        // `printf("%s\n", line)` — the string then a single newline, with no
        // extra spacing and no field width.
        let mut buf = PENDING.lock().unwrap_or_else(|e| e.into_inner());
        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
        // Line buffered to a tty; otherwise flushed only once the stdio buffer
        // would have filled up.
        if stdout_is_line_buffered() || buf.len() >= STDIO_BUF_SIZE {
            flush_pending(&mut buf);
        }
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
    // Match the signal disposition a C program starts with, before any output.
    restore_default_sigpipe();

    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));

    // C `main` returns 0. Returning from `main` flushes stdio first, which is
    // where a fully buffered stream actually hits the descriptor.
    {
        let mut buf = PENDING.lock().unwrap_or_else(|e| e.into_inner());
        flush_pending(&mut buf);
    }
    std::process::exit(0);
}
