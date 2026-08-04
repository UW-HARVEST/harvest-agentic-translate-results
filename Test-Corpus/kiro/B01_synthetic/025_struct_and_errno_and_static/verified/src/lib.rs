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
    unsafe { add_floor(&mut THE_HOUSE) };
}

fn print_the_house() {
    unsafe {
        println!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
            THE_HOUSE.floors, THE_HOUSE.bedrooms, THE_HOUSE.bathrooms
        );
    }
}

#[no_mangle]
pub extern "C" fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe { THE_HOUSE.bathrooms += 1.0 };
    print_the_house();
    unsafe { add_bedrooms(&mut THE_HOUSE, extra_bedrooms) };
    print_the_house();
}

fn parse_val(s: &str) -> Option<i32> {
    let trimmed = s.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let (sign, rest) = if trimmed.starts_with('-') {
        (-1i64, &trimmed[1..])
    } else if trimmed.starts_with('+') {
        (1i64, &trimmed[1..])
    } else {
        (1i64, trimmed)
    };
    let digit_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    if digit_end == 0 {
        return None;
    }
    let digits = &rest[..digit_end];
    let magnitude: i64 = digits.parse().ok()?;
    let val = sign * magnitude;
    if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
        Some(val as i32)
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut line = String::new();
    let stdin = io::stdin();
    stdin.lock().read_line(&mut line).ok();
    if let Some(x) = parse_val(&line) {
        run(x);
        run(x);
    } else {
        println!("An error occurred");
    }
    0
}
