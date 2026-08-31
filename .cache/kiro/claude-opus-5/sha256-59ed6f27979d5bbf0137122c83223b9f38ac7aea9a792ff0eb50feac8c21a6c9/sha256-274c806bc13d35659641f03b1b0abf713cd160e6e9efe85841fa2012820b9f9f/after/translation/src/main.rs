// Rust translation of c_src/src/main.c
//
// Original C:
//     void driver(int x) {
//         auto int y = 2*x;
//         y += 300;
//         printf("%d\n", y);
//     }
//
//     int main() {
//         int x = 0;
//         scanf("%d", &x);
//         driver(x);
//         return 0;
//     }
//
// Behavior preserved exactly:
//   * `scanf("%d", &x)` skips leading whitespace (spaces, tabs, newlines, ...),
//     so the number may appear on any later line. On a matching failure or EOF
//     the C code ignores the return value and `x` keeps its initial value of 0.
//   * glibc's `%d` converts using a `long` accumulator that saturates at
//     LONG_MIN/LONG_MAX, then stores the low 32 bits into the `int` target.
//   * `2*x + 300` overflows as two's-complement wraparound (UB in C, but this
//     is what the compiled C does); reproduced with wrapping arithmetic.
//   * Output is `%d` followed by a single '\n'.

use std::io::{self, Read, Write};

/// Byte-at-a-time reader over stdin with one byte of pushback, mirroring how
/// C's stdio stream is consumed by `scanf` (it "ungets" the character that
/// terminated the conversion).
struct Stdin {
    inner: io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Stdin {
    fn new() -> Self {
        Stdin {
            inner: io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    /// Read the next byte, or `None` at end of input / on a read error
    /// (C's stdio treats both as a failed `getc`).
    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.inner.read(&mut buf) {
            Ok(1) => Some(buf[0]),
            _ => {
                self.eof = true;
                None
            }
        }
    }

    fn unget(&mut self, b: u8) {
        self.peeked = Some(b);
    }
}

/// True for the bytes that C's `isspace` accepts in the default "C" locale;
/// `scanf`'s `%d` directive skips these before converting.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Equivalent of `scanf("%d", out)`. Returns 1 on a successful assignment,
/// 0 on a matching failure, and -1 (EOF) on input failure before any
/// conversion, exactly like the C library function.
fn scanf_d(input: &mut Stdin, out: &mut i32) -> i32 {
    // Skip leading whitespace, including newlines.
    let mut b = loop {
        match input.next_byte() {
            Some(c) if is_c_space(c) => continue,
            Some(c) => break c,
            None => return -1, // EOF hit before anything was matched
        }
    };

    // Optional sign.
    let mut negative = false;
    if b == b'+' || b == b'-' {
        negative = b == b'-';
        match input.next_byte() {
            Some(c) => b = c,
            None => return 0, // sign consumed but no digits: matching failure
        }
    }

    if !b.is_ascii_digit() {
        // Not a number: push the offending byte back and report a match failure.
        input.unget(b);
        return 0;
    }

    // Accumulate into an i64 (glibc's `long` on 64-bit) with saturation, then
    // truncate to 32 bits when storing, matching glibc's overflow handling.
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = i64::from(b - b'0');
        if !saturated {
            match acc
                .checked_mul(10)
                .and_then(|v| if negative { v.checked_sub(digit) } else { v.checked_add(digit) })
            {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }

        match input.next_byte() {
            Some(c) if c.is_ascii_digit() => b = c,
            Some(c) => {
                input.unget(c);
                break;
            }
            None => break,
        }
    }

    if saturated {
        acc = if negative { i64::MIN } else { i64::MAX };
    }

    // Store the low 32 bits, as the C assignment `*(int *)ptr = (int)value` does.
    *out = acc as i32;
    1
}

fn driver(x: i32) {
    // `auto int y = 2*x;` then `y += 300;`
    let y = (2i32).wrapping_mul(x).wrapping_add(300);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", y);
    let _ = out.flush();
}

fn main() {
    let mut x: i32 = 0;
    let mut input = Stdin::new();
    // Return value is ignored by the original C, so `x` stays 0 on failure.
    let _ = scanf_d(&mut input, &mut x);
    driver(x);
}
