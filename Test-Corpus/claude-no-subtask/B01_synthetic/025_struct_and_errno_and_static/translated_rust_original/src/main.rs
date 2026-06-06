use std::io::{self, Read, Write};

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

fn add_floor_to_the_house(house: &mut House) {
    add_floor(house);
}

fn print_the_house(house: &House) {
    print!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run(house: &mut House, extra_bedrooms: i32) {
    print_the_house(house);
    add_floor_to_the_house(house);
    print_the_house(house);
    house.bathrooms += 1.0;
    print_the_house(house);
    add_bedrooms(house, extra_bedrooms);
    print_the_house(house);
}

/// Mimics C's strtol(str, &endp, 10):
/// - skips leading whitespace
/// - optional + or - sign
/// - reads decimal digits
/// Returns (parsed_value, num_chars_consumed_from_start_of_str_including_skipped_whitespace_and_sign).
/// num_chars_consumed reflects how far endp advanced from str. If no digits are seen,
/// endp == str (consumed = 0).
fn strtol_like(s: &[u8]) -> (i64, usize, bool /* overflow */) {
    let mut i = 0usize;
    // Skip whitespace as defined by isspace
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }
    let start_after_ws = i;

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        if s[i] == b'-' {
            negative = true;
        }
        i += 1;
    }

    let digits_start = i;
    let mut overflow = false;
    let mut value: i64 = 0;
    while i < s.len() && (s[i] as char).is_ascii_digit() {
        let digit = (s[i] - b'0') as i64;
        // Detect overflow with i64 saturating arithmetic
        let next = value.checked_mul(10).and_then(|v| v.checked_add(digit));
        match next {
            Some(v) => value = v,
            None => {
                overflow = true;
                value = if negative { i64::MIN } else { i64::MAX };
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits were consumed; endp == str
        return (0, 0, false);
    }

    if negative && !overflow {
        value = -value;
    }

    // endp points to position i; number of chars consumed from start_of_str
    let _ = start_after_ws;
    (value, i, overflow)
}

fn parse_val(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let (val, consumed, overflow) = strtol_like(bytes);
    if consumed == 0 {
        // endp == str
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

fn main() {
    // Mimic fgets(in, 100, stdin): read up to 99 bytes or until newline (inclusive),
    // then null-terminate. We track whatever string was read.
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buf: Vec<u8> = Vec::new();
    // Read byte by byte to mimic fgets exactly (stops at newline or after 99 bytes or EOF)
    let mut read_count = 0usize;
    let max_chars = 99usize; // fgets reads up to size-1 chars
    let mut byte = [0u8; 1];
    while read_count < max_chars {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                read_count += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    // If fgets fails (EOF with nothing read), `in` remains "" (initialized to empty string).
    // In our case, buf is empty in that scenario, so strtol_like("") returns no digits.
    let s = std::str::from_utf8(&buf).unwrap_or("");

    let mut house = House {
        floors: 2,
        bedrooms: 5,
        bathrooms: 2.5,
    };

    match parse_val(s) {
        Some(x) => {
            run(&mut house, x);
            run(&mut house, x);
        }
        None => {
            print!("An error occurred\n");
        }
    }

    let _ = io::stdout().flush();
}
