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

use std::io::{Read, Write};

/// Mirrors the C `house_t`:
///
/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
///
/// On the LP64 little-endian ABI the original program targets, this is 16 bytes
/// with `floors` at offset 0, `bedrooms` at offset 4, `bathrooms` at offset 8,
/// and no padding holes.
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

const HOUSE_SIZE: usize = 16;
const OFF_FLOORS: usize = 0;
const OFF_BEDROOMS: usize = 4;
const OFF_BATHROOMS: usize = 8;

impl House {
    /// `house_t house = {0};` — every byte, padding included, starts as zero.
    fn zeroed() -> Self {
        House {
            floors: 0,
            bedrooms: 0,
            bathrooms: 0.0,
        }
    }

    /// The object representation of the struct, i.e. what `memcpy(raw, &house,
    /// sizeof(house))` copies out.
    fn to_object_repr(&self) -> [u8; HOUSE_SIZE] {
        let mut raw = [0u8; HOUSE_SIZE];
        raw[OFF_FLOORS..OFF_FLOORS + 4].copy_from_slice(&self.floors.to_le_bytes());
        raw[OFF_BEDROOMS..OFF_BEDROOMS + 4].copy_from_slice(&self.bedrooms.to_le_bytes());
        raw[OFF_BATHROOMS..OFF_BATHROOMS + 8].copy_from_slice(&self.bathrooms.to_le_bytes());
        raw
    }
}

/// `static void print_hex(unsigned char *p, int len)`
fn print_hex(out: &mut impl Write, p: &[u8], len: usize) {
    for i in 0..len {
        let _ = write!(out, "{:02x}", p[i]);
    }
    let _ = writeln!(out);
}

/// `void driver(int floors)`
fn driver(out: &mut impl Write, floors: i32) {
    let mut house = House::zeroed();
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    // char raw[sizeof(house)]; memcpy(raw, &house, sizeof(house));
    let raw = house.to_object_repr();
    print_hex(out, &raw, raw.len());
}

/// Byte-at-a-time stdin reader, so parsing consumes exactly the characters that
/// C's `scanf` would consume (and freely crosses newlines).
struct StdinBytes {
    inner: std::io::Stdin,
    peeked: Option<u8>,
}

impl StdinBytes {
    fn new() -> Self {
        StdinBytes {
            inner: std::io::stdin(),
            peeked: None,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
                Ok(0) => return None,
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    fn unread(&mut self, b: u8) {
        self.peeked = Some(b);
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `scanf("%d", &x)`: returns `Some(value)` on a successful conversion, `None`
/// on matching failure or EOF (in which case the C code leaves `x` untouched).
///
/// Out-of-range input reproduces glibc's behaviour: the value saturates to
/// `LONG_MAX`/`LONG_MIN` (64-bit `long`) and is then truncated to `int`.
fn scanf_d(input: &mut StdinBytes) -> Option<i32> {
    // Leading whitespace is skipped, including across newlines.
    let mut b = loop {
        let b = input.next_byte()?;
        if !is_c_space(b) {
            break b;
        }
    };

    let mut negative = false;
    if b == b'+' || b == b'-' {
        negative = b == b'-';
        match input.next_byte() {
            Some(nb) => b = nb,
            None => return None, // sign then EOF: matching failure
        }
    }

    if !b.is_ascii_digit() {
        // Matching failure; the offending character is pushed back.
        input.unread(b);
        return None;
    }

    // Accumulate the magnitude, clamping so it stays bounded.
    const CLAMP: u128 = u64::MAX as u128 + 1;
    let mut mag: u128 = 0;
    loop {
        mag = mag * 10 + u128::from(b - b'0');
        if mag > CLAMP {
            mag = CLAMP;
        }
        match input.next_byte() {
            Some(nb) if nb.is_ascii_digit() => b = nb,
            Some(nb) => {
                input.unread(nb);
                break;
            }
            None => break,
        }
    }

    let as_long: i64 = if negative {
        if mag > (i64::MAX as u128) + 1 {
            i64::MIN
        } else {
            (-(mag as i128)) as i64
        }
    } else if mag > i64::MAX as u128 {
        i64::MAX
    } else {
        mag as i64
    };

    // *ARG(int *) = num.l;  /* narrowing conversion */
    Some(as_long as i32)
}

fn main() {
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    let mut x: i32 = 0;
    let mut input = StdinBytes::new();
    if let Some(v) = scanf_d(&mut input) {
        x = v;
    }
    driver(&mut out, x);

    let _ = out.flush();
}
