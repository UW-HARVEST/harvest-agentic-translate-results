use std::io::{self, Read};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

static THE_HOUSE: OnceLock<Mutex<House>> = OnceLock::new();

fn the_house() -> &'static Mutex<House> {
    THE_HOUSE.get_or_init(|| {
        Mutex::new(House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        })
    })
}

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn add_floor_to_the_house() {
    let mut house = the_house().lock().unwrap();
    add_floor(&mut house);
}

fn print_the_house() {
    let house = the_house().lock().unwrap();
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    {
        let mut house = the_house().lock().unwrap();
        house.bathrooms += 1.0;
    }
    print_the_house();
    {
        let mut house = the_house().lock().unwrap();
        add_bedrooms(&mut house, extra_bedrooms);
    }
    print_the_house();
}

fn fgets_like_stdin() -> Vec<u8> {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);

    let mut out = Vec::new();
    for &byte in input.iter().take(99) {
        out.push(byte);
        if byte == b'\n' {
            break;
        }
    }
    out
}

fn is_c_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\x0c' | b'\n' | b'\r' | b'\t' | 0x0b)
}

fn parse_val(bytes: &[u8]) -> Option<i32> {
    const LONG_MAX: u128 = 9_223_372_036_854_775_807;
    const LONG_MIN_MAG: u128 = 9_223_372_036_854_775_808;

    let nul = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let s = &bytes[..nul];

    let mut idx = 0;
    while idx < s.len() && is_c_space(s[idx]) {
        idx += 1;
    }

    let start = idx;
    let mut negative = false;
    if idx < s.len() {
        if s[idx] == b'-' {
            negative = true;
            idx += 1;
        } else if s[idx] == b'+' {
            idx += 1;
        }
    }

    let mut value: u128 = 0;
    let mut saw_digit = false;
    let limit = if negative { LONG_MIN_MAG } else { LONG_MAX };
    let mut overflow = false;

    while idx < s.len() && s[idx].is_ascii_digit() {
        saw_digit = true;
        let digit = (s[idx] - b'0') as u128;
        if value > (limit - digit) / 10 {
            overflow = true;
        } else if !overflow {
            value = value * 10 + digit;
        }
        idx += 1;
    }

    if idx == start || !saw_digit || overflow {
        return None;
    }

    let signed = if negative {
        if value == LONG_MIN_MAG {
            i128::from(i64::MIN)
        } else {
            -(value as i128)
        }
    } else {
        value as i128
    };

    if signed >= i32::MIN as i128 && signed <= i32::MAX as i128 {
        Some(signed as i32)
    } else {
        None
    }
}

fn main() {
    let input = fgets_like_stdin();
    if let Some(x) = parse_val(&input) {
        run(x);
        run(x);
    } else {
        println!("An error occurred");
    }
}
