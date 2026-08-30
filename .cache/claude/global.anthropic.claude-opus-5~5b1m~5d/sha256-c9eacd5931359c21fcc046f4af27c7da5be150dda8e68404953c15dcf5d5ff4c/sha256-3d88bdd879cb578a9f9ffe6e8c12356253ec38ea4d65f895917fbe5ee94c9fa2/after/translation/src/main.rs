// Rust translation of c_src/src/main.c
//
// Copyright 2025 MIT Lincoln Laboratory (original C source)
//
// Behavior is intended to be byte-identical to the original C program.

use std::io::{Read, Write};

struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn add_floor(house: &mut House) {
    // house->floors++
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // house->bedrooms += extra_bedrooms
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

/// Emulates the C code:
///
/// ```c
/// errno = 0;
/// char *endp = (char *)str;
/// long tmp = strtol(str, &endp, 10);
/// if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) { ... }
/// ```
///
/// `str` is the NUL-terminated C string content (bytes before the first NUL).
fn parse_val(str_bytes: &[u8], val: &mut i32) -> bool {
    let (tmp, consumed, erange) = strtol10(str_bytes);
    // endp != str  <=>  strtol consumed at least one character
    // errno == 0   <=>  no ERANGE from strtol
    if consumed != 0 && !erange && tmp >= i32::MIN as i64 && tmp <= i32::MAX as i64 {
        *val = tmp as i32;
        true
    } else {
        false
    }
}

/// A base-10 `strtol` for 64-bit `long`.
///
/// Returns `(value, chars_consumed, erange)`. `chars_consumed` is 0 when no
/// conversion could be performed (in which case C sets `*endp == str`).
fn strtol10(s: &[u8]) -> (i64, usize, bool) {
    let mut i = 0usize;

    // Skip leading white space (C locale isspace).
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
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
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as u64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => {
                    acc = v;
                    let limit = if negative {
                        (i64::MAX as u64) + 1
                    } else {
                        i64::MAX as u64
                    };
                    if acc > limit {
                        overflow = true;
                    }
                }
                None => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits: no conversion performed, endp is set back to str.
        return (0, 0, false);
    }

    if overflow {
        // strtol sets ERANGE and returns LONG_MAX / LONG_MIN.
        let clamped = if negative { i64::MIN } else { i64::MAX };
        return (clamped, i, true);
    }

    let value = if negative {
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    };
    (value, i, false)
}

/// Emulates `fgets(in, 100, stdin)` over a `char in[100] = ""` buffer, and
/// returns the resulting C-string contents (bytes up to the first NUL).
fn fgets_line(size: usize) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut stdin = std::io::stdin();
    let mut byte = [0u8; 1];
    // fgets reads at most size-1 characters, stopping after a newline.
    while buf.len() + 1 < size {
        match stdin.read(&mut byte) {
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
    // The buffer was zero-initialized, so the effective C string ends at the
    // first NUL byte read (or at the end of what was read).
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    buf.truncate(end);
    buf
}

fn main() {
    let in_str = fgets_line(100);
    let mut x: i32 = 0;
    if parse_val(&in_str, &mut x) {
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
