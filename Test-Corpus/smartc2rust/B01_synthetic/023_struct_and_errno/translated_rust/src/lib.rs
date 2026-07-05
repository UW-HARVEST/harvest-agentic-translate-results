
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::{self, BufRead, Write};

#[derive(Copy, Clone, Debug)]
pub struct House {
    pub floors: i32,
    pub bedrooms: i32,
    pub bathrooms: f64,
}

fn rust_add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn rust_add_floor(house: &mut House) {
    house.floors += 1;
}

fn rust_print_house(house: &House) {
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

pub fn rust_run(the_house: &mut House, extra_bedrooms: i32) {
    rust_print_house(the_house);
    rust_add_floor(the_house);
    rust_print_house(the_house);
    the_house.bathrooms += 1.0;
    rust_print_house(the_house);
    rust_add_bedrooms(the_house, extra_bedrooms);
    rust_print_house(the_house);
}

fn rust_parse_val(s: &str) -> Option<i32> {
    // Emulate strtol: skip leading whitespace, optional sign, then parse digits.
    let trimmed = s.trim_start();
    let bytes = trimmed.as_bytes();
    let mut idx = 0;

    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        idx += 1;
    }

    let digits_start = idx;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }

    if digits_start == idx {
        return None;
    }

    trimmed[..idx]
        .parse::<i64>()
        .ok()
        .filter(|&n| (i32::MIN as i64..=i32::MAX as i64).contains(&n))
        .map(|n| n as i32)
}

fn rust_read_line_bounded(max_len: usize) -> io::Result<String> {
    let stdin = io::stdin();
    let mut input = String::new();
    stdin.lock().read_line(&mut input)?;
    if input.len() > max_len {
        input.truncate(max_len);
    }
    Ok(input)
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    // Emulate fgets with a 100-byte buffer (99 chars + null terminator).
    let input = rust_read_line_bounded(99).unwrap_or_default();

    match rust_parse_val(&input) {
        Some(x) => {
            let mut the_house = House {
                floors: 2,
                bedrooms: 5,
                bathrooms: 2.5,
            };
            rust_run(&mut the_house, x);
            rust_run(&mut the_house, x);
        }
        None => {
            println!("An error occurred");
        }
    }

    let _ = io::stdout().flush();
    0
}