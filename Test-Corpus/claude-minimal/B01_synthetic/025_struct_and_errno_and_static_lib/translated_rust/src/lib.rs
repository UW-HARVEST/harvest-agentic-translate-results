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

use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Mutex;

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

static THE_HOUSE: Mutex<House> = Mutex::new(House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
});

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    let mut house = THE_HOUSE.lock().unwrap();
    add_floor(&mut *house);
}

fn print_the_house() {
    let house = THE_HOUSE.lock().unwrap();
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

pub fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    {
        let mut house = THE_HOUSE.lock().unwrap();
        house.bathrooms += 1.0;
    }
    print_the_house();
    {
        let mut house = THE_HOUSE.lock().unwrap();
        add_bedrooms(&mut *house, extra_bedrooms);
    }
    print_the_house();
}

fn parse_val(s: &str) -> Option<i32> {
    // Mimic strtol: parse a leading optional sign and digits, accept trailing
    // garbage, but require at least one digit to be consumed.
    let bytes = s.as_bytes();
    let mut idx = 0;

    // Skip leading whitespace (strtol behavior).
    while idx < bytes.len() && (bytes[idx] as char).is_whitespace() {
        idx += 1;
    }

    let start_after_ws = idx;

    // Optional sign.
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        idx += 1;
    }

    let digits_start = idx;
    while idx < bytes.len() && (bytes[idx] as char).is_ascii_digit() {
        idx += 1;
    }

    if idx == digits_start {
        // No digits consumed.
        return None;
    }

    let parsed = &s[start_after_ws..idx];
    match parsed.parse::<i64>() {
        Ok(tmp) if tmp >= i32::MIN as i64 && tmp <= i32::MAX as i64 => Some(tmp as i32),
        _ => None,
    }
}

/// # Safety
///
/// `input` must be a valid pointer to a NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn driver(input: *const c_char) {
    if input.is_null() {
        println!("An error occurred");
        return;
    }
    let cstr = CStr::from_ptr(input);
    let s = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => {
            println!("An error occurred");
            return;
        }
    };
    match parse_val(s) {
        Some(x) => {
            run(x);
            run(x);
        }
        None => {
            println!("An error occurred");
        }
    }
}
