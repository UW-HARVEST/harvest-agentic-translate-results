// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust to produce byte-identical output to the C version.

use std::ffi::c_int;
use std::sync::Mutex;

#[derive(Clone, Copy)]
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
    let mut h = THE_HOUSE.lock().unwrap();
    add_floor(&mut h);
}

fn print_the_house() {
    let h = THE_HOUSE.lock().unwrap();
    // Use libc::printf to ensure byte-identical output (same buffering and
    // formatting semantics as the original C code, including %.1f rounding).
    let fmt = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    unsafe {
        libc::printf(
            fmt.as_ptr() as *const libc::c_char,
            h.floors as c_int,
            h.bedrooms as c_int,
            h.bathrooms as libc::c_double,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    {
        let mut h = THE_HOUSE.lock().unwrap();
        h.bathrooms += 1.0;
    }
    print_the_house();
    {
        let mut h = THE_HOUSE.lock().unwrap();
        add_bedrooms(&mut h, extra_bedrooms);
    }
    print_the_house();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    run(x);
    run(x);
}
