use std::io::{self, Read};

struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

static mut THE_HOUSE: House = House { floors: 2, bedrooms: 5, bathrooms: 2.5 };

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    unsafe { add_floor(&mut THE_HOUSE); }
}

fn print_the_house() {
    unsafe {
        print!("The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
            THE_HOUSE.floors, THE_HOUSE.bedrooms, THE_HOUSE.bathrooms);
    }
}

#[no_mangle]
pub extern "C" fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe { THE_HOUSE.bathrooms += 1.0; }
    print_the_house();
    unsafe { add_bedrooms(&mut THE_HOUSE, extra_bedrooms); }
    print_the_house();
}

/// Mimics scanf("%d", &x): skip whitespace, parse one integer
fn scanf_int() -> i32 {
    let mut buf = Vec::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    // Skip leading whitespace
    let mut byte = [0u8; 1];
    loop {
        if handle.read(&mut byte).unwrap_or(0) == 0 {
            break;
        }
        if !byte[0].is_ascii_whitespace() {
            buf.push(byte[0]);
            break;
        }
    }
    // Read digits (and possible leading sign)
    loop {
        if handle.read(&mut byte).unwrap_or(0) == 0 {
            break;
        }
        if byte[0].is_ascii_digit() || (buf.is_empty() && (byte[0] == b'-' || byte[0] == b'+')) {
            buf.push(byte[0]);
        } else {
            break;
        }
    }
    String::from_utf8(buf).unwrap().parse::<i32>().unwrap_or(0)
}

fn main() {
    let x = scanf_int();
    run(x);
    run(x);
}
