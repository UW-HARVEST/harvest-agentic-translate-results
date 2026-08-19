// Rust translation of c_src/src/main.c
//
// Original copyright notice from the C source:
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

use std::io::{self, Read, Write};

/// Direct access to file descriptor 0, mirroring what C's stdio does.
///
/// `std::io::stdin()` interposes its own 8 KiB `BufReader`, which would consume
/// 8192 bytes of a shared stream even when only three are needed.  glibc instead
/// reads `st_blksize` (4096 for the usual case) bytes at a time straight from the
/// descriptor and — for a *seekable* stream — seeks the descriptor back to the
/// logical read position when the stream is cleaned up at exit, so the C program
/// leaves the shared file offset exactly at the last byte it actually consumed
/// (observable as `{ ./driver; cat; } < file`).  Both halves are reproduced here.
#[cfg(unix)]
mod sys {
    use std::io::{self, Read};

    extern "C" {
        #[link_name = "read"]
        fn c_read(fd: i32, buf: *mut u8, count: usize) -> isize;
        #[cfg(target_pointer_width = "64")]
        #[link_name = "lseek"]
        fn c_lseek(fd: i32, offset: i64, whence: i32) -> i64;
    }

    const STDIN_FILENO: i32 = 0;
    #[cfg(target_pointer_width = "64")]
    const SEEK_CUR: i32 = 1;

    /// Unbuffered reader over file descriptor 0.
    pub struct Stdin;

    impl Read for Stdin {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = unsafe { c_read(STDIN_FILENO, buf.as_mut_ptr(), buf.len()) };
            if n < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(n as usize)
            }
        }
    }

    /// Push `n` buffered-but-unconsumed bytes back onto a seekable stdin, the way
    /// glibc's exit-time stream cleanup does.  Fails harmlessly (`ESPIPE`) for
    /// pipes and character devices, exactly as it does in C.
    #[cfg(target_pointer_width = "64")]
    pub fn rewind_unread(n: usize) {
        if n > 0 {
            unsafe {
                c_lseek(STDIN_FILENO, -(n as i64), SEEK_CUR);
            }
        }
    }

    #[cfg(not(target_pointer_width = "64"))]
    pub fn rewind_unread(_n: usize) {}
}

#[cfg(not(unix))]
mod sys {
    pub fn rewind_unread(_n: usize) {}
}

/// Minimal emulation of C's `scanf("%d")` conversion reading from stdin.
///
/// `scanf` treats the input as a single stream and freely reads across newline
/// boundaries, but it only ever pulls in as much as the conversion needs (one
/// stdio buffer at a time) and stops the moment it sees a character that cannot
/// extend the conversion, pushing that character back.  The scanner below
/// mirrors that: it reads on demand into a fixed buffer and inspects the next
/// byte with `peek` (never consuming it unless it is part of the conversion), so
/// an unbounded stdin such as `/dev/zero` terminates immediately just as the C
/// program does instead of being slurped into memory.
///
/// Leading whitespace is skipped, an optional sign is accepted, and then a run
/// of decimal digits is consumed.  A conversion that finds no digits is a
/// matching failure and leaves the destination variable untouched (which is
/// exactly what the C program relies on for its `int x = 0, y = 0;` defaults).
struct Scanner<R: Read> {
    inner: R,
    buf: [u8; 4096],
    /// Number of valid bytes in `buf`.
    len: usize,
    /// Read cursor into `buf`.
    pos: usize,
    /// Sticky: once the stream reports end-of-file (or an error, which behaves
    /// like end-of-file here) no further reads are attempted.
    eof: bool,
}

impl<R: Read> Scanner<R> {
    fn new(inner: R) -> Scanner<R> {
        Scanner {
            inner,
            buf: [0u8; 4096],
            len: 0,
            pos: 0,
            eof: false,
        }
    }

