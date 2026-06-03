use std::io::{self, Read};

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn print_house(house: &House) {
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
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

/// Mimic C's strtol(str, &endp, 10) followed by the parse_val checks:
/// - skip leading whitespace
/// - optional '+' or '-'
/// - one or more decimal digits
/// - no conversion (endp == str) => fail
/// - overflow of long => fail
/// - value not in [INT_MIN, INT_MAX] => fail
fn parse_val(bytes: &[u8]) -> Option<i32> {
    let mut i: usize = 0;
    // Skip whitespace per isspace(): ' ', '\t', '\n', '\v', '\f', '\r'
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0B || c == 0x0C || c == b'\r' {
            i += 1;
        } else {
            break;
        }
    }
    let after_ws = i;
    let mut neg = false;
    if i < bytes.len() {
        if bytes[i] == b'+' {
            i += 1;
        } else if bytes[i] == b'-' {
            neg = true;
            i += 1;
        }
    }
    let digits_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        // No digits consumed: endp == str (the original strtol semantics)
        // Note: also reached when only sign or whitespace was present.
        let _ = after_ws;
        return None;
    }
    // Accumulate as i64 (matching `long` on a 64-bit system) and detect overflow.
    let mut tmp: i64 = 0;
    let mut overflow = false;
    for &b in &bytes[digits_start..i] {
        let d = (b - b'0') as i64;
        match tmp.checked_mul(10).and_then(|v| v.checked_add(d)) {
            Some(v) => tmp = v,
            None => {
                overflow = true;
                break;
            }
        }
    }
    if overflow {
        return None;
    }
    let signed = if neg { -tmp } else { tmp };
    if signed < i32::MIN as i64 || signed > i32::MAX as i64 {
        return None;
    }
    Some(signed as i32)
}

fn main() {
    // Mimic: char in[100] = ""; fgets(in, sizeof(in), stdin);
    // fgets reads up to size-1 bytes, stopping after a newline (which is kept)
    // or at EOF, whichever comes first. The buffer is null-terminated.
    let mut buf = [0u8; 100];
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut idx: usize = 0;
    let mut byte = [0u8; 1];
    while idx < 99 {
        match handle.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf[idx] = byte[0];
                idx += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if let Some(x) = parse_val(&buf[..idx]) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        println!("An error occurred");
    }
}
