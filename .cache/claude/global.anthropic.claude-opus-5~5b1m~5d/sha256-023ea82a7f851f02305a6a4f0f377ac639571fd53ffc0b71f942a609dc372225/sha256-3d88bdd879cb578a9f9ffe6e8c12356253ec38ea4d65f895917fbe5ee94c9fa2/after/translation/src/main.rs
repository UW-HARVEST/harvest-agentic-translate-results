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

use std::cell::RefCell;
use std::io::{Read, Write};

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

thread_local! {
    // static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};
    static THE_HOUSE: RefCell<House> = RefCell::new(House {
        floors: 2,
        bedrooms: 5,
        bathrooms: 2.5,
    });
}

fn add_floor(house: &mut House) {
    // house->floors++;  (wrapping mirrors the typical hardware behavior of the
    // C signed-overflow case)
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // house->bedrooms += extra_bedrooms;
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn add_floor_to_the_house() {
    THE_HOUSE.with(|h| add_floor(&mut h.borrow_mut()));
}

fn print_the_house() {
    let house = THE_HOUSE.with(|h| *h.borrow());
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    THE_HOUSE.with(|h| h.borrow_mut().bathrooms += 1.0);
    print_the_house();
    THE_HOUSE.with(|h| add_bedrooms(&mut h.borrow_mut(), extra_bedrooms));
    print_the_house();
}

/// Emulates `strtol(str, &endp, 10)` combined with the C checks:
/// `endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX`.
///
/// `str` is the NUL-terminated C string content (without the terminator).
fn parse_val(str_bytes: &[u8]) -> Option<i32> {
    let mut i = 0usize;

    // strtol skips leading whitespace as recognized by isspace().
    while i < str_bytes.len()
        && matches!(str_bytes[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < str_bytes.len() && (str_bytes[i] == b'+' || str_bytes[i] == b'-') {
        negative = str_bytes[i] == b'-';
        i += 1;
    }

    // Digits (base 10).
    let digits_start = i;
    // Accumulate with saturation at the `long` (i64) boundaries; saturation
    // corresponds to strtol setting errno == ERANGE, which the caller rejects.
    let mut acc: i64 = 0;
    let mut range_error = false;
    while i < str_bytes.len() && str_bytes[i].is_ascii_digit() {
        let d = i64::from(str_bytes[i] - b'0');
        if !range_error {
            match acc.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(d)
                } else {
                    v.checked_add(d)
                }
            }) {
                Some(v) => acc = v,
                None => range_error = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: strtol stores nptr in *endp, so endp == str.
        return None;
    }

    if range_error {
        // errno == ERANGE
        return None;
    }

    // tmp >= INT_MIN && tmp <= INT_MAX
    if acc >= i64::from(i32::MIN) && acc <= i64::from(i32::MAX) {
        Some(acc as i32)
    } else {
        None
    }
}

/// Emulates `fgets(in, 100, stdin)` into a 100-byte buffer, returning the
/// resulting C-string contents (bytes up to, but not including, the first NUL).
fn read_line_fgets(size: usize) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    // fgets reads at most size-1 characters, stopping after a newline.
    while buf.len() + 1 < size {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    // The buffer was initialized to the empty string; if fgets read nothing it
    // stays empty. Otherwise the C string ends at the first embedded NUL.
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    buf.truncate(end);
    buf
}

fn main() {
    let input = read_line_fgets(100);
    match parse_val(&input) {
        Some(x) => {
            run(x);
            run(x);
        }
        None => {
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            let _ = write!(out, "An error occurred\n");
        }
    }
    let _ = std::io::stdout().flush();
}
