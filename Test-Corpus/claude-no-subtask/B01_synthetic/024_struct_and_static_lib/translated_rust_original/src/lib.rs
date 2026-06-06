// Translation of c_src/src/driver.c to Rust
// Produces byte-identical output to the original C library.

use std::ffi::c_int;
use std::os::raw::c_double;

#[repr(C)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

static mut THE_HOUSE: HouseT = HouseT {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

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
    // Use libc::printf to match the exact formatting of the C version
    // (e.g., %.1f rounding behavior) for byte-identical output.
    let fmt = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    unsafe {
        let h = &*std::ptr::addr_of!(THE_HOUSE);
        libc::printf(
            fmt.as_ptr() as *const i8,
            h.floors as c_int,
            h.bedrooms as c_int,
            h.bathrooms as c_double,
        );
    }
}

fn run_impl(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        let h = &mut *std::ptr::addr_of_mut!(THE_HOUSE);
        h.bathrooms += 1.0;
    }
    print_the_house();
    unsafe {
        let h = &mut *std::ptr::addr_of_mut!(THE_HOUSE);
        add_bedrooms(h, extra_bedrooms);
    }
    print_the_house();
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    run_impl(extra_bedrooms);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    run_impl(x);
    run_impl(x);
}
