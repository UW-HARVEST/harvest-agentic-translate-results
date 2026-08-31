// Rust translation of c_src/src/main.c
//
// Original work:
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

use std::io::{self, Read, Write};

/// Mirrors `typedef struct { int floors; int bedrooms; double bathrooms; } house_t;`
#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

/// Mirrors the file-scope mutable global:
/// `static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};`
///
/// The C program mutates this global across both `run()` calls, so the state
/// must persist between them. Modelled here as a value threaded through the
/// helpers so that no `unsafe`/`static mut` is required.
const THE_HOUSE_INIT: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

/// `static void add_floor(house_t *house)`
fn add_floor(house: &mut House) {
    // C: house->floors++  (wraps in practice on overflow)
    house.floors = house.floors.wrapping_add(1);
}

/// `static void add_bedrooms(house_t *house, int extra_bedrooms)`
fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // C: house->bedrooms += extra_bedrooms
    // Signed overflow is UB in C but wraps on the usual targets; reproduce that.
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

/// `static void add_floor_to_the_house()`
fn add_floor_to_the_house(the_house: &mut House) {
    add_floor(the_house);
}

/// `static void print_the_house()`
///
/// C: printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
fn print_the_house(out: &mut impl Write, the_house: &House) {
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        the_house.floors,
        the_house.bedrooms,
        the_house.bathrooms
    );
}

/// `void run(int extra_bedrooms)`
fn run(out: &mut impl Write, the_house: &mut House, extra_bedrooms: i32) {
    print_the_house(out, the_house);
    add_floor_to_the_house(the_house);
    print_the_house(out, the_house);
    the_house.bathrooms += 1.0;
    print_the_house(out, the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_the_house(out, the_house);
}

/// Result of a `strtol(str, &endp, 10)` call.
struct StrtolResult {
    /// Return value of `strtol` (saturated to LONG_MIN/LONG_MAX on overflow).
    value: i64,
    /// Number of bytes consumed; 0 means `endp == str`.
    consumed: usize,
    /// True when `strtol` would have set `errno` to `ERANGE`.
    range_error: bool,
}

/// C `isspace()` for the default "C" locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Faithful `strtol(str, &endp, 10)` over a NUL-terminated byte buffer.
fn strtol_base10(buf: &[u8]) -> StrtolResult {
    let mut i = 0usize;
    // strtol operates on the C string, i.e. it stops at the first NUL byte.
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());

    // 1. Skip leading whitespace.
    while i < len && is_c_space(buf[i]) {
        i += 1;
    }

    // 2. Optional sign.
    let mut negative = false;
    if i < len && (buf[i] == b'+' || buf[i] == b'-') {
        negative = buf[i] == b'-';
        i += 1;
    }

    // 3. Digit sequence.
    let digits_start = i;
    let mut acc: i64 = 0;
    let mut range_error = false;
    while i < len && buf[i].is_ascii_digit() {
        let d = i64::from(buf[i] - b'0');
        if !range_error {
            // Accumulate the magnitude with respect to the signed bound so
            // that "-9223372036854775808" is representable, as in C.
            if negative {
                match acc.checked_mul(10).and_then(|v| v.checked_sub(d)) {
                    Some(v) => acc = v,
                    None => range_error = true,
                }
            } else {
                match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) => acc = v,
                    None => range_error = true,
                }
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: strtol returns 0 and sets endp back to str.
        return StrtolResult {
            value: 0,
            consumed: 0,
            range_error: false,
        };
    }

    let value = if range_error {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else {
        acc
    };

    StrtolResult {
        value,
        consumed: i,
        range_error,
    }
}

/// `static bool parse_val(const char *str, int *val)`
fn parse_val(str_bytes: &[u8]) -> Option<i32> {
    // errno = 0;
    // char *endp = (char *)str;
    // long tmp = strtol(str, &endp, 10);
    let r = strtol_base10(str_bytes);
    // errno is only ever set to ERANGE here, so `errno == 0` is `!range_error`.
    let errno_is_zero = !r.range_error;

    // if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX)
    if r.consumed != 0
        && errno_is_zero
        && r.value >= i64::from(i32::MIN)
        && r.value <= i64::from(i32::MAX)
    {
        Some(r.value as i32)
    } else {
        None
    }
}

/// `fgets(in, sizeof(in), stdin)` with `char in[100]`.
///
/// Reads at most 99 bytes, stopping after a newline (which is kept) or at EOF.
/// The buffer starts out as the empty C string, matching `char in[100] = ""`,
/// and is left untouched when fgets fails (immediate EOF).
fn fgets_stdin(capacity: usize) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    while buf.len() + 1 < capacity {
        match handle.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    // NUL terminator, as written by fgets.
    buf.push(0);
    buf
}

fn main() {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let mut the_house = THE_HOUSE_INIT;

    let input = fgets_stdin(100);

    match parse_val(&input) {
        Some(x) => {
            run(&mut out, &mut the_house, x);
            run(&mut out, &mut the_house, x);
        }
        None => {
            let _ = write!(out, "An error occurred\n");
        }
    }

    let _ = out.flush();
}
