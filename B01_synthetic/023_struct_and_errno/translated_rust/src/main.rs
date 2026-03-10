use std::io::BufRead;

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

fn parse_val(s: &str) -> Option<i32> {
    let trimmed = s.trim_end_matches('\n');
    // Match strtol behavior: skip leading whitespace, parse digits
    let trimmed = trimmed.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    // Find the numeric prefix (optional sign + digits), matching strtol
    let mut end = 0;
    let bytes = trimmed.as_bytes();
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == 0 || (end == 1 && (bytes[0] == b'+' || bytes[0] == b'-')) {
        return None; // endp == str equivalent
    }
    let num_str = &trimmed[..end];
    // strtol returns long; check parse and i32 range
    match num_str.parse::<i64>() {
        Ok(tmp) if tmp >= i32::MIN as i64 && tmp <= i32::MAX as i64 => Some(tmp as i32),
        _ => None,
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut line = String::new();
    let _ = stdin.lock().read_line(&mut line);
    if let Some(x) = parse_val(&line) {
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
