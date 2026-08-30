use std::ffi::{c_char, c_double, c_int};

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

const HOUSE_FORMAT: &[u8] =
    b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

unsafe fn print_the_house(house: *const House) {
    unsafe {
        printf(
            HOUSE_FORMAT.as_ptr().cast(),
            (*house).floors,
            (*house).bedrooms,
            (*house).bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    let house = &raw mut THE_HOUSE;

    unsafe {
        print_the_house(house);
        (*house).floors += 1;
        print_the_house(house);
        (*house).bathrooms += 1.0;
        print_the_house(house);
        (*house).bedrooms += extra_bedrooms;
        print_the_house(house);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    run(x);
    run(x);
}
