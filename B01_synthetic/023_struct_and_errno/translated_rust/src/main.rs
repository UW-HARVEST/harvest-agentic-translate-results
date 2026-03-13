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

/// Mimics C parse_val: strtol with endp != str, errno == 0, INT_MIN..=INT_MAX
fn parse_val(s: &str) -> Option<i32> {
    let s = s.trim_end_matches('\n');
    // strtol skips leading whitespace, then parses digits
    let trimmed = s.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    // Find the numeric prefix that strtol would consume
    let mut chars = trimmed.chars();
    let first = chars.next().unwrap();
    let start = if first == '+' || first == '-' { 1 } else { 0 };
    let rest = &trimmed[start..];
    if rest.is_empty() || !rest.starts_with(|c: char| c.is_ascii_digit()) {
        return None; // endp == str equivalent (no digits consumed)
    }
    let end = start + rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    let num_str = &trimmed[..end];
    // Parse as i64 to check range like C's long -> int
    match num_str.parse::<i64>() {
        Ok(v) if v >= i32::MIN as i64 && v <= i32::MAX as i64 => Some(v as i32),
        _ => None,
    }
}

fn main() {
    let stdin = io::stdin();
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
