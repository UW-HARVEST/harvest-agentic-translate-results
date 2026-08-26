// Translation of c_src/src/driver.c to Rust.
// Preserves byte-identical behavior, including the use of libc's printf and strtol
// to match exact output formatting and parsing semantics of the original C code.

use std::ffi::c_char;
use std::os::raw::{c_double, c_int, c_long};

#[repr(C)]
pub struct house_t {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: c_double,
}

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
    fn __errno_location() -> *mut c_int;
}

fn add_floor(house: &mut house_t) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut house_t, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn print_house(house: &house_t) {
    // printf format string: "The house has %d floors, %d bedrooms, and %.1f bathrooms\n"
    let fmt = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    unsafe {
        printf(
            fmt.as_ptr() as *const c_char,
            house.floors,
            house.bedrooms,
            house.bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(the_house: *mut house_t, extra_bedrooms: c_int) {
    let h = &mut *the_house;
    print_house(h);
    add_floor(h);
    print_house(h);
    h.bathrooms += 1.0;
    print_house(h);
    add_bedrooms(h, extra_bedrooms);
    print_house(h);
}

fn parse_val(s: *const c_char, val: *mut c_int) -> bool {
    unsafe {
        *__errno_location() = 0;
        let mut endp: *mut c_char = s as *mut c_char;
        let tmp: c_long = strtol(s, &mut endp as *mut *mut c_char, 10);
        if endp != (s as *mut c_char)
            && *__errno_location() == 0
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
pub unsafe extern "C" fn driver(in_: *const c_char) {
    let mut x: c_int = 0;
    if parse_val(in_, &mut x as *mut c_int) {
        let mut the_house = house_t {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house as *mut house_t, x);
        run(&mut the_house as *mut house_t, x);
    } else {
        let msg = b"An error occurred\n\0";
        printf(msg.as_ptr() as *const c_char);
    }
}
