// Rust translation of c_src/src/main.c
//
// The original C source is written using ISO 646 digraphs and alternative
// operator spellings:
//   `%:` == `#`, `<%` == `{`, `%>` == `}`
//   `bitor` == `|`, `compl` == `~`   (from <iso646.h>)
//
// so `int result = x bitor compl y;` is `int result = x | ~y;`.

use std::io::{Read, Write};

/// A minimal stdin reader that mimics C `FILE*` semantics closely enough for
/// `scanf("%d", ...)`: byte oriented, with a single byte of pushback
/// (the equivalent of `ungetc`).
struct CStdin {
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
    pushback: Option<u8>,
}

impl CStdin {
    fn new() -> CStdin {
        CStdin {
            buf: Vec::new(),
            pos: 0,
            eof: false,
            pushback: None,
        }
    }

    /// Read the next byte, or `None` at end of input.
    fn getc(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.take() {
            return Some(b);
        }
        if self.pos >= self.buf.len() {
            if self.eof {
                return None;
            }
            let mut chunk = [0u8; 4096];
            match std::io::stdin().read(&mut chunk) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.buf.clear();
                    self.buf.extend_from_slice(&chunk[..n]);
                    self.pos = 0;
                }
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Some(b)
    }

    /// Push a byte back onto the stream (like `ungetc`).
    fn ungetc(&mut self, b: u8) {
        self.pushback = Some(b);
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates `scanf("%d", &out)`.
///
/// Returns `Some(value)` on a successful conversion; `None` on a matching
/// failure or on end of input (in which case the caller leaves its variable
/// untouched, exactly as C does).
///
/// Leading whitespace (including newlines) is skipped, so this reads across
/// line boundaries just like C's `scanf`.
fn scanf_i32(input: &mut CStdin) -> Option<i32> {
    // Skip leading whitespace.
    let mut c = loop {
        match input.getc() {
            None => return None, // EOF before any conversion
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match input.getc() {
            None => return None,
            Some(b) => c = b,
        }
    }

    if !c.is_ascii_digit() {
        // Matching failure: the offending character stays in the stream.
        input.ungetc(c);
        return None;
    }

    // Accumulate into a `long` (64-bit), saturating like strtol does, then
    // truncate to `int` on assignment, matching glibc's behaviour.
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = (c - b'0') as i64;
        if !saturated {
            match acc
                .checked_mul(10)
                .and_then(|v| v.checked_add(digit))
            {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        match input.getc() {
            None => break,
            Some(b) if b.is_ascii_digit() => c = b,
            Some(b) => {
                input.ungetc(b);
                break;
            }
        }
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        acc.wrapping_neg()
    } else {
        acc
    };

    Some(value as i32)
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
    let mut input = CStdin::new();
    let mut x: i32 = 0;
    let mut y: i32 = 0;
    if let Some(v) = scanf_i32(&mut input) {
        x = v;
    }
    if let Some(v) = scanf_i32(&mut input) {
        y = v;
    }
    driver(x, y);
}
