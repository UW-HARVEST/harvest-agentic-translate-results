use std::io::{self, Read, Write};

struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn is_c_whitespace(byte: &u8) -> bool {
    matches!(*byte, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

fn scan_decimal_int(input: impl Iterator<Item = io::Result<u8>>) -> Option<i32> {
    let mut input = input.map_while(Result::ok);
    let mut byte = input.next();
    while byte.as_ref().is_some_and(is_c_whitespace) {
        byte = input.next();
    }

    let negative = match byte {
        Some(b'+') => {
            byte = input.next();
            false
        }
        Some(b'-') => {
            byte = input.next();
            true
        }
        _ => false,
    };

    if !byte.is_some_and(|value| value.is_ascii_digit()) {
        return None;
    }

    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut magnitude = 0_u64;
    while let Some(digit) = byte.filter(u8::is_ascii_digit) {
        magnitude = magnitude
            .saturating_mul(10)
            .saturating_add(u64::from(digit - b'0'))
            .min(limit);
        byte = input.next();
    }

    let value = if negative {
        if magnitude == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else {
        magnitude as i64
    };
    Some(value as i32)
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
    print_house(output, house)
}

fn main() {
    let extra_bedrooms = scan_decimal_int(io::stdin().lock().bytes()).unwrap_or(0);

    let mut house = House {
        floors: 2,
        bedrooms: 5,
        bathrooms: 2.5,
    };
    let mut output = io::BufWriter::new(io::stdout().lock());

    run(&mut output, &mut house, extra_bedrooms);
    run(&mut output, &mut house, extra_bedrooms)
}
