use std::ffi::{c_char, c_int};

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

const HOUSE_FORMAT: &[u8] = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
const INTEGER_FORMAT: &[u8] = b"%d\0";

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn scanf(format: *const c_char, ...) -> c_int;
}

unsafe fn print_the_house(house: *const House) {
    printf(
        HOUSE_FORMAT.as_ptr().cast(),
        (*house).floors,
        (*house).bedrooms,
        (*house).bathrooms,
    );
}

fn run(extra_bedrooms: c_int) {
    unsafe {
        let house = &raw mut THE_HOUSE;

        print_the_house(house);
        (*house).floors = (*house).floors.wrapping_add(1);
        print_the_house(house);
        (*house).bathrooms += 1.0;
        print_the_house(house);
        (*house).bedrooms = (*house).bedrooms.wrapping_add(extra_bedrooms);
        print_the_house(house);
    }
}

fn main() {
    let mut x: c_int = 0;

    unsafe {
        scanf(INTEGER_FORMAT.as_ptr().cast(), &raw mut x);
    }
    run(x);
    run(x);
}
