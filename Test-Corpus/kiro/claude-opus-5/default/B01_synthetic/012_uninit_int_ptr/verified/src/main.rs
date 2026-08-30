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
//! The original program is a CWE-457 (use of an uninitialized variable) test
//! case: `bad()` declares `int *data;` without initializing it and then
//! dereferences it. That is undefined behavior in C, so there is no "correct"
//! value to reproduce -- only the behavior of the reference build.
//!
//! The reference build (`CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so gcc
//! runs unoptimized) reliably reads a stack slot that dereferences to `0` and
//! prints `0\n` with exit status 0. That observed behavior is reproduced here
//! with the bug preserved: `bad()` still prints an "uninitialized" value rather
//! than the `5` that `good()` prints.

use std::io::{Read, Write};

/// `void printIntPtrLine(const int *intNumber)` -- `printf("%d\n", *intNumber)`.
fn print_int_ptr_line(int_number: &i32) {
    // C's printf("%d\n", ...) — decimal, newline, no padding.
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", *int_number);
}

/// `void bad()` -- dereferences an uninitialized `int *`.
///
/// Undefined behavior in the original. Modeled with the value the reference
/// (unoptimized) build observes for that stack slot: `0`.
fn bad() {
    // `int *data;` is never assigned; `*data` is whatever the stack held.
    let data: i32 = UNINITIALIZED_STACK_VALUE;
    print_int_ptr_line(&data);
}

/// The value `*data` yields in the reference build of `bad()`.
const UNINITIALIZED_STACK_VALUE: i32 = 0;

/// `void good()` -- takes the address of an initialized `int`.
fn good() {
    let data: i32 = 5;
    let data_addr: &i32 = &data;
    print_int_ptr_line(data_addr);
}

/// Reads stdin one byte at a time, mirroring how C's `scanf` consumes input.
struct Stdin {
    inner: std::io::Stdin,
    pushback: Option<u8>,
}

impl Stdin {
    fn new() -> Self {
        Stdin {
            inner: std::io::stdin(),
            pushback: None,
        }
    }

    fn getc(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.take() {
            return Some(b);
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
                Ok(0) => return None,
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    fn ungetc(&mut self, b: u8) {
        self.pushback = Some(b);
    }
}

/// `scanf("%d", &x)`: skips leading whitespace (including newlines), then reads
/// an optional sign followed by decimal digits. Returns `None` on matching
/// failure or EOF, in which case the caller leaves `x` untouched -- exactly
/// like C, where a failed conversion performs no assignment.
///
/// Overflow follows glibc: the digits are accumulated with saturation at the
/// `long` bounds and the result is then truncated to `int`.
fn scanf_d(input: &mut Stdin) -> Option<i32> {
    // Skip whitespace, as the %d conversion specifier does.
    let mut c = loop {
        match input.getc() {
            None => return None, // EOF before any conversion.
            Some(b) if (b as char).is_ascii_whitespace() => continue,
            Some(b) => break b,
        }
    };

    let negative = match c {
        b'-' => {
            c = match input.getc() {
                Some(b) => b,
                None => return None,
            };
            true
        }
        b'+' => {
            c = match input.getc() {
                Some(b) => b,
                None => return None,
            };
            false
        }
        _ => false,
    };

    if !c.is_ascii_digit() {
        // Matching failure: no digits consumed, no assignment performed.
        input.ungetc(c);
        return None;
    }

    // Accumulate as `long` (i64 on the reference platform) with saturation.
    let mut acc: i64 = 0;
    loop {
        let digit = i64::from(c - b'0');
        acc = acc
            .checked_mul(10)
            .and_then(|v| v.checked_add(digit))
            .unwrap_or(i64::MAX);
        match input.getc() {
            Some(b) if b.is_ascii_digit() => c = b,
            Some(b) => {
                input.ungetc(b);
                break;
            }
            None => break,
        }
    }

    let value: i64 = if negative {
        // glibc saturates at LONG_MIN for negative overflow.
        if acc == i64::MAX {
            i64::MIN
        } else {
            -acc
        }
    } else {
        acc
    };

    // Assignment to an `int` object truncates.
    Some(value as i32)
}

/// Restore the default `SIGPIPE` disposition.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, which a C
/// program does not do. Without this, a write to a closed pipe makes the Rust
/// binary exit 0 while the C binary is killed by signal 13 (shell status 141).
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

    let mut input = Stdin::new();

    let mut x: i32 = 0;
    if let Some(v) = scanf_d(&mut input) {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    let _ = std::io::stdout().flush();
}
