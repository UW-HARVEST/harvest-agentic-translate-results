use std::ffi::{c_char, c_int};

#[repr(C)]
#[derive(Copy, Clone)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

fn add_floor(house: *mut House) {
    unsafe {
        (*house).floors += 1;
    }
}

fn add_bedrooms(house: *mut House, extra_bedrooms: c_int) {
    unsafe {
        (*house).bedrooms += extra_bedrooms;
    }
}

fn add_floor_to_the_house() {
    add_floor(&raw mut THE_HOUSE);
}

fn print_the_house() {
    unsafe {
        printf(
            c"The house has %d floors, %d bedrooms, and %.1f bathrooms\n".as_ptr(),
            THE_HOUSE.floors,
            THE_HOUSE.bedrooms,
            THE_HOUSE.bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        THE_HOUSE.bathrooms += 1.0;
    }
    print_the_house();
    add_bedrooms(&raw mut THE_HOUSE, extra_bedrooms);
    print_the_house();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    run(x);
    run(x);
}
