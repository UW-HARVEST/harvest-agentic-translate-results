// Rust translation of c_src/src/driver.c
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::cell::UnsafeCell;
use std::ffi::{c_char, c_double, c_int};

// `printf` from libc is used directly so that formatting (`%d`, `%.1f`) and
// stdout buffering behaviour are byte-for-byte identical to the C original.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Mirrors the C `house_t` struct.
#[repr(C)]
#[derive(Clone, Copy)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

/// Wrapper allowing a mutable file-scope global, matching the C `static house_t`.
/// The C code is not thread safe either; this reproduces its semantics.
struct Global(UnsafeCell<House>);
unsafe impl Sync for Global {}

/// `static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};`
static THE_HOUSE: Global = Global(UnsafeCell::new(House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
}));

#[inline]
fn the_house() -> &'static mut House {
    // Single, process-wide instance, exactly like the C file-scope object.
    unsafe { &mut *THE_HOUSE.0.get() }
}

/// `static void add_floor(house_t *house)`
fn add_floor(house: &mut House) {
    // C `house->floors++`; wrapping matches the observed behaviour of the
    // original on overflow rather than panicking.
    house.floors = house.floors.wrapping_add(1);
}

/// `static void add_bedrooms(house_t *house, int extra_bedrooms)`
fn add_bedrooms(house: &mut House, extra_bedrooms: c_int) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

/// `static void add_floor_to_the_house()`
fn add_floor_to_the_house() {
    add_floor(the_house());
}

/// `static void print_the_house()`
fn print_the_house() {
    let house = the_house();
    unsafe {
        printf(
            c"The house has %d floors, %d bedrooms, and %.1f bathrooms\n".as_ptr(),
            house.floors,
            house.bedrooms,
            house.bathrooms,
        );
    }
}

/// `void run(int extra_bedrooms)`
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

/// `void driver(int x)`
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    run(x);
    run(x);
}
