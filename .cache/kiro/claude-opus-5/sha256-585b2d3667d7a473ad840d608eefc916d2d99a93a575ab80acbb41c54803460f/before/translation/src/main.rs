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

//! Rust translation of `c_src/src/main.c`.
//!
//! The C program reads one integer with `scanf("%d", &x)` and then calls
//! `good()` when `x` is non-zero or `bad()` when `x` is zero. `bad()` is a
//! CWE-457 (use of uninitialized variable) demonstration.

use std::io::{Read, Write};

/// C: `void printIntPtrLine(const int *intNumber) { printf("%d\n", *intNumber); }`
fn print_int_ptr_line(int_number: &i32) {
    // `%d\n` for an `int`; Rust's Display for i32 is byte-identical.
    println!("{}", *int_number);
}

/// C:
/// ```c
/// void bad() { int *data; printIntPtrLine(data); }
/// ```
///
/// `data` is never initialized, so dereferencing it is undefined behavior.
/// This is the intentional defect of the original program and must NOT be
/// "fixed" here. It cannot be expressed literally in safe Rust, so instead we
/// reproduce the *observable behavior* of the reference build (the CMake
/// project built by gcc on x86-64 Linux with no optimization flags): the
/// leftover stack slot read by `bad()` holds a pointer to a zero word, so the
/// program prints `0`.
fn bad() {
    // Stands in for the indeterminate value that the reference build reads.
    let data: i32 = 0;
    print_int_ptr_line(&data);
}

/// C:
/// ```c
/// void good() { int data; data = 5; int *data_addr; data_addr = &data; printIntPtrLine(data_addr); }
/// ```
fn good() {
    let data: i32;
    data = 5;
    let data_addr: &i32;
    data_addr = &data;
    print_int_ptr_line(data_addr);
}

/// C's `isspace` for the "C" locale.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// One-byte-at-a-time view of stdin with a single character of pushback, which
/// is what `scanf` needs in order to leave the terminating character in the
/// stream.
struct Stdin {
    inner: std::io::Stdin,
    peeked: Option<u8>,
}

impl Stdin {
    fn new() -> Self {
        Stdin {
            inner: std::io::stdin(),
            peeked: None,
        }
    }

    /// Returns `None` on EOF (or read error, which `scanf` also treats as an
    /// input failure).
    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
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

    fn unget(&mut self, b: u8) {
        self.peeked = Some(b);
    }
}

/// Faithful model of `scanf("%d", out)`.
///
/// Returns the number of assigned conversions (`1`), `0` on a matching
/// failure, or `-1` (EOF) when the input ends before any conversion could
/// start. `*out` is left untouched unless the conversion succeeds.
///
/// glibc converts `%d` with `strtol` semantics: the digits are accumulated
/// into a `long`, saturating at `LONG_MAX`/`LONG_MIN` on overflow, and the
/// result is then truncated to `int`. That truncation is observable here,
/// because e.g. `4294967296` yields `x == 0` and therefore selects `bad()`.
fn scanf_int(input: &mut Stdin, out: &mut i32) -> i32 {
    // Skip leading whitespace.
    let mut c = loop {
        match input.next_byte() {
            None => return -1,
            Some(b) if is_space(b) => continue,
            Some(b) => break b,
        }
    };

    // Optional sign.
    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match input.next_byte() {
            None => return 0,
            Some(b) => c = b,
        }
    }

    if !c.is_ascii_digit() {
        input.unget(c);
        return 0;
    }

    // Accumulate the magnitude, clamping so the running value cannot wrap.
    const CLAMP: u128 = u128::MAX / 16;
    let mut magnitude: u128 = 0;
    loop {
        if magnitude < CLAMP {
            magnitude = magnitude * 10 + u128::from(c - b'0');
        }
        match input.next_byte() {
            None => break,
            Some(b) if b.is_ascii_digit() => c = b,
            Some(b) => {
                input.unget(b);
                break;
            }
        }
    }

    // strtol-style saturation into `long`, then truncation to `int`.
    let as_long: i64 = if negative {
        if magnitude > (i64::MAX as u128) + 1 {
            i64::MIN
        } else {
            (magnitude as i128).wrapping_neg() as i64
        }
    } else if magnitude > i64::MAX as u128 {
        i64::MAX
    } else {
        magnitude as i64
    };

    *out = as_long as i32;
    1
}

fn main() {
    let mut x: i32 = 0;
    let mut input = Stdin::new();
    scanf_int(&mut input, &mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }

    // C's `return 0` from main flushes stdout.
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}
