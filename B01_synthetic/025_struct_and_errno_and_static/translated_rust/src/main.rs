use std::io::BufRead;

struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    unsafe {
        add_floor(&mut THE_HOUSE);
    }
}

fn print_the_house() {
    unsafe {
        println!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
            THE_HOUSE.floors, THE_HOUSE.bedrooms, THE_HOUSE.bathrooms
        );
    }
}

fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        THE_HOUSE.bathrooms += 1.0;
    }
    print_the_house();
    unsafe {
        add_bedrooms(&mut THE_HOUSE, extra_bedrooms);
    }
    print_the_house();
}

/// Mimics C strtol parse_val: trims leading whitespace, parses an integer,
/// returns true if at least one digit was consumed and value fits in i32.
fn parse_val(s: &str) -> Option<i32> {
    let trimmed = s.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    // Determine sign and digit start
    let (rest, negative) = if trimmed.starts_with('-') {
        (&trimmed[1..], true)
    } else if trimmed.starts_with('+') {
        (&trimmed[1..], false)
    } else {
        (trimmed, false)
    };
    // Must have at least one digit (strtol: endp != str check)
    if rest.is_empty() || !rest.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    // Collect digits
    let digit_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    let digits = &rest[..digit_end];
    // Parse as i64 to check overflow like strtol with long->int range check
    let magnitude: i64 = digits.parse().ok()?;
    let value: i64 = if negative { -magnitude } else { magnitude };
    if value >= i32::MIN as i64 && value <= i32::MAX as i64 {
        Some(value as i32)
    } else {
        None
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut line = String::new();
    // fgets reads one line
    let _ = stdin.lock().read_line(&mut line);
    // Remove trailing newline to match strtol stopping at non-digit
    if let Some(x) = parse_val(&line) {
        run(x);
        run(x);
    } else {
        println!("An error occurred");
    }
}
