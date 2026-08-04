use std::io::{self, Read};

const INT_MIN: i128 = i32::MIN as i128;
const INT_MAX: i128 = i32::MAX as i128;
const LONG_MIN: i128 = i64::MIN as i128;
const LONG_MAX: i128 = i64::MAX as i128;

struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn add_floor(house: &mut House) {
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
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

fn is_c_space(byte: u8) -> bool {
    matches!(byte, b' ' | 0x0c | b'\n' | b'\r' | b'\t' | 0x0b)
}

fn parse_val(bytes: &[u8]) -> Option<i32> {
    let nul = bytes.iter().position(|&byte| byte == 0).unwrap_or(bytes.len());
    let bytes = &bytes[..nul];

    let mut index = 0;
    while index < bytes.len() && is_c_space(bytes[index]) {
        index += 1;
    }

    let mut negative = false;
    if index < bytes.len() {
        if bytes[index] == b'-' {
            negative = true;
            index += 1;
        } else if bytes[index] == b'+' {
            index += 1;
        }
    }

    let digits_start = index;
    let limit = if negative { -LONG_MIN } else { LONG_MAX };
    let mut value: i128 = 0;
    let mut overflow = false;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        let digit = i128::from(bytes[index] - b'0');
        if value > (limit - digit) / 10 {
            overflow = true;
        } else if !overflow {
            value = value * 10 + digit;
        }
        index += 1;
    }

    if index == digits_start {
        return None;
    }

    if overflow {
        return None;
    }

    let signed = if negative { -value } else { value };
    if !(LONG_MIN..=LONG_MAX).contains(&signed) || !(INT_MIN..=INT_MAX).contains(&signed) {
        return None;
    }

    Some(signed as i32)
}

fn read_fgets_100() -> Vec<u8> {
    let mut stdin = Vec::new();
    let _ = io::stdin().read_to_end(&mut stdin);

    let max_len = stdin.len().min(99);
    match stdin[..max_len].iter().position(|&byte| byte == b'\n') {
        Some(newline) => stdin[..=newline].to_vec(),
        None => stdin[..max_len].to_vec(),
    }
}

fn main() {
    let input = read_fgets_100();
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
