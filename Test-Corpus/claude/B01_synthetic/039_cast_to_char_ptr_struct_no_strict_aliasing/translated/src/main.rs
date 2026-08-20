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

//! Faithful Rust translation of `c_src/src/main.c`.
//!
//! The C program reads a single integer with `scanf("%d", &x)` and then dumps the
//! raw little-endian byte representation of a `house_t` struct as lowercase hex.

use std::io::{Read, Write};

/// Mirror of the C `house_t`:
///
/// ```c
/// typedef struct {
///     int floors;      // offset 0, 4 bytes
///     int bedrooms;    // offset 4, 4 bytes
///     double bathrooms;// offset 8, 8 bytes
/// } house_t;           // sizeof == 16, alignof == 8
/// ```
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

/// Size of `house_t` on the LP64 x86-64 ABI targeted by the original program.
const HOUSE_SIZE: usize = 16;

impl House {
    /// Produce the exact `memcpy` image of the struct (System V x86-64, little
    /// endian). The C code zero-initializes the struct with `= {0}` before
    /// assigning every field, so any padding bytes are deterministically zero.
    /// (Here there is in fact no padding: 4 + 4 + 8 == 16.)
    fn to_raw_bytes(&self) -> [u8; HOUSE_SIZE] {
        let mut raw = [0u8; HOUSE_SIZE];
        raw[0..4].copy_from_slice(&self.floors.to_le_bytes());
        raw[4..8].copy_from_slice(&self.bedrooms.to_le_bytes());
        raw[8..16].copy_from_slice(&self.bathrooms.to_le_bytes());
        raw
    }
}

/// Equivalent of the C `print_hex`: `printf("%02x", ...)` per byte, then `"\n"`.
fn print_hex(out: &mut dyn Write, p: &[u8]) {
    let mut line = String::with_capacity(p.len() * 2 + 1);
    for &b in p {
        // "%02x" -> lowercase, zero padded to two digits.
        line.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        line.push(char::from_digit((b & 0x0f) as u32, 16).unwrap());
    }
    line.push('\n');
    let _ = out.write_all(line.as_bytes());
}

/// Equivalent of the C `driver` function.
fn driver(out: &mut dyn Write, floors: i32) {
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let raw = house.to_raw_bytes();
    print_hex(out, &raw);
}

/// True for the characters `isspace()` accepts in the C locale; `scanf`
/// conversions skip these before a `%d` directive.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Emulates a single `scanf("%d", &x)` directive against a byte buffer.
///
/// Returns `Some(value)` when the conversion succeeds, `None` on matching
/// failure or EOF (in which case the C variable keeps its previous value).
///
/// Overflow reproduces glibc's behaviour: the digits are accumulated as a
/// `long` (64-bit here) which saturates at `LONG_MAX` / `LONG_MIN`, and the
/// result is then truncated when stored into an `int`. That is why the C
/// program prints `ffffffff` for `99999999999999999999`.
fn scanf_i32(input: &[u8]) -> Option<i32> {
    let mut i = 0usize;

    // Skip leading whitespace (this may cross newlines, matching scanf).
    while i < input.len() && is_c_space(input[i]) {
        i += 1;
    }
    if i >= input.len() {
        return None; // input failure (EOF)
    }

    // Optional sign.
    let negative = match input[i] {
        b'-' => {
            i += 1;
            true
        }
        b'+' => {
            i += 1;
            false
        }
        _ => false,
    };

    // At least one decimal digit is required.
    let start = i;
    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < input.len() && input[i].is_ascii_digit() {
        let digit = i64::from(input[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }
    if i == start {
        return None; // matching failure: no digits consumed
    }

    let as_long: i64 = if overflow {
        // glibc: strtol clamps and sets ERANGE.
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        acc.wrapping_neg()
    } else {
        acc
    };

    // Storing a `long` into an `int` truncates to the low 32 bits.
    Some(as_long as i32)
}

fn main() {
    // The C program performs exactly one `scanf` and then exits, so slurping
    // stdin is indistinguishable from incremental reads.
    let mut input = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut input);

    // `int x = 0;` — left untouched if the conversion fails.
    let x: i32 = scanf_i32(&input).unwrap_or(0);

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    driver(&mut out, x);
    let _ = out.flush();
}
