use std::io::{self, BufRead};

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

fn run(house: &mut House, extra_bedrooms: i32) {
    print_house(house);
    add_floor(house);
    print_house(house);
    house.bathrooms += 1.0;
    print_house(house);
    add_bedrooms(house, extra_bedrooms);
    print_house(house);
}

/// Mimics C's strtol parse: succeeds if at least one digit consumed and
/// value fits in i32 range. Leading whitespace and trailing garbage are OK.
fn parse_val(s: &str) -> Option<i32> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    // Determine sign and digit start
    let (negative, digits_start) = if s.starts_with('-') {
        (true, &s[1..])
    } else if s.starts_with('+') {
        (false, &s[1..])
    } else {
        (false, s)
    };
    // Must have at least one digit
    if digits_start.is_empty() || !digits_start.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    // Consume digits
    let mut val: i64 = 0;
    let mut overflow = false;
    for &b in digits_start.as_bytes() {
        if !b.is_ascii_digit() {
            break;
        }
        val = val.wrapping_mul(10).wrapping_add((b - b'0') as i64);
        if val > i32::MAX as i64 + 1 {
            overflow = true;
        }
    }
    if negative {
        val = -val;
    }
    if overflow || val < i32::MIN as i64 || val > i32::MAX as i64 {
        return None;
    }
    Some(val as i32)
}

fn main() {
    let mut input = String::new();
    let stdin = io::stdin();
    // fgets reads one line (up to newline or EOF)
    let _ = stdin.lock().read_line(&mut input);
    if let Some(x) = parse_val(&input) {
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
