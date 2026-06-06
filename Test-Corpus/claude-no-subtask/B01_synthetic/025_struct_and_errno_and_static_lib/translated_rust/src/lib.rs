// Translation of c_src/src/driver.c to Rust.
// Produces byte-identical output to the C version by using libc's
// printf and strtol directly.

use std::ffi::c_char;
use std::os::raw::c_int;
use std::sync::Mutex;

#[derive(Clone, Copy)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

// Equivalent to C's `static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};`
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
    // Use libc::printf to match C's stdio formatting byte-for-byte.
    let h = THE_HOUSE.lock().unwrap();
    let fmt = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    unsafe {
        libc::printf(
            fmt.as_ptr() as *const c_char,
            h.floors,
            h.bedrooms,
            h.bathrooms,
        );
    }
}

fn run(extra_bedrooms: c_int) {
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

// Mimics the C parse_val function exactly using strtol from libc.
fn parse_val(s: *const c_char, val: &mut c_int) -> bool {
    unsafe {
        // errno = 0
        *libc::__errno_location() = 0;
        let mut endp: *mut c_char = s as *mut c_char;
        let tmp = libc::strtol(s, &mut endp as *mut *mut c_char, 10);
        let errno_val = *libc::__errno_location();
        if endp != s as *mut c_char
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
pub unsafe extern "C" fn driver(input: *const c_char) {
    let mut x: c_int = 0;
    if parse_val(input, &mut x) {
        run(x);
        run(x);
    } else {
        let msg = b"An error occurred\n\0";
        unsafe {
            libc::printf(msg.as_ptr() as *const c_char);
        }
    }
}
