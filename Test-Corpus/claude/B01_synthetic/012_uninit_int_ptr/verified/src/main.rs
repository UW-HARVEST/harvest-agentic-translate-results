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

// Translation of c_src/src/main.c (CWE-457 / uninitialized pointer test driver).
//
// The `bad()` path in the original C dereferences an uninitialized `int *`,
// which is undefined behavior. The reference build (CMake with no
// CMAKE_BUILD_TYPE, i.e. no optimization) observably reads a zeroed stack
// slot and prints "0\n". That observed behavior is reproduced here verbatim
// rather than "fixed".

use std::io::{Read, Write};

/// `printf("%d\n", *intNumber);`
fn print_int_ptr_line(int_number: &i32) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", *int_number);
    let _ = out.flush();
}

/// Original C:
///     int *data;                 /* uninitialized */
///     printIntPtrLine(data);     /* undefined behavior */
///
/// The reference (-O0) build reads through the stale stack slot and prints 0,
/// so the same output is produced here.
fn bad() {
    // Value observed through the uninitialized pointer in the reference build.
    let uninitialized_read: i32 = 0;
    print_int_ptr_line(&uninitialized_read);
}

/// Original C:
///     int data = 5; int *data_addr = &data; printIntPtrLine(data_addr);
fn good() {
    let data: i32 = 5;
    let data_addr: &i32 = &data;
    print_int_ptr_line(data_addr);
}

/// A single-byte-pushback reader over stdin, mirroring how `scanf` consumes
/// characters (it reads only as far as it needs and ungets the terminator).
struct Scanner {
    input: std::io::Stdin,
    pushed_back: Option<u8>,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            input: std::io::stdin(),
            pushed_back: None,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushed_back.take() {
            return Some(b);
        }
        let mut buf = [0u8; 1];
        loop {
            match self.input.read(&mut buf) {
                Ok(0) => return None,
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    fn unget(&mut self, b: u8) {
        self.pushed_back = Some(b);
    }

    /// Emulates `scanf("%d", &x)`: returns `Some(value)` when a conversion is
    /// performed, `None` on matching failure or input failure (in which case
    /// the C code leaves `x` untouched).
    ///
    /// Overflow follows glibc's `strtol` behavior: the value saturates at
    /// LONG_MAX / LONG_MIN and is then truncated to `int`.
    fn scan_int(&mut self) -> Option<i32> {
        // Skip leading whitespace, exactly as isspace() classifies it.
        let mut c = loop {
            let b = self.next_byte()?; // EOF before any conversion
            if !matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
                break b;
            }
        };

        // Optional sign.
        let mut negative = false;
        if c == b'+' || c == b'-' {
            negative = c == b'-';
            // A sign with no following digits is a matching failure.
            c = self.next_byte()?;
        }

        if !c.is_ascii_digit() {
            self.unget(c);
            return None; // matching failure
        }

        let mut magnitude: u128 = 0;
        let mut saturated = false;
        loop {
            if !c.is_ascii_digit() {
                self.unget(c);
                break;
            }
            if !saturated {
                magnitude = magnitude * 10 + u128::from(c - b'0');
                // Once past the 64-bit range there is nothing left to track.
                if magnitude > u128::from(u64::MAX) {
                    saturated = true;
                }
            }
            match self.next_byte() {
                Some(b) => c = b,
                None => break,
            }
        }

        let as_long: i64 = if negative {
            if saturated || magnitude >= (i64::MAX as u128) + 1 {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else if saturated || magnitude > i64::MAX as u128 {
            i64::MAX
        } else {
            magnitude as i64
        };

        // Assignment to an `int` object truncates the low 32 bits.
        Some(as_long as i32)
    }
}

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` is entered, so a
/// write to a pipe with no reader returns `EPIPE` instead of terminating the
/// process. The C program never touches the signal, leaving it at `SIG_DFL`, so
/// the reference dies with `SIGPIPE` (wait status 141) when `printf` writes to a
/// closed pipe. Restoring the default disposition here keeps the exit status
/// identical to the C build.
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_default_sigpipe();

    let mut x: i32 = 0;
    let mut scanner = Scanner::new();
    if let Some(v) = scanner.scan_int() {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
