use std::io::{self, Read, Write};

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

fn print_house(output: &mut impl Write, house: &House) {
    let _ = writeln!(
        output,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run(output: &mut impl Write, house: &mut House, extra_bedrooms: i32) {
    print_house(output, house);
    add_floor(house);
    print_house(output, house);
    house.bathrooms += 1.0;
    print_house(output, house);
    add_bedrooms(house, extra_bedrooms);
    print_house(output, house);
}

fn parse_val(input: &[u8]) -> Option<i32> {
    let input = match input.iter().position(|&byte| byte == 0) {
        Some(end) => &input[..end],
        None => input,
    };

    let mut index = 0;
    while index < input.len() && matches!(input[index], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
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
        Some((0_i64 - magnitude as i64) as i32)
    } else {
        Some(magnitude as i32)
    }
}

fn read_fgets_line() -> Vec<u8> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut line = Vec::with_capacity(99);
    let mut byte = [0_u8; 1];

    while line.len() < 99 {
        match input.read(&mut byte) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                line.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
        }
    }

    line
}

fn main() {
    let input = read_fgets_line();
    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());

    if let Some(extra_bedrooms) = parse_val(&input) {
        let mut house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut output, &mut house, extra_bedrooms);
        run(&mut output, &mut house, extra_bedrooms);
    } else {
        let _ = writeln!(output, "An error occurred");
    }
}
