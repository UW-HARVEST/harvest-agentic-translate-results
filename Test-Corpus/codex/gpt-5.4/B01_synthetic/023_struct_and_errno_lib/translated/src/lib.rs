use libc::{INT_MAX, INT_MIN};
use std::ffi::{c_char, c_double, c_int};

#[repr(C)]
pub struct house_t {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

fn add_floor(house: &mut house_t) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut house_t, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn print_house(house: &mut house_t) {
    unsafe {
        libc::printf(
            c"The house has %d floors, %d bedrooms, and %.1f bathrooms\n".as_ptr(),
            house.floors,
            house.bedrooms,
            house.bathrooms,
        );
    }
}

fn parse_val(str_: *const c_char, val: &mut c_int) -> bool {
    unsafe {
        *libc::__errno_location() = 0;
        let mut endp = str_ as *mut c_char;
        let tmp = libc::strtol(str_, &mut endp, 10);
        if endp != str_ as *mut c_char && *libc::__errno_location() == 0 && tmp >= INT_MIN.into() && tmp <= INT_MAX.into() {
            *val = tmp as c_int;
            true
        } else {
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(the_house: *mut house_t, extra_bedrooms: c_int) {
    let the_house = unsafe { &mut *the_house };
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    the_house.bathrooms += 1.0;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(in_: *const c_char) {
    let mut x = 0;
    if parse_val(in_, &mut x) {
        let mut the_house = house_t {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        unsafe {
            libc::printf(c"An error occurred\n".as_ptr());
        }
    }
}
