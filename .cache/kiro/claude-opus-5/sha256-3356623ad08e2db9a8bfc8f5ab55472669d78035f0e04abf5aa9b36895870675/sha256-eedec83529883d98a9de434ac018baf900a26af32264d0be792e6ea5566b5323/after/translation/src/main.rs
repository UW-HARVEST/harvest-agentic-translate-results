// Rust translation of c_src/src/main.c
//
// The original C source is written with ISO 646 digraphs / alternative tokens:
//   `%:include` == `#include`, `<%` == `{`, `%>` == `}`
// and, via <iso646.h>:
//   `bitor` == `|`, `compl` == `~`
//
// So `int result = x bitor compl y;` is `int result = x | ~y;`
//
// Behavior reproduced exactly:
//   * `x` and `y` are initialized to 0; a failed/EOF `scanf` leaves them at 0
//     (the C code ignores scanf's return value).
//   * `scanf("%d", ...)` skips leading whitespace *including newlines* and
//     stops at the first character that cannot extend the integer, pushing
//     that character back onto the stream.
//   * Integer conversion follows glibc: the digit sequence is accumulated with
//     `long` (i64) saturation, then truncated to `int` (i32) on store.
//   * Output is `printf("%d", result)` followed by `puts("")`, i.e. the decimal
//     value and then a single newline, with no trailing space.

use std::io::{self, Read, Write};

/// Byte-oriented view of stdin with one byte of pushback, mirroring the way C's
/// `scanf` consumes a shared `FILE *` stream across successive calls.
struct CStdin {
    inner: io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl CStdin {
    fn new() -> Self {
        CStdin {
            inner: io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    /// Read one byte, or `None` at end of input.
    fn getc(&mut self) -> Option<u8> {
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
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    /// Push a byte back onto the stream (`ungetc`).
    fn ungetc(&mut self, b: u8) {
        self.peeked = Some(b);
    }
}

/// True for the characters C's `isspace` accepts in the "C" locale, which is
/// what a `%d` conversion skips over before the number.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Equivalent of a single `scanf("%d", &out)` conversion.
///
/// Returns `Some(value)` when the conversion succeeds, `None` on matching
/// failure or input failure (in which case the caller leaves its variable
/// untouched, exactly like the C program does).
fn scanf_int(stream: &mut CStdin) -> Option<i32> {
    // Skip leading whitespace, newlines included.
    let mut c = loop {
        match stream.getc() {
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
            None => return None, // input failure (EOF before any conversion)
        }
    };

    // Optional sign. glibc consumes the sign into its work buffer and reads the
    // next character; if that character cannot start a number the conversion is
    // a matching failure and the *sign* is not pushed back -- only the one
    // offending character is.
    let mut negative = false;
    if c == b'-' || c == b'+' {
        negative = c == b'-';
        match stream.getc() {
            Some(b) => c = b,
            None => {
                // EOF right after the sign. glibc's internal `ungetc` macro is a
                // no-op for EOF, so nothing is pushed back here.
                return None;
            }
        }
    }

    if !c.is_ascii_digit() {
        // Matching failure: the offending character stays in the stream.
        stream.ungetc(c);
        return None;
    }

    // Accumulate with `long` saturation, as glibc's strtol-based conversion does.
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = (c - b'0') as i64;
        if !saturated {
            match acc
                .checked_mul(10)
                .and_then(|v| if negative { v.checked_sub(digit) } else { v.checked_add(digit) })
            {
                Some(v) => acc = v,
                None => {
                    saturated = true;
                    acc = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        match stream.getc() {
            Some(b) if b.is_ascii_digit() => c = b,
            Some(b) => {
                stream.ungetc(b);
                break;
            }
            None => break,
        }
    }

    // Storing a `long` into an `int` truncates.
    Some(acc as i32)
}

fn driver(x: i32, y: i32, out: &mut impl Write) {
    let result: i32 = x | !y; // x bitor compl y
    let _ = write!(out, "{}", result); // printf("%d", result)
    let _ = writeln!(out); // puts("")
}

fn main() {
    let mut stream = CStdin::new();

    let mut x: i32 = 0;
    let mut y: i32 = 0;

    if let Some(v) = scanf_int(&mut stream) {
        x = v;
    }
    if let Some(v) = scanf_int(&mut stream) {
        y = v;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(x, y, &mut out);
    let _ = out.flush();
}
