// Rust translation of c_src/src/main.c
//
// Original C:
//     int main() {
//         int x = 1, y = 1;
//         scanf("%d %d", &x, &y);
//         div_t result = div(x, y);
//         printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
//         return 0;
//     }
//
// Behaviors that must be preserved exactly:
//   * `x` and `y` are pre-initialized to 1.  `scanf` return value is ignored,
//     so on input/matching failure the affected variables keep the value 1.
//   * `%d` skips leading whitespace (including newlines), accepts an optional
//     sign, then digits; the first non-digit is pushed back onto the stream.
//   * glibc's `%d` conversion runs through `strtol`, which saturates at
//     LONG_MAX / LONG_MIN on overflow; the result is then truncated to `int`.
//   * `div(x, y)` performs a raw hardware division.  With `y == 0`, or with
//     `x == INT_MIN && y == -1`, x86 `idiv` traps and the process dies from
//     SIGFPE with nothing written to stdout.  That is undefined behavior in C,
//     and it is reproduced here rather than "fixed".

use std::io::{Read, Write};

extern "C" {
    fn raise(sig: core::ffi::c_int) -> core::ffi::c_int;
}

const SIGFPE: core::ffi::c_int = 8;

/// Terminate the way an x86 divide fault does: killed by SIGFPE, no output.
fn die_by_sigfpe() -> ! {
    unsafe {
        raise(SIGFPE);
    }
    // SIGFPE's default disposition terminates the process, so this is not
    // reached in practice.
    std::process::abort();
}

/// A byte stream over stdin with one byte of pushback, mirroring the way
/// `scanf` peeks one character past a conversion and `ungetc`s it.
struct Stream {
    inner: std::io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Stream {
    fn new() -> Self {
        Stream {
            inner: std::io::stdin(),
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
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    fn unget(&mut self, b: u8) {
        self.peeked = Some(b);
    }
}

/// C `isspace` for the default locale.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// One `%d` conversion. Returns `None` on input failure (EOF before any
/// non-whitespace) or matching failure (no digits), leaving the caller's
/// variable untouched, exactly as `scanf` does.
fn scan_int(s: &mut Stream) -> Option<i32> {
    // Skip leading whitespace.
    let mut b = loop {
        match s.next_byte() {
            Some(c) if is_space(c) => continue,
            Some(c) => break c,
            None => return None, // input failure
        }
    };

    let mut negative = false;
    if b == b'+' || b == b'-' {
        negative = b == b'-';
        match s.next_byte() {
            Some(c) => b = c,
            None => return None, // matching failure
        }
    }

    if !b.is_ascii_digit() {
        s.unget(b);
        return None; // matching failure
    }

    // Accumulate as i64 with strtol-style saturation, then truncate to int.
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        if !saturated {
            let digit = (b - b'0') as i64;
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        match s.next_byte() {
            Some(c) if c.is_ascii_digit() => b = c,
            Some(c) => {
                s.unget(c);
                break;
            }
            None => break,
        }
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };

    Some(value as i32)
}

/// `scanf("%d %d", &x, &y)`. The literal space between the two directives
/// matches any amount of whitespace, which the leading-whitespace skip of the
/// second `%d` already covers.
fn scanf_two_ints(x: &mut i32, y: &mut i32) {
    let mut stream = Stream::new();
    match scan_int(&mut stream) {
        Some(v) => *x = v,
        None => return,
    }
    if let Some(v) = scan_int(&mut stream) {
        *y = v;
    }
}

struct DivT {
    quot: i32,
    rem: i32,
}

/// `div(3)`: truncating division. Reproduces the x86 divide fault for the
/// cases where the C original invokes undefined behavior.
fn c_div(numer: i32, denom: i32) -> DivT {
    if denom == 0 || (numer == i32::MIN && denom == -1) {
        die_by_sigfpe();
    }
    DivT {
        quot: numer / denom,
        rem: numer % denom,
    }
}

fn main() {
    let mut x: i32 = 1;
    let mut y: i32 = 1;

    scanf_two_ints(&mut x, &mut y);

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
