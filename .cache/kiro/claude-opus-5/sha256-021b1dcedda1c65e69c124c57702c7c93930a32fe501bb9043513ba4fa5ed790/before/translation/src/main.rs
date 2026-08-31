// Rust translation of c_src/src/main.c
//
// Behavioral notes (the C semantics that are reproduced verbatim):
//
//  * `scanf("%d", &data[i])` skips *any* leading whitespace, including newlines,
//    so a single number per line and all numbers on one line are equivalent.
//  * The loop stops as soon as `scanf` does not return 1, i.e. on EOF or on a
//    matching failure (a non-numeric token). `i` then holds the count of values
//    successfully read, and only those are processed/printed.
//  * glibc implements `%d` on a 64-bit platform by running `strtol` and then
//    assigning the `long` result to an `int`. `strtol` saturates at
//    `LONG_MIN`/`LONG_MAX` on overflow, and the assignment truncates. That
//    two-step behaviour is emulated here (see `saturate_then_truncate`).
//  * `fma_array(out, out, out, out, len)` passes the same buffer for every
//    parameter, so each element becomes `x * x + x` computed from its own
//    current value. Signed overflow is undefined behaviour in C; the compiled
//    code wraps, so wrapping arithmetic is used.
//
// The `restrict`-free aliasing, the argument order, and the error-check order
// are all preserved as in the original.

use std::io::{Read, Write};

/// Byte-level view of stdin with a single-byte "unget", mirroring the way
/// `scanf` consumes characters from a `FILE *` stream.
struct Scanner {
    buf: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new() -> Scanner {
        let mut buf = Vec::new();
        // A read error is indistinguishable from EOF for this program's
        // purposes: `scanf` would return EOF and the loop would break.
        let _ = std::io::stdin().read_to_end(&mut buf);
        Scanner { buf, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    fn bump(&mut self) {
        if self.pos < self.buf.len() {
            self.pos += 1;
        }
    }

    /// C's `isspace` for the default locale.
    fn is_space(c: u8) -> bool {
        matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    /// Equivalent of `scanf("%d", out)`, returning the number of items
    /// assigned: `Some(v)` for a successful conversion, `None` for either EOF
    /// or a matching failure (both make the C code `break`).
    fn scan_i32(&mut self) -> Option<i32> {
        while let Some(c) = self.peek() {
            if Scanner::is_space(c) {
                self.bump();
            } else {
                break;
            }
        }

        // EOF before any conversion: `scanf` returns EOF.
        self.peek()?;

        let mut negative = false;
        match self.peek() {
            Some(b'-') => {
                negative = true;
                self.bump();
            }
            Some(b'+') => {
                self.bump();
            }
            _ => {}
        }

        let mut digits = 0usize;
        // Accumulate in i128 with saturation so arbitrarily long digit runs
        // cannot panic; the magnitude is clamped to the `long` range below.
        let mut magnitude: i128 = 0;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            digits += 1;
            if magnitude <= OVERFLOW_GUARD {
                magnitude = magnitude * 10 + i128::from(c - b'0');
            }
            self.bump();
        }

        // No digits consumed: matching failure, `scanf` returns 0.
        if digits == 0 {
            return None;
        }

        Some(saturate_then_truncate(negative, magnitude))
    }
}

/// Past this point further digits cannot change the saturated result.
const OVERFLOW_GUARD: i128 = i64::MAX as i128;

/// glibc's `%d` path: `strtol` clamps to the `long` range and sets `ERANGE`,
/// then the result is stored into an `int`, truncating the upper bits.
fn saturate_then_truncate(negative: bool, magnitude: i128) -> i32 {
    let as_long: i64 = if negative {
        let signed = -magnitude;
        if signed < i64::MIN as i128 {
            i64::MIN
        } else {
            signed as i64
        }
    } else if magnitude > i64::MAX as i128 {
        i64::MAX
    } else {
        magnitude as i64
    };
    as_long as i32
}

/// `void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)`
///
/// The C version is called with all four pointers aliasing the same buffer, so
/// this is expressed as an in-place transform over one slice to stay in safe
/// Rust while producing identical results.
fn fma_array_aliased(out: &mut [i32], len: usize) {
    for i in 0..len {
        out[i] = out[i].wrapping_mul(out[i]).wrapping_add(out[i]);
    }
}

/// `void driver(int *out, int len)`
fn driver<W: Write>(w: &mut W, out: &mut [i32], len: usize) {
    fma_array_aliased(out, len);
    for i in 0..len {
        let _ = writeln!(w, "{}", out[i]);
    }
}

fn main() {
    // `int data[100];` — only the first `i` entries are ever read back.
    let mut data = [0i32; 100];
    let mut scanner = Scanner::new();

    let mut i = 0usize;
    while i < 100 {
        match scanner.scan_i32() {
            Some(v) => data[i] = v,
            None => break,
        }
        i += 1;
    }

    let stdout = std::io::stdout();
    let mut w = std::io::BufWriter::new(stdout.lock());
    driver(&mut w, &mut data, i);
    let _ = w.flush();
}
