// Rust translation of c_src/src/driver.c
//
// Original copyright 2025 MIT Lincoln Laboratory (MIT license); see c_src for
// the full notice. Behavior is reproduced exactly, including the shared
// mutable global state that persists across calls.

use std::ffi::{c_char, c_double, c_int};

unsafe extern "C" {
    // Variadic C printf, used so formatting and stdout buffering match the C
    // original byte-for-byte.
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

// static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};
static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

/// static void add_floor(house_t *house)
fn add_floor(house: &mut House) {
    house.floors = house.floors.wrapping_add(1);
}

/// static void add_bedrooms(house_t *house, int extra_bedrooms)
fn add_bedrooms(house: &mut House, extra_bedrooms: c_int) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

/// Borrow the single global house instance.
fn the_house() -> &'static mut House {
    unsafe { &mut *(&raw mut THE_HOUSE) }
}

/// static void add_floor_to_the_house()
fn add_floor_to_the_house() {
    add_floor(the_house());
}

/// static void print_the_house()
fn print_the_house() {
    let house = the_house();
    unsafe {
        c_printf(
            c"The house has %d floors, %d bedrooms, and %.1f bathrooms\n".as_ptr(),
            house.floors,
            house.bedrooms,
            house.bathrooms,
        );
    }
}

/// void run(int extra_bedrooms)
#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    the_house().bathrooms += 1.0;
    print_the_house();
    add_bedrooms(the_house(), extra_bedrooms);
    print_the_house();
}

/// void driver(int x)
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    run(x);
    run(x);
}
