// Rust translation of c_src/src/main.c
//
// Original C:
//   int x = 0;
//   scanf("%d", &x);
//   driver(x);            // memcpy x's bytes into char raw[4], print each as %02x
//
// Behavior preserved exactly:
//   * scanf("%d") skips leading whitespace (including newlines) before the number.
//   * On matching failure / EOF, scanf leaves `x` untouched, so x stays 0.
//   * glibc's %d conversion is strtol-based: values outside long's range saturate
//     to LONG_MAX / LONG_MIN and are then truncated when stored into an int.
//   * Output is the object representation of the int, byte 0 first (little-endian
//     on the x86-64 target the C was built for), lower-case hex, two digits per
//     byte, followed by a single '\n'.

use std::io::{self, BufRead, Write};

/// Byte-oriented reader over stdin with one byte of push-back, mirroring the
/// `ungetc` that scanf performs on a non-matching character.
struct Input<R: BufRead> {
    inner: R,
    peeked: Option<u8>,
    eof: bool,
}

impl<R: BufRead> Input<R> {
    fn new(inner: R) -> Self {
        Input {
            inner,
            peeked: None,
            eof: false,
        }
    }

    /// Look at the next byte without consuming it.
    fn peek(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.inner.read(&mut buf) {
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

    /// Consume the byte returned by the most recent `peek`.
    fn bump(&mut self) {
        self.peeked = None;
    }
}

/// C's `isspace` for the default "C" locale.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Emulates a single `scanf("%d", &out)` conversion.
///
/// Returns `true` when a value was assigned (scanf returning 1); `false` on a
/// matching failure or input failure, in which case `out` is left unchanged.
fn scanf_i32<R: BufRead>(input: &mut Input<R>, out: &mut i32) -> bool {
    // Skip leading whitespace; this crosses newlines just like scanf does.
    while let Some(b) = input.peek() {
        if is_space(b) {
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
        Some(_) => {}
        None => return false, // input failure before any conversion
    }

    // Digit sequence; at least one digit is required.
    let mut saw_digit = false;
    // Accumulate with saturation at long's bounds, as strtol does (ERANGE).
    let mut acc: i128 = 0;
    let mut saturated = false;
    while let Some(b) = input.peek() {
        if !b.is_ascii_digit() {
            break;
        }
        input.bump();
        saw_digit = true;
        if !saturated {
            acc = acc * 10 + i128::from(b - b'0');
            if negative {
                if -acc < i128::from(i64::MIN) {
                    saturated = true;
                }
            } else if acc > i128::from(i64::MAX) {
                saturated = true;
            }
        }
    }

    if !saw_digit {
        // Matching failure: the sign (if any) is pushed back and nothing is
        // stored. Since this is the program's only conversion, the push-back
        // state is not observable afterwards.
        return false;
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        (-acc) as i64
    } else {
        acc as i64
    };

    // Storing a long into an int* truncates to the low 32 bits.
    *out = value as i32;
    true
}

/// `print_hex(p, len)` from the C source.
fn print_hex(out: &mut dyn Write, p: &[u8]) -> io::Result<()> {
    for &byte in p {
        write!(out, "{:02x}", byte)?;
    }
    writeln!(out)
}

/// `driver(int x)`: copy x's object representation into a 4-byte buffer and
/// print it byte by byte.
fn driver(out: &mut dyn Write, x: i32) -> io::Result<()> {
    // `to_ne_bytes` is exactly what `memcpy(raw, &x, sizeof(x))` does: the
    // object representation in the target's native byte order.
    let raw: [u8; std::mem::size_of::<i32>()] = x.to_ne_bytes();
    print_hex(out, &raw)
}

fn main() {
    let stdin = io::stdin();
    let mut input = Input::new(stdin.lock());

    let mut x: i32 = 0;
    let _ = scanf_i32(&mut input, &mut x);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = driver(&mut out, x);
    let _ = out.flush();
}
