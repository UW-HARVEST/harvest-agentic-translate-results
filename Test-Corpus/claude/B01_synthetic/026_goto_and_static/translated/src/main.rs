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
// Rust translation of c_src/src/main.c
//
// The C program declares `y` as a file-scope (static) `int` initialized to 123,
// then reads three integers with a single `scanf("%d %d %d", &x, &y, &z)` whose
// return value is ignored.  Any variable that scanf fails to convert keeps its
// previous value (x = 0, y = 123, z = 0).  The translation reproduces this
// exactly, including the "partial conversion" behavior on malformed / short
// input.

use std::io::{Read, Write};

// ---------------------------------------------------------------------------
// Global mutable state mirroring the C file-scope `static int y = 123;`
// ---------------------------------------------------------------------------
struct Globals {
    y: i32,
}

// ---------------------------------------------------------------------------
// Minimal `scanf("%d")`-compatible reader.
//
// Reads stdin one byte at a time (with a single byte of pushback) so that we
// never consume more of the stream than C's scanf would for the same format.
// ---------------------------------------------------------------------------
struct ScanReader<R: Read> {
    inner: R,
    pushback: Option<u8>,
    eof: bool,
}

impl<R: Read> ScanReader<R> {
    fn new(inner: R) -> Self {
        ScanReader {
            inner,
            pushback: None,
            eof: false,
        }
    }

    /// Returns the next byte of the stream, or None at end-of-file.
    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    /// Equivalent of ungetc(): pushes a single byte back onto the stream.
    fn unget(&mut self, b: u8) {
        self.pushback = Some(b);
    }

    /// C `isspace()` for the "C" locale.
    fn is_space(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    /// Skips leading whitespace, exactly as a `%d` directive (and the literal
    /// spaces in the format string) does.  Whitespace includes newlines, so a
    /// single scanf call happily reads across line boundaries.
    fn skip_whitespace(&mut self) {
        while let Some(b) = self.next_byte() {
            if !Self::is_space(b) {
                self.unget(b);
                return;
            }
        }
    }

    /// Performs one `%d` conversion.
    ///
    /// Returns `Some(value)` on success, or `None` on input failure (EOF before
    /// any non-whitespace character) or matching failure (no digits present).
    /// On overflow the value saturates like glibc's internal `strtol` and is
    /// then truncated to `int`, matching the observable glibc behavior.
    fn scan_i32(&mut self) -> Option<i32> {
        self.skip_whitespace();

        let mut negative = false;
        let first = self.next_byte()?; // EOF -> input failure
        let mut cur = match first {
            b'-' => {
                negative = true;
                self.next_byte()
            }
            b'+' => self.next_byte(),
            other => Some(other),
        };

        let mut digits = 0usize;
        let mut acc: i128 = 0;
        let mut saturated = false;

        while let Some(c) = cur {
            if !c.is_ascii_digit() {
                self.unget(c);
                break;
            }
            digits += 1;
            if !saturated {
                acc = acc * 10 + i128::from(c - b'0');
                if acc > i128::from(u64::MAX) {
                    // Far past any 64-bit magnitude; clamping is unavoidable.
                    saturated = true;
                }
            }
            cur = self.next_byte();
        }

        if digits == 0 {
            // Matching failure: scanf stops here without assigning anything.
            return None;
        }

        let signed: i128 = if negative { -acc } else { acc };
        let clamped: i64 = if signed > i128::from(i64::MAX) {
            i64::MAX
        } else if signed < i128::from(i64::MIN) {
            i64::MIN
        } else {
            signed as i64
        };
        Some(clamped as i32)
    }
}

// ---------------------------------------------------------------------------
// Translation of `static int multi_stage(int x, int z)`
// ---------------------------------------------------------------------------
fn multi_stage<W: Write>(out: &mut W, g: &Globals, x: i32, z: i32) -> i32 {
    // The three validation stages, in the exact order the C code checks them.
    // `Err(code)` corresponds to a `goto fail` with `result = code`.
    let stages: Result<(), i32> = (|| {
        if x != 1 {
            print_str(out, "Error: x != 1\n");
            return Err(1);
        }

        if g.y != 2 {
            print_str(out, "Error: x == 1 but y != 2\n");
            return Err(2);
        }

        if z != 3 {
            print_str(out, "Error: x == 1 and y == 2, but z != 3\n");
            return Err(3);
        }

        Ok(())
    })();

    let result = match stages {
        Ok(()) => {
            print_str(out, "Ok!\n");
            return 0; // `result` is still 0 on the success path
        }
        Err(code) => code,
    };

    // fail:
    print_str(out, "Operation failed\n");
    result
}

fn print_str<W: Write>(out: &mut W, s: &str) {
    // printf() ignores write errors as far as this program is concerned.
    let _ = out.write_all(s.as_bytes());
}

// ---------------------------------------------------------------------------
// Translation of `int main()`
// ---------------------------------------------------------------------------
fn main() {
    let mut g = Globals { y: 123 };

    let mut x: i32 = 0;
    let mut z: i32 = 0;

    // scanf("%d %d %d", &x, &y, &z); -- return value ignored, so variables that
    // are not converted retain their prior values.
    {
        let stdin = std::io::stdin();
        let mut reader = ScanReader::new(stdin.lock());
        if let Some(v) = reader.scan_i32() {
            x = v;
            if let Some(v) = reader.scan_i32() {
                g.y = v;
                if let Some(v) = reader.scan_i32() {
                    z = v;
                }
            }
        }
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let result = multi_stage(&mut out, &g, x, z);
    let _ = writeln!(out, "Result: {}", result);
    let _ = out.flush();
}
