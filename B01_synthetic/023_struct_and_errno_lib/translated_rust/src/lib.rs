use std::ffi::{c_char, c_int, CStr};

pub struct House {
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
    print!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
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
    // Replicate C strtol behavior: parse leading integer, ignore trailing chars
    let trimmed = s.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    // Find the longest leading substring that forms a valid integer
    let mut end = 0;
    for (i, c) in trimmed.char_indices() {
        if i == 0 && (c == '+' || c == '-') {
            end = i + c.len_utf8();
            continue;
        }
        if c.is_ascii_digit() {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 || (end == 1 && (trimmed.starts_with('+') || trimmed.starts_with('-'))) {
        return None;
    }
    let num_str = &trimmed[..end];
    // strtol parses as long; check for overflow into c_int range
    match num_str.parse::<i64>() {
        Ok(tmp) if tmp >= c_int::MIN as i64 && tmp <= c_int::MAX as i64 => Some(tmp as c_int),
        _ => None,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(input) };
    let s = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            print!("An error occurred\n");
            return;
        }
    };
    if let Some(x) = parse_val(s) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        print!("An error occurred\n");
    }
}
