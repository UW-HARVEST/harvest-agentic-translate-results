use std::io::{self, Read, Write};

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
    // %.1f mimics C's printf with one digit after the decimal point.
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

/// Mimics the C strtol-based parse_val: parses a leading optional sign and
/// decimal digits from `s`, returning the parsed value and whether parsing
/// consumed at least one character (and stayed within i32 range).
fn parse_val(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // Skip leading whitespace, matching strtol's behavior.
    while i < bytes.len()
        && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
    {
        i += 1;
    }

    let start_after_ws = i;

    // Optional sign.
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            negative = true;
        }
        i += 1;
    }

    // Digits.
    let digits_start = i;
    let mut value: i64 = 0;
    let mut overflow = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as i64;
        if !overflow {
            // Check against i64 range while accumulating. For matching C
            // strtol semantics on a long that's at least 64-bit, we set the
            // overflow flag if we exceed the i64 range; the calling code in C
            // additionally checks INT_MIN/INT_MAX bounds, which we replicate
            // below.
            let next = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(d));
            match next {
                Some(v) => value = v,
                None => {
                    overflow = true;
                }
            }
        }
        i += 1;
    }

    // strtol requires at least one digit to count as a successful parse.
    // The C code uses `endp != str` as the success indicator. With strtol,
    // when no conversion is performed, endp is set back to the original
    // string. If we consumed only whitespace and/or sign with no digits,
    // strtol leaves endp == str. Mirror that:
    if digits_start == i {
        // No digits consumed.
        // Whether or not we skipped whitespace/sign, strtol's contract here
        // is that endp == str and no value is produced.
        // Note: technically if we consumed whitespace then a sign with no
        // digits, strtol on most platforms still leaves endp at str. We
        // treat it as failure.
        let _ = start_after_ws; // silence unused warning if any
        return None;
    }

    let signed: i64 = if negative { -value } else { value };

    if overflow {
        return None;
    }

    if signed < i32::MIN as i64 || signed > i32::MAX as i64 {
        return None;
    }

    Some(signed as i32)
}

/// Reads a single line in the manner of C's `fgets(buf, 100, stdin)`:
/// up to 99 bytes, stopping at and including a '\n'. Returns the bytes
/// read as a String (lossy on invalid UTF-8 to keep printing safe; only
/// ASCII prefix is used by parse_val anyway).
fn fgets_like(max_bytes: usize) -> String {
    let mut stdin = io::stdin();
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    // fgets reads at most max_bytes - 1 characters.
    let cap = max_bytes.saturating_sub(1);
    while buf.len() < cap {
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
    String::from_utf8_lossy(&buf).into_owned()
}

fn main() {
    // Mirror: char in[100] = ""; fgets(in, sizeof(in), stdin);
    let input = fgets_like(100);

    if let Some(x) = parse_val(&input) {
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

    // Ensure output is flushed before exit, matching C stdio's atexit flush.
    let _ = io::stdout().flush();
}
