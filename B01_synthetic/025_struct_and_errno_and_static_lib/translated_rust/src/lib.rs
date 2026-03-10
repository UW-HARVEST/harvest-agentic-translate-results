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
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    add_floor(&mut THE_HOUSE.lock().unwrap());
}

fn print_the_house() {
    let h = THE_HOUSE.lock().unwrap();
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        h.floors, h.bedrooms, h.bathrooms
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    THE_HOUSE.lock().unwrap().bathrooms += 1.0;
    print_the_house();
    add_bedrooms(&mut THE_HOUSE.lock().unwrap(), extra_bedrooms);
    print_the_house();
}

fn parse_val(s: &[u8]) -> Option<c_int> {
    // Skip leading whitespace to match strtol behavior
    let s = match std::str::from_utf8(s.split(|&b| b == 0).next().unwrap_or(s)) {
        Ok(s) => s.trim_start(),
        Err(_) => return None,
    };
    if s.is_empty() {
        return None;
    }
    // strtol parses as long; we check endp != str (i.e. something was parsed),
    // errno == 0 (no overflow), and INT_MIN <= tmp <= INT_MAX
    match s.parse::<i64>() {
        Ok(tmp) if tmp >= c_int::MIN as i64 && tmp <= c_int::MAX as i64 => Some(tmp as c_int),
        _ => {
            // Try parsing just the leading numeric portion (strtol stops at first non-digit)
            let end = if s.starts_with('-') || s.starts_with('+') {
                1 + s[1..].find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len() - 1)
            } else {
                s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len())
            };
            if end == 0 || (end == 1 && (s.starts_with('-') || s.starts_with('+'))) {
                return None; // endp == str
            }
            match s[..end].parse::<i64>() {
                Ok(tmp) if tmp >= c_int::MIN as i64 && tmp <= c_int::MAX as i64 => {
                    Some(tmp as c_int)
                }
                _ => None,
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    let cstr = unsafe { CStr::from_ptr(input) };
    if let Some(x) = parse_val(cstr.to_bytes_with_nul()) {
        run(x);
        run(x);
    } else {
        println!("An error occurred");
    }
}
