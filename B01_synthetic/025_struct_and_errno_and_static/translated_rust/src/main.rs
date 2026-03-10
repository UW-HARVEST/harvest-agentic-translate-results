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

fn add_bedrooms(house: &mut House, extra: i32) {
    house.bedrooms += extra;
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

fn run(extra_bedrooms: i32) {
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
    // Match strtol behavior: optional sign then digits
    let mut chars = trimmed.chars().peekable();
    let mut num_str = String::new();
    if matches!(chars.peek(), Some('+') | Some('-')) {
        num_str.push(chars.next().unwrap());
    }
    let had_digit = chars.peek().map_or(false, |c| c.is_ascii_digit());
    if !had_digit {
        return None;
    }
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num_str.push(chars.next().unwrap());
        } else {
            break;
        }
    }
    // strtol returns long; C code checks INT_MIN..INT_MAX and errno==0
    let val: i64 = num_str.parse().ok()?;
    if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
        Some(val as i32)
    } else {
        None
    }
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().lock().read_line(&mut input);
    if let Some(x) = parse_val(&input) {
        run(x);
        run(x);
    } else {
        println!("An error occurred");
    }
}
