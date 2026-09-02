// Rust translation of the C library in c_src/.
//
// Original copyright notice from the C sources:
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

extern "C" {
    /// libc `printf`. Used directly so that number formatting (`%d`, `%.1f`)
    /// and stdout buffering are byte-for-byte identical to the C library.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

/// Mirrors the C file-scope mutable global. The C library is not thread safe;
/// this wrapper reproduces that behavior rather than adding synchronization.
struct Global(UnsafeCell<HouseT>);

// SAFETY: matches the (absent) thread-safety guarantees of the original C
// translation unit, which mutates `the_house` without any locking.
unsafe impl Sync for Global {}

/// ```c
/// static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};
/// ```
static THE_HOUSE: Global = Global(UnsafeCell::new(HouseT {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
}));

#[inline]
fn the_house() -> *mut HouseT {
    THE_HOUSE.0.get()
}

/// ```c
/// static void add_floor(house_t *house) {
///     house->floors++;
/// }
/// ```
///
/// The C increment on `int` wraps in practice (and is UB on overflow); use
/// `wrapping_add` to reproduce the observable behavior without panicking.
unsafe fn add_floor(house: *mut HouseT) {
    (*house).floors = (*house).floors.wrapping_add(1);
}

/// ```c
/// static void add_bedrooms(house_t *house, int extra_bedrooms) {
///     house->bedrooms += extra_bedrooms;
/// }
/// ```
unsafe fn add_bedrooms(house: *mut HouseT, extra_bedrooms: c_int) {
    (*house).bedrooms = (*house).bedrooms.wrapping_add(extra_bedrooms);
}

/// ```c
/// static void add_floor_to_the_house() {
///     add_floor(&the_house);
/// }
/// ```
unsafe fn add_floor_to_the_house() {
    add_floor(the_house());
}

/// ```c
/// static void print_the_house() {
///     printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n",
///            the_house.floors, the_house.bedrooms, the_house.bathrooms);
/// }
/// ```
unsafe fn print_the_house() {
    let house = the_house();
    printf(
        b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0".as_ptr() as *const c_char,
        (*house).floors,
        (*house).bedrooms,
        (*house).bathrooms,
    );
}

/// ```c
/// void run(int extra_bedrooms) {
///     print_the_house();
///     add_floor_to_the_house();
///     print_the_house();
///     the_house.bathrooms += 1.0;
///     print_the_house();
///     add_bedrooms(&the_house, extra_bedrooms);
///     print_the_house();
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    let house = the_house();
    (*house).bathrooms += 1.0;
    print_the_house();
    add_bedrooms(the_house(), extra_bedrooms);
    print_the_house();
}

/// ```c
/// void driver(int x) {
///     run(x);
///     run(x);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_int) {
    run(x);
    run(x);
}
