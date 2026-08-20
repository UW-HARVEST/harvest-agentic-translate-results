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

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

/// Mirrors the C file-scope `static house_t the_house`.
static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

fn add_floor(house: &mut House) {
    // C: house->floors++
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // C: house->bedrooms += extra_bedrooms
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn the_house() -> &'static mut House {
    // Single-threaded program; equivalent to C's access of the global object.
    unsafe { &mut *std::ptr::addr_of_mut!(THE_HOUSE) }
}

fn add_floor_to_the_house() {
    add_floor(the_house());
}

fn print_the_house(out: &mut dyn Write) {
    let h = *the_house();
    // printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {} bathrooms\n",
        h.floors,
        h.bedrooms,
        format_f64_1(h.bathrooms)
    );
}

/// Formats a double the way C's `%.1f` does.
fn format_f64_1(v: f64) -> String {
    if v.is_nan() {
        // C prints "nan" / "-nan"
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
    // Rust's {:.1} matches glibc's round-half-to-even on the exact binary value.
    format!("{:.1}", v)
}

fn run(out: &mut dyn Write, extra_bedrooms: i32) {
    print_the_house(out);
    add_floor_to_the_house();
    print_the_house(out);
    the_house().bathrooms += 1.0;
    print_the_house(out);
    add_bedrooms(the_house(), extra_bedrooms);
    print_the_house(out);
}

/// Emulates `scanf("%d", &x)`: skips leading whitespace (including newlines),
/// accepts an optional sign, then decimal digits. On matching failure the
/// destination is left untouched.
fn scanf_i32(input: &[u8], pos: &mut usize) -> Option<i32> {
    let mut i = *pos;

    // Skip whitespace, as the %d conversion does.
    while i < input.len() && matches!(input[i], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') {
        i += 1;
    }
    if i >= input.len() {
        *pos = i;
        return None;
    }

    let start = i;
    let mut negative = false;
    if input[i] == b'+' || input[i] == b'-' {
        negative = input[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < input.len() && input[i].is_ascii_digit() {
        let d = i64::from(input[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|a| a.checked_add(d)) {
                Some(next) => acc = next,
                None => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: matching failure, nothing is stored.
        *pos = start;
        return None;
    }

    *pos = i;

    // glibc parses with strtol and stores the (possibly saturated) long
    // truncated to int.
    let value: i64 = if overflow {
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

    Some(value as i32)
}

fn main() {
    let mut input = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut input);
    let mut pos = 0usize;

    let mut x: i32 = 0;
    if let Some(v) = scanf_i32(&input, &mut pos) {
        x = v;
    }

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    run(&mut out, x);
    run(&mut out, x);
    let _ = out.flush();
}
