// Rust translation of c_src/src/main.c
//
// Original C:
//   static void print_hex(unsigned char *p, int len);
//   void driver(int x);
//   int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
//
// The program reads a single decimal integer with scanf("%d", &x) and then
// prints the raw bytes of that `int` object, in memory (native/little-endian)
// order, as lowercase two-digit hex, followed by a newline.
//
// Notes on fidelity:
//   * scanf("%d") skips *any* leading whitespace, including newlines, before
//     the number, so it reads across line boundaries.
//   * If the conversion fails (EOF or no digits), `x` keeps its initial value
//     of 0 and the program still prints the bytes of 0 -- the C code ignores
//     scanf's return value, so this "bug" is reproduced exactly.
//   * glibc implements %d by running its strtol machinery, which saturates at
//     LONG_MAX / LONG_MIN (64-bit), and then assigns that `long` to an `int`
//     (an implementation-defined truncation).  That behavior is reproduced
//     here: accumulate with i64 saturation, then truncate to i32.

use std::io::{Read, Write};

/// Print `len` bytes starting at `p` as `%02x` each, then a newline.
/// Mirrors the C `print_hex(unsigned char *p, int len)`.
fn print_hex(p: &[u8], len: i32, out: &mut Vec<u8>) {
    let mut i: i32 = 0;
    while i < len {
        // printf("%02x", p[i]) -- lowercase, zero padded to width 2.
        let b = p[i as usize];
        out.push(hex_digit(b >> 4));
        out.push(hex_digit(b & 0x0f));
        i += 1;
    }
    // printf("\n")
    out.push(b'\n');
}

fn hex_digit(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        _ => b'a' + (nibble - 10),
    }
}

/// Mirrors the C `driver(int x)`: reinterpret the storage of `x` as
/// `unsigned char[sizeof(int)]` and hex-dump it.
fn driver(x: i32, out: &mut Vec<u8>) {
    let bytes = x.to_ne_bytes(); // same object representation as (unsigned char *)&x
    print_hex(&bytes, std::mem::size_of::<i32>() as i32, out);
}

/// A byte source over stdin with a single-byte pushback, mimicking the
/// getc/ungetc pair that scanf uses.
struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new(data: Vec<u8>) -> Self {
        Scanner { data, pos: 0 }
    }

    fn getc(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let c = self.data[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }

    fn ungetc(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }
}

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Emulate `scanf("%d", &x)`.
/// Returns `Some(value)` on a successful conversion, `None` on matching
/// failure or input failure (in which case the caller leaves `x` untouched).
fn scanf_d(s: &mut Scanner) -> Option<i32> {
    // Skip leading whitespace (crosses newlines).
    let mut c = loop {
        match s.getc() {
            None => return None, // input failure (EOF)
            Some(c) if is_c_space(c) => continue,
            Some(c) => break c,
        }
    };

    // Optional sign.
    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match s.getc() {
            None => {
                // Sign then EOF: matching failure, nothing stored.
                return None;
            }
            Some(nc) => c = nc,
        }
    }

    if !c.is_ascii_digit() {
        // Matching failure: push the offending character back.
        s.ungetc();
        return None;
    }

    // Accumulate as a 64-bit `long`, saturating like glibc's strtol.
    let mut acc: i64 = 0;
    let mut overflow = false;
    loop {
        let d = (c - b'0') as i64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        match s.getc() {
            Some(nc) if nc.is_ascii_digit() => c = nc,
            Some(_) => {
                s.ungetc();
                break;
            }
            None => break,
        }
    }

    let as_long: i64 = if overflow {
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

    // Assignment of the `long` result to an `int` object.
    Some(as_long as i32)
}

fn main() {
    let mut input = Vec::new();
    // Ignore read errors the same way an input failure is ignored below.
    let _ = std::io::stdin().read_to_end(&mut input);
    let mut scanner = Scanner::new(input);

    let mut x: i32 = 0;
    if let Some(v) = scanf_d(&mut scanner) {
        x = v;
    }

    let mut out: Vec<u8> = Vec::new();
    driver(x, &mut out);

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(&out);
    let _ = lock.flush();
}
