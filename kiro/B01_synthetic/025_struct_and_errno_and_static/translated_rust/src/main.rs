use std::io::{self, BufRead};

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

fn parse_val(s: &str) -> Option<i32> {
    let trimmed = s.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return None;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let num_str = &trimmed[..i];
    match num_str.parse::<i64>() {
        Ok(v) if v >= i32::MIN as i64 && v <= i32::MAX as i64 => Some(v as i32),
        _ => None,
    }
}

fn main() {
    let stdin = io::stdin();
    let mut line = String::new();
    let _ = stdin.lock().read_line(&mut line);
    match parse_val(&line) {
        Some(x) => {
            run(x);
            run(x);
        }
        None => {
            println!("An error occurred");
        }
    }
}
