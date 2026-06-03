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

use std::io::{self, Read};

#[derive(Copy, Clone)]
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

fn add_floor(house: *mut House) {
    unsafe {
        (*house).floors += 1;
    }
}

fn add_bedrooms(house: *mut House, extra_bedrooms: i32) {
    unsafe {
        (*house).bedrooms += extra_bedrooms;
    }
}

fn add_floor_to_the_house() {
    add_floor(&raw mut THE_HOUSE);
}

fn print_the_house() {
    let house_ptr: *const House = &raw const THE_HOUSE;
    unsafe {
        let h = &*house_ptr;
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
        (*(&raw mut THE_HOUSE)).bathrooms += 1.0;
    }
    print_the_house();
    add_bedrooms(&raw mut THE_HOUSE, extra_bedrooms);
    print_the_house();
}

fn main() {
    // Match scanf("%d", &x) behavior: read an integer from stdin.
    // If parsing fails, x remains 0 (matching the C program where x = 0
    // and scanf would leave it unchanged on failure).
    let mut x: i32 = 0;

    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    if let Some(token) = input.split_whitespace().next() {
        if let Ok(parsed) = token.parse::<i32>() {
            x = parsed;
        }
    }

    run(x);
    run(x);
}
