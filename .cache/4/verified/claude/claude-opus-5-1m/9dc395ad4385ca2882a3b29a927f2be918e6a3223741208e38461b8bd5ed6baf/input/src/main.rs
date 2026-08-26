// Translation of c_src/src/main.c to Rust.
//
// Original copyright notice from the C source:
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

/// Mirrors the C struct:
///
/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
///
/// On the x86-64 SysV ABI this occupies 16 bytes with no padding:
/// `floors` at offset 0, `bedrooms` at offset 4, `bathrooms` at offset 8.
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

impl House {
    /// Zero-initialized, equivalent to `house_t house = {0};`
    /// (GCC zeroes the full object, including any padding).
    fn zeroed() -> House {
        House {
            floors: 0,
            bedrooms: 0,
            bathrooms: 0.0,
        }
    }

    /// The raw object representation of the struct, exactly as
    /// `(unsigned char *)&house` would observe it on a little-endian
    /// x86-64 target.
    fn as_bytes(&self) -> [u8; 16] {
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(&self.floors.to_le_bytes());
        buf[4..8].copy_from_slice(&self.bedrooms.to_le_bytes());
        buf[8..16].copy_from_slice(&self.bathrooms.to_bits().to_le_bytes());
        buf
    }
}

/// `static void print_hex(unsigned char *p, int len)`
fn print_hex(p: &[u8], len: usize) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut s = String::with_capacity(len * 2 + 1);
    for i in 0..len {
        s.push_str(&format!("{:02x}", p[i]));
    }
    s.push('\n');
    let _ = out.write_all(s.as_bytes());
    let _ = out.flush();
}

/// `void driver(int floors)`
fn driver(floors: i32) {
    let mut house = House::zeroed();
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;
    let bytes = house.as_bytes();
    print_hex(&bytes, bytes.len());
}

/// Matches the C locale's `isspace()` for the characters `scanf` skips.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Reads one byte from stdin, returning `None` on EOF or error.
fn next_byte<R: Read>(r: &mut R) -> Option<u8> {
    let mut b = [0u8; 1];
    loop {
        match r.read(&mut b) {
            Ok(0) => return None,
            Ok(_) => return Some(b[0]),
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

/// Emulates `scanf("%d", &x)` from glibc.
///
/// Skips leading whitespace, accepts an optional sign followed by decimal
/// digits, and converts via a `long` accumulator that saturates at
/// `LONG_MAX`/`LONG_MIN` (as glibc's internal `strtol` does) before being
/// truncated to `int`. On a matching failure the destination is left
/// untouched, exactly like `scanf`.
fn scanf_d<R: Read>(r: &mut R, dst: &mut i32) -> i32 {
    // Skip leading whitespace.
    let mut c = loop {
        match next_byte(r) {
            None => return -1, // EOF before any conversion => returns EOF
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match next_byte(r) {
            None => return -1, // input failure / EOF after the sign
            Some(b) => c = b,
        }
    }

    if !c.is_ascii_digit() {
        // Matching failure: nothing is stored.
        return 0;
    }

    // Accumulate as `long`, saturating like glibc's strtol does on overflow.
    let mut acc: i64 = 0;
    let mut overflow = false;
    loop {
        let digit = (c - b'0') as i64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(digit)
                } else {
                    v.checked_add(digit)
                }
            }) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        match next_byte(r) {
            None => break,
            Some(b) if b.is_ascii_digit() => c = b,
            Some(_) => break, // non-digit terminates the conversion (pushed back)
        }
    }

    if overflow {
        acc = if negative { i64::MIN } else { i64::MAX };
    }

    // Stored as `int`: implementation-defined truncation of the `long` value.
    *dst = acc as i32;
    1
}

fn main() {
    let mut x: i32 = 0;
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let _ = scanf_d(&mut input, &mut x);
    driver(x);
}
