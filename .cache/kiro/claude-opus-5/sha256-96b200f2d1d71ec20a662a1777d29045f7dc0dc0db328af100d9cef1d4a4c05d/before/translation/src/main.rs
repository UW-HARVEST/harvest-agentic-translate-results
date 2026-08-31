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

/// Mirrors the C `house_t` struct.
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn add_floor(house: &mut House) {
    // C: house->floors++  (wraps in practice on overflow)
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // C: house->bedrooms += extra_bedrooms  (wraps in practice on overflow)
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn print_house(house: &House) {
    // printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
    print!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        house.floors, house.bedrooms, house.bathrooms
    );
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

/// Result of an emulated `strtol(str, &endp, 10)` call.
struct StrtolResult {
    value: i64,
    /// Number of bytes consumed; 0 means `endp == str` (no conversion).
    consumed: usize,
    /// True when C would have set `errno` to `ERANGE`.
    erange: bool,
}

/// C `isspace` for the default "C" locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Faithful emulation of glibc `strtol(str, &endp, 10)` on a 64-bit `long`.
fn strtol_base10(s: &[u8]) -> StrtolResult {
    let mut i = 0usize;

    // Skip leading whitespace.
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    // Digit sequence.
    let digits_start = i;
    let mut acc: u64 = 0;
    let mut erange = false;
    // Magnitude limit: LONG_MAX for positives, |LONG_MIN| for negatives.
    let limit: u64 = if negative {
        (i64::MIN as i128).unsigned_abs() as u64
    } else {
        i64::MAX as u64
    };

    while i < s.len() && s[i].is_ascii_digit() {
        let digit = (s[i] - b'0') as u64;
        if !erange {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) if v <= limit => acc = v,
                _ => erange = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: strtol returns 0 and leaves endp == str.
        return StrtolResult {
            value: 0,
            consumed: 0,
            erange: false,
        };
    }

    let value = if erange {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        (acc as i128).wrapping_neg() as i64
    } else {
        acc as i64
    };

    StrtolResult {
        value,
        consumed: i,
        erange,
    }
}

/// Mirrors the C `parse_val`: errno is cleared, strtol runs, and the value is
/// accepted only when some characters were consumed, errno stayed 0, and the
/// result fits in an `int`.
fn parse_val(s: &[u8], val: &mut i32) -> bool {
    // errno = 0;
    let r = strtol_base10(s);
    let errno_is_zero = !r.erange;
    if r.consumed != 0
        && errno_is_zero
        && r.value >= i32::MIN as i64
        && r.value <= i32::MAX as i64
    {
        *val = r.value as i32; // C: *val = tmp;
        true
    } else {
        false
    }
}

/// Emulates `fgets(buf, 100, stdin)`: reads at most 99 bytes, stops after a
/// newline (which is kept), and NUL-terminates. Unlike scanf it never reads
/// past the end of the line. On immediate EOF the buffer is left untouched.
fn fgets_stdin(capacity: usize) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    while out.len() + 1 < capacity {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                out.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    out
}

fn main() {
    // char in[100] = "";
    let raw = fgets_stdin(100);
    // The C code treats `in` as a NUL-terminated string, so anything at or
    // after an embedded NUL byte is invisible to strtol.
    let in_str: &[u8] = match raw.iter().position(|&b| b == 0) {
        Some(pos) => &raw[..pos],
        None => &raw[..],
    };

    let mut x: i32 = 0;
    if parse_val(in_str, &mut x) {
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
