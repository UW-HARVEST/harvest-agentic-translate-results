use std::io::{self, Read};

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

fn print_the_house(the_house: &House) {
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        the_house.floors, the_house.bedrooms, the_house.bathrooms
    );
}

fn run(the_house: &mut House, extra_bedrooms: i32) {
    print_the_house(the_house);
    add_floor_to_the_house(the_house);
    print_the_house(the_house);
    the_house.bathrooms += 1.0;
    print_the_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_the_house(the_house);
}

fn is_c_space(byte: u8) -> bool {
    matches!(byte, b' ' | 0x0c | b'\n' | b'\r' | b'\t' | 0x0b)
}

fn scanf_int(input: &[u8]) -> i32 {
    let mut index = 0;
    while index < input.len() && is_c_space(input[index]) {
        index += 1;
    }

    let mut sign = 1_i64;
    if index < input.len() {
        if input[index] == b'-' {
            sign = -1;
            index += 1;
        } else if input[index] == b'+' {
            index += 1;
        }
    }

    if index >= input.len() || !input[index].is_ascii_digit() {
        return 0;
    }

    let mut value = 0_i64;
    while index < input.len() && input[index].is_ascii_digit() {
        value = value
            .saturating_mul(10)
            .saturating_add((input[index] - b'0') as i64);
        index += 1;
    }

    (value.saturating_mul(sign)) as i32
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

    run(&mut the_house, x);
    run(&mut the_house, x);
}
