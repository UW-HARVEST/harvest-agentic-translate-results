// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust from the original C source.

use std::ffi::c_char;
use std::os::raw::{c_double, c_int, c_long};

#[repr(C)]
pub struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
    fn __errno_location() -> *mut c_int;
}

#[inline]
unsafe fn errno_get() -> c_int {
    *__errno_location()
}

#[inline]
unsafe fn errno_set(v: c_int) {
    *__errno_location() = v;
}

unsafe fn add_floor(house: *mut HouseT) {
    (*house).floors += 1;
}

unsafe fn add_bedrooms(house: *mut HouseT, extra_bedrooms: c_int) {
    (*house).bedrooms += extra_bedrooms;
}

unsafe fn print_house(house: *mut HouseT) {
    let fmt = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    printf(
        fmt.as_ptr() as *const c_char,
        (*house).floors,
        (*house).bedrooms,
        (*house).bathrooms,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(the_house: *mut HouseT, extra_bedrooms: c_int) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    (*the_house).bathrooms += 1.0;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

unsafe fn parse_val(s: *const c_char, val: *mut c_int) -> bool {
    errno_set(0);
    let mut endp: *mut c_char = s as *mut c_char;
    let tmp: c_long = strtol(s, &mut endp as *mut *mut c_char, 10);
    if endp != (s as *mut c_char)
        && errno_get() == 0
        && tmp >= c_int::MIN as c_long
        && tmp <= c_int::MAX as c_long
    {
        *val = tmp as c_int;
        true
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    let mut x: c_int = 0;
    if parse_val(input, &mut x) {
        let mut the_house = HouseT {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house as *mut HouseT, x);
        run(&mut the_house as *mut HouseT, x);
    } else {
        let msg = b"An error occurred\n\0";
        printf(msg.as_ptr() as *const c_char);
    }
}
