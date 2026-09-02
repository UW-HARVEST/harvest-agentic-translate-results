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

/// Mirrors `house_t` from the C source.
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn add_floor(house: &mut House) {
    // C: house->floors++ (wraps in practice on overflow)
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // C: house->bedrooms += extra_bedrooms (wraps in practice on overflow)
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn print_house(house: &House) {
    // C: printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
    print!(
        "The house has {} floors, {} bedrooms, and {} bathrooms\n",
        house.floors,
        house.bedrooms,
        format_f1(house.bathrooms)
    );
}

/// Formats a double the way C's `%.1f` does.
fn format_f1(v: f64) -> String {
    if v.is_nan() {
        // glibc prints "nan" / "-nan" depending on the sign bit.
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
    let s = format!("{:.1}", v);
    // Rust prints "-0.0" for negative zero, as does glibc's %.1f; both agree.
    s
}

fn run(the_house: &mut House, extra_bedrooms: i32) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    the_house.bathrooms += 1.0;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

/// Result of a `strtol(str, &endp, 10)` call on a 64-bit `long` platform.
struct StrtolResult {
    value: i64,
    /// Offset of `endp` relative to the start of the string. `0` means no
    /// conversion was performed (i.e. `endp == str`).
    end_offset: usize,
    /// Whether ERANGE would have been stored in `errno`.
    erange: bool,
}

fn c_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Faithful emulation of `strtol(s, &endp, 10)` for a NUL-terminated C string
/// whose bytes (excluding the terminator) are given by `s`.
fn strtol_base10(s: &[u8]) -> StrtolResult {
    let mut i = 0usize;

    // Skip leading whitespace.
    while i < s.len() && c_isspace(s[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut acc: u128 = 0;
    let mut overflow = false;
    // Magnitude limits for a 64-bit long.
    let limit: u128 = if negative {
        i64::MAX as u128 + 1
    } else {
        i64::MAX as u128
    };

    while i < s.len() && s[i].is_ascii_digit() {
        let d = u128::from(s[i] - b'0');
        if !overflow {
            acc = acc * 10 + d;
            if acc > limit {
                overflow = true;
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: strtol sets *endptr = str and returns 0.
        return StrtolResult {
            value: 0,
            end_offset: 0,
            erange: false,
        };
    }

    let value = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // acc <= 2^63, so this negation is exact.
        (acc as i128).wrapping_neg() as i64
    } else {
        acc as i64
    };

    StrtolResult {
        value,
        end_offset: i,
        erange: overflow,
    }
}

/// Mirrors `parse_val` from the C source.
fn parse_val(s: &[u8], val: &mut i32) -> bool {
    // errno = 0;
    let r = strtol_base10(s);
    // if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX)
    if r.end_offset != 0
        && !r.erange
        && r.value >= i64::from(i32::MIN)
        && r.value <= i64::from(i32::MAX)
    {
        *val = r.value as i32;
        true
    } else {
        false
    }
}

/// Mirrors `fgets(in, size, stdin)` where `in` is a `char[size]` pre-filled
/// with NUL bytes. Returns the resulting C string contents (bytes up to, but
/// not including, the terminating NUL).
fn fgets_stdin(size: usize) -> Vec<u8> {
    let max = size - 1; // room for the terminating NUL
    let mut out: Vec<u8> = Vec::new();
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    while out.len() < max {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                out.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    // The C buffer is NUL-terminated, so the effective string stops at the
    // first embedded NUL byte (if any). On fgets failure the buffer keeps its
    // initial "" value, which is the same as an empty result here.
    let end = out.iter().position(|&b| b == 0).unwrap_or(out.len());
    out.truncate(end);
    out
}

fn main() {
    let input = fgets_stdin(100);
    let mut x: i32 = 0;
    if parse_val(&input, &mut x) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        print!("An error occurred\n");
    }
    let _ = std::io::stdout().flush();
}
