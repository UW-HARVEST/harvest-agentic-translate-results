use std::ffi::{c_char, c_int, CStr};

#[repr(C)]
pub struct house_t {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: f64,
}

fn add_floor(house: &mut house_t) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut house_t, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn print_house(house: &house_t) {
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn run(the_house: *mut house_t, extra_bedrooms: c_int) {
    let house = unsafe { &mut *the_house };
    print_house(house);
    add_floor(house);
    print_house(house);
    house.bathrooms += 1.0;
    print_house(house);
    add_bedrooms(house, extra_bedrooms);
    print_house(house);
}

fn parse_val(s: &str) -> Option<c_int> {
    // Replicate C strtol behavior: parse leading integer, ignore trailing chars.
    // strtol skips leading whitespace, then reads optional sign + digits.
    let trimmed = s.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    // Find the prefix that forms a valid integer (optional sign + digits)
    let mut chars = trimmed.chars();
    let first = chars.next().unwrap();
    let start = if first == '+' || first == '-' { 1 } else { 0 };
    let digits_start = if start == 1 && !chars.next().map_or(false, |c| c.is_ascii_digit()) {
        return None;
    } else {
        start
    };
    let _ = digits_start;

    // Find end of digit run
    let num_end = if first == '+' || first == '-' {
        1 + trimmed[1..].find(|c: char| !c.is_ascii_digit()).unwrap_or(trimmed.len() - 1)
    } else {
        trimmed.find(|c: char| !c.is_ascii_digit()).unwrap_or(trimmed.len())
    };

    if num_end == start {
        // No digits found after sign
        return None;
    }

    let num_str = &trimmed[..num_end];
    // Parse as i64 first to check range like C's strtol with INT_MIN/INT_MAX check
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
            println!("An error occurred");
            return;
        }
    };

    if let Some(x) = parse_val(s) {
        let mut the_house = house_t {
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
