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

// Global mutable state mirroring the C `static int y = 123;`
static mut Y: i32 = 123;

fn multi_stage(x: i32, z: i32) -> i32 {
    let mut result: i32 = 0;

    // Use a labeled block to emulate the C `goto fail` control flow.
    'fail: {
        if x != 1 {
            print!("Error: x != 1\n");
            result = 1;
            break 'fail;
        }

        // SAFETY: single-threaded program; this mirrors C `static int y`.
        let y_val = unsafe { Y };
        if y_val != 2 {
            print!("Error: x == 1 but y != 2\n");
            result = 2;
            break 'fail;
        }

        if z != 3 {
            print!("Error: x == 1 and y == 2, but z != 3\n");
            result = 3;
            break 'fail;
        }

        print!("Ok!\n");
        return result;
    }

    print!("Operation failed\n");
    result
}

/// Read all of stdin and produce successive `i32` tokens, skipping any
/// whitespace (including newlines) between them. Mirrors `scanf("%d ...")`.
struct IntScanner {
    data: Vec<u8>,
    pos: usize,
}

impl IntScanner {
    fn new() -> io::Result<Self> {
        let mut data = Vec::new();
        io::stdin().read_to_end(&mut data)?;
        Ok(Self { data, pos: 0 })
    }

    fn next_i32(&mut self) -> Option<i32> {
        // Skip any whitespace (matches C isspace for the "C" locale: space,
        // tab, newline, vertical tab, form feed, carriage return).
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == 0x0B || b == 0x0C {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos >= self.data.len() {
            return None;
        }

        let start = self.pos;
        // Optional sign
        if self.data[self.pos] == b'+' || self.data[self.pos] == b'-' {
            self.pos += 1;
        }

        let digits_start = self.pos;
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if b.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos == digits_start {
            // No digits consumed: scanf would fail to assign here.
            // Reset position to start so callers can stop.
            self.pos = start;
            return None;
        }

        let s = std::str::from_utf8(&self.data[start..self.pos]).ok()?;
        // Use wrapping behavior to align with how integer overflow could occur.
        s.parse::<i64>().ok().map(|v| v as i32)
    }
}

fn main() {
    let mut x: i32 = 0;
    let mut z: i32 = 0;

    // Mirror `scanf("%d %d %d", &x, &y, &z);` — assignments happen left-to-
    // right and stop on first failure, leaving later variables untouched.
    if let Ok(mut sc) = IntScanner::new() {
        if let Some(v) = sc.next_i32() {
            x = v;
            if let Some(v) = sc.next_i32() {
                // SAFETY: single-threaded program.
                unsafe { Y = v };
                if let Some(v) = sc.next_i32() {
                    z = v;
                }
            }
        }
    }

    let result = multi_stage(x, z);
    print!("Result: {}\n", result);

    // Ensure stdout is flushed before exit.
    let _ = io::stdout().flush();
}
