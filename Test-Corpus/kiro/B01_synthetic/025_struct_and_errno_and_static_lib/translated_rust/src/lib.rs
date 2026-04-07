use std::ffi::{c_char, c_int, CStr};
use std::sync::Mutex;

struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

static THE_HOUSE: Mutex<House> = Mutex::new(House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
});

fn add_floor(house: &mut House) {
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: c_int) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn print_the_house(house: &House) {
    print!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        house.floors, house.bedrooms, house.bathrooms
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    let mut house = THE_HOUSE.lock().unwrap();
    print_the_house(&house);
    add_floor(&mut house);
    print_the_house(&house);
    house.bathrooms += 1.0;
    print_the_house(&house);
    add_bedrooms(&mut house, extra_bedrooms);
    print_the_house(&house);
}

fn parse_val(s: &[u8]) -> Option<c_int> {
    // Replicate C strtol behavior: skip leading whitespace, parse base-10 integer,
    // succeed if at least one digit was consumed and result fits in c_int range.
    let str = std::str::from_utf8(s).ok()?;
    let trimmed = str.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    // Determine sign and digit start
    let (negative, digits_start) = if trimmed.starts_with('-') {
        (true, &trimmed[1..])
    } else if trimmed.starts_with('+') {
        (false, &trimmed[1..])
    } else {
        (false, trimmed)
    };

    // Must have at least one digit (endp != str check)
    if digits_start.is_empty() || !digits_start.as_bytes()[0].is_ascii_digit() {
        return None;
    }

    // Parse digits, detecting overflow like strtol (sets errno=ERANGE)
    let mut result: i64 = 0;
    let mut overflowed = false;
    for &b in digits_start.as_bytes() {
        if !b.is_ascii_digit() {
            break;
        }
        result = result.wrapping_mul(10).wrapping_add((b - b'0') as i64);
        if result > i64::from(i32::MAX) + 1 {
            overflowed = true;
        }
    }

    if negative {
        result = -result;
    }

    if overflowed {
        return None; // errno == ERANGE equivalent
    }

    // Range check: tmp >= INT_MIN && tmp <= INT_MAX
    if result < c_int::MIN as i64 || result > c_int::MAX as i64 {
        return None;
    }

    Some(result as c_int)
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    let cstr = unsafe { CStr::from_ptr(input) };
    match parse_val(cstr.to_bytes()) {
        Some(x) => {
            run(x);
            run(x);
        }
        None => {
            print!("An error occurred\n");
        }
    }
}
