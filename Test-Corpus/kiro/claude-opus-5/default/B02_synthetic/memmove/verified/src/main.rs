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

//! Direct translation of `c_src/src/main.c`.

use std::io::{Read, Write};

use driver::process_buffer;

/// The C original declares `uint8_t buffer[256]`, but `compact_runs` can grow
/// the logical length up to `2 * length` and writes those bytes through the same
/// pointer. Backing storage of `2 * 256` keeps those writes inside real memory.
const BUFFER_CAPACITY: usize = 512;

/// Emulates the `scanf` numeric-conversion scanner: whitespace (including
/// newlines) is skipped, an optional sign is accepted, then decimal digits are
/// consumed. Behaves like glibc, which hands the collected digits to
/// `strtoul`/`strtol` and truncates the result to the destination width.
struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

/// Result of one numeric conversion attempt.
struct RawNumber {
    negative: bool,
    magnitude: u64,
    overflow: bool,
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

impl Scanner {
    fn new(data: Vec<u8>) -> Self {
        Scanner { data, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.data.len() && is_c_space(self.data[self.pos]) {
            self.pos += 1;
        }
    }

    /// Scans `[+-]?[0-9]+`. Returns `None` on EOF or matching failure, exactly
    /// like a `scanf` conversion that yields fewer than the requested items.
    fn scan_number(&mut self) -> Option<RawNumber> {
        self.skip_whitespace();

        let start = self.pos;
        let mut negative = false;
        if self.pos < self.data.len() {
            match self.data[self.pos] {
                b'+' => self.pos += 1,
                b'-' => {
                    negative = true;
                    self.pos += 1;
                }
                _ => {}
            }
        }

        let digits_start = self.pos;
        let mut magnitude: u64 = 0;
        let mut overflow = false;
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_digit() {
            let digit = u64::from(self.data[self.pos] - b'0');
            match magnitude.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => magnitude = v,
                None => overflow = true,
            }
            self.pos += 1;
        }

        if self.pos == digits_start {
            /* Matching failure: push the consumed characters back. */
            self.pos = start;
            return None;
        }

        Some(RawNumber {
            negative,
            magnitude,
            overflow,
        })
    }

    /// `scanf("%u", ...)`: `strtoul` semantics, truncated to `unsigned int`.
    fn scan_u32(&mut self) -> Option<u32> {
        self.scan_unsigned().map(|v| v as u32)
    }

    /// `scanf("%zu", ...)`: `strtoul` semantics, stored into a `size_t`.
    fn scan_usize(&mut self) -> Option<u64> {
        self.scan_unsigned()
    }

    fn scan_unsigned(&mut self) -> Option<u64> {
        let n = self.scan_number()?;
        if n.overflow {
            /* strtoul reports ULONG_MAX on overflow. */
            return Some(u64::MAX);
        }
        Some(if n.negative {
            n.magnitude.wrapping_neg()
        } else {
            n.magnitude
        })
    }

    /// `scanf("%d", ...)`: `strtol` semantics, truncated to `int`.
    fn scan_i32(&mut self) -> Option<i32> {
        let n = self.scan_number()?;
        let value: i64 = if n.overflow {
            if n.negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else if n.negative {
            if n.magnitude > (i64::MAX as u64) + 1 {
                i64::MIN
            } else {
                (n.magnitude as i64).wrapping_neg()
            }
        } else if n.magnitude > i64::MAX as u64 {
            i64::MAX
        } else {
            n.magnitude as i64
        };
        Some(value as i32)
    }
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let mut input = Vec::new();
    if std::io::stdin().read_to_end(&mut input).is_err() {
        input.clear();
    }
    let mut scanner = Scanner::new(input);

    let stderr = std::io::stderr();
    let mut err = stderr.lock();

    /* Read flags */
    let flags: u32 = match scanner.scan_u32() {
        Some(v) => v,
        None => {
            let _ = write!(err, "Error reading flags\n");
            return 1;
        }
    };

    /* Read param1 */
    let param1: i32 = match scanner.scan_i32() {
        Some(v) => v,
        None => {
            let _ = write!(err, "Error reading param1\n");
            return 1;
        }
    };

    /* Read param2 */
    let param2: i32 = match scanner.scan_i32() {
        Some(v) => v,
        None => {
            let _ = write!(err, "Error reading param2\n");
            return 1;
        }
    };

    /* Read buffer length */
    let length: u64 = match scanner.scan_usize() {
        Some(v) => v,
        None => {
            let _ = write!(err, "Error reading length\n");
            return 1;
        }
    };

    if length > 256 {
        let _ = write!(err, "Error: length {} exceeds maximum 256\n", length);
        return 1;
    }

    let length = length as usize;
    let mut buffer = [0u8; BUFFER_CAPACITY];

    /* Read buffer data */
    for i in 0..length {
        let byte: u32 = match scanner.scan_u32() {
            Some(v) => v,
            None => {
                let _ = write!(err, "Error reading byte {}\n", i);
                return 1;
            }
        };
        buffer[i] = byte as u8;
    }

    /* Process the buffer */
    let new_length = process_buffer(&mut buffer, length, flags, param1, param2);

    /* Output new length */
    let mut out = String::new();
    out.push_str(&new_length.to_string());

    /* Output buffer contents */
    for i in 0..new_length {
        out.push(' ');
        out.push_str(&buffer[i].to_string());
    }
    out.push('\n');

    let stdout = std::io::stdout();
    let mut w = stdout.lock();
    let _ = w.write_all(out.as_bytes());
    let _ = w.flush();

    0
}
