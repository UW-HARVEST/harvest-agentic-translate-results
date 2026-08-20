// Faithful translation of the body of `main()` in c_src/src/main.c
//
// Original C:
//
//     #include <stdio.h>
//
//     /* interactive echo; ignores arguments, copies stdin to stdout */
//     int main() {
//         char text[128];
//
//         while (fgets(text, 128, stdin)) {
//             fputs(text, stdout);
//         }
//         return 0;
//     }
//
// The observable behaviour of that program is defined entirely by the C
// standard library semantics of `fgets` and `fputs` on the `stdin`/`stdout`
// streams, so this module re-implements those semantics on top of the raw
// file descriptors instead of relying on Rust's (different) `std::io`
// conventions.  The details that matter, and that a naive translation gets
// wrong, are:
//
//  * `fgets(text, 128, stdin)` stores AT MOST 127 bytes.  It stops early after
//    a newline (which is kept in the buffer) and always writes a terminating
//    NUL byte after the last byte stored.  It returns NULL - and therefore
//    ends the loop - only when no byte at all could be read (EOF or error).
//    So the input is consumed in "chunks" that end at a newline, at 127 bytes,
//    or at end of input.
//
//  * `fputs(text, stdout)` writes the C string in `text`, i.e. the bytes up to
//    the FIRST NUL byte.  If the input itself contains NUL bytes, everything in
//    the chunk from the first NUL onwards is silently dropped.
//
//  * `text` is never cleared between iterations, but `fgets` always writes a
//    NUL terminator, so stale bytes are never printed.
//
//  * `main` ignores the result of `fputs`; a failing write does not stop the
//    loop, so stdin is always drained to EOF.
//
//  * `stdout` is block buffered (glibc uses the st_blksize of the file, 4096
//    for pipes and ordinary files) unless it refers to a terminal, in which
//    case it is line buffered.  Nothing is written before the buffer fills up
//    (or, for a terminal, before a newline is queued); the C runtime flushes
//    whatever is left when `main` returns.

use std::fs::File;
use std::io::{IsTerminal, Read, Write};
use std::mem::ManuallyDrop;
use std::os::fd::FromRawFd;

/// Size of the C `char text[128]` buffer.
pub const BUF_SIZE: usize = 128;

/// Buffer size glibc picks for `stdin`/`stdout` when they refer to a pipe or an
/// ordinary file (`st_blksize`).
const STDIO_BUF: usize = 4096;

const STDIN_FD: i32 = 0;
const STDOUT_FD: i32 = 1;

/// A buffered reader over file descriptor 0 that mimics glibc's `stdin`.
struct CStdin {
    fd: ManuallyDrop<File>,
    buf: [u8; STDIO_BUF],
    start: usize,
    end: usize,
    /// Set once the stream reported EOF or an error.  `main`'s `while (fgets(..))`
    /// loop terminates on the first NULL, so a sticky flag is indistinguishable
    /// from glibc's separate EOF/error indicators here.
    done: bool,
}

impl CStdin {
    fn new() -> Self {
        CStdin {
            // Borrowed, not owned: dropping this must not close fd 0.
            fd: ManuallyDrop::new(unsafe { File::from_raw_fd(STDIN_FD) }),
            buf: [0u8; STDIO_BUF],
            start: 0,
            end: 0,
            done: false,
        }
    }

    /// One byte from the stream, or `None` at EOF / on a read error - the
    /// equivalent of `getc` returning `EOF`.
    fn next_byte(&mut self) -> Option<u8> {
        if self.start == self.end {
            if self.done {
                return None;
            }
            loop {
                match self.fd.read(&mut self.buf) {
                    Ok(0) => {
                        self.done = true;
                        return None;
                    }
                    Ok(n) => {
                        self.start = 0;
                        self.end = n;
                        break;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        self.done = true;
                        return None;
                    }
                }
            }
        }
        let b = self.buf[self.start];
        self.start += 1;
        Some(b)
    }
}

/// A buffered writer over file descriptor 1 that mimics glibc's `stdout`,
/// including its choice between line buffering (terminal) and block buffering
/// (everything else).
struct CStdout {
    fd: ManuallyDrop<File>,
    buf: Vec<u8>,
    line_buffered: bool,
}

impl CStdout {
    fn new() -> Self {
        CStdout {
            // Borrowed, not owned: dropping this must not close fd 1.
            fd: ManuallyDrop::new(unsafe { File::from_raw_fd(STDOUT_FD) }),
            buf: Vec::with_capacity(STDIO_BUF),
            line_buffered: std::io::stdout().is_terminal(),
        }
    }

    /// `fputs(s, stdout)` where `s` is the NUL-terminated C string starting at
    /// `text`; `data` is the string content without its terminator.
    fn fputs(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
        if self.line_buffered {
            // glibc flushes a line buffered stream through the last newline.
            if let Some(i) = self.buf.iter().rposition(|&b| b == b'\n') {
                self.flush_first(i + 1);
            }
        } else {
            while self.buf.len() >= STDIO_BUF {
                self.flush_first(STDIO_BUF);
            }
        }
    }

    /// Hand the first `n` buffered bytes to `write(2)`.  Write errors are
    /// dropped, matching both glibc (which just raises the stream's error
    /// indicator and discards the buffer) and `main` (which ignores `fputs`).
    fn flush_first(&mut self, n: usize) {
        let n = n.min(self.buf.len());
        if n == 0 {
            return;
        }
        let chunk: Vec<u8> = self.buf.drain(..n).collect();
        let mut off = 0;
        while off < chunk.len() {
            match self.fd.write(&chunk[off..]) {
                Ok(0) => break,
                Ok(k) => off += k,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    }

    /// The flush the C runtime performs on `exit`.
    fn flush_all(&mut self) {
        self.flush_first(self.buf.len());
    }
}

/// `fgets(buf, BUF_SIZE, stdin)`.
///
/// Stores at most `BUF_SIZE - 1` bytes, stopping after a newline, and appends a
/// NUL terminator.  Returns the number of bytes stored, or `None` where `fgets`
/// would return `NULL` (no byte available at all).
fn c_fgets(stdin: &mut CStdin, buf: &mut [u8; BUF_SIZE]) -> Option<usize> {
    let limit = BUF_SIZE - 1;
    let mut n = 0usize;
    while n < limit {
        match stdin.next_byte() {
            None => break,
            Some(b) => {
                buf[n] = b;
                n += 1;
                if b == b'\n' {
                    break;
                }
            }
        }
    }
    if n == 0 {
        // EOF or error before the first byte: fgets returns NULL and leaves the
        // buffer untouched.
        return None;
    }
    buf[n] = 0;
    Some(n)
}

/// The translated body of the C `main`; the return value is `main`'s exit code.
pub fn run() -> i32 {
    let mut stdin = CStdin::new();
    let mut stdout = CStdout::new();

    // char text[128]; -- deliberately reused, never cleared, across iterations.
    let mut text = [0u8; BUF_SIZE];

    while c_fgets(&mut stdin, &mut text).is_some() {
        // fputs(text, stdout): the C string, i.e. up to the first NUL byte.
        let end = text
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(BUF_SIZE);
        stdout.fputs(&text[..end]);
    }

    // return 0; -- the C runtime flushes stdout on the way out.
    stdout.flush_all();
    0
}
