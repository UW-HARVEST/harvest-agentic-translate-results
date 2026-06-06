// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust.

use std::ffi::c_char;
use std::os::raw::c_int;
use std::ptr;
use std::sync::Mutex;

#[repr(C)]
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
    add_floor(&mut *h);
}

fn print_the_house() {
    let h = THE_HOUSE.lock().unwrap();
    let floors = h.floors;
    let bedrooms = h.bedrooms;
    let bathrooms = h.bathrooms;
    drop(h);
    // Use libc::printf to match C output formatting exactly.
    let fmt = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    unsafe {
        libc::printf(
            fmt.as_ptr() as *const c_char,
            floors as c_int,
            bedrooms as c_int,
            bathrooms,
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
        add_bedrooms(&mut *h, extra_bedrooms);
    }
    print_the_house();
}

/// Mirrors the C parse_val: uses strtol with errno checking and INT range checks.
/// Returns Some(value) on success, None on failure.
fn parse_val(s: *const c_char) -> Option<c_int> {
    unsafe {
        // Set errno = 0
        *libc::__errno_location() = 0;
        let mut endp: *mut c_char = s as *mut c_char;
        let tmp: libc::c_long = libc::strtol(s, &mut endp as *mut *mut c_char, 10);
        let errno_val = *libc::__errno_location();
        if endp != (s as *mut c_char)
            && errno_val == 0
            && tmp >= c_int::MIN as libc::c_long
            && tmp <= c_int::MAX as libc::c_long
        {
            Some(tmp as c_int)
        } else {
            None
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    if input.is_null() {
        // The C code would crash; replicate by also returning early to avoid UB.
        // But to match exact behavior on valid inputs, we proceed only if non-null.
        return;
    }
    match parse_val(input) {
        Some(x) => {
            run(x);
            run(x);
        }
        None => {
            let msg = b"An error occurred\n\0";
            unsafe {
                libc::printf(msg.as_ptr() as *const c_char);
            }
        }
    }
    // Suppress unused warning
    let _ = ptr::null::<()>();
}
