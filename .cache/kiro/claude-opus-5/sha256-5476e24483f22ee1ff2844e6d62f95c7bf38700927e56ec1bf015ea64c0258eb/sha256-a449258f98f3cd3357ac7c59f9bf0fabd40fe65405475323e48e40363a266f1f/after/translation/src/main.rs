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

/// Byte-at-a-time reader over stdin with a one-byte pushback slot, mirroring
/// the way `scanf` consumes exactly the characters it needs (and pushes the
/// first non-matching character back onto the stream).
struct Scanner {
    input: io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            input: io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.input.read(&mut buf) {
            Ok(1) => Some(buf[0]),
            _ => {
                self.eof = true;
                None
            }
        }
    }

    fn unget(&mut self, b: u8) {
        self.peeked = Some(b);
    }

    /// Equivalent of `scanf("%d", &out)`.
    ///
    /// Skips leading whitespace (including newlines, exactly like C's `%d`
    /// conversion), then reads an optional sign followed by decimal digits.
    /// Returns `true` when a value was assigned; on a matching failure the
    /// destination is left untouched, just as C leaves `x` at its initial 0.
    fn scan_i32(&mut self, out: &mut i32) -> bool {
        // Skip whitespace as recognized by isspace() in the C locale.
        let first = loop {
            match self.next_byte() {
                Some(b) if matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c') => {}
                Some(b) => break b,
                None => return false, // input failure
            }
        };

        let mut negative = false;
        let mut cur = Some(first);
        if let Some(b) = cur {
            if b == b'+' || b == b'-' {
                negative = b == b'-';
                cur = self.next_byte();
            }
        }

        // Accumulate saturating in a wide type, then truncate to int, which is
        // what glibc's %d conversion effectively does for out-of-range input.
        let mut digits = 0usize;
        let mut acc: i64 = 0;
        loop {
            match cur {
                Some(b) if b.is_ascii_digit() => {
                    digits += 1;
                    let d = i64::from(b - b'0');
                    acc = acc.saturating_mul(10);
                    acc = if negative {
                        acc.saturating_sub(d)
                    } else {
                        acc.saturating_add(d)
                    };
                    cur = self.next_byte();
                }
                Some(b) => {
                    self.unget(b);
                    break;
                }
                None => break,
            }
        }

        if digits == 0 {
            // Matching failure: nothing is stored. Push back the offending
            // character so the stream position mirrors C's behavior.
            if let Some(b) = cur {
                self.unget(b);
            }
            return false;
        }

        *out = acc as i32;
        true
    }
}

/// Restore the default disposition of `SIGPIPE`.
///
/// The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main` runs, so
/// a write to a closed pipe returns `EPIPE` and the process goes on to exit 0.
/// The C program keeps the default disposition and is therefore *killed* by
/// `SIGPIPE` (shell status 141) the moment `printf` hits a closed stdout.
/// Undoing the runtime's change keeps the observable exit status identical.
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13; // same value on Linux and macOS
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    // Safety: `signal` with SIG_DFL is async-signal-safe and takes no pointer
    // to Rust-owned memory; we only reset a disposition the runtime changed.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn driver(x: i32, out: &mut impl Write) {
    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        let _ = writeln!(out, "{} {}", i, j);
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

fn main() {
    restore_default_sigpipe();

    let mut x: i32 = 0;
    let mut scanner = Scanner::new();
    scanner.scan_i32(&mut x);

    // glibc sizes stdout's buffer from st_blksize, which is 4096 for pipes,
    // files and terminals here (Rust's BufWriter default is 8192). Matching it
    // keeps the write() boundaries -- and hence the bytes that make it out
    // before a SIGPIPE death -- the same as the C program's.
    let stdout = io::stdout();
    let mut out = io::BufWriter::with_capacity(4096, stdout.lock());
    driver(x, &mut out);
    let _ = out.flush();
}
