// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust preserving exact behavior.

use std::ffi::c_int;
use std::io::{self, Read, Write};
use std::sync::Mutex;

#[derive(Clone, Copy)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

static THE_HOUSE: Mutex<House> = Mutex::new(House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
});

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    let mut h = THE_HOUSE.lock().unwrap();
    add_floor(&mut *h);
}

/// Format a double with %.1f the way C printf does.
/// C uses round-half-to-even (banker's rounding) by default in glibc when
/// FLT_ROUNDS is round-to-nearest. Rust's `{:.1}` formatting matches this
/// behavior for the values used in this program.
fn format_f1(v: f64) -> String {
    format!("{:.1}", v)
}

fn print_the_house() {
    let h = THE_HOUSE.lock().unwrap();
    let s = format!(
        "The house has {} floors, {} bedrooms, and {} bathrooms\n",
        h.floors,
        h.bedrooms,
        format_f1(h.bathrooms)
    );
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(s.as_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    {
        let mut h = THE_HOUSE.lock().unwrap();
        h.bathrooms += 1.0;
    }
    print_the_house();
    {
        let mut h = THE_HOUSE.lock().unwrap();
        add_bedrooms(&mut *h, extra_bedrooms);
    }
    print_the_house();
}

/// Mimic C's `strtol` parsing semantics for base 10:
/// - skip leading whitespace
/// - optional sign
/// - parse digits
/// Returns (value as i64, number of bytes consumed from start, overflow flag).
fn strtol_base10(bytes: &[u8]) -> (i64, usize, bool) {
    let mut i = 0usize;
    // Skip whitespace as isspace() does (space, \t, \n, \v, \f, \r)
    while i < bytes.len() {
        let b = bytes[i];
        if b == b' ' || b == b'\t' || b == b'\n' || b == 0x0b || b == 0x0c || b == b'\r' {
            i += 1;
        } else {
            break;
        }
    }

    let start_digits = i;
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        negative = bytes[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut overflow = false;
    let mut acc: i64 = 0;
    let cutoff: i64 = if negative { i64::MIN } else { i64::MAX };
    let cutlim = (cutoff % 10).abs();
    let cutoff_div = cutoff / 10;

    while i < bytes.len() {
        let b = bytes[i];
        if !b.is_ascii_digit() {
            break;
        }
        let d = (b - b'0') as i64;
        if !overflow {
            if negative {
                // For negative, accumulating as negative: acc = acc*10 - d
                if acc < cutoff_div || (acc == cutoff_div && d > cutlim) {
                    overflow = true;
                    acc = i64::MIN;
                } else {
                    acc = acc * 10 - d;
                }
            } else {
                if acc > cutoff_div || (acc == cutoff_div && d > cutlim) {
                    overflow = true;
                    acc = i64::MAX;
                } else {
                    acc = acc * 10 + d;
                }
            }
        }
        i += 1;
    }

    // If no digits were consumed, endp should be set to the original string (i.e. consumed = 0).
    if i == digits_start {
        return (0, 0, false);
    }
    let _ = start_digits;
    (acc, i, overflow)
}

fn parse_val(s: &[u8]) -> Option<c_int> {
    let (tmp, consumed, overflow) = strtol_base10(s);
    // endp != str  <=>  consumed > 0
    if consumed > 0 && !overflow && tmp >= c_int::MIN as i64 && tmp <= c_int::MAX as i64 {
        Some(tmp as c_int)
    } else {
        None
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    // Read up to 99 bytes from stdin (mimic fgets with size 100).
    // fgets reads until newline (kept) or EOF, up to size-1 chars, plus NUL.
    let mut input = Vec::with_capacity(100);
    let mut buf = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    while input.len() < 99 {
        match handle.read(&mut buf) {
            Ok(0) => break,
            Ok(_) => {
                input.push(buf[0]);
                if buf[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    match parse_val(&input) {
        Some(x) => {
            run(x);
            run(x);
        }
        None => {
            let s = "An error occurred\n";
            let stdout = io::stdout();
            let mut out = stdout.lock();
            let _ = out.write_all(s.as_bytes());
        }
    }
    0
}
