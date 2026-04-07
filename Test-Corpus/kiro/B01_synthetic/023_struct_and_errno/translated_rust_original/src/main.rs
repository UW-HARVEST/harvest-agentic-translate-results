struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_house(house: &House) {
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run(house: &mut House, extra_bedrooms: i32) {
    print_house(house);
    house.floors += 1;
    print_house(house);
    house.bathrooms += 1.0;
    print_house(house);
    house.bedrooms += extra_bedrooms;
    print_house(house);
}

/// Mimics C strtol parse: skip leading whitespace, parse decimal integer,
/// succeed if at least one digit consumed and value fits in i32.
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
    // Must have at least one digit
    let digit_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    if digit_end == 0 {
        return None;
    }
    let digits = &rest[..digit_end];
    // Parse as i64 to check i32 range (like C checks INT_MIN..INT_MAX)
    let val: i64 = digits.parse().ok()?;
    let val = if negative { -val } else { val };
    if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
        Some(val as i32)
    } else {
        None
    }
}

fn main() {
    let mut input = String::new();
    // fgets reads one line (up to 99 chars in C); read_line is equivalent
    if std::io::stdin().read_line(&mut input).is_err() {
        println!("An error occurred");
        return;
    }
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
