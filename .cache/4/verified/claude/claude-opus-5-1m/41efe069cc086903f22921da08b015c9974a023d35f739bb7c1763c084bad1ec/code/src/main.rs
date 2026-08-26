/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Faithful Rust translation of `c_src/src/main.c`.

#![forbid(unsafe_code)]

use std::io::{Read, Write};

use driver::process_buffer;

/// The C code declares `uint8_t buffer[256]`, however `compact_runs()` can
/// legitimately grow the logical length past 256 (a bug in the original that
/// smashes the stack).  A larger backing store is used here so that the
/// translation keeps computing instead of panicking; for every input that does
/// not overflow the original 256 byte array the observable behaviour is
/// identical.
const BUFFER_CAPACITY: usize = 1024;

fn main() {
    let mut stdin_data = Vec::new();
    if std::io::stdin().read_to_end(&mut stdin_data).is_err() {
        stdin_data.clear();
    }
    let mut scanner = Scanner::new(stdin_data);

    /* Read flags */
    let flags: u32 = match scanner.scan_unsigned() {
        Some(v) => v as u32,
        None => {
            fail("Error reading flags\n");
        }
    };

    /* Read param1 */
    let param1: i32 = match scanner.scan_signed() {
        Some(v) => v as i32,
        None => {
            fail("Error reading param1\n");
        }
    };

    /* Read param2 */
    let param2: i32 = match scanner.scan_signed() {
        Some(v) => v as i32,
        None => {
            fail("Error reading param2\n");
        }
    };

    /* Read buffer length */
    let length: u64 = match scanner.scan_unsigned() {
        Some(v) => v,
        None => {
            fail("Error reading length\n");
        }
    };

    if length > 256 {
        fail(&format!("Error: length {} exceeds maximum 256\n", length));
    }

    let length = length as usize;
    let mut buffer = [0u8; BUFFER_CAPACITY];

    /* Read buffer data */
    for i in 0..length {
        match scanner.scan_unsigned() {
            Some(byte) => buffer[i] = (byte as u32) as u8,
            None => {
                fail(&format!("Error reading byte {}\n", i));
            }
        }
    }

    /* Process the buffer */
    let new_length = process_buffer(&mut buffer, length, flags, param1, param2);

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    /* Output new length */
    let _ = write!(out, "{}", new_length);

    /* Output buffer contents */
    for i in 0..new_length {
        let _ = write!(out, " {}", buffer[i]);
    }
    let _ = writeln!(out);

    let _ = out.flush();
}

/// Emit a message on stderr and exit with status 1, mirroring
/// `fprintf(stderr, ...); return 1;`
fn fail(message: &str) -> ! {
    let stderr = std::io::stderr();
    let mut err = stderr.lock();
    let _ = err.write_all(message.as_bytes());
    let _ = err.flush();
    std::process::exit(1);
}

/// Minimal `scanf` emulation for the `%u`, `%d` and `%zu` conversions used by
/// the original program: leading whitespace is skipped (crossing newlines),
/// an optional sign is accepted and then a run of decimal digits.  Out of
/// range values behave like glibc, which funnels the digits through
/// `strtoul`/`strtol` (saturating) before truncating to the target type.
struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new(data: Vec<u8>) -> Self {
        Scanner { data, pos: 0 }
    }

    fn is_space(c: u8) -> bool {
        matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.data.len() && Self::is_space(self.data[self.pos]) {
            self.pos += 1;
        }
    }

    /// Returns `(negative, magnitude, overflowed)` for the next integer token,
    /// or `None` on EOF / matching failure.
    fn scan_token(&mut self) -> Option<(bool, u64, bool)> {
        self.skip_whitespace();
        let start = self.pos;

        let mut negative = false;
        if self.pos < self.data.len() && (self.data[self.pos] == b'+' || self.data[self.pos] == b'-')
        {
            negative = self.data[self.pos] == b'-';
            self.pos += 1;
        }

        let digits_start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos == digits_start {
            /* Matching failure: the characters are pushed back. */
            self.pos = start;
            return None;
        }

        let mut value: u64 = 0;
        let mut overflowed = false;
        for &d in &self.data[digits_start..self.pos] {
            let digit = u64::from(d - b'0');
            match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => value = v,
                None => overflowed = true,
            }
        }

        Some((negative, value, overflowed))
    }

    /// `%u` / `%zu`: `strtoul` semantics (ULONG_MAX on overflow, wrapping
    /// negation for a leading `-`).
    fn scan_unsigned(&mut self) -> Option<u64> {
        let (negative, value, overflowed) = self.scan_token()?;
        if overflowed {
            return Some(u64::MAX);
        }
        Some(if negative { value.wrapping_neg() } else { value })
    }

    /// `%d`: `strtol` semantics (saturating at LONG_MIN / LONG_MAX).
    fn scan_signed(&mut self) -> Option<i64> {
        let (negative, value, overflowed) = self.scan_token()?;
        if overflowed {
            return Some(if negative { i64::MIN } else { i64::MAX });
        }
        if negative {
            if value > (i64::MAX as u64) + 1 {
                Some(i64::MIN)
            } else {
                Some((value as i64).wrapping_neg())
            }
        } else if value > i64::MAX as u64 {
            Some(i64::MAX)
        } else {
            Some(value as i64)
        }
    }
}
