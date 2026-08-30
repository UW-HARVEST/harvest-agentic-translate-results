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

use std::ffi::{c_char, c_double, c_int, c_long};

// ---------------------------------------------------------------------------
// libc bindings.
//
// The C original writes through `printf`, i.e. the process-wide C `stdout`
// stream. Going through the very same stream (rather than Rust's `std::io`)
// keeps buffering, flush-at-exit behaviour and therefore the emitted byte
// stream identical to the C library's.
// `strtol` / `errno` are likewise used directly so that the parsing corner
// cases (leading whitespace, partial parses, ERANGE saturation) match exactly.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;

    #[link_name = "strtol"]
    unsafe fn c_strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;

    // glibc's thread-local `errno` accessor (`errno` is a macro in C).
    unsafe fn __errno_location() -> *mut c_int;
}

#[inline]
fn errno_get() -> c_int {
    unsafe { *__errno_location() }
}

#[inline]
fn errno_set(value: c_int) {
    unsafe { *__errno_location() = value };
}

// `printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)`
const HOUSE_FMT: &[u8] = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
// `printf("An error occurred\n")`
const ERROR_FMT: &[u8] = b"An error occurred\n\0";

/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
#[repr(C)]
pub struct house_t {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: c_double,
}

/// `static void add_floor(house_t *house)`
fn add_floor(house: &mut house_t) {
    // `house->floors++` — wrapping keeps the observable behaviour of the
    // compiled C on overflow instead of panicking.
    house.floors = house.floors.wrapping_add(1);
}

/// `static void add_bedrooms(house_t *house, int extra_bedrooms)`
fn add_bedrooms(house: &mut house_t, extra_bedrooms: c_int) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

/// `static void print_house(house_t *house)`
fn print_house(house: &house_t) {
    unsafe {
        c_printf(
            HOUSE_FMT.as_ptr() as *const c_char,
            house.floors,
            house.bedrooms,
            house.bathrooms,
        );
    }
}

/// `void run(house_t *the_house, int extra_bedrooms)` — non-static in the C
/// source, so it is part of the shared object's exported surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(the_house: *mut house_t, extra_bedrooms: c_int) {
    let the_house = unsafe { &mut *the_house };

    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    the_house.bathrooms += 1.0;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

/// `static bool parse_val(const char *str, int *val)`
fn parse_val(str: *const c_char, val: &mut c_int) -> bool {
    errno_set(0);
    let mut endp: *mut c_char = str as *mut c_char;
    let tmp: c_long = unsafe { c_strtol(str, &mut endp, 10) };
    if endp != (str as *mut c_char)
        && errno_get() == 0
        && tmp >= c_int::MIN as c_long
        && tmp <= c_int::MAX as c_long
    {
        *val = tmp as c_int;
        true
    } else {
        false
    }
}

/// `void driver(const char *in)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    let mut x: c_int = 0; // `int x;` is uninitialised in C but only read on the
    // success path, where `parse_val` has written to it.
    if parse_val(in_, &mut x) {
        let mut the_house = house_t {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        unsafe {
            run(&mut the_house, x);
            run(&mut the_house, x);
        }
    } else {
        unsafe {
            c_printf(ERROR_FMT.as_ptr() as *const c_char);
        }
    }
}
