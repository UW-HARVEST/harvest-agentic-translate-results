// Copyright 2025 MIT Lincoln Laboratory
// Rust translation that reproduces the byte-identical output of the original C.

use std::ffi::c_char;
use std::ffi::c_double;
use std::ffi::c_int;
use std::ffi::c_long;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

unsafe extern "C" {
    // Use libc's printf so the formatting (%.1f, %d, etc.) is byte-identical to C.
    fn printf(fmt: *const c_char, ...) -> c_int;

    // Use libc's strtol so parsing semantics (errno, overflow handling, base 10
    // detection, leading whitespace, sign handling, etc.) match C exactly.
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;

    // Access errno via __errno_location (glibc) for behavior identical to <errno.h>.
    fn __errno_location() -> *mut c_int;
}

fn add_floor(house: &mut HouseT) {
    // Use wrapping arithmetic to match C's signed integer wrap-around in
    // practice (gcc/clang implement signed overflow as 2's-complement wrap
    // by default in this code path).
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut HouseT, extra_bedrooms: c_int) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn print_house(house: &HouseT) {
    // Format string identical to the C source.
    static FMT: &[u8] = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    unsafe {
        printf(
            FMT.as_ptr() as *const c_char,
            house.floors,
            house.bedrooms,
            house.bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(the_house: *mut HouseT, extra_bedrooms: c_int) {
    let the_house = unsafe { &mut *the_house };
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    the_house.bathrooms += 1.0;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

fn parse_val(str_ptr: *const c_char, val: &mut c_int) -> bool {
    unsafe {
        // errno = 0;
        *__errno_location() = 0;

        // char *endp = (char *)str;
        let mut endp: *mut c_char = str_ptr as *mut c_char;

        // long tmp = strtol(str, &endp, 10);
        let tmp: c_long = strtol(str_ptr, &mut endp as *mut *mut c_char, 10);

        // Match the C predicate exactly:
        //   endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX
        let errno_val = *__errno_location();
        if endp != (str_ptr as *mut c_char)
            && errno_val == 0
            && tmp >= c_int::MIN as c_long
            && tmp <= c_int::MAX as c_long
        {
            *val = tmp as c_int;
            true
        } else {
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_ptr: *const c_char) {
    let mut x: c_int = 0;
    if parse_val(in_ptr, &mut x) {
        let mut the_house = HouseT {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        unsafe {
            run(&mut the_house as *mut HouseT, x);
            run(&mut the_house as *mut HouseT, x);
        }
    } else {
        static MSG: &[u8] = b"An error occurred\n\0";
        unsafe {
            printf(MSG.as_ptr() as *const c_char);
        }
    }
}
