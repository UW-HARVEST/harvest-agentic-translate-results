// Translation of c_src/src/main.c to Rust.
//
// Original C:
//   static void print_hex(unsigned char *p, int len);
//   void driver(int x);
//   int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
//
// Behavior notes preserved from the C program:
//   * `scanf("%d", &x)` skips leading whitespace (including newlines), accepts an
//     optional sign, then decimal digits. On matching failure or EOF, `x` keeps
//     its initial value of 0.
//   * glibc converts `%d` via `strtol` into a `long`, saturating at
//     LONG_MAX / LONG_MIN on range errors, and then assigns (truncates) that
//     `long` into the `int` object. That exact behavior is reproduced here.
//   * The 4 bytes of the `int` are copied verbatim (native byte order) and
//     printed as lowercase two-digit hex, followed by a newline.

use std::io::{Read, Write};

/// A byte-at-a-time reader over stdin with a single byte of pushback, so that
/// only the bytes `scanf` would consume are actually read.
struct ByteReader<R: Read> {
    inner: R,
    peeked: Option<u8>,
    eof: bool,
}

impl<R: Read> ByteReader<R> {
    fn new(inner: R) -> Self {
        ByteReader {
            inner,
            peeked: None,
            eof: false,
        }
    }

    fn peek(&mut self) -> Option<u8> {
        if self.peeked.is_none() && !self.eof {
            let mut b = [0u8; 1];
            loop {
                match self.inner.read(&mut b) {
                    Ok(0) => {
                        self.eof = true;
                        break;
                    }
                    Ok(_) => {
                        self.peeked = Some(b[0]);
                        break;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        self.eof = true;
                        break;
                    }
                }
            }
        }
        self.peeked
    }

    fn bump(&mut self) {
        self.peeked = None;
    }
}

/// C `isspace` for the "C" locale.
fn c_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates a single `scanf("%d", &out)` conversion.
///
/// Returns `Some(value)` on a successful conversion, `None` on input failure
/// (EOF before any non-whitespace) or matching failure (no digits), in which
/// case the caller's variable is left unmodified -- exactly as C does.
fn scanf_int<R: Read>(r: &mut ByteReader<R>) -> Option<i32> {
    // Skip leading whitespace (crosses newlines, like scanf).
    loop {
        match r.peek() {
            Some(b) if c_isspace(b) => r.bump(),
            Some(_) => break,
            None => return None, // input failure (EOF)
        }
    }

    let mut negative = false;
    match r.peek() {
        Some(b'-') => {
            negative = true;
            r.bump();
        }
        Some(b'+') => {
            r.bump();
        }
        _ => {}
    }

    // Accumulate the magnitude; stop growing once it can no longer matter.
    const CAP: u128 = 1u128 << 70;
    let mut magnitude: u128 = 0;
    let mut digits = 0usize;
    while let Some(b) = r.peek() {
        if b.is_ascii_digit() {
            r.bump();
            digits += 1;
            if magnitude < CAP {
                magnitude = magnitude * 10 + u128::from(b - b'0');
            }
        } else {
            break;
        }
    }

    if digits == 0 {
        return None; // matching failure
    }

    // strtol-style saturation into `long` (64-bit), then truncation to `int`.
    let as_long: i64 = if negative {
        if magnitude >= (1u128 << 63) {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else if magnitude > i64::MAX as u128 {
        i64::MAX
    } else {
        magnitude as i64
    };

    Some(as_long as i32)
}

fn print_hex(out: &mut Vec<u8>, p: &[u8], len: usize) {
    for i in 0..len {
        let _ = write!(out, "{:02x}", p[i]);
    }
    let _ = write!(out, "\n");
}

fn driver(out: &mut Vec<u8>, x: i32) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw = x.to_ne_bytes();
    print_hex(out, &raw, raw.len());
}

fn main() {
    let mut x: i32 = 0;

    let stdin = std::io::stdin();
    let mut reader = ByteReader::new(stdin.lock());
    if let Some(v) = scanf_int(&mut reader) {
        x = v;
    }

    let mut out: Vec<u8> = Vec::new();
    driver(&mut out, x);

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(&out);
    let _ = lock.flush();
}
