// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// The C library builds `src/driver.c` into a shared object exporting exactly
// two public symbols: `run` and `driver`. Everything else in the translation
// unit is `static` (internal) and is reproduced here as private Rust items.
//
// Behaviour notes preserved from the C:
//   * `the_house` is a mutable file-scope global initialised to
//     {floors = 2, bedrooms = 5, bathrooms = 2.5}. Its state persists across
//     calls, so repeated calls to `run`/`driver` keep mutating the same house.
//   * `driver(x)` calls `run(x)` twice.
//   * Output goes through C `printf` so that stdio buffering / interleaving
//     with any C caller is byte-for-byte identical.

#![allow(non_camel_case_types)]

use std::ffi::c_char;
use std::ffi::c_double;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct house_t {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

/// `static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};`
static mut THE_HOUSE: house_t = house_t {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

/// `static void add_floor(house_t *house) { house->floors++; }`
unsafe fn add_floor(house: *mut house_t) {
    (*house).floors = (*house).floors.wrapping_add(1);
}

/// `static void add_bedrooms(house_t *house, int extra_bedrooms)`
unsafe fn add_bedrooms(house: *mut house_t, extra_bedrooms: c_int) {
    (*house).bedrooms = (*house).bedrooms.wrapping_add(extra_bedrooms);
}

/// `static void add_floor_to_the_house() { add_floor(&the_house); }`
unsafe fn add_floor_to_the_house() {
    add_floor(std::ptr::addr_of_mut!(THE_HOUSE));
}

/// `static void print_the_house()`
unsafe fn print_the_house() {
    const FMT: &[u8] = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    let h = std::ptr::addr_of!(THE_HOUSE);
    printf(
        FMT.as_ptr() as *const c_char,
        (*h).floors,
        (*h).bedrooms,
        (*h).bathrooms,
    );
}

/// `void run(int extra_bedrooms)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    let h = std::ptr::addr_of_mut!(THE_HOUSE);
    (*h).bathrooms += 1.0;
    print_the_house();
    add_bedrooms(std::ptr::addr_of_mut!(THE_HOUSE), extra_bedrooms);
    print_the_house();
}

/// `void driver(int x)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_int) {
    run(x);
    run(x);
}
