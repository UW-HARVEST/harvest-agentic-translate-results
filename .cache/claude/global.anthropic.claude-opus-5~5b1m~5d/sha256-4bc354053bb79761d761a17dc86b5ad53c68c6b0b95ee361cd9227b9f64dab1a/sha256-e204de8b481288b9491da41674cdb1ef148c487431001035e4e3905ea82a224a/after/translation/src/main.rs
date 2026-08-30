// Rust translation of c_src/src/main.c
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

/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
///
/// On the x86-64 SysV ABI (the platform the C program targets) this struct has
/// size 16 and the following layout, with no padding bytes:
///   offset 0: `floors`    (int,    4 bytes, little endian)
///   offset 4: `bedrooms`  (int,    4 bytes, little endian)
///   offset 8: `bathrooms` (double, 8 bytes, little endian IEEE-754)
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

const HOUSE_SIZE: usize = 16;

impl House {
    /// `house_t house = {0};` — every member (and therefore, here, every byte)
    /// is zero-initialized.
    fn zeroed() -> House {
        House {
            floors: 0,
            bedrooms: 0,
            bathrooms: 0.0,
        }
    }

    /// Reproduces the object representation that
    /// `(unsigned char *)&house` walks over in the C code.
    fn to_object_representation(&self) -> [u8; HOUSE_SIZE] {
        let mut bytes = [0u8; HOUSE_SIZE];
        bytes[0..4].copy_from_slice(&self.floors.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.bedrooms.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.bathrooms.to_le_bytes());
        bytes
    }
}

/// ```c
/// static void print_hex(unsigned char *p, int len) {
///     for (int i = 0; i < len; i++) {
///         printf("%02x", p[i]);
///     }
///     printf("\n");
/// }
/// ```
fn print_hex(p: &[u8], len: usize) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut s = String::new();
    for i in 0..len {
        s.push_str(&format!("{:02x}", p[i]));
    }
    s.push('\n');
    let _ = out.write_all(s.as_bytes());
    let _ = out.flush();
}

/// ```c
/// void driver(int floors) { ... }
/// ```
fn driver(floors: i32) {
    let mut house = House::zeroed();
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;
    let bytes = house.to_object_representation();
    print_hex(&bytes, HOUSE_SIZE);
}

/// Mimics C's `isspace()` for the "C" locale, as used by `scanf` when skipping
/// leading whitespace for the `%d` conversion.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Emulates `scanf("%d", &x)`.
///
/// Returns `Some(value)` on a successful conversion (i.e. `scanf` returned 1),
/// or `None` on matching failure / EOF, in which case the caller must leave its
/// variable untouched — exactly like C.
///
/// `%d` skips any amount of leading whitespace (including newlines), then reads
/// an optional sign followed by decimal digits, stopping at the first character
/// that cannot be part of the number. Out-of-range values are saturated to the
/// platform `long` range (glibc's strtol behavior) and then truncated on
/// assignment to `int`.
fn scanf_d<R: Read>(input: &mut R) -> Option<i32> {
    let mut byte = [0u8; 1];

    // Read one byte, or None at EOF / on error.
    let mut next = move |input: &mut R| -> Option<u8> {
        match input.read(&mut byte) {
            Ok(1) => Some(byte[0]),
            _ => None,
        }
    };

    // Skip leading whitespace.
    let mut c = loop {
        match next(input) {
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
            None => return None, // input failure before any conversion
        }
    };

    // Optional sign.
    let negative = match c {
        b'-' => {
            c = next(input)?;
            true
        }
        b'+' => {
            c = next(input)?;
            false
        }
        _ => false,
    };

    // At least one digit is required, otherwise it is a matching failure.
    if !c.is_ascii_digit() {
        return None;
    }

    let mut acc: i64 = 0;
    let mut saturated = false;
    let mut cur = Some(c);
    while let Some(d) = cur {
        if !d.is_ascii_digit() {
            break; // the offending character is "pushed back" by scanf
        }
        let digit = i64::from(d - b'0');
        if !saturated {
            match acc.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(digit)
                } else {
                    v.checked_add(digit)
                }
            }) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        cur = next(input);
    }

    if saturated {
        acc = if negative { i64::MIN } else { i64::MAX };
    }

    // Assignment of a `long` to an `int`: implementation-defined truncation.
    Some(acc as i32)
}

fn main() {
    let mut x: i32 = 0;
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    if let Some(v) = scanf_d(&mut handle) {
        x = v;
    }
    driver(x);
}
