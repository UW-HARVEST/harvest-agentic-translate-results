// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Translation of `c_src/src/driver.c`.

use core::cell::UnsafeCell;
use core::ffi::{c_char, c_double, c_int};

// `driver.c` prints via `printf` from <stdio.h>. We call the very same libc
// entry point rather than Rust's `std::io::stdout`, so that number formatting
// (`%d`, `%.1f`) and stdout buffering/flush-at-exit semantics are identical to
// the C library's, byte for byte.
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// C: `typedef struct { int floors; int bedrooms; double bathrooms; } house_t;`
#[repr(C)]
#[derive(Clone, Copy)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

/// Wrapper granting the file-scope `static house_t the_house` its C semantics:
/// a single mutable instance with process lifetime whose contents persist
/// across calls into the library.
///
/// Like the C original, this is not thread safe; the `unsafe impl Sync` simply
/// records that the C library made the same (un)guarantee.
struct TheHouse(UnsafeCell<HouseT>);

unsafe impl Sync for TheHouse {}

impl TheHouse {
    /// Raw pointer to the singleton, standing in for C's `&the_house`.
    #[inline]
    fn as_ptr(&self) -> *mut HouseT {
        self.0.get()
    }
}

/// C: `static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};`
static THE_HOUSE: TheHouse = TheHouse(UnsafeCell::new(HouseT {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
}));

/// C format string for `print_the_house`, NUL terminated for the variadic call.
const PRINT_FMT: &[u8] =
    b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";

/// C: `static void add_floor(house_t *house) { house->floors++; }`
///
/// `wrapping_add` reproduces the two's-complement result a C compiler emits for
/// `int` overflow instead of panicking; it is not a behavioral fix.
///
/// # Safety
/// `house` must point to a valid `HouseT`, as in the C original.
unsafe fn add_floor(house: *mut HouseT) {
    (*house).floors = (*house).floors.wrapping_add(1);
}

/// C: `static void add_bedrooms(house_t *house, int extra_bedrooms) { house->bedrooms += extra_bedrooms; }`
///
/// # Safety
/// `house` must point to a valid `HouseT`, as in the C original.
unsafe fn add_bedrooms(house: *mut HouseT, extra_bedrooms: c_int) {
    (*house).bedrooms = (*house).bedrooms.wrapping_add(extra_bedrooms);
}

/// C: `static void add_floor_to_the_house() { add_floor(&the_house); }`
fn add_floor_to_the_house() {
    unsafe { add_floor(THE_HOUSE.as_ptr()) }
}

/// C: `static void print_the_house()`
fn print_the_house() {
    // Read the current state, then hand the values to libc's printf exactly as
    // the C code does: `int`, `int`, `double`.
    let house = unsafe { *THE_HOUSE.as_ptr() };
    unsafe {
        printf(
            PRINT_FMT.as_ptr() as *const c_char,
            house.floors,
            house.bedrooms,
            house.bathrooms,
        );
    }
}

/// C: `void run(int extra_bedrooms)`
///
/// Exported by the C shared library even though it is absent from `driver.h`,
/// so it is part of the public ABI and reproduced here.
#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    // C: `the_house.bathrooms += 1.0;`
    unsafe {
        let house = THE_HOUSE.as_ptr();
        (*house).bathrooms += 1.0;
    }
    print_the_house();
    unsafe { add_bedrooms(THE_HOUSE.as_ptr(), extra_bedrooms) };
    print_the_house();
}

/// C: `void driver(int x) { run(x); run(x); }`
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    run(x);
    run(x);
}
