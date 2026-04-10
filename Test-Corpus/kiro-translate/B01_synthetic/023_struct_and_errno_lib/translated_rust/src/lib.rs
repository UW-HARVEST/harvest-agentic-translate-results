use std::ffi::{c_char, c_int, CStr};

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
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn run(house: &mut House, extra_bedrooms: c_int) {
    print_house(house);
    add_floor(house);
    print_house(house);
    house.bathrooms += 1.0;
    print_house(house);
    add_bedrooms(house, extra_bedrooms);
    print_house(house);
}

fn parse_val(s: &str) -> Option<c_int> {
    // Replicate C strtol behavior: parse leading decimal integer,
    // must consume at least one char, result must fit in c_int.
    let trimmed = s.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    // Find the prefix that strtol would consume: optional sign then digits
    let mut chars = trimmed.chars();
    let mut end = 0;
    let first = chars.next().unwrap();
    if first == '+' || first == '-' {
        end += 1;
    } else if first.is_ascii_digit() {
        end += 1;
    } else {
        return None;
    }
    let digit_start = end - if first.is_ascii_digit() { 1 } else { 0 };
    for ch in chars {
        if ch.is_ascii_digit() {
            end += 1;
        } else {
            break;
        }
    }
    // Must have at least one digit
    if end - digit_start == 0 {
        // sign only, no digits — strtol returns 0 with endp == str
        // but endp == str means we return None
        return None;
    }

    // strtol parses as long; we check INT_MIN..=INT_MAX
    let substr = &trimmed[..end];
    match substr.parse::<i64>() {
        Ok(tmp) if tmp >= c_int::MIN as i64 && tmp <= c_int::MAX as i64 => Some(tmp as c_int),
        _ => None,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(input) };
    let s = c_str.to_str().unwrap_or("");
    if let Some(x) = parse_val(s) {
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
