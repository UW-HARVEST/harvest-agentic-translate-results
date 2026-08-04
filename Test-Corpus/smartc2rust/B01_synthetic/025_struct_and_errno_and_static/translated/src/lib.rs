

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::BufRead;

// Fallback definition of `house_t` in case bindgen did not export it
// (e.g., when the C typedef is only visible inside main.c). This mirrors
// the C definition exactly so it is layout-compatible with the C side.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct rust_house_t {
    pub floors: std::os::raw::c_int,
    pub bedrooms: std::os::raw::c_int,
    pub bathrooms: std::os::raw::c_double,
}

// Since `the_house` in C is declared `static`, it is not exported and cannot
// be linked from Rust. Maintain the state entirely on the Rust side using a
// thread-safe wrapper (Mutex) to avoid using unsafe globals.
use std::sync::Mutex;

fn rust_the_house() -> &'static Mutex<rust_house_t> {
    static INSTANCE: std::sync::OnceLock<Mutex<rust_house_t>> = std::sync::OnceLock::new();
    INSTANCE.get_or_init(|| {
        Mutex::new(rust_house_t {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        })
    })
}

pub fn rust_get_the_house() -> rust_house_t {
    *rust_the_house().lock().unwrap()
}

pub fn rust_set_the_house(val: rust_house_t) {
    *rust_the_house().lock().unwrap() = val;
}


fn rust_add_bedrooms(house: &mut rust_house_t, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn rust_add_floor(house: &mut rust_house_t) {
    house.floors += 1;
}

fn rust_add_floor_to_the_house() {
    let mut h = rust_get_the_house();
    rust_add_floor(&mut h);
    rust_set_the_house(h);
}

fn rust_print_the_house() {
    let h = rust_get_the_house();
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        h.floors, h.bedrooms, h.bathrooms
    );
}

pub fn rust_run(extra_bedrooms: i32) {
    rust_print_the_house();
    rust_add_floor_to_the_house();
    rust_print_the_house();

    let mut h = rust_get_the_house();
    h.bathrooms += 1.0;
    rust_set_the_house(h);
    rust_print_the_house();

    let mut h = rust_get_the_house();
    rust_add_bedrooms(&mut h, extra_bedrooms);
    rust_set_the_house(h);
    rust_print_the_house();
}

fn rust_parse_val(s: &str) -> Option<i32> {
    // Mimic strtol behavior: skip leading whitespace, parse the longest valid
    // numeric prefix (optional sign + digits) and validate against i32 range.
    let trimmed = s.trim_start();
    let mut end = 0usize;
    let bytes = trimmed.as_bytes();

    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    let digits_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if digits_start == end {
        return None;
    }

    trimmed[..end].parse::<i64>().ok().and_then(|v| i32::try_from(v).ok())
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> std::os::raw::c_int {
    let mut input = String::new();
    let _ = std::io::stdin().lock().read_line(&mut input);

    // Match fgets with a 100-byte buffer (99 chars + NUL terminator).
    if input.len() > 99 {
        input.truncate(99);
    }

    match rust_parse_val(&input) {
        Some(x) => {
            rust_run(x);
            rust_run(x);
        }
        None => {
            println!("An error occurred");
        }
    }
    0
}

