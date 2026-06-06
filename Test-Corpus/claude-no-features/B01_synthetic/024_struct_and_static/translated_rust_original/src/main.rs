use std::io::{self, Read};

#[derive(Clone, Copy)]
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
    unsafe {
        add_floor(&mut *std::ptr::addr_of_mut!(THE_HOUSE));
    }
}

fn print_the_house() {
    unsafe {
        let h = &*std::ptr::addr_of!(THE_HOUSE);
        println!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
            h.floors, h.bedrooms, h.bathrooms
        );
    }
}

fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        (*std::ptr::addr_of_mut!(THE_HOUSE)).bathrooms += 1.0;
    }
    print_the_house();
    unsafe {
        add_bedrooms(
            &mut *std::ptr::addr_of_mut!(THE_HOUSE),
            extra_bedrooms,
        );
    }
    print_the_house();
}

fn read_first_int() -> i32 {
    // Mimic scanf("%d", &x): skip leading whitespace, then read an optional
    // sign followed by decimal digits. If no valid integer is read, x stays 0.
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return 0;
    }
    let bytes = input.as_bytes();
    let mut i = 0usize;
    // Skip whitespace (matches C isspace for typical scanf)
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return 0;
    }
    let mut sign: i64 = 1;
    if bytes[i] == b'+' {
        i += 1;
    } else if bytes[i] == b'-' {
        sign = -1;
        i += 1;
    }
    let start = i;
    let mut value: i64 = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        return 0;
    }
    let signed = sign.wrapping_mul(value);
    signed as i32
}

fn main() {
    let x = read_first_int();
    run(x);
    run(x);
}
