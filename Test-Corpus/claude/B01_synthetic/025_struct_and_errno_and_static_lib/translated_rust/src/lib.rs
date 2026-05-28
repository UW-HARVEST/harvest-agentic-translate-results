// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_char;
use std::sync::Mutex;

#[derive(Clone, Copy)]
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
    let mut house = THE_HOUSE.lock().unwrap();
    add_floor(&mut house);
}

fn print_the_house() {
    let house = THE_HOUSE.lock().unwrap();
    // Use libc::printf to match C's printf formatting byte-for-byte.
    let fmt = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    unsafe {
        libc::printf(
            fmt.as_ptr() as *const c_char,
            house.floors as libc::c_int,
            house.bedrooms as libc::c_int,
            house.bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: libc::c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    {
        let mut house = THE_HOUSE.lock().unwrap();
        house.bathrooms += 1.0;
    }
    print_the_house();
    {
        let mut house = THE_HOUSE.lock().unwrap();
        add_bedrooms(&mut house, extra_bedrooms as i32);
    }
    print_the_house();
}

fn parse_val(s: *const c_char) -> Option<i32> {
    if s.is_null() {
        return None;
    }
    unsafe {
        // Reset errno to 0 before strtol, mirroring the C code.
        *libc::__errno_location() = 0;
        let mut endp: *mut c_char = s as *mut c_char;
        let tmp = libc::strtol(s, &mut endp as *mut *mut c_char, 10);
        let errno = *libc::__errno_location();
        if endp != (s as *mut c_char)
            && errno == 0
            && tmp >= libc::c_int::MIN as libc::c_long
            && tmp <= libc::c_int::MAX as libc::c_long
        {
            Some(tmp as i32)
        } else {
            None
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    match parse_val(input) {
        Some(x) => {
            run(x as libc::c_int);
            run(x as libc::c_int);
        }
        None => {
            let msg = b"An error occurred\n\0";
            unsafe {
                libc::printf(msg.as_ptr() as *const c_char);
            }
        }
    }
}
