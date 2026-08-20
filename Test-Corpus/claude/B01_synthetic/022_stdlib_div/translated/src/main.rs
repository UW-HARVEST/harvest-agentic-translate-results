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
// Behaviors that are reproduced exactly (including the original bugs):
//   * The return value of scanf() is ignored, so on input/matching failure the
//     corresponding variable keeps its initial value of 1.
//   * "%d %d": each conversion first skips arbitrary leading whitespace
//     (including newlines), then matches an optional sign and one or more
//     decimal digits.  A matching failure on the first conversion means the
//     second conversion is never attempted, so both x and y stay 1.
//   * glibc accumulates the digits of a "%d" conversion into a `long int`
//     (saturating at LONG_MAX / LONG_MIN like strtol does) and then assigns
//     that value to the `int*` argument, truncating it.  This is reproduced.
//   * div(x, 0) -- and div(INT_MIN, -1) -- are undefined behavior in C; on
//     x86-64/ARM64 Linux the hardware divide instruction raises SIGFPE and the
//     process dies without printing anything.  The translation deliberately
//     reproduces that fatal signal instead of "fixing" the bug.

use std::io::{self, Read, Write};

extern "C" {
    /// libc's raise(3).  libc is already linked by the Rust standard library,
    /// so no external crate is required.
    fn raise(sig: i32) -> i32;
}

const SIGFPE: i32 = 8;

/// Byte-at-a-time reader over stdin with one byte of push-back, mirroring the
/// way C's scanf consumes only as much input as it needs (and ungetc's the
/// first character that cannot be part of the current conversion).
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

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        loop {
            match self.stdin.read(&mut buf) {
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

    fn unget(&mut self, b: u8) {
        self.peeked = Some(b);
    }
}

/// True for the characters that C's isspace() accepts in the "C" locale, which
/// is the set of characters a scanf conversion skips over.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// One "%d" conversion.  Returns None on input failure (EOF before any
/// non-whitespace) or matching failure (no digits), in which case the caller
/// must leave its variable untouched -- exactly like scanf.
fn scan_i32(input: &mut Input) -> Option<i32> {
    // Skip leading whitespace.
    let mut c = loop {
        match input.next_byte() {
            None => return None, // input failure
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
        }
    };

    // Optional sign.
    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match input.next_byte() {
            None => return None, // sign then EOF: matching failure
            Some(b) => c = b,
        }
    }

    if !c.is_ascii_digit() {
        input.unget(c);
        return None; // matching failure
    }

    // Accumulate the magnitude, saturating so that the strtol()-style clamping
    // to LONG_MAX / LONG_MIN can be applied below.
    let mut magnitude: u64 = 0;
    loop {
        let digit = u64::from(c - b'0');
        magnitude = magnitude.saturating_mul(10).saturating_add(digit);
        match input.next_byte() {
            None => break,
            Some(b) if b.is_ascii_digit() => c = b,
            Some(b) => {
                input.unget(b);
                break;
            }
        }
    }

    // strtol() clamps out-of-range results; glibc then truncates the `long` to
    // an `int` when storing it through the `int*` argument.
    const LONG_MAX: u64 = i64::MAX as u64;
    let as_long: i64 = if negative {
        if magnitude > LONG_MAX + 1 {
            i64::MIN
        } else {
            (magnitude as i128).wrapping_neg() as i64
        }
    } else if magnitude > LONG_MAX {
        i64::MAX
    } else {
        magnitude as i64
    };

    Some(as_long as i32)
}

fn main() {
    let mut x: i32 = 1;
    let mut y: i32 = 1;

    // scanf("%d %d", &x, &y);  -- return value ignored, just like the C code.
    let mut input = Input::new();
    if let Some(v) = scan_i32(&mut input) {
        x = v;
        if let Some(v) = scan_i32(&mut input) {
            y = v;
        }
    }

    // div(x, y): undefined behavior for y == 0 and for INT_MIN / -1.  Both
    // trap on the platforms this program targets, so reproduce the SIGFPE.
    if y == 0 || (x == i32::MIN && y == -1) {
        unsafe {
            raise(SIGFPE);
        }
        // Should be unreachable: SIGFPE's default action terminates the
        // process.  If it were somehow ignored, fall back to aborting rather
        // than printing output the C program would never produce.
        std::process::abort();
    }

    // div() truncates toward zero for both members, matching Rust's / and %.
    let quot = x / y;
    let rem = x % y;

    print!("quotient: {}, remainder: {}\n", quot, rem);
    let _ = io::stdout().flush();
}
