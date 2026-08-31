// Rust translation of c_src/src/main.c
//
// Original C:
//     int x = 1, y = 1;
//     scanf("%d %d", &x, &y);
//     div_t result = div(x, y);
//     printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
//
// Behaviors that are deliberately reproduced (not "fixed"):
//   * `scanf` returns are ignored, so a failed/partial conversion leaves `x`
//     and/or `y` at their initial value of 1. E.g. input "42" -> 42 / 1.
//   * `%d` skips leading whitespace, including newlines, and stops at the
//     first character that cannot extend the integer (that character stays
//     unconsumed).
//   * glibc converts `%d` with `strtol`, so an out-of-range literal saturates
//     to LONG_MIN/LONG_MAX and is then truncated to `int`.
//     E.g. "4294967297" -> 1, "99999999999999999999999" -> -1.
//   * `div(x, 0)` and `div(INT_MIN, -1)` trap on x86-64 (SIGFPE, no output),
//     which is what the hardware `idiv` below does as well.

use std::io::{Read, Write};

/// Byte-at-a-time reader over stdin with a single byte of push-back, which is
/// all the C `%d` conversion needs.
struct Scanner<R: Read> {
    inner: R,
    peeked: Option<u8>,
    eof: bool,
}

impl<R: Read> Scanner<R> {
    fn new(inner: R) -> Self {
        Scanner {
            inner,
            peeked: None,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
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
                // glibc's stdio restarts a read that a signal interrupted, so a
                // caught signal must not look like end of input here.
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    fn push_back(&mut self, b: u8) {
        self.peeked = Some(b);
    }

    /// C `isspace` for the default locale.
    fn is_space(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    fn skip_whitespace(&mut self) {
        while let Some(b) = self.next_byte() {
            if !Self::is_space(b) {
                self.push_back(b);
                return;
            }
        }
    }

    /// One `%d` conversion. `None` means EOF or a matching failure, in which
    /// case the caller leaves its variable untouched, exactly like `scanf`.
    fn scan_i32(&mut self) -> Option<i32> {
        self.skip_whitespace();

        let mut negative = false;
        let first = self.next_byte()?;
        let mut cur = match first {
            b'+' => self.next_byte()?,
            b'-' => {
                negative = true;
                self.next_byte()?
            }
            other => other,
        };

        if !cur.is_ascii_digit() {
            // Sign with no digits, or no digit at all: matching failure.
            self.push_back(cur);
            return None;
        }

        // Accumulate as `long` with strtol-style saturation. `2^63` is kept
        // representable so that "-9223372036854775808" is exact.
        const LIMIT: i128 = i64::MAX as i128 + 1;
        let mut acc: i128 = 0;
        let mut saturated = false;
        loop {
            if !saturated {
                acc = acc * 10 + i128::from(cur - b'0');
                if acc > LIMIT {
                    saturated = true;
                }
            }
            match self.next_byte() {
                Some(b) if b.is_ascii_digit() => cur = b,
                Some(b) => {
                    self.push_back(b);
                    break;
                }
                None => break,
            }
        }

        let as_long: i64 = if saturated {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else {
            let signed = if negative { -acc } else { acc };
            if signed > i64::MAX as i128 {
                i64::MAX
            } else if signed < i64::MIN as i128 {
                i64::MIN
            } else {
                signed as i64
            }
        };

        // Stored into an `int`, i.e. truncated.
        Some(as_long as i32)
    }
}

struct DivT {
    quot: i32,
    rem: i32,
}

/// `div()` from <stdlib.h>: truncation toward zero. Division by zero and
/// `INT_MIN / -1` are undefined in C; on x86-64 the hardware raises SIGFPE,
/// so the real `idiv` instruction is used to reproduce that faithfully.
#[cfg(target_arch = "x86_64")]
fn c_div(x: i32, y: i32) -> DivT {
    let quot: i32;
    let rem: i32;
    unsafe {
        std::arch::asm!(
            "cdq",
            "idiv {divisor:e}",
            divisor = in(reg) y,
            inout("eax") x => quot,
            out("edx") rem,
            // Not `pure`: the instruction can trap, which is a side effect the
            // optimizer must not assume away.
            options(nomem, nostack),
        );
    }
    DivT { quot, rem }
}

#[cfg(not(target_arch = "x86_64"))]
fn c_div(x: i32, y: i32) -> DivT {
    // Other architectures: the C program's undefined cases cannot be mapped to
    // a trap portably, so let the checked operators abort the process instead.
    DivT {
        quot: x / y,
        rem: x % y,
    }
}

fn main() {
    let mut x: i32 = 1;
    let mut y: i32 = 1;

    let stdin = std::io::stdin();
    let mut scanner = Scanner::new(stdin.lock());

    // scanf("%d %d", &x, &y); the literal space between the conversions is
    // redundant because %d already skips leading whitespace.
    if let Some(v) = scanner.scan_i32() {
        x = v;
        if let Some(v) = scanner.scan_i32() {
            y = v;
        }
    }

    let result = c_div(x, y);

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(
        out,
        "quotient: {}, remainder: {}\n",
        result.quot, result.rem
    );
    let _ = out.flush();
}
