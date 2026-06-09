// Translated from c_src/src/driver.c
// Preserves the original library's behavior; provides a thin main that
// reads stdin and invokes `driver` exactly once, mirroring the C library's
// public entry point.

use std::io::{self, Read, Write};

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

// Mirror of the C `static house_t the_house` initial state.
fn initial_house() -> House {
    House {
        floors: 2,
        bedrooms: 5,
        bathrooms: 2.5,
    }
}

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house(house: &mut House) {
    add_floor(house);
}

fn print_the_house(house: &House, out: &mut impl Write) {
    // The C code uses %.1f which prints exactly one fractional digit using
    // round-half-to-even semantics. Rust's default {:.1} formatting matches
    // glibc's behavior for the values produced by this program (2.5 -> "2.5",
    // 3.5 -> "3.5"), so we use that here.
    let _ = writeln!(
        out,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run(house: &mut House, extra_bedrooms: i32, out: &mut impl Write) {
    print_the_house(house, out);
    add_floor_to_the_house(house);
    print_the_house(house, out);
    house.bathrooms += 1.0;
    print_the_house(house, out);
    add_bedrooms(house, extra_bedrooms);
    print_the_house(house, out);
}

/// Mimic of C's `strtol(str, &endp, 10)` followed by the int range check used
/// by `parse_val` in the original C source. Returns `Some(v)` on success and
/// `None` otherwise (matching the C function's boolean return).
fn parse_val(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i: usize = 0;

    // strtol skips leading isspace() characters.
    while i < bytes.len() && is_c_space(bytes[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        negative = bytes[i] == b'-';
        i += 1;
    }

    // Parse decimal digits, tracking overflow as strtol does (sets errno=ERANGE).
    let digit_start = i;
    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as i64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    // If no digits were consumed, strtol leaves *endp == str => parse_val
    // returns false.
    if i == digit_start {
        return None;
    }

    if overflow {
        return None;
    }

    let value: i64 = if negative { -acc } else { acc };

    // The C check is `tmp >= INT_MIN && tmp <= INT_MAX` after assigning to a
    // long. On the platforms this code targets, that's the i32 range.
    if value < i32::MIN as i64 || value > i32::MAX as i64 {
        return None;
    }

    Some(value as i32)
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

fn driver(input: &str, house: &mut House, out: &mut impl Write) {
    if let Some(x) = parse_val(input) {
        run(house, x, out);
        run(house, x, out);
    } else {
        let _ = writeln!(out, "An error occurred");
    }
}

fn main() {
    // Read all of stdin (matches scanf-like behavior of consuming the full
    // input stream until EOF). strtol stops at the first non-digit, so any
    // trailing newline or whitespace is harmless.
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        // If stdin can't be read, treat as empty input.
        input.clear();
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut house = initial_house();
    driver(&input, &mut house, &mut out);
    let _ = out.flush();
}
