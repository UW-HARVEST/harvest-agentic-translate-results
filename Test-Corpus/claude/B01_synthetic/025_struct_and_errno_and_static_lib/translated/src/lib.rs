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
use std::ffi::{c_char, c_double, c_int, c_long};

// ---------------------------------------------------------------------------
// C runtime bindings.
//
// The original C code uses `printf` and `strtol` (plus `errno`).  Those exact
// routines are used here so that formatting, buffering and parsing behaviour
// are byte-for-byte identical to the C library.
// ---------------------------------------------------------------------------
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
    fn __errno_location() -> *mut c_int;
}

#[inline]
fn errno_set(value: c_int) {
    unsafe {
        *__errno_location() = value;
    }
}

#[inline]
fn errno_get() -> c_int {
    unsafe { *__errno_location() }
}

// ---------------------------------------------------------------------------
// typedef struct { int floors; int bedrooms; double bathrooms; } house_t;
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

/// Wrapper giving the file-scope `static house_t the_house` mutable, process
/// wide storage (exactly like the C translation unit's private global).
struct TheHouse(UnsafeCell<HouseT>);

// The C code is not thread safe either; this mirrors it exactly.
unsafe impl Sync for TheHouse {}

// static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};
static THE_HOUSE: TheHouse = TheHouse(UnsafeCell::new(HouseT {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
}));

#[inline]
fn the_house() -> *mut HouseT {
    THE_HOUSE.0.get()
}

// static void add_floor(house_t *house) { house->floors++; }
fn add_floor(house: *mut HouseT) {
    unsafe {
        (*house).floors = (*house).floors.wrapping_add(1);
    }
}

// static void add_bedrooms(house_t *house, int extra_bedrooms)
fn add_bedrooms(house: *mut HouseT, extra_bedrooms: c_int) {
    unsafe {
        (*house).bedrooms = (*house).bedrooms.wrapping_add(extra_bedrooms);
    }
}

// static void add_floor_to_the_house()
fn add_floor_to_the_house() {
    add_floor(the_house());
}

// static void print_the_house()
fn print_the_house() {
    let fmt = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    unsafe {
        let h = *the_house();
        printf(
            fmt.as_ptr() as *const c_char,
            h.floors,
            h.bedrooms,
            h.bathrooms,
        );
    }
}

// void run(int extra_bedrooms)
#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        (*the_house()).bathrooms += 1.0;
    }
    print_the_house();
    add_bedrooms(the_house(), extra_bedrooms);
    print_the_house();
}

// static bool parse_val(const char *str, int *val)
fn parse_val(str: *const c_char, val: &mut c_int) -> bool {
    errno_set(0);
    let mut endp: *mut c_char = str as *mut c_char;
    let tmp: c_long = unsafe { strtol(str, &mut endp, 10) };
    if endp != str as *mut c_char
        && errno_get() == 0
        && tmp >= c_int::MIN as c_long
        && tmp <= c_int::MAX as c_long
    {
        // *val = tmp;  (implicit long -> int conversion; value is in range)
        *val = tmp as c_int;
        true
    } else {
        false
    }
}

// void driver(const char *in)
#[unsafe(no_mangle)]
pub extern "C" fn driver(in_: *const c_char) {
    let mut x: c_int = 0; // `int x;` (uninitialized in C, always written before use)
    if parse_val(in_, &mut x) {
        run(x);
        run(x);
    } else {
        let msg = b"An error occurred\n\0";
        unsafe {
            printf(msg.as_ptr() as *const c_char);
        }
    }
}