    /// Look at the next byte of the stream without consuming it (`ungetc`-like).
    fn peek(&mut self) -> Option<u8> {
        while self.pos == self.len {
            if self.eof {
                return None;
            }
            match self.inner.read(&mut self.buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.pos = 0;
                    self.len = n;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    // A read error behaves like end-of-input for our purposes.
                    self.eof = true;
                    return None;
                }
            }
        }
        Some(self.buf[self.pos])
    }

    /// Consume the byte last returned by `peek`.
    fn bump(&mut self) {
        self.pos += 1;
    }

    /// Bytes pulled from the descriptor but not consumed by any conversion.
    fn unread(&self) -> usize {
        self.len - self.pos
    }

    /// C's `isspace` for the default "C" locale.
    fn is_c_space(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    /// Perform one `%d` conversion.  Returns `None` on input failure (EOF
    /// before any non-whitespace character) or matching failure (no digits).
    fn scan_int(&mut self) -> Option<i32> {
        // %d skips any amount of leading whitespace, including newlines.
        loop {
            match self.peek() {
                Some(b) if Scanner::<R>::is_c_space(b) => self.bump(),
                Some(_) => break,
                None => return None, // input failure
            }
        }

        let mut negative = false;
        match self.peek() {
            Some(b @ (b'+' | b'-')) => {
                negative = b == b'-';
                self.bump();
            }
            _ => {}
        }

        // Accumulate in i128 and saturate: glibc clamps an out-of-range value to
        // the `long` limits and then the `%d` store truncates it to `int`.
        let mut value: i128 = 0;
        let mut digits = false;
        while let Some(b) = self.peek() {
            if !b.is_ascii_digit() {
                break;
            }
            digits = true;
            let digit = i128::from(b - b'0');
            if value <= (i64::MAX as i128) {
                value = value * 10 + digit;
            }
            self.bump();
        }

        if !digits {
            // Matching failure: no digits were converted.
            return None;
        }

        let signed: i128 = if negative { -value } else { value };
        let clamped = signed.clamp(i64::MIN as i128, i64::MAX as i128) as i64;
        Some(clamped as i32)
    }
}

/// Restore the default disposition of `SIGPIPE`.
///
/// The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main` runs, which
/// makes a failed write merely return `EPIPE`.  A C program keeps the inherited
/// default disposition, so `driver` dies from `SIGPIPE` (wait status 141 from a
/// shell) as soon as the reader of its stdout goes away.  Reset it so the two
/// programs terminate identically.
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    extern "C" {
        fn signal(signum: i32, handler: *const ()) -> *const ();
    }
    // SIG_DFL is the null handler value.
    unsafe {
        signal(SIGPIPE, std::ptr::null());
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

/// Labels of the original `foo` body, used to reproduce its `goto`/`continue`
/// control flow exactly.
enum State {
    /// The `while (x > 0 || y > 0)` condition test.
    WhileCond,
    /// Top of the loop body (`printf("loop\n")` and the `goto label2` test).
    Body,
    /// `label1:`
    Label1,
    /// `label2:`
    Label2,
}

fn foo(out: &mut impl Write, mut x: i32, mut y: i32) {
    let mut state = State::WhileCond;
    loop {
        match state {
            State::WhileCond => {
                if x > 0 || y > 0 {
                    state = State::Body;
                } else {
                    return;
                }
            }
            State::Body => {
                let _ = out.write_all(b"loop\n");

                if x == 1 && y == 4 {
                    state = State::Label2; // goto label2;
                } else {
                    state = State::Label1; // fall through to label1
                }
            }
            State::Label1 => {
                if x > 0 {
                    let _ = out.write_all(b"x\n");
                    x = x.wrapping_sub(1);
                }
                state = State::Label2; // fall through to label2
            }
            State::Label2 => {
                if y == 0 {
                    state = State::WhileCond; // continue;
                    continue;
                }
                let _ = out.write_all(b"y\n");
                y = y.wrapping_sub(1);
                state = if x < 3 {
                    State::Label1 // goto label1;
                } else {
                    State::WhileCond // end of body
                };
            }
        }
    }
}

fn main() {
    restore_default_sigpipe();

    let mut x: i32 = 0;
    let mut y: i32 = 0;

    #[cfg(unix)]
    let mut scanner = Scanner::new(sys::Stdin);
    #[cfg(not(unix))]
    let mut scanner = Scanner::new(io::stdin());

    // scanf("%d %d", &x, &y): the second conversion is only attempted if the
    // first one succeeded, and a failed conversion leaves its variable alone.
    if let Some(v) = scanner.scan_int() {
        x = v;
        if let Some(v) = scanner.scan_int() {
            y = v;
        }
    }
    let unread = scanner.unread();

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    foo(&mut out, x, y);
    let _ = out.flush();

    // glibc's exit-time stream cleanup gives back the part of the stdio buffer
    // that no conversion consumed, so do it here — after the output, at the same
    // point in the program's lifetime.
    sys::rewind_unread(unread);
}
