use std::cell::RefCell;
use std::io::{self, Read};

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

thread_local! {
    static THE_HOUSE: RefCell<House> = const { RefCell::new(House {
        floors: 2,
        bedrooms: 5,
        bathrooms: 2.5,
    }) };
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
    let h = THE_HOUSE.with(|h| *h.borrow());
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        h.floors, h.bedrooms, h.bathrooms
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

/// Mimics C's `strtol(str, &endp, 10)` then validates as in parse_val:
/// - returns `Some(val)` if at least one digit is parsed and the value fits in i32
/// - returns `None` otherwise (no conversion, or overflow, or out of i32 range)
fn parse_val(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // Skip leading whitespace (per isspace in the "C" locale).
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0B || c == 0x0C || c == b'\r' {
            i += 1;
        } else {
            break;
        }
    }

    // Optional sign.
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        negative = bytes[i] == b'-';
        i += 1;
    }

    let digit_start = i;
    let mut val: i64 = 0;
    let mut overflow = false;

    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as i64;
        if !overflow {
            if negative {
                match val.checked_mul(10).and_then(|v| v.checked_sub(d)) {
                    Some(v) => val = v,
                    None => {
                        overflow = true;
                    }
                }
            } else {
                match val.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) => val = v,
                    None => {
                        overflow = true;
                    }
                }
            }
        }
        i += 1;
    }

    // No digits consumed -> strtol sets endp == str -> parse_val returns false.
    if i == digit_start {
        return None;
    }

    if overflow {
        return None;
    }

    if val < i32::MIN as i64 || val > i32::MAX as i64 {
        return None;
    }

    Some(val as i32)
}

/// Mirror of `fgets(in, 100, stdin)`:
/// - reads at most 99 bytes (then leaves room for the null terminator)
/// - stops at and includes a newline
/// - returns the string seen so far on EOF
fn fgets_like(max: usize) -> String {
    let mut stdin = io::stdin();
    let mut out = Vec::with_capacity(max);
    let mut buf = [0u8; 1];
    // Read up to (max - 1) bytes, stopping after a newline.
    while out.len() < max - 1 {
        match stdin.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(_) => {
                out.push(buf[0]);
                if buf[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn main() {
    let in_str = fgets_like(100);
    match parse_val(&in_str) {
        Some(x) => {
            run(x);
            run(x);
        }
        None => {
            println!("An error occurred");
        }
    }
}
