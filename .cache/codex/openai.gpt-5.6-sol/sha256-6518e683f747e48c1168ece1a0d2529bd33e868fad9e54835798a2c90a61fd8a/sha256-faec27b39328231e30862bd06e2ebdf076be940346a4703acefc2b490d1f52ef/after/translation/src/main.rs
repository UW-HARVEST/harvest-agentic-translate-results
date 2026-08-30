use std::io::{self, BufRead, Read, Write};

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

fn print_house(house: &House, output: &mut Vec<u8>) {
    writeln!(
        output,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    )
    .unwrap();
}

fn run(house: &mut House, extra_bedrooms: i32, output: &mut Vec<u8>) {
    print_house(house, output);
    add_floor(house);
    print_house(house, output);
    house.bathrooms += 1.0;
    print_house(house, output);
    add_bedrooms(house, extra_bedrooms);
    print_house(house, output);
}

fn parse_val(input: &[u8]) -> Option<i32> {
    let input = &input[..input
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(input.len())];
    let mut index = 0;

    while index < input.len() && matches!(input[index], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        index += 1;
    }

    let negative = match input.get(index) {
        Some(b'-') => {
            index += 1;
            true
        }
        Some(b'+') => {
            index += 1;
            false
        }
        _ => false,
    };

    let first_digit = index;
    let limit = if negative {
        i32::MAX as u64 + 1
    } else {
        i32::MAX as u64
    };
    let mut magnitude = 0_u64;

    while let Some(&byte @ b'0'..=b'9') = input.get(index) {
        magnitude = match magnitude
            .checked_mul(10)
            .and_then(|value| value.checked_add((byte - b'0') as u64))
        {
            Some(value) if value <= limit => value,
            _ => return None,
        };
        index += 1;
    }

    if index == first_digit {
        return None;
    }

    if negative {
        if magnitude == i32::MAX as u64 + 1 {
            Some(i32::MIN)
        } else {
            Some(-(magnitude as i32))
        }
    } else {
        Some(magnitude as i32)
    }
}

fn main() {
    let stdin = io::stdin();
    let mut input = Vec::new();
    let _ = stdin.lock().take(99).read_until(b'\n', &mut input);

    let mut output = Vec::new();
    if let Some(x) = parse_val(&input) {
        let mut house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut house, x, &mut output);
        run(&mut house, x, &mut output);
    } else {
        output.extend_from_slice(b"An error occurred\n");
    }

    let _ = io::stdout().write_all(&output);
}
