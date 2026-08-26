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

// libc bindings. The C library used the platform's stdio/strtol, so we call the
// very same routines to guarantee byte-identical output (formatting rules,
// rounding of `%.1f`, and stdout buffering behaviour all match exactly).
extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
    fn __errno_location() -> *mut c_int;
}

/// C: `typedef struct { int floors; int bedrooms; double bathrooms; } house_t;`
///
/// `#[repr(C)]` keeps the layout (offsets 0, 4, 8; size 16, align 8) identical
/// to the C struct, since `run` is part of the public ABI.
#[repr(C)]
pub struct house_t {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: c_double,
}

// C: static void add_floor(house_t *house) { house->floors++; }
unsafe fn add_floor(house: *mut house_t) {
    (*house).floors = (*house).floors.wrapping_add(1);
}

// C: static void add_bedrooms(house_t *house, int extra_bedrooms)
unsafe fn add_bedrooms(house: *mut house_t, extra_bedrooms: c_int) {
    (*house).bedrooms = (*house).bedrooms.wrapping_add(extra_bedrooms);
}

// C: static void print_house(house_t *house)
unsafe fn print_house(house: *mut house_t) {
    printf(
        b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0".as_ptr() as *const c_char,
        (*house).floors,
        (*house).bedrooms,
        (*house).bathrooms,
    );
}

/// C: `void run(house_t *the_house, int extra_bedrooms)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(the_house: *mut house_t, extra_bedrooms: c_int) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    (*the_house).bathrooms += 1.0;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

// C: static bool parse_val(const char *str, int *val)
unsafe fn parse_val(str: *const c_char, val: *mut c_int) -> bool {
    *__errno_location() = 0;
    let mut endp: *mut c_char = str as *mut c_char;
    let tmp: c_long = strtol(str, &mut endp, 10);
    if endp != str as *mut c_char
        && *__errno_location() == 0
        && tmp >= c_int::MIN as c_long
        && tmp <= c_int::MAX as c_long
    {
        // C: *val = tmp;  (implicit long -> int conversion)
        *val = tmp as c_int;
        true
    } else {
        false
    }
}

/// C: `void driver(const char *in)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    // C: int x;  (uninitialized)
    let mut x: c_int = 0;
    if parse_val(in_, &mut x) {
        // C: house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};
        let mut the_house = house_t {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        printf(b"An error occurred\n\0".as_ptr() as *const c_char);
    }
}
