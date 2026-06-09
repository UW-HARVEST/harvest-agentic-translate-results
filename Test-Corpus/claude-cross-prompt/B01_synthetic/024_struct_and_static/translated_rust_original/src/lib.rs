// Translated from c_src/src/main.c
// Library crate exposing the public `run` function with byte-identical output.

use std::ffi::c_int;
use std::os::raw::c_char;

#[repr(C)]
#[derive(Copy, Clone)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

static mut THE_HOUSE: HouseT = HouseT {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn add_floor(house: &mut HouseT) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut HouseT, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    unsafe {
        add_floor(&mut *std::ptr::addr_of_mut!(THE_HOUSE));
    }
}

fn print_the_house() {
    unsafe {
        let h = &*std::ptr::addr_of!(THE_HOUSE);
        let fmt = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
        printf(
            fmt.as_ptr() as *const c_char,
            h.floors,
            h.bedrooms,
            h.bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        let h = &mut *std::ptr::addr_of_mut!(THE_HOUSE);
        h.bathrooms += 1.0;
    }
    print_the_house();
    unsafe {
        add_bedrooms(&mut *std::ptr::addr_of_mut!(THE_HOUSE), extra_bedrooms);
    }
    print_the_house();
}
