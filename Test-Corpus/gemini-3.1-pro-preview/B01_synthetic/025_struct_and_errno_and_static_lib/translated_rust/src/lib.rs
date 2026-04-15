use std::ffi::{c_char, CStr};
use std::sync::Mutex;

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
    if let Ok(mut house) = THE_HOUSE.lock() {
        add_floor(&mut house);
    }
}

fn print_the_house() {
    if let Ok(house) = THE_HOUSE.lock() {
        println!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
            house.floors, house.bedrooms, house.bathrooms
        );
    }
}

fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    if let Ok(mut house) = THE_HOUSE.lock() {
        house.bathrooms += 1.0;
    }
    print_the_house();
    if let Ok(mut house) = THE_HOUSE.lock() {
        add_bedrooms(&mut house, extra_bedrooms);
    }
    print_the_house();
}

fn parse_val(s: &str) -> Option<i32> {
    let s = s.trim_start();
    let mut end = 0;
    let mut has_digits = false;
    for (i, c) in s.char_indices() {
        if i == 0 && (c == '+' || c == '-') {
            end += 1;
        } else if c.is_ascii_digit() {
            has_digits = true;
            end += 1;
        } else {
            break;
        }
    }
    if has_digits {
        s[..end].parse::<i32>().ok()
    } else {
        None
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(in_str: *const c_char) {
    if in_str.is_null() {
        println!("An error occurred");
        return;
    }
    let c_str = unsafe { CStr::from_ptr(in_str) };
    let s = c_str.to_string_lossy();

    if let Some(x) = parse_val(&s) {
        run(x);
        run(x);
    } else {
        println!("An error occurred");
    }
}
