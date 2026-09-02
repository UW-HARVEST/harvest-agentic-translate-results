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
// Rust translation of c_src/src/main.c. Behaviour-preserving: the original
// reads a single `int` with scanf("%d") and dumps its in-memory bytes as hex.

use std::io::{self, Read, Write};

/// Byte-at-a-time stdin reader with a single-byte pushback slot, mirroring the
/// `getc`/`ungetc` pair that glibc's scanf uses. Reading incrementally (rather
/// than slurping all of stdin) keeps the same blocking behaviour as C: only the
/// bytes scanf actually needs are consumed.
struct Input<R: Read> {
    inner: R,
    peeked: Option<u8>,
    eof: bool,
}

impl<R: Read> Input<R> {
    fn new(inner: R) -> Self {
        Input {
            inner,
            peeked: None,
            eof: false,
        }
    }

    /// Look at the next byte without consuming it. `None` means EOF (or a read
    /// error, which C's `getc` also surfaces as a failed read).
    fn peek(&mut self) -> Option<u8> {
        if self.peeked.is_none() && !self.eof {
            let mut b = [0u8; 1];
            loop {
                match self.inner.read(&mut b) {
                    Ok(0) => {
                        self.eof = true;
                        break;
                    }
                    Ok(_) => {
                        self.peeked = Some(b[0]);
                        break;
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        self.eof = true;
                        break;
                    }
                }
            }
        }
        self.peeked
    }

    /// Consume the byte returned by the most recent `peek`.
    fn bump(&mut self) {
        self.peeked = None;
    }
}

/// `isspace` under the C locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates `scanf("%d", &x)` as implemented by glibc.
///
/// Returns `Some(value)` on a successful conversion, `None` on EOF or a
/// matching failure (in both of those cases the C code leaves `x` untouched).
///
/// glibc converts the digit run with `strtol`, which saturates at `LONG_MIN` /
/// `LONG_MAX`, and then stores the result through an `int *`, truncating. So
/// e.g. "4294967296" yields 0 and "9223372036854775808" yields -1. That quirk
/// is reproduced here rather than corrected.
fn scanf_d<R: Read>(input: &mut Input<R>) -> Option<i32> {
    // Leading whitespace is skipped, and it may span newlines.
    while let Some(b) = input.peek() {
        if is_c_space(b) {
            input.bump();
        } else {
            break;
        }
    }

    // Optional sign.
    let mut negative = false;
    match input.peek() {
        Some(b'+') => input.bump(),
        Some(b'-') => {
            negative = true;
            input.bump();
        }
        _ => {}
    }

    // Digit run. No digits at all is a matching failure.
    let mut saw_digit = false;
    let mut saturated = false;
    let mut magnitude: u64 = 0;
    // strtol saturates once the magnitude passes these bounds.
    let limit: u64 = if negative {
        // -(i64::MIN) as u64
        1u64 << 63
    } else {
        i64::MAX as u64
    };

    while let Some(b) = input.peek() {
        if !b.is_ascii_digit() {
            break;
        }
        saw_digit = true;
        input.bump();
        if !saturated {
            let digit = u64::from(b - b'0');
            match magnitude
                .checked_mul(10)
                .and_then(|m| m.checked_add(digit))
            {
                Some(m) if m <= limit => magnitude = m,
                _ => saturated = true,
            }
        }
    }

    if !saw_digit {
        return None;
    }

    let wide: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // Safe for magnitude == 1 << 63 as well.
        (magnitude as i64).wrapping_neg()
    } else {
        magnitude as i64
    };

    // Truncation performed by glibc's `*ARG(int *) = num.l;`.
    Some(wide as i32)
}

/// Mirrors the C `print_hex`: two lowercase hex digits per byte, then a newline.
fn print_hex(out: &mut impl Write, p: &[u8]) -> io::Result<()> {
    let mut buf = String::with_capacity(p.len() * 2 + 1);
    for &byte in p {
        buf.push_str(&format!("{:02x}", byte));
    }
    buf.push('\n');
    out.write_all(buf.as_bytes())
}

/// Mirrors the C `driver`: reinterprets the `int` as `sizeof(int)` raw bytes.
/// `to_ne_bytes` reproduces the host byte order the C program observes.
fn driver(out: &mut impl Write, x: i32) -> io::Result<()> {
    print_hex(out, &x.to_ne_bytes())
}

fn main() {
    let mut x: i32 = 0;

    let stdin = io::stdin();
    let mut input = Input::new(stdin.lock());
    // The C code ignores scanf's return value; on failure `x` keeps its 0.
    if let Some(v) = scanf_d(&mut input) {
        x = v;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = driver(&mut out, x);
    let _ = out.flush();
}
