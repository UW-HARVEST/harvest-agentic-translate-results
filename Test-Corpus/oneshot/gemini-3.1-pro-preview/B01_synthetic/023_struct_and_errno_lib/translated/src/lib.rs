use std::ffi::{CStr, c_char};
use std::os::raw::c_int;

struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn print_house(house: &House) {
    println!("The house has {} floors, {} bedrooms, and {:.1} bathrooms", house.floors, house.bedrooms, house.bathrooms);
}

fn run(the_house: &mut House, extra_bedrooms: c_int) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    the_house.bathrooms += 1.0;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

fn parse_val(s: &CStr) -> Option<c_int> {
    let bytes = s.to_bytes();
    let mut start = 0;
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    if start == bytes.len() {
        return None;
    }
    let mut end = start;
    if bytes[end] == b'+' || bytes[end] == b'-' {
        end += 1;
    }
    let mut has_digits = false;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        has_digits = true;
        end += 1;
    }
    if !has_digits {
        return None;
    }
    let num_str = std::str::from_utf8(&bytes[start..end]).ok()?;
    num_str.parse::<c_int>().ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(in_ptr: *const c_char) {
    if in_ptr.is_null() {
        println!("An error occurred");
        return;
    }
    let c_str = unsafe { CStr::from_ptr(in_ptr) };
    if let Some(x) = parse_val(c_str) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        println!("An error occurred");
    }
}
