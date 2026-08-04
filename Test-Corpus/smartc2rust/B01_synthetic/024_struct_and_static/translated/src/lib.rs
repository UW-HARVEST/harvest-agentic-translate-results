
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::{self, Read};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct house_t {
    pub floors: std::os::raw::c_int,
    pub bedrooms: std::os::raw::c_int,
    pub bathrooms: f64,
}

use std::sync::Mutex;

static THE_HOUSE: Mutex<house_t> = Mutex::new(house_t {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
});

pub fn rust_get_the_house() -> house_t {
    *THE_HOUSE.lock().unwrap()
}

pub fn rust_set_the_house(val: house_t) {
    *THE_HOUSE.lock().unwrap() = val;
}


fn rust_add_bedrooms(house: &mut house_t, extra_bedrooms: std::os::raw::c_int) {
    house.bedrooms += extra_bedrooms;
}

fn rust_add_floor(house: &mut house_t) {
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

fn rust_run(extra_bedrooms: std::os::raw::c_int) {
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

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> std::os::raw::c_int {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return 0;
    }
    let x: std::os::raw::c_int = input
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    rust_run(x);
    rust_run(x);
    0
}

