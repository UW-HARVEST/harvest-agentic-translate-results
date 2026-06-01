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

fn print_house(house: &House, out: &mut dyn Write) {
    writeln!(
        out,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    )
    .unwrap();
}

fn run(the_house: &mut House, extra_bedrooms: i32, out: &mut dyn Write) {
    print_house(the_house, out);
    add_floor(the_house);
    print_house(the_house, out);
    the_house.bathrooms += 1.0;
    print_house(the_house, out);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house, out);
}

/// Mimic C's `strtol(str, &endp, 10)` followed by the checks in `parse_val`:
/// - skip leading whitespace (C "C" locale isspace)
/// - optional sign
/// - one or more decimal digits
/// - return Some(v) only if at least one digit was consumed AND the value fits in i32
///   (C also requires no `long` overflow via errno, but any value fitting in int32
///   trivially fits in long, so the int32 check subsumes it.)
fn parse_val(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // Skip leading whitespace as C isspace would: space, \t, \n, \v, \f, \r.
    while i < bytes.len() {
        let c = bytes[i];
        if matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
            i += 1;
        } else {
            break;
        }
    }

    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        negative = bytes[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: strtol leaves endp == str, parse_val returns false.
        return None;
    }

    // Compute value in i128 to detect overflow safely.
    let mut val: i128 = 0;
    for &ch in &bytes[digits_start..i] {
        val = val * 10 + (ch - b'0') as i128;
        if val > i64::MAX as i128 + 1 {
            // Overflow against C `long` => strtol sets errno=ERANGE => parse_val fails.
            return None;
        }
    }
    let signed: i128 = if negative { -val } else { val };

    if signed < i32::MIN as i128 || signed > i32::MAX as i128 {
        return None;
    }
    Some(signed as i32)
}

fn main() {
    // Replicate `fgets(in, 100, stdin)`: read up to 99 bytes or until '\n' (inclusive).
    // The trailing '\0' isn't relevant for parse_val since digit scanning stops at non-digits.
    let mut buf = Vec::with_capacity(99);
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    while buf.len() < 99 {
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

    let s = std::str::from_utf8(&buf).unwrap_or("");
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if let Some(x) = parse_val(s) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x, &mut out);
        run(&mut the_house, x, &mut out);
    } else {
        writeln!(out, "An error occurred").unwrap();
    }
}
