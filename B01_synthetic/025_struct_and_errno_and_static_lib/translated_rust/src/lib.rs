use std::ffi::{c_char, c_int, CStr};
use std::ptr::addr_of_mut;

struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    unsafe {
        add_floor(&mut *addr_of_mut!(THE_HOUSE));
    }
}

fn print_the_house() {
    unsafe {
        let h = &*addr_of_mut!(THE_HOUSE);
        print!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
            h.floors, h.bedrooms, h.bathrooms
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        (*addr_of_mut!(THE_HOUSE)).bathrooms += 1.0;
    }
    print_the_house();
    unsafe {
        add_bedrooms(&mut *addr_of_mut!(THE_HOUSE), extra_bedrooms);
    }
    print_the_house();
}

fn parse_val(s: &[u8]) -> Option<c_int> {
    let mut i = 0;
    while i < s.len() && s[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= s.len() {
        return None;
    }
    let negative = if s[i] == b'-' {
        i += 1;
        true
    } else {
        if s[i] == b'+' {
            i += 1;
        }
        false
    };
    let start = i;
    let mut val: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        val = match val.checked_mul(10).and_then(|v| v.checked_add((s[i] - b'0') as i64)) {
            Some(v) => v,
            None => {
                overflow = true;
                break;
            }
        };
        i += 1;
    }
    if i == start || overflow {
        return None;
    }
    let val = if negative { -val } else { val };
    if val < c_int::MIN as i64 || val > c_int::MAX as i64 {
        return None;
    }
    Some(val as c_int)
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    let cstr = unsafe { CStr::from_ptr(input) };
    let bytes = cstr.to_bytes();
    match parse_val(bytes) {
        Some(x) => {
            run(x);
            run(x);
        }
        None => {
            print!("An error occurred\n");
        }
    }
}
