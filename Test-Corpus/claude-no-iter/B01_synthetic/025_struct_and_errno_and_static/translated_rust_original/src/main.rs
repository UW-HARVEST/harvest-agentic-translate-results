// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.
//
// This program preserves the byte-identical output of the original C program.

use std::io::{self, Read, Write};

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

#[inline]
fn the_house() -> &'static mut House {
    // Single-threaded executable; this matches the C global.
    unsafe { &mut *std::ptr::addr_of_mut!(THE_HOUSE) }
}

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    add_floor(the_house());
}

fn print_the_house() {
    let h = the_house();
    // C: printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
    print!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        h.floors, h.bedrooms, h.bathrooms
    );
}

fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    the_house().bathrooms += 1.0;
    print_the_house();
    add_bedrooms(the_house(), extra_bedrooms);
    print_the_house();
}

/// Mimics the C `parse_val` using `strtol` semantics for base 10:
/// - skips leading whitespace
/// - accepts an optional `+` or `-` sign
/// - parses decimal digits
/// - returns Some(i32) only if at least one digit was consumed,
///   no overflow occurred, and the value fits in `[INT_MIN, INT_MAX]`.
fn parse_val(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;

    // Skip leading whitespace (C isspace in the "C" locale).
    while i < s.len()
        && matches!(
            s[i],
            b' ' | b'\t' | b'\n' | b'\r' | 0x0B /* \v */ | 0x0C /* \f */
        )
    {
        i += 1;
    }

    // Optional sign.
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut acc: i64 = 0;
    let mut overflowed_long = false;

    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        if !overflowed_long {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => overflowed_long = true,
            }
        }
        i += 1;
    }

    // If no digits were consumed, strtol leaves endp == str -> the C check fails.
    if i == digits_start {
        return None;
    }

    // strtol overflow -> errno == ERANGE -> C check fails.
    if overflowed_long {
        return None;
    }

    let val = if neg { -acc } else { acc };

    // Bounds check: tmp >= INT_MIN && tmp <= INT_MAX
    if val < i32::MIN as i64 || val > i32::MAX as i64 {
        return None;
    }

    Some(val as i32)
}

/// Mimics C's `fgets(in, 100, stdin)`:
/// reads up to 99 bytes, stopping after a newline (inclusive) or EOF.
fn fgets_99(stdin: &mut impl Read) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while buf.len() < 99 {
        match stdin.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    buf
}

fn main() {
    // C: char in[100] = ""; fgets(in, sizeof(in), stdin);
    // If fgets returns NULL (e.g. immediate EOF), `in` is unchanged (empty).
    let mut stdin = io::stdin();
    let buf = fgets_99(&mut stdin);

    match parse_val(&buf) {
        Some(x) => {
            run(x);
            run(x);
        }
        None => {
            print!("An error occurred\n");
        }
    }

    // Make sure stdout is flushed before exit.
    let _ = io::stdout().flush();
}
