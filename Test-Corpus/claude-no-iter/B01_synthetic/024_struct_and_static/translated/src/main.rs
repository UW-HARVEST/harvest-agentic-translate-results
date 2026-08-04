use std::io::Read;
use std::sync::Mutex;

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
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

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    let mut h = THE_HOUSE.lock().unwrap();
    add_floor(&mut *h);
}

fn print_the_house() {
    let h = THE_HOUSE.lock().unwrap();
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        h.floors, h.bedrooms, h.bathrooms
    );
}

fn run(extra_bedrooms: i32) {
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

/// Mimic scanf("%d", &x). Returns Some(value) if a number was parsed,
/// or None if no conversion was made (in which case caller leaves variable unchanged,
/// matching C's scanf behavior where the destination is not modified on match failure).
fn scanf_int() -> Option<i64> {
    let mut stdin = std::io::stdin();
    let mut buf = [0u8; 1];

    // Skip leading whitespace (space, tab, newline, etc.)
    let mut c;
    loop {
        match stdin.read(&mut buf) {
            Ok(0) => return None, // EOF
            Ok(_) => {
                c = buf[0];
                if !(c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C) {
                    break;
                }
            }
            Err(_) => return None,
        }
    }

    // Optional sign
    let mut negative = false;
    let mut have_sign = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        have_sign = true;
        match stdin.read(&mut buf) {
            Ok(0) => return None, // sign with no digit -> match failure
            Ok(_) => c = buf[0],
            Err(_) => return None,
        }
    }

    // Must have at least one digit
    if !c.is_ascii_digit() {
        return None;
    }

    let mut value: i64 = 0;
    let mut any_digit = false;
    loop {
        if c.is_ascii_digit() {
            value = value
                .wrapping_mul(10)
                .wrapping_add((c - b'0') as i64);
            any_digit = true;
        } else {
            break;
        }
        match stdin.read(&mut buf) {
            Ok(0) => break,
            Ok(_) => c = buf[0],
            Err(_) => break,
        }
    }

    if !any_digit {
        return None;
    }
    let _ = have_sign;
    if negative {
        Some(value.wrapping_neg())
    } else {
        Some(value)
    }
}

fn main() {
    let mut x: i32 = 0;
    if let Some(v) = scanf_int() {
        x = v as i32;
    }
    run(x);
    run(x);
}
