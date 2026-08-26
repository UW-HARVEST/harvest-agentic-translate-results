// Translated from c_src/src/main.c
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
//
//! Pure translation of `c_src/src/main.c`.
//!
//! This module contains no `#[no_mangle]` items so that it can be compiled
//! into both the `cdylib` (where `lib.rs` adds the C-ABI export wrappers,
//! including one for `main`) and the `driver` binary (which has its own Rust
//! `main`) without producing duplicate `main` symbols.

use std::io::{self, Read, Write};

/// Byte-oriented stdin reader with one byte of pushback, mimicking the way the
/// C library's `scanf` consumes characters (it reads across newlines and pushes
/// back the first character that does not belong to the conversion).
pub struct Scanner {
    stdin: io::Stdin,
    pushed: Option<u8>,
    eof: bool,
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner {
    pub fn new() -> Self {
        Scanner {
            stdin: io::stdin(),
            pushed: None,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushed.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.stdin.read(&mut buf) {
            Ok(1) => Some(buf[0]),
            _ => {
                self.eof = true;
                None
            }
        }
    }

    fn push_back(&mut self, b: u8) {
        self.pushed = Some(b);
    }

    /// Emulates `scanf("%d", &out)`.
    ///
    /// Returns the number of successfully assigned items (1 on success,
    /// 0 on matching failure, -1 (EOF) if input ends before any conversion).
    /// `out` is left untouched unless a value is assigned, exactly like scanf.
    pub fn scan_int(&mut self, out: &mut i32) -> i32 {
        // Skip leading whitespace.
        let mut cur;
        loop {
            match self.next_byte() {
                Some(b) if is_space(b) => continue,
                Some(b) => {
                    cur = Some(b);
                    break;
                }
                None => return -1, // EOF before any input
            }
        }

        let mut negative = false;
        if let Some(b) = cur {
            if b == b'+' || b == b'-' {
                negative = b == b'-';
                cur = self.next_byte();
            }
        }

        let mut digits = 0usize;
        // Accumulate in i128 with saturation at the `long` range, then truncate
        // to `int`, which is how glibc behaves: `%d` is parsed with strtol
        // (saturating at LONG_MAX / LONG_MIN) and the resulting `long` is then
        // stored into an `int` with an implicit truncating conversion.
        let mut acc: i128 = 0;
        let saturated_lo: i128 = i64::MIN as i128;
        let saturated_hi: i128 = i64::MAX as i128;

        while let Some(b) = cur {
            if !b.is_ascii_digit() {
                break;
            }
            digits += 1;
            if acc <= saturated_hi {
                acc = acc * 10 + i128::from(b - b'0');
                if acc > saturated_hi + 1 {
                    acc = saturated_hi + 1;
                }
            }
            cur = self.next_byte();
        }

        // Push back the character that terminated the conversion.
        if let Some(b) = cur {
            self.push_back(b);
        }

        if digits == 0 {
            // Matching failure: nothing assigned.
            return 0;
        }

        let value: i128 = if negative { -acc } else { acc };
        let clamped: i64 = if value > saturated_hi {
            i64::MAX
        } else if value < saturated_lo {
            i64::MIN
        } else {
            value as i64
        };

        *out = clamped as i32; // truncating conversion, as in C
        1
    }
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `void printLine (const char * line)`
///
/// The C implementation is `if (line != NULL) printf("%s\n", line);`, i.e. it
/// emits the raw NUL-terminated bytes followed by a single newline and emits
/// nothing at all for a NULL pointer. `line` is modelled as `Option<&[u8]>`
/// (rather than `&str`) so that non-UTF-8 payloads stay byte-identical.
#[allow(dead_code)] // unused by `main`, exported through `printLine` by lib.rs
pub fn print_line(line: Option<&[u8]>) {
    if let Some(line) = line {
        let stdout = io::stdout();
        let mut lock = stdout.lock();
        let _ = lock.write_all(line);
        let _ = lock.write_all(b"\n");
    }
}

/// `void printIntLine (int intNumber)` -> `printf("%d\n", intNumber);`
pub fn print_int_line(int_number: i32) {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(int_number.to_string().as_bytes());
    let _ = lock.write_all(b"\n");
}

/// `void bad()`
///
/// The C code allocates only 10 bytes with `alloca` but writes 10 `int`s
/// (40 bytes), overflowing the buffer (CWE-131 style defect). Every value
/// copied is zero and only `data[0]` is read back, so the observable output is
/// `0\n`; the defect is modelled here with a correctly sized safe buffer, which
/// reproduces the C program's observable behaviour without the UB.
pub fn bad() {
    let mut data = vec![0i32; 10];
    {
        let source = [0i32; 10];
        for i in 0..10usize {
            data[i] = source[i];
        }
        print_int_line(data[0]);
    }
}

/// `void good()`
pub fn good() {
    let mut data = vec![0i32; 10];
    {
        let source = [0i32; 10];
        for i in 0..10usize {
            data[i] = source[i];
        }
        print_int_line(data[0]);
    }
}

/// `int main()`
///
/// Reads one `%d` from stdin (ignoring the scanf return value, exactly like the
/// C code) and dispatches to `good()` when it is non-zero, `bad()` otherwise.
/// Always returns 0.
pub fn c_main() -> i32 {
    let mut scanner = Scanner::new();
    let mut x: i32 = 0;
    let _ = scanner.scan_int(&mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }

    io::stdout().flush().ok();
    0
}
