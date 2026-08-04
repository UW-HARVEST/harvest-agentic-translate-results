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

fn print_house(house: &House) {
    // C: printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
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

/// Mimics C's strtol(str, &endp, 10) behavior: skip leading whitespace,
/// optional sign, parse decimal digits. Returns Some(value) on success
/// only when at least one digit was consumed and the value fits in i32.
fn parse_val(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;

    // Skip leading whitespace (C isspace: space, \t, \n, \v, \f, \r)
    while i < bytes.len() {
        let b = bytes[i];
        if b == b' ' || b == b'\t' || b == b'\n' || b == 0x0b || b == 0x0c || b == b'\r' {
            i += 1;
        } else {
            break;
        }
    }

    let start_after_ws = i;

    // Optional sign
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            negative = true;
        }
        i += 1;
    }

    let digits_start = i;
    // Parse digits, accumulate as i64 to detect overflow vs i32
    let mut acc: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as i64;
        acc = acc.saturating_mul(10).saturating_add(d);
        i += 1;
    }

    // If no digits were parsed, endp == str (no progress); return None
    if i == digits_start {
        // Special case: if there was a sign but no digits, endp would still be at start in
        // strtol behavior — but in glibc it actually does set endp back. Either way, no digits = fail.
        let _ = start_after_ws;
        return None;
    }

    let value = if negative { -acc } else { acc };

    if value < i32::MIN as i64 || value > i32::MAX as i64 {
        return None;
    }

    Some(value as i32)
}

fn driver(input: &str) {
    if let Some(x) = parse_val(input) {
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

fn main() {
    // Read entire stdin like a buffer; driver only inspects up to first non-digit.
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        // On read error, treat as empty input -> parse will fail -> error message
        buf.clear();
    }
    driver(&buf);
    let _ = io::stdout().flush();
}
