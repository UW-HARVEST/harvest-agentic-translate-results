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

// Translation of the original C program, which was written using digraphs and
// the <iso646.h> alternative operator spellings:
//
//     %:include <stdio.h>       ->  #include <stdio.h>
//     void driver(int x, int y) <%  ->  { ...
//         int result = x bitor compl y;  ->  int result = x | ~y;
//         printf("%d", result);
//         puts("");
//     %>                        ->  }
//
// Behaviour that must be preserved byte-for-byte:
//   * `x` and `y` are initialised to 0 and are left untouched when a `scanf`
//     conversion fails (matching failure or EOF), so bad/missing input yields
//     0 for the affected variable.
//   * `scanf("%d", ...)` skips leading whitespace (including newlines), accepts
//     an optional sign, then decimal digits only.
//   * glibc performs the `%d` conversion through `strtol` (a 64-bit `long` on
//     LP64) and then stores the result into an `int`, so out-of-range input is
//     clamped to LONG_MAX/LONG_MIN and afterwards truncated to 32 bits.

use std::io::{Read, Write};

/// A minimal `stdin` model that supports unlimited push-back, which is what
/// glibc's `scanf` effectively provides while it is matching a directive.
struct Stdin {
    data: Vec<u8>,
    pos: usize,
}

impl Stdin {
    fn new() -> Stdin {
        let mut data = Vec::new();
        // Errors are ignored: a read error simply behaves like end-of-file,
        // which in turn leaves the scanned variables at their initial values.
        let _ = std::io::stdin().read_to_end(&mut data);
        Stdin { data, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn bump(&mut self) {
        if self.pos < self.data.len() {
            self.pos += 1;
        }
    }

    /// `isspace()` in the "C" locale.
    fn is_space(c: u8) -> bool {
        matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
    }

    /// Emulates a single `scanf("%d", &out)` directive.
    ///
    /// Returns `true` when the conversion succeeded (and `out` was written),
    /// `false` on an input or matching failure (leaving `out` untouched, just
    /// like C).
    fn scan_int(&mut self, out: &mut i32) -> bool {
        // Leading whitespace is consumed unconditionally, even if the
        // directive later fails to match.
        while let Some(c) = self.peek() {
            if Stdin::is_space(c) {
                self.bump();
            } else {
                break;
            }
        }

        let negative = match self.peek() {
            Some(b'-') => {
                self.bump();
                true
            }
            Some(b'+') => {
                self.bump();
                false
            }
            _ => false,
        };

        let mut saw_digit = false;
        // Accumulate in i128 and clamp to the `long` range the way strtol does.
        let mut acc: i128 = 0;
        let mut saturated = false;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            saw_digit = true;
            self.bump();
            if !saturated {
                acc = acc * 10 + i128::from(c - b'0');
                if acc > i128::from(i64::MAX) + 1 {
                    saturated = true;
                }
            }
        }

        if !saw_digit {
            // Matching failure. glibc only pushes back the single offending
            // character (which we never consumed, since digits are peeked
            // before being taken); an already-consumed sign character stays
            // consumed. So for input "--5" the first conversion fails having
            // eaten just the first '-', leaving "-5" for the next directive.
            return false;
        }

        let magnitude = if negative { -acc } else { acc };
        // strtol clamps on overflow (and sets ERANGE); glibc then assigns the
        // resulting `long` to an `int`, truncating the value.
        let as_long: i64 = if magnitude > i128::from(i64::MAX) {
            i64::MAX
        } else if magnitude < i128::from(i64::MIN) {
            i64::MIN
        } else {
            magnitude as i64
        };

        *out = as_long as i32;
        true
    }
}

fn driver(x: i32, y: i32) {
    let result: i32 = x | !y; // x bitor compl y
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    // printf("%d", result);
    let _ = write!(out, "{}", result);
    // puts("");
    let _ = writeln!(out);
    let _ = out.flush();
}

fn main() {
    let mut x: i32 = 0;
    let mut y: i32 = 0;
    let mut input = Stdin::new();
    input.scan_int(&mut x);
    input.scan_int(&mut y);
    driver(x, y);
}
