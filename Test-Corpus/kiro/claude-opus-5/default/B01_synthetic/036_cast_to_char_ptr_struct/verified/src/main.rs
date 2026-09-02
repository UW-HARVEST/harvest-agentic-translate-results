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

//! Rust translation of c_src/src/main.c
//!
//! The C program dumps the raw bytes of a `house_t` struct as lowercase hex.
//! Reproducing that byte-for-byte requires reproducing the platform's struct
//! layout (x86-64 / LP64 System V ABI):
//!
//! ```text
//! typedef struct {
//!     int    floors;     // offset 0, 4 bytes
//!     int    bedrooms;   // offset 4, 4 bytes
//!     double bathrooms;  // offset 8, 8 bytes
//! } house_t;             // sizeof == 16, no padding holes
//! ```
//!
//! `house_t house = {0};` zero-initializes the object, so any padding reads as
//! zero. With this layout there is no padding, so the serialized image is just
//! the three fields in declaration order, little-endian.

use std::io::{Read, Write};

/// Size and field offsets of the C `house_t` on the reference platform.
const HOUSE_SIZE: usize = 16;
const OFF_FLOORS: usize = 0;
const OFF_BEDROOMS: usize = 4;
const OFF_BATHROOMS: usize = 8;

/// Mirror of the C `house_t` struct.
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

impl House {
    /// Equivalent of `house_t house = {0};`
    fn zeroed() -> Self {
        House {
            floors: 0,
            bedrooms: 0,
            bathrooms: 0.0,
        }
    }

    /// The object representation of the struct, as `(unsigned char *)&house`
    /// would see it. Padding bytes (none on this layout) stay zero, matching
    /// the zero-initialization done in `driver`.
    fn as_object_bytes(&self) -> [u8; HOUSE_SIZE] {
        let mut buf = [0u8; HOUSE_SIZE];
        buf[OFF_FLOORS..OFF_FLOORS + 4].copy_from_slice(&self.floors.to_le_bytes());
        buf[OFF_BEDROOMS..OFF_BEDROOMS + 4].copy_from_slice(&self.bedrooms.to_le_bytes());
        buf[OFF_BATHROOMS..OFF_BATHROOMS + 8].copy_from_slice(&self.bathrooms.to_le_bytes());
        buf
    }
}

/// `static void print_hex(unsigned char *p, int len)`
fn print_hex<W: Write>(out: &mut W, p: &[u8], len: usize) {
    for i in 0..len {
        // printf("%02x", p[i]);
        let _ = write!(out, "{:02x}", p[i]);
    }
    // printf("\n");
    let _ = writeln!(out);
}

/// `void driver(int floors)`
fn driver<W: Write>(out: &mut W, floors: i32) {
    let mut house = House::zeroed();
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.;
    let bytes = house.as_object_bytes();
    print_hex(out, &bytes, HOUSE_SIZE);
}

/// Byte-at-a-time stdin reader, so we consume exactly what `scanf` would.
struct ByteReader<R: Read> {
    inner: R,
    peeked: Option<u8>,
}

impl<R: Read> ByteReader<R> {
    fn new(inner: R) -> Self {
        ByteReader {
            inner,
            peeked: None,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        let mut b = [0u8; 1];
        match self.inner.read(&mut b) {
            Ok(1) => Some(b[0]),
            _ => None,
        }
    }

    /// Push a byte back, as `scanf` does with the character that terminated a
    /// conversion.
    fn unget(&mut self, b: u8) {
        self.peeked = Some(b);
    }
}

/// True for the characters `isspace()` treats as whitespace in the C locale;
/// `scanf`'s `%d` directive skips a run of these first.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// `scanf("%d", &x)`: returns `Some(value)` on a successful conversion and
/// `None` on input failure (EOF before any character) or matching failure, in
/// which case the C code leaves `x` untouched.
///
/// glibc accumulates the digits and converts with `strtol`, which saturates at
/// `LONG_MAX` / `LONG_MIN` on overflow, then stores the result through an
/// `int *`, truncating to 32 bits. Both effects are reproduced here.
fn scanf_d<R: Read>(r: &mut ByteReader<R>) -> Option<i32> {
    // Skip leading whitespace.
    let mut cur = loop {
        match r.next_byte() {
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
            None => return None, // input failure
        }
    };

    // Optional sign.
    let negative = match cur {
        b'-' => {
            negative_sign_seen(&mut cur, r)?;
            true
        }
        b'+' => {
            negative_sign_seen(&mut cur, r)?;
            false
        }
        _ => false,
    };

    // At least one digit is required, otherwise this is a matching failure.
    if !cur.is_ascii_digit() {
        r.unget(cur);
        return None;
    }

    // Accumulate as C `long` (64-bit here) with strtol-style saturation.
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = i64::from(cur - b'0');
        if !saturated {
            match acc
                .checked_mul(10)
                .and_then(|v| v.checked_add(if negative { -digit } else { digit }))
            {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }

        match r.next_byte() {
            Some(b) if b.is_ascii_digit() => cur = b,
            Some(b) => {
                r.unget(b);
                break;
            }
            None => break,
        }
    }

    if saturated {
        acc = if negative { i64::MIN } else { i64::MAX };
    }

    // Store through `int *`: truncate the long to 32 bits.
    Some(acc as i32)
}

/// Consume the character after a sign; EOF right after a sign is a matching
/// failure in glibc (nothing is stored).
fn negative_sign_seen<R: Read>(cur: &mut u8, r: &mut ByteReader<R>) -> Option<()> {
    match r.next_byte() {
        Some(b) => {
            *cur = b;
            Some(())
        }
        None => None,
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut reader = ByteReader::new(stdin.lock());

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    // int x = 0;
    let mut x: i32 = 0;
    // scanf("%d", &x);  -- x keeps its value if the conversion fails
    if let Some(v) = scanf_d(&mut reader) {
        x = v;
    }
    // driver(x);
    driver(&mut out, x);

    let _ = out.flush();
    // return 0;
}
