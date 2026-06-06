// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

#![allow(non_camel_case_types)]

use std::ffi::c_char;
use std::os::raw::c_int;

#[repr(C)]
#[derive(Copy, Clone)]
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
    // Use libc::printf to guarantee byte-identical output (including locale,
    // floating-point formatting, etc.) compared to the original C program.
    let fmt = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    unsafe {
        libc::printf(
            fmt.as_ptr() as *const c_char,
            house.floors,
            house.bedrooms,
            house.bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(the_house: *mut house_t, extra_bedrooms: c_int) {
    if the_house.is_null() {
        return;
    }
    let house = unsafe { &mut *the_house };
    print_house(house);
    add_floor(house);
    print_house(house);
    house.bathrooms += 1.0;
    print_house(house);
    add_bedrooms(house, extra_bedrooms);
    print_house(house);
}

fn parse_val(s: *const c_char, val: &mut c_int) -> bool {
    unsafe {
        // Mirror the C: errno = 0; char *endp = (char *)str; long tmp = strtol(str, &endp, 10);
        // Reset errno.
        *libc::__errno_location() = 0;
        let mut endp: *mut c_char = s as *mut c_char;
        let tmp: libc::c_long = libc::strtol(s, &mut endp as *mut *mut c_char, 10);
        let errno_val = *libc::__errno_location();
        if endp != (s as *mut c_char)
            && errno_val == 0
            && tmp >= c_int::MIN as libc::c_long
            && tmp <= c_int::MAX as libc::c_long
        {
            *val = tmp as c_int;
            true
        } else {
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    let mut x: c_int = 0;
    if parse_val(input, &mut x) {
        let mut the_house = house_t {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house as *mut house_t, x);
        run(&mut the_house as *mut house_t, x);
    } else {
        let msg = b"An error occurred\n\0";
        unsafe {
            libc::printf(msg.as_ptr() as *const c_char);
        }
    }
}
