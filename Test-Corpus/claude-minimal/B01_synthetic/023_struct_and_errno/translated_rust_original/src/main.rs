// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::io::{self, BufRead};

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

fn parse_val(s: &str) -> Option<i32> {
    // Mirror C's strtol behavior: parse leading whitespace and optional sign,
    // then as many digits as possible. Accept the value if any digits were
    // consumed and the parsed long fits in an i32.
    let bytes = s.as_bytes();
    let mut i = 0;

    // Skip leading whitespace (matches isspace for typical ASCII inputs).
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }

    let start = i;

    // Optional sign.
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    let digits_start = i;

    // Consume digits.
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }

    if i == digits_start {
        // No digits parsed -> endp == str semantics in C means failure.
        return None;
    }

    let token = &s[start..i];
    match token.parse::<i64>() {
        Ok(v) if v >= i32::MIN as i64 && v <= i32::MAX as i64 => Some(v as i32),
        _ => None,
    }
}

fn main() {
    let stdin = io::stdin();
    let mut input = String::new();
    // fgets reads up to sizeof(in) - 1 = 99 chars, including newline if present.
    // Reading a line is the closest equivalent here.
    let _ = stdin.lock().read_line(&mut input);

    match parse_val(&input) {
        Some(x) => {
            let mut the_house = House {
                floors: 2,
                bedrooms: 5,
                bathrooms: 2.5,
            };
            run(&mut the_house, x);
            run(&mut the_house, x);
        }
        None => {
            println!("An error occurred");
        }
    }
}
