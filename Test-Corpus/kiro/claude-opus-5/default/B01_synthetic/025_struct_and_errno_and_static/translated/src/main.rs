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

use std::io::{self, Read, Write};

/// C: `typedef struct { int floors; int bedrooms; double bathrooms; } house_t;`
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

/// C: `static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};`
static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

/// C: `static void add_floor(house_t *house) { house->floors++; }`
///
/// `int` increment; C signed overflow is undefined behavior but wraps in
/// practice on the target platform, so wrapping arithmetic is used.
fn add_floor(house: &mut House) {
    house.floors = house.floors.wrapping_add(1);
}

/// C: `static void add_bedrooms(house_t *house, int extra_bedrooms)`
fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

/// C: `static void add_floor_to_the_house() { add_floor(&the_house); }`
fn add_floor_to_the_house() {
    add_floor(the_house());
}

/// C: `printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)`
fn print_the_house(out: &mut impl Write) {
    let house = the_house();
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {} bathrooms\n",
        house.floors,
        house.bedrooms,
        format_f1(house.bathrooms)
    );
}

/// C: `void run(int extra_bedrooms)`
fn run(out: &mut impl Write, extra_bedrooms: i32) {
    print_the_house(out);
    add_floor_to_the_house();
    print_the_house(out);
    the_house().bathrooms += 1.0;
    print_the_house(out);
    add_bedrooms(the_house(), extra_bedrooms);
    print_the_house(out);
}

/// Access to the single global `the_house` instance. The program is
/// single-threaded, mirroring the C original.
#[allow(static_mut_refs)]
fn the_house() -> &'static mut House {
    unsafe { &mut THE_HOUSE }
}

/// `%.1f` formatting as glibc's printf performs it: round-to-nearest on the
/// exact binary value of the double, ties resolved to even.
fn format_f1(v: f64) -> String {
    if v.is_nan() {
        // glibc prints "nan" / "-nan"
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
    format!("{:.1}", v)
}

/// Result of the `strtol` emulation.
struct StrtolResult {
    value: i64,
    /// Number of bytes consumed; 0 means `endptr == nptr` (no conversion).
    consumed: usize,
    /// Whether ERANGE would have been set.
    range_error: bool,
}

/// C locale `isspace`.
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Emulation of `strtol(str, &endp, 10)` for a NUL-terminated C string given as
/// a byte slice without the terminator. `long` is 64-bit on the target.
fn strtol_base10(s: &[u8]) -> StrtolResult {
    let mut i = 0usize;

    // Skip leading whitespace.
    while i < s.len() && is_space(s[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut acc: u64 = 0;
    let mut overflow = false;

    // Positive magnitude limit is i64::MAX; negative is i64::MAX + 1.
    let limit: u64 = if negative {
        i64::MAX as u64 + 1
    } else {
        i64::MAX as u64
    };

    while i < s.len() && s[i].is_ascii_digit() {
        let d = u64::from(s[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|a| a.checked_add(d)) {
                Some(next) if next <= limit => acc = next,
                _ => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits: no conversion performed, endptr is set back to nptr.
        return StrtolResult {
            value: 0,
            consumed: 0,
            range_error: false,
        };
    }

    if overflow {
        return StrtolResult {
            value: if negative { i64::MIN } else { i64::MAX },
            consumed: i,
            range_error: true,
        };
    }

    let value = if negative {
        (acc as i128 * -1) as i64
    } else {
        acc as i64
    };

    StrtolResult {
        value,
        consumed: i,
        range_error: false,
    }
}

/// C: `static bool parse_val(const char *str, int *val)`
fn parse_val(s: &[u8], val: &mut i32) -> bool {
    // errno = 0;
    let mut errno_erange = false;
    let r = strtol_base10(s);
    if r.range_error {
        errno_erange = true;
    }
    let tmp = r.value;
    // endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX
    if r.consumed != 0
        && !errno_erange
        && tmp >= i64::from(i32::MIN)
        && tmp <= i64::from(i32::MAX)
    {
        *val = tmp as i32;
        true
    } else {
        false
    }
}

/// C: `fgets(in, sizeof(in), stdin)` with `sizeof(in) == 100`.
///
/// Reads at most `size - 1` bytes, stopping after a newline (which is kept) or
/// at EOF. Returns the bytes read; an empty vector corresponds to fgets
/// returning NULL and leaving the buffer untouched (it is pre-set to "").
fn fgets(size: usize) -> Vec<u8> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut out: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while out.len() + 1 < size {
        match handle.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                out.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    out
}

fn main() {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    // char in[100] = "";
    let raw = fgets(100);
    // The buffer is used as a C string, so it terminates at the first NUL byte.
    let in_str: &[u8] = match raw.iter().position(|&b| b == 0) {
        Some(p) => &raw[..p],
        None => &raw[..],
    };

    let mut x: i32 = 0;
    if parse_val(in_str, &mut x) {
        run(&mut out, x);
        run(&mut out, x);
    } else {
        let _ = write!(out, "An error occurred\n");
    }

    let _ = out.flush();
}
