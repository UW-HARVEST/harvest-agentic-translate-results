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

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_double, c_int, c_long};
use core::ptr::addr_of_mut;

// ---------------------------------------------------------------------------
// libc bindings.
//
// The C library formats its output with `printf(3)` and parses with
// `strtol(3)`. In order to be byte-identical -- and to share the very same
// stdio buffer, so that interleaving/flushing behaviour is preserved -- we
// call straight through to the platform C library rather than reimplementing
// the formatting in Rust.
// ---------------------------------------------------------------------------
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
    fn __errno_location() -> *mut c_int;
}

#[inline]
unsafe fn errno_get() -> c_int {
    *__errno_location()
}

#[inline]
unsafe fn errno_set(v: c_int) {
    *__errno_location() = v;
}

// <errno.h>
const EOK: c_int = 0;

// <limits.h>
const INT_MIN_L: c_long = c_int::MIN as c_long;
const INT_MAX_L: c_long = c_int::MAX as c_long;

// ---------------------------------------------------------------------------
// typedef struct { int floors; int bedrooms; double bathrooms; } house_t;
// ---------------------------------------------------------------------------
#[repr(C)]
struct house_t {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

// static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};
static mut THE_HOUSE: house_t = house_t {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

// static void add_floor(house_t *house) { house->floors++; }
unsafe fn add_floor(house: *mut house_t) {
    // `++` on a signed int; reproduce the two's-complement wrap the C
    // compiler emits rather than trapping.
    (*house).floors = (*house).floors.wrapping_add(1);
}

// static void add_bedrooms(house_t *house, int extra_bedrooms)
unsafe fn add_bedrooms(house: *mut house_t, extra_bedrooms: c_int) {
    (*house).bedrooms = (*house).bedrooms.wrapping_add(extra_bedrooms);
}

// static void add_floor_to_the_house()
unsafe fn add_floor_to_the_house() {
    add_floor(addr_of_mut!(THE_HOUSE));
}

// static void print_the_house()
unsafe fn print_the_house() {
    const FMT: &[u8] =
        b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    let h = addr_of_mut!(THE_HOUSE);
    printf(
        FMT.as_ptr() as *const c_char,
        (*h).floors,
        (*h).bedrooms,
        (*h).bathrooms,
    );
}

/// void run(int extra_bedrooms);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    let h = addr_of_mut!(THE_HOUSE);
    (*h).bathrooms += 1.0;
    print_the_house();
    add_bedrooms(h, extra_bedrooms);
    print_the_house();
}

// static bool parse_val(const char *str, int *val)
unsafe fn parse_val(str_: *const c_char, val: *mut c_int) -> bool {
    errno_set(EOK);
    let mut endp: *mut c_char = str_ as *mut c_char;
    let tmp: c_long = strtol(str_, &mut endp, 10);
    if endp != (str_ as *mut c_char)
        && errno_get() == EOK
        && tmp >= INT_MIN_L
        && tmp <= INT_MAX_L
    {
        *val = tmp as c_int;
        true
    } else {
        false
    }
}

/// void driver(const char *in);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    // `int x;` -- deliberately left uninitialised in the C source; it is only
    // read when parse_val() succeeded and therefore wrote it.
    let mut x: c_int = 0;
    if parse_val(in_, &mut x) {
        run(x);
        run(x);
    } else {
        const MSG: &[u8] = b"An error occurred\n\0";
        printf(MSG.as_ptr() as *const c_char);
    }
}
