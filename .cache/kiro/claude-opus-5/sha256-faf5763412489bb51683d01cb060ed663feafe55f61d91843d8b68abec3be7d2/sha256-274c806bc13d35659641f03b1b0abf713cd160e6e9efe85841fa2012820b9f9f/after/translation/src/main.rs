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

use std::io::{self, Read, Write};

use driver::{process_buffer, BUFFER_CAPACITY};

/// Minimal emulation of the `scanf` numeric conversions used by the C driver.
///
/// `scanf` skips leading whitespace (including newlines), accepts an optional
/// sign, then consumes decimal digits. A conversion that finds no digits is a
/// matching failure (or EOF), both of which make `scanf` return something other
/// than 1 in the original program.
struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

/// Result of the digit-consuming core shared by every conversion.
struct Digits {
    negative: bool,
    /// Magnitude of the digit sequence, saturated once it exceeds `u64::MAX`.
    magnitude: u128,
    /// Whether the magnitude did not fit in `unsigned long` / `long`.
    overflow: bool,
}

impl Scanner {
    fn new(data: Vec<u8>) -> Self {
        Scanner { data, pos: 0 }
    }

    /// C `isspace` for the default locale.
    fn is_space(c: u8) -> bool {
        matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.data.len() && Self::is_space(self.data[self.pos]) {
            self.pos += 1;
        }
    }

    fn read_digits(&mut self) -> Option<Digits> {
        self.skip_whitespace();

        let mut negative = false;
        if self.pos < self.data.len()
            && (self.data[self.pos] == b'+' || self.data[self.pos] == b'-')
        {
            negative = self.data[self.pos] == b'-';
            self.pos += 1;
        }

        let start = self.pos;
        let mut magnitude: u128 = 0;
        let mut overflow = false;
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_digit() {
            if !overflow {
                magnitude = magnitude * 10 + u128::from(self.data[self.pos] - b'0');
                if magnitude > u128::from(u64::MAX) {
                    overflow = true;
                }
            }
            self.pos += 1;
        }

        if self.pos == start {
            /* Matching failure or EOF: scanf returns 0 or EOF, never 1. */
            return None;
        }

        Some(Digits {
            negative,
            magnitude,
            overflow,
        })
    }

    /// `%u` / `%zu`: glibc converts via `strtoul`, so a leading `-` wraps and an
    /// out-of-range value clamps to `ULONG_MAX`.
    fn scan_unsigned(&mut self) -> Option<u64> {
        let d = self.read_digits()?;
        if d.overflow {
            return Some(u64::MAX);
        }
        let value = d.magnitude as u64;
        Some(if d.negative { value.wrapping_neg() } else { value })
    }

    /// `%d`: glibc converts via `strtol`, clamping to `LONG_MIN` / `LONG_MAX`
    /// on overflow before the value is narrowed to `int`.
    fn scan_signed(&mut self) -> Option<i64> {
        let d = self.read_digits()?;
        let limit = i64::MAX as u128;
        if d.overflow || d.magnitude > limit {
            /* -(2^63) is exactly representable and also clamps to LONG_MIN. */
            return Some(if d.negative { i64::MIN } else { i64::MAX });
        }
        let value = d.magnitude as i64;
        Some(if d.negative { -value } else { value })
    }
}

fn main() {
    let mut input = Vec::new();
    /* Only scanf is used, so slurping stdin is equivalent to incremental reads. */
    let _ = io::stdin().read_to_end(&mut input);
    let mut scanner = Scanner::new(input);

    /* Read flags */
    let flags: u32 = match scanner.scan_unsigned() {
        Some(v) => v as u32,
        None => {
            eprintln!("Error reading flags");
            std::process::exit(1);
        }
    };

    /* Read param1 */
    let param1: i32 = match scanner.scan_signed() {
        Some(v) => v as i32,
        None => {
            eprintln!("Error reading param1");
            std::process::exit(1);
        }
    };

    /* Read param2 */
    let param2: i32 = match scanner.scan_signed() {
        Some(v) => v as i32,
        None => {
            eprintln!("Error reading param2");
            std::process::exit(1);
        }
    };

    /* Read buffer length */
    let length: u64 = match scanner.scan_unsigned() {
        Some(v) => v,
        None => {
            eprintln!("Error reading length");
            std::process::exit(1);
        }
    };

    if length > 256 {
        eprintln!("Error: length {} exceeds maximum 256", length);
        std::process::exit(1);
    }
    let length = length as usize;

    /* uint8_t buffer[256]; over-allocated and zeroed so that the original's
     * out-of-bounds writes (compact_runs with threshold 1) stay deterministic. */
    let mut buffer = vec![0u8; BUFFER_CAPACITY];

    /* Read buffer data */
    for i in 0..length {
        match scanner.scan_unsigned() {
            Some(byte) => buffer[i] = byte as u8,
            None => {
                eprintln!("Error reading byte {}", i);
                std::process::exit(1);
            }
        }
    }

    /* Process the buffer */
    let new_length = process_buffer(&mut buffer, length, flags, param1, param2);

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    /* Output new length */
    let _ = write!(out, "{}", new_length);

    /* Output buffer contents */
    for i in 0..new_length {
        let _ = write!(out, " {}", buffer[i]);
    }
    let _ = writeln!(out);

    let _ = out.flush();
}
