use std::fmt::Write as _;
use std::io::{self, Read, Write as _};

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

fn print_the_house(house: &House, output: &mut String) {
    writeln!(
        output,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    )
    .unwrap();
}

fn run(house: &mut House, extra_bedrooms: i32, output: &mut String) {
    print_the_house(house, output);
    add_floor(house);
    print_the_house(house, output);
    house.bathrooms += 1.0;
    print_the_house(house, output);
    add_bedrooms(house, extra_bedrooms);
    print_the_house(house, output);
}

fn parse_val(input: &[u8]) -> Option<i32> {
    let mut position = 0;
    while position < input.len()
        && matches!(input[position], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        position += 1;
    }

    let negative = match input.get(position) {
        Some(b'-') => {
            position += 1;
            true
        }
        Some(b'+') => {
            position += 1;
            false
        }
        _ => false,
    };

    let first_digit = position;
    let limit = if negative {
        i32::MAX as u32 + 1
    } else {
        i32::MAX as u32
    };
    let mut magnitude = 0_u32;

    while let Some(&byte) = input.get(position) {
        if !byte.is_ascii_digit() {
            break;
        }
        magnitude = magnitude
            .checked_mul(10)?
            .checked_add(u32::from(byte - b'0'))?;
        if magnitude > limit {
            return None;
        }
        position += 1;
    }

    if position == first_digit {
        return None;
    }

    if negative {
        if magnitude == i32::MAX as u32 + 1 {
            Some(i32::MIN)
        } else {
            Some(-(magnitude as i32))
        }
    } else {
        Some(magnitude as i32)
    }
}

fn read_fgets_input() -> Vec<u8> {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let mut input = Vec::with_capacity(99);

    while input.len() < 99 {
        let mut byte = [0_u8; 1];
        match stdin.read(&mut byte) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                input.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
        }
    }

    input
}

fn main() {
    let input = read_fgets_input();
    let mut output = String::new();

    if let Some(extra_bedrooms) = parse_val(&input) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, extra_bedrooms, &mut output);
        run(&mut the_house, extra_bedrooms, &mut output);
    } else {
        output.push_str("An error occurred\n");
    }

    let _ = io::stdout().write_all(output.as_bytes());
}
