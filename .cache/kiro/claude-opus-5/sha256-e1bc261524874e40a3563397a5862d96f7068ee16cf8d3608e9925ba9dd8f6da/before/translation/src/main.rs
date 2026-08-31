// Rust translation of c_src/src/main.c
//
// Original C:
//   Copyright 2025 MIT Lincoln Laboratory (MIT-style license, see c_src/src/main.c)
//
// Behavior is reproduced exactly, including the original control flow that uses
// `goto` to jump backwards into the middle of a `while` body (which bypasses the
// loop condition test) and forwards past a label.

use std::io::{self, Read, Write};

/// Minimal emulation of the parts of C's `scanf` that this program uses.
///
/// Reads bytes from stdin one at a time so that only the characters actually
/// consumed by a conversion are taken from the stream, matching C's behavior of
/// skipping whitespace (including newlines) before a `%d` conversion.
struct Scanner {
    input: io::Stdin,
    /// One-byte pushback slot, standing in for `ungetc`.
    peeked: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            input: io::stdin(),
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
            match self.input.read(&mut buf) {
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

    fn unread(&mut self, b: u8) {
        self.peeked = Some(b);
    }

    /// C `isspace` for the "C" locale.
    fn is_space(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
    }

    /// Performs a single `%d` conversion.
    ///
    /// Returns `Some(value)` on success, `None` on input failure or a matching
    /// failure (in which case the caller must not attempt further conversions,
    /// just as `scanf` stops at the first failing directive).
    fn scan_i32(&mut self) -> Option<i32> {
        // Skip leading whitespace.
        let mut b = loop {
            match self.next_byte() {
                Some(c) if Self::is_space(c) => continue,
                Some(c) => break c,
                None => return None,
            }
        };

        // Optional sign.
        let mut negative = false;
        if b == b'+' || b == b'-' {
            negative = b == b'-';
            match self.next_byte() {
                Some(c) => b = c,
                None => return None,
            }
        }

        if !b.is_ascii_digit() {
            // Matching failure: push the offending character back.
            self.unread(b);
            return None;
        }

        // Accumulate digits. glibc converts the digit string with `strtol`
        // (which saturates) and then assigns the result to an `int`, so
        // saturate at the 64-bit bounds and truncate to 32 bits.
        let mut acc: i64 = 0;
        loop {
            let digit = (b - b'0') as i64;
            acc = acc
                .checked_mul(10)
                .and_then(|v| {
                    if negative {
                        v.checked_sub(digit)
                    } else {
                        v.checked_add(digit)
                    }
                })
                .unwrap_or(if negative { i64::MIN } else { i64::MAX });

            match self.next_byte() {
                Some(c) if c.is_ascii_digit() => b = c,
                Some(c) => {
                    self.unread(c);
                    break;
                }
                None => break,
            }
        }

        Some(acc as i32)
    }
}

/// The label a jump targets in `foo`'s reconstructed control flow.
enum State {
    /// Evaluate the `while` condition.
    LoopCond,
    /// Start of the loop body (`printf("loop\n")`).
    LoopTop,
    /// `label1:`
    Label1,
    /// `label2:`
    Label2,
}

fn foo(out: &mut impl Write, mut x: i32, mut y: i32) {
    let mut state = State::LoopCond;

    loop {
        match state {
            State::LoopCond => {
                if !(x > 0 || y > 0) {
                    return;
                }
                state = State::LoopTop;
            }

            State::LoopTop => {
                let _ = write!(out, "loop\n");

                if x == 1 && y == 4 {
                    state = State::Label2; // goto label2;
                } else {
                    state = State::Label1; // fall through to label1
                }
            }

            State::Label1 => {
                if x > 0 {
                    let _ = write!(out, "x\n");
                    x = x.wrapping_sub(1);
                }
                state = State::Label2; // fall through to label2
            }

            State::Label2 => {
                if y == 0 {
                    state = State::LoopCond; // continue;
                    continue;
                }
                let _ = write!(out, "y\n");
                y = y.wrapping_sub(1);
                if x < 3 {
                    state = State::Label1; // goto label1;
                } else {
                    state = State::LoopCond; // end of body
                }
            }
        }
    }
}

fn main() {
    let mut x: i32 = 0;
    let mut y: i32 = 0;

    let mut scanner = Scanner::new();
    // scanf("%d %d", &x, &y): the whitespace directive between the two
    // conversions is redundant here because %d already skips leading
    // whitespace. If the first conversion fails, the second is not attempted,
    // leaving the corresponding variable at its initial value.
    if let Some(v) = scanner.scan_i32() {
        x = v;
        if let Some(v) = scanner.scan_i32() {
            y = v;
        }
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    foo(&mut out, x, y);
    let _ = out.flush();
}
