// Rust translation of c_src/src/main.c
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::io::{self, Read, Write};

/// Mirrors the C `house_t` struct.
#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

/// Mirrors the C file-scope `static house_t the_house` initializer.
const THE_HOUSE_INIT: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

fn add_floor(house: &mut House) {
    // C: house->floors++ on an int -> wraps on overflow in practice.
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // C: house->bedrooms += extra_bedrooms on ints.
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn add_floor_to_the_house(the_house: &mut House) {
    add_floor(the_house);
}

fn print_the_house<W: Write>(out: &mut W, the_house: &House) {
    // C: printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {} bathrooms\n",
        the_house.floors,
        the_house.bedrooms,
        format_f1(the_house.bathrooms)
    );
}

/// Formats a double the way C's `%.1f` conversion does.
fn format_f1(v: f64) -> String {
    if v.is_nan() {
        // glibc prints "nan" / "-nan" for %f.
        return if v.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if v.is_infinite() {
        return if v < 0.0 {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    // Rust's `{:.1}` rounds the exact binary value half-to-even, matching glibc.
    format!("{:.1}", v)
}

fn run<W: Write>(out: &mut W, the_house: &mut House, extra_bedrooms: i32) {
    print_the_house(out, the_house);
    add_floor_to_the_house(the_house);
    print_the_house(out, the_house);
    the_house.bathrooms += 1.0;
    print_the_house(out, the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_the_house(out, the_house);
}

/// Emulates `scanf("%d", &x)`.
///
/// Returns `Some(value)` on a successful conversion, `None` on a matching
/// failure or EOF (in which case the C code leaves `x` untouched).
/// Consumes exactly the bytes `scanf` would consume: leading whitespace is
/// skipped (including newlines), then an optional sign and a digit run. One
/// byte of lookahead may be read and pushed back, matching stdio's `ungetc`.
fn scanf_int(stdin: &mut ByteReader) -> Option<i32> {
    // Skip whitespace, exactly as the %d directive's leading whitespace skip does.
    loop {
        match stdin.peek() {
            Some(c) if is_c_space(c) => {
                stdin.consume();
            }
            _ => break,
        }
    }

    let mut negative = false;
    match stdin.peek() {
        Some(b'+') => {
            stdin.consume();
        }
        Some(b'-') => {
            negative = true;
            stdin.consume();
        }
        _ => {}
    }

    let mut saw_digit = false;
    // glibc's %d directive converts through the same code path as `strtol`:
    // the magnitude is accumulated as an *unsigned long* and, if it exceeds
    // the representable range, the result saturates to LONG_MAX / LONG_MIN
    // (with ERANGE) rather than wrapping. The saturated `long` is then
    // narrowed to `int` by plain truncation when it is stored through the
    // `int *` argument.
    let mut acc: u64 = 0;
    let mut overflow = false;
    while let Some(c) = stdin.peek() {
        if c.is_ascii_digit() {
            saw_digit = true;
            if !overflow {
                match acc
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(u64::from(c - b'0')))
                {
                    Some(v) => acc = v,
                    None => overflow = true,
                }
            }
            stdin.consume();
        } else {
            break;
        }
    }

    if !saw_digit {
        // Matching failure: no assignment is performed.
        return None;
    }

    // Largest magnitude representable in a `long` for this sign.
    let limit: u64 = if negative {
        0x8000_0000_0000_0000 // -LONG_MIN
    } else {
        0x7fff_ffff_ffff_ffff // LONG_MAX
    };

    let as_long: i64 = if overflow || acc > limit {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // `wrapping_neg` on the unsigned magnitude handles acc == -LONG_MIN.
        acc.wrapping_neg() as i64
    } else {
        acc as i64
    };

    // Truncating store into the `int` object.
    Some(as_long as i32)
}

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Minimal buffered reader with one byte of pushback, so stdin consumption
/// matches C stdio semantics.
struct ByteReader {
    inner: io::Stdin,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl ByteReader {
    fn new() -> Self {
        ByteReader {
            inner: io::stdin(),
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    fn fill(&mut self) {
        if self.pos < self.buf.len() || self.eof {
            return;
        }
        self.buf.clear();
        self.pos = 0;
        let mut chunk = [0u8; 4096];
        match self.inner.read(&mut chunk) {
            Ok(0) => self.eof = true,
            Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
            Err(_) => self.eof = true,
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.fill();
        if self.pos < self.buf.len() {
            Some(self.buf[self.pos])
        } else {
            None
        }
    }

    fn consume(&mut self) {
        if self.pos < self.buf.len() {
            self.pos += 1;
        }
    }
}

fn main() {
    let mut the_house = THE_HOUSE_INIT;

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    // C: int x = 0; scanf("%d", &x);
    let mut x: i32 = 0;
    let mut stdin = ByteReader::new();
    if let Some(v) = scanf_int(&mut stdin) {
        x = v;
    }

    run(&mut out, &mut the_house, x);
    run(&mut out, &mut the_house, x);

    let _ = out.flush();
}
