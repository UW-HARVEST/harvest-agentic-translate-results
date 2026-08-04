// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust - byte-identical output to original C.

use std::cell::RefCell;
use std::io::{self, Read, Write};

struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

thread_local! {
    static THE_HOUSE: RefCell<House> = RefCell::new(House {
        floors: 2,
        bedrooms: 5,
        bathrooms: 2.5,
    });
}

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    THE_HOUSE.with(|h| add_floor(&mut h.borrow_mut()));
}

fn print_the_house() {
    THE_HOUSE.with(|h| {
        let h = h.borrow();
        // Match C printf "%.1f" formatting which always uses the C locale.
        print!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
            h.floors, h.bedrooms, h.bathrooms
        );
    });
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

/// Mimic C's strtol+range check behavior for parsing into an int.
/// Returns Some(val) if at least one numeric character was consumed,
/// no overflow occurred, and the result fits in i32.
fn parse_val(bytes: &[u8]) -> Option<i32> {
    let mut i = 0;

    // Skip leading whitespace as per C isspace().
    while i < bytes.len()
        && matches!(
            bytes[i],
            b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'
        )
    {
        i += 1;
    }

    // Optional sign.
    let negative;
    if i < bytes.len() && bytes[i] == b'-' {
        negative = true;
        i += 1;
    } else if i < bytes.len() && bytes[i] == b'+' {
        negative = false;
        i += 1;
    } else {
        negative = false;
    }

    let digits_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }

    if digits_start == i {
        // No digits consumed: endp == str in C, so parse_val returns false.
        return None;
    }

    // Accumulate digits, detecting overflow (mirrors errno=ERANGE in strtol).
    let mut result: i64 = 0;
    for j in digits_start..i {
        let d = (bytes[j] - b'0') as i64;
        result = match result.checked_mul(10).and_then(|r| r.checked_add(d)) {
            Some(r) => r,
            None => return None,
        };
    }

    let signed = if negative {
        match result.checked_neg() {
            Some(s) => s,
            None => return None,
        }
    } else {
        result
    };

    if signed >= i32::MIN as i64 && signed <= i32::MAX as i64 {
        Some(signed as i32)
    } else {
        None
    }
}

/// Mimic fgets(in, 100, stdin):
/// reads up to 99 bytes, stops after a newline (which is included), or EOF.
fn fgets_like(max_chars: usize) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    while buf.len() < max_chars {
        match handle.read(&mut byte) {
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
    // fgets reads at most sizeof-1 = 99 chars, then null-terminates.
    let input = fgets_like(99);

    let mut x: i32 = 0;
    let parsed = match parse_val(&input) {
        Some(v) => {
            x = v;
            true
        }
        None => false,
    };

    if parsed {
        run(x);
        run(x);
    } else {
        print!("An error occurred\n");
    }

    // Ensure all output is flushed before exit.
    let _ = io::stdout().flush();
}
