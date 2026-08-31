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

/// Mirrors:
///
/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
///
/// On the x86-64 SysV ABI (and every other mainstream 64-bit target) this lays
/// out as: `floors` at offset 0, `bedrooms` at offset 4, `bathrooms` at offset
/// 8, alignment 8, total size 16 — i.e. there are no padding holes. The C code
/// zero-initializes the struct with `= {0}` before assigning every field, so the
/// whole 16-byte image is fully determined.
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

/// Size of `house_t`, i.e. what `sizeof(house)` evaluates to in the C code.
const HOUSE_SIZE: usize = 16;

impl House {
    /// `house_t house = {0};` — the object starts out as all-zero bytes.
    fn zeroed() -> Self {
        House {
            floors: 0,
            bedrooms: 0,
            bathrooms: 0.0,
        }
    }

    /// Produces the raw object representation that the C code hands to
    /// `print_hex` via `(unsigned char *)&house`.
    ///
    /// This is done by starting from the zeroed image (matching `= {0}`) and
    /// writing each field's little-endian representation at its ABI offset,
    /// which avoids any `unsafe` transmute of the struct itself.
    fn to_object_representation(&self) -> [u8; HOUSE_SIZE] {
        let mut bytes = [0u8; HOUSE_SIZE];
        bytes[0..4].copy_from_slice(&self.floors.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.bedrooms.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.bathrooms.to_le_bytes());
        bytes
    }
}

/// `static void print_hex(unsigned char *p, int len)`
///
/// Prints each byte as `%02x`, then a single newline.
fn print_hex(out: &mut impl Write, p: &[u8]) {
    let mut buf = String::with_capacity(p.len() * 2 + 1);
    for &b in p {
        // "%02x": lowercase hex, zero padded to two digits.
        buf.push_str(&format!("{:02x}", b));
    }
    buf.push('\n');
    let _ = out.write_all(buf.as_bytes());
}

/// `void driver(int floors)`
fn driver(out: &mut impl Write, floors: i32) {
    let mut house = House::zeroed();
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;
    print_hex(out, &house.to_object_representation());
}

/// True for the characters `isspace()` accepts in the C locale; these are what
/// a `%d` conversion silently skips over (newlines included).
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates `scanf("%d", &x)` reading from the given buffer of stdin bytes.
///
/// Returns `Some(value)` when the conversion succeeds, or `None` on a matching
/// failure or end of input — in which case the C code leaves `x` untouched.
///
/// Behavioral notes that this reproduces:
/// * Leading whitespace, including newlines, is skipped, so the conversion
///   happily reads across line boundaries.
/// * An optional `+`/`-` sign may precede the digits; a sign with no following
///   digit is a matching failure.
/// * Only base-10 digits are consumed, so input like `0x10` yields `0`.
/// * glibc accumulates the digits into a `long` (saturating at `LONG_MAX` /
///   `LONG_MIN`, as `strtol` does) and then stores it through an `int *`,
///   truncating the low 32 bits. Out-of-range input is undefined behavior in
///   ISO C, but this is what the reference implementation actually does.
fn scanf_int(input: &[u8]) -> Option<i32> {
    let mut i = 0usize;

    while i < input.len() && is_c_space(input[i]) {
        i += 1;
    }
    if i >= input.len() {
        return None; // EOF before any conversion.
    }

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

    let digits_start = i;
    // Accumulate as a `long` (i64 on LP64) with strtol-style saturation.
    let mut magnitude: i64 = 0;
    let mut saturated = false;
    while i < input.len() && input[i].is_ascii_digit() {
        let digit = i64::from(input[i] - b'0');
        if !saturated {
            match magnitude
                .checked_mul(10)
                .and_then(|acc| acc.checked_add(digit))
            {
                Some(next) => magnitude = next,
                None => saturated = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        return None; // No digits: matching failure, `x` keeps its old value.
    }

    let as_long: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        magnitude.wrapping_neg()
    } else {
        magnitude
    };

    // Stored through an `int *`: keep the low 32 bits.
    Some(as_long as i32)
}

fn main() {
    let mut input = Vec::new();
    // `scanf` pulls from the stdin stream on demand; since this program performs
    // exactly one conversion and then exits, slurping the stream up front is
    // observationally equivalent.
    let _ = std::io::stdin().read_to_end(&mut input);

    // `int x = 0;` — the initial value survives a failed conversion.
    let mut x: i32 = 0;
    if let Some(value) = scanf_int(&input) {
        x = value;
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    driver(&mut out, x);
    let _ = out.flush();
}
