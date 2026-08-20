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

//! Rust translation of `c_src/src/main.c`.
//!
//! The original program reads one line from stdin with `fgets` into a
//! `char[100]` buffer, parses it with `strtol` (wrapped in `parse_val`), and on
//! success calls `run()` twice against a single mutable global `house_t`.
//!
//! Behaviour that is faithfully reproduced here:
//!   * `fgets` semantics: at most `sizeof(in) - 1 == 99` bytes are consumed,
//!     stopping after the first `'\n'` (which is kept in the buffer). On an
//!     immediate EOF the buffer is left as the zero-initialised `""`.
//!   * The C string is NUL-terminated, so an embedded NUL byte truncates the
//!     value seen by `strtol`.
//!   * `strtol` semantics for base 10: leading whitespace is skipped, an
//!     optional sign is accepted, and `ERANGE` is reported when the value does
//!     not fit in a (64-bit) `long`.
//!   * `parse_val`'s exact acceptance order: a conversion must have happened
//!     (`endp != str`), `errno` must still be zero, and the value must lie
//!     within `[INT_MIN, INT_MAX]`.
//!   * The global `house_t` is *not* reset between the two `run()` calls, so the
//!     second call continues from the mutated state.
//!   * `bedrooms += extra_bedrooms` is signed-overflow UB in C; the generated
//!     code wraps, so `wrapping_add` is used to match.

use std::io::{Read, Write};

/// Mirrors the C `house_t` struct.
#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

/// `static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};`
const THE_HOUSE_INIT: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

/// `static void add_floor(house_t *house)`
fn add_floor(house: &mut House) {
    // `house->floors++` — wrapping matches the emitted C code on overflow.
    house.floors = house.floors.wrapping_add(1);
}

/// `static void add_bedrooms(house_t *house, int extra_bedrooms)`
fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // `house->bedrooms += extra_bedrooms`
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

/// `static void add_floor_to_the_house()`
fn add_floor_to_the_house(the_house: &mut House) {
    add_floor(the_house);
}

/// `static void print_the_house()`
///
/// `printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)`
fn print_the_house(out: &mut impl Write, the_house: &House) {
    // Every reachable `bathrooms` value here is an exactly representable
    // multiple of 0.5, so `%.1f` and Rust's `{:.1}` agree bit-for-bit.
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        the_house.floors, the_house.bedrooms, the_house.bathrooms
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

/// True for the characters C's `isspace()` accepts in the "C" locale, which is
/// the set of leading characters `strtol` skips.
fn c_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Result of an emulated base-10 `strtol` call.
struct StrtolResult {
    /// The converted value (saturated to `LONG_MIN`/`LONG_MAX` on `ERANGE`).
    value: i64,
    /// Number of bytes consumed; `0` means "no conversion", i.e. `endp == str`.
    consumed: usize,
    /// Whether `errno` would have been set to `ERANGE`.
    erange: bool,
}

/// Emulates `strtol(str, &endp, 10)` for a NUL-terminated C string given as the
/// bytes preceding the terminator.
fn strtol_base10(s: &[u8]) -> StrtolResult {
    let mut i = 0usize;

    // Skip leading whitespace.
    while i < s.len() && c_isspace(s[i]) {
        i += 1;
    }

    // Optional sign.
    let negative = match s.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };

    let digits_start = i;
    // Magnitude limit: |LONG_MIN| == 2^63 for negatives, LONG_MAX for positives.
    let limit: u64 = if negative {
        i64::MAX as u64 + 1
    } else {
        i64::MAX as u64
    };

    let mut acc: u64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = u64::from(s[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) if v <= limit => acc = v,
                _ => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: `strtol` performs no conversion, stores `nptr`
        // into `endptr` and returns 0. glibc does not set `errno` in this case.
        return StrtolResult {
            value: 0,
            consumed: 0,
            erange: false,
        };
    }

    if overflow {
        return StrtolResult {
            value: if negative { i64::MIN } else { i64::MAX },
            consumed: i,
            erange: true,
        };
    }

    let value = if negative {
        // Two's-complement negation, correct even for acc == 2^63.
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    };

    StrtolResult {
        value,
        consumed: i,
        erange: false,
    }
}

/// `static bool parse_val(const char *str, int *val)`
fn parse_val(str_bytes: &[u8]) -> Option<i32> {
    // errno = 0;
    let r = strtol_base10(str_bytes);

    // if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX)
    if r.consumed != 0
        && !r.erange
        && r.value >= i64::from(i32::MIN)
        && r.value <= i64::from(i32::MAX)
    {
        Some(r.value as i32)
    } else {
        None
    }
}

/// Emulates `fgets(in, 100, stdin)` over a `char in[100]` buffer that was
/// zero-initialised to `""`.
///
/// Returns the bytes stored in the buffer up to (but excluding) the first NUL
/// terminator — exactly what `strtol` would observe.
fn fgets_line(buf_size: usize) -> Vec<u8> {
    debug_assert!(buf_size > 0);
    let max_bytes = buf_size - 1; // Room for the NUL terminator.

    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut buf: Vec<u8> = Vec::with_capacity(max_bytes);
    let mut byte = [0u8; 1];

    while buf.len() < max_bytes {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break; // fgets keeps the newline and stops.
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    // The buffer is a C string: everything from the first NUL byte on is
    // invisible to `strtol`.
    if let Some(nul) = buf.iter().position(|&b| b == 0) {
        buf.truncate(nul);
    }

    buf
}

fn main() {
    // char in[100] = "";
    let input = fgets_line(100);

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    match parse_val(&input) {
        Some(x) => {
            let mut the_house = THE_HOUSE_INIT;
            run(&mut out, &mut the_house, x);
            run(&mut out, &mut the_house, x);
        }
        None => {
            let _ = write!(out, "An error occurred\n");
        }
    }

    let _ = out.flush();
    // return 0;
}
