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

//! Rust translation of c_src/src/main.c
//!
//! Behavior is preserved exactly, including the intentional defect in `bad()`
//! where the result of `intOne + intTwo` is computed and discarded rather than
//! assigned to `intSum`, so `bad()` prints `0` twice.

use std::cell::RefCell;
use std::io::Write;
use std::os::unix::io::FromRawFd;

// Declared directly rather than pulling in the `libc` crate: both symbols come
// from the C runtime that Rust already links against on Linux.
extern "C" {
    fn isatty(fd: i32) -> i32;
    fn signal(signum: i32, handler: usize) -> usize;
}

const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

/// glibc sizes the stdout buffer from the target's `st_blksize`, which is 4096
/// on ordinary Linux filesystems and pipes.
const BUFSIZ: usize = 4096;

/// Emulates C `stdout` as glibc presents it, because the buffering mode is
/// observable through exit status when the reader of a pipe goes away:
///
/// * to a TTY, stdio is line buffered, so each `printf` reaches the fd at its
///   newline;
/// * to anything else (pipe, file, closed fd) stdio is fully buffered, so the
///   program's whole output leaves in a single `write` at exit.
///
/// A line-buffered Rust `LineWriter` would issue one `write` per line, letting
/// a reader that stops early break the pipe mid-output and kill us with
/// `SIGPIPE` where the C program had already handed all 74 bytes over in one
/// successful write and exited 0.
struct CStdout {
    buf: Vec<u8>,
    line_buffered: bool,
}

impl CStdout {
    fn new() -> Self {
        // isatty on a closed descriptor fails, which lands on the fully
        // buffered path -- the same choice glibc makes.
        let line_buffered = unsafe { isatty(1) } == 1;
        CStdout {
            buf: Vec::with_capacity(BUFSIZ),
            line_buffered,
        }
    }

    /// Hands the accumulated bytes to fd 1. Write errors are discarded: C's
    /// `printf`/`fflush` only return a negative value and set `errno`, and this
    /// program inspects neither, so a failed write must not alter the exit
    /// status. `SIGPIPE` still terminates the process from inside the syscall,
    /// which is exactly how the C binary dies.
    fn flush_buf(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        // Borrow fd 1 without taking ownership; ManuallyDrop keeps the File
        // from closing the real stdout when it goes out of scope.
        let mut f = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(1) });
        let _ = f.write_all(&self.buf);
        self.buf.clear();
    }

    fn write_str(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
        // A full buffer drains even in fully buffered mode, matching stdio.
        if self.buf.len() >= BUFSIZ || (self.line_buffered && self.buf.contains(&b'\n')) {
            self.flush_buf();
        }
    }
}

thread_local! {
    static STDOUT: RefCell<CStdout> = RefCell::new(CStdout::new());
}

/// `printf`-equivalent used by the two print helpers below.
fn c_print(s: &str) {
    STDOUT.with(|o| o.borrow_mut().write_str(s));
}

/// The implicit `fflush(stdout)` that C performs when `main` returns.
fn c_flush() {
    STDOUT.with(|o| o.borrow_mut().flush_buf());
}

/// Mirrors the C `printLine(const char *line)`.
///
/// The C function guards against a NULL pointer before printing; the
/// equivalent guard here is the `Option` being `Some`. Callers in this program
/// always pass a non-null literal, so the guard never rejects anything, but it
/// is retained to keep the control flow identical.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        // C: printf("%s\n", line);
        c_print(line);
        c_print("\n");
    }
}

/// Mirrors the C `printIntLine(int intNumber)`.
fn print_int_line(int_number: i32) {
    // C: printf("%d\n", intNumber);
    // Rust's Display for i32 matches "%d" over the whole range, including
    // i32::MIN, so no special casing is required.
    c_print(&int_number.to_string());
    c_print("\n");
}

/// Mirrors the C `bad()`.
///
/// The original C body is:
///
/// ```c
/// int intOne = 1, intTwo = 1, intSum = 0;
/// printIntLine(intSum);
/// intOne + intTwo;   /* result discarded -- the defect */
/// printIntLine(intSum);
/// ```
///
/// `intSum` is never updated, so both prints emit `0`. This is reproduced
/// faithfully and deliberately not "fixed".
fn bad() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = 0;
    print_int_line(int_sum);
    // The unused sum, exactly as in the C source: computed, then discarded.
    let _ = int_one + int_two;
    print_int_line(int_sum);
}

/// Mirrors the C `good()`, which correctly assigns the sum to `intSum`.
fn good() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
    print_int_line(int_sum);
    int_sum = int_one + int_two;
    print_int_line(int_sum);
}

fn main() {
    // The Rust runtime installs SIG_IGN for SIGPIPE before main; C programs
    // inherit the default disposition. Restoring SIG_DFL makes a write to a
    // broken pipe terminate this process by signal 13, as the C binary does,
    // instead of silently swallowing EPIPE and exiting 0.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }

    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));

    // C `main` returns 0; stdio flushes at exit.
    c_flush();
    std::process::exit(0);
}
