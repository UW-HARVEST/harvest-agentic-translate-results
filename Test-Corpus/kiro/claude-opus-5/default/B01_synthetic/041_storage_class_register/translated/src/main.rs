// Rust translation of c_src/src/main.c
//
// Original C:
//     void driver(int x) {
//         register int y = 2*x;
//         y += 300;
//         printf("%d\n", y);
//     }
//     int main() {
//         int x = 0;
//         scanf("%d", &x);
//         driver(x);
//         return 0;
//     }
//
// Behavioral notes preserved here:
//   * `scanf("%d", &x)` skips *any* leading whitespace, including newlines, then
//     consumes an optional sign followed by one or more decimal digits. It stops
//     at the first character that cannot extend the number, and does not read
//     past it.
//   * On a matching failure or EOF the conversion assigns nothing, so `x` keeps
//     its initializer value of 0 and the program prints 300.
//   * On integer overflow glibc's `%d` conversion clamps the intermediate value
//     to LONG_MAX / LONG_MIN (64-bit on Linux) and then stores the low 32 bits
//     into the `int` destination. That truncation is reproduced below.
//   * `2*x` and `y += 300` are `int` arithmetic; overflow is reproduced as
//     two's-complement wraparound, matching what the C compiler emits.

use std::io::{self, Read, Write};

/// A byte-at-a-time reader over stdin with a one-byte pushback slot, so that
/// scanning consumes exactly the bytes that C's `scanf` would consume.
struct Input {
    stdin: io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Input {
    fn new() -> Self {
        Input {
            stdin: io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    /// Returns the next byte without consuming it, or `None` at end of input.
    fn peek(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.stdin.read(&mut buf) {
            Ok(0) => {
                self.eof = true;
                None
            }
            Ok(_) => {
                self.peeked = Some(buf[0]);
                Some(buf[0])
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => self.peek(),
            Err(_) => {
                self.eof = true;
                None
            }
        }
    }

    /// Consumes and returns the next byte, or `None` at end of input.
    fn next_byte(&mut self) -> Option<u8> {
        let b = self.peek();
        if b.is_some() {
            self.peeked = None;
        }
        b
    }
}

/// `isspace` for the C locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates the `%d` conversion of `scanf`.
///
/// Returns `Some(value)` when a conversion happened, or `None` on input failure
/// (EOF before any character) or matching failure (no digits found). In the
/// `None` case the caller must leave its destination variable untouched, exactly
/// like C.
fn scan_int(inp: &mut Input) -> Option<i32> {
    // Directive whitespace: skip over spaces, tabs, newlines, ... without limit.
    loop {
        match inp.peek() {
            Some(b) if is_c_space(b) => {
                inp.next_byte();
            }
            _ => break,
        }
    }

    // Optional sign.
    let negative = match inp.peek() {
        Some(b'-') => {
            inp.next_byte();
            true
        }
        Some(b'+') => {
            inp.next_byte();
            false
        }
        _ => false,
    };

    // At least one digit is required, otherwise this is a matching failure.
    let mut saw_digit = false;
    // Magnitude accumulated with saturation, mirroring glibc's clamping to the
    // range of `long` before the value is narrowed to `int`.
    const LONG_MAX_MAG: u128 = i64::MAX as u128; //  9223372036854775807
    const LONG_MIN_MAG: u128 = LONG_MAX_MAG + 1; //  9223372036854775808
    let mut mag: u128 = 0;
    let mut overflow = false;

    while let Some(b) = inp.peek() {
        if !b.is_ascii_digit() {
            break;
        }
        inp.next_byte();
        saw_digit = true;
        if !overflow {
            mag = mag * 10 + u128::from(b - b'0');
            if mag > LONG_MIN_MAG {
                overflow = true;
            }
        }
    }

    if !saw_digit {
        return None;
    }

    let as_long: i64 = if negative {
        if overflow || mag >= LONG_MIN_MAG {
            i64::MIN
        } else {
            -(mag as i64)
        }
    } else if overflow || mag > LONG_MAX_MAG {
        i64::MAX
    } else {
        mag as i64
    };

    // Storing a `long` through an `int *`: keep the low 32 bits.
    Some(as_long as i32)
}

fn driver(x: i32, out: &mut impl Write) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    let _ = write!(out, "{}\n", y);
}

fn main() {
    let mut input = Input::new();

    let mut x: i32 = 0;
    if let Some(v) = scan_int(&mut input) {
        x = v;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(x, &mut out);
    let _ = out.flush();
}
