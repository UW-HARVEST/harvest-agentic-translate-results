use std::io::{self, Read, Write};

#[derive(Clone, Copy)]
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

fn add_floor_to_the_house(the_house: &mut House) {
    add_floor(the_house);
}

fn print_the_house(the_house: &House, stdout: &mut dyn Write) {
    writeln!(
        stdout,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        the_house.floors, the_house.bedrooms, the_house.bathrooms
    )
    .unwrap();
}

fn run(extra_bedrooms: i32, the_house: &mut House, stdout: &mut dyn Write) {
    print_the_house(the_house, stdout);
    add_floor_to_the_house(the_house);
    print_the_house(the_house, stdout);
    the_house.bathrooms += 1.0;
    print_the_house(the_house, stdout);
    add_bedrooms(the_house, extra_bedrooms);
    print_the_house(the_house, stdout);
}

fn scanf_int(input: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < input.len() && input[i].is_ascii_whitespace() {
        i += 1;
    }

    if i >= input.len() {
        return 0;
    }

    let mut sign = 1i64;
    if input[i] == b'+' {
        i += 1;
    } else if input[i] == b'-' {
        sign = -1;
        i += 1;
    }

    let start = i;
    let mut value = 0i64;
    while i < input.len() && input[i].is_ascii_digit() {
        value = value
            .saturating_mul(10)
            .saturating_add((input[i] - b'0') as i64);
        i += 1;
    }

    if i == start {
        0
    } else {
        let signed = value.saturating_mul(sign);
        if signed > i32::MAX as i64 {
            i32::MAX
        } else if signed < i32::MIN as i64 {
            i32::MIN
        } else {
            signed as i32
        }
    }
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();

    let x = scanf_int(&input);
    let mut the_house = House {
        floors: 2,
        bedrooms: 5,
        bathrooms: 2.5,
    };

    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    run(x, &mut the_house, &mut stdout);
    run(x, &mut the_house, &mut stdout);
}
