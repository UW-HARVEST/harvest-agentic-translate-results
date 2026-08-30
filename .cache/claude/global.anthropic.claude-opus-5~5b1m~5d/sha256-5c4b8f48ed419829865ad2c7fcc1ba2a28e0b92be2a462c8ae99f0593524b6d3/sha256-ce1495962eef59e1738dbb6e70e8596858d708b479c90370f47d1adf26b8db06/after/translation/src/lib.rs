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

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_double, c_int, c_long};

// ---------------------------------------------------------------------------
// libc bindings
//
// The C code writes to stdout via `printf` and parses integers via `strtol`.
// We call straight through to the platform C library so that both the exact
// byte formatting *and* the stdio buffering behaviour are bit-for-bit
// identical to the original library.
// ---------------------------------------------------------------------------
mod ffi {
    use std::ffi::{c_char, c_int, c_long};

    unsafe extern "C" {
        pub unsafe fn printf(fmt: *const c_char, ...) -> c_int;

        pub unsafe fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int)
        -> c_long;

        pub unsafe fn __errno_location() -> *mut c_int;
    }
}

#[inline]
fn errno_ptr() -> *mut c_int {
    unsafe { ffi::__errno_location() }
}

#[inline]
fn get_errno() -> c_int {
    unsafe { *errno_ptr() }
}

#[inline]
fn set_errno(v: c_int) {
    unsafe { *errno_ptr() = v };
}

// C: #define INT_MIN / INT_MAX  (as `long` values, matching the C comparison)
const C_INT_MIN: c_long = c_int::MIN as c_long;
const C_INT_MAX: c_long = c_int::MAX as c_long;

// ---------------------------------------------------------------------------
// typedef struct {
//     int floors;
//     int bedrooms;
//     double bathrooms;
// } house_t;
// ---------------------------------------------------------------------------
#[repr(C)]
pub struct house_t {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: c_double,
}

// static void add_floor(house_t *house)
//
// NOTE ON POINTER HANDLING
//
// These helpers take a *raw pointer*, exactly like the C, and deliberately
// avoid two things:
//
//  1. Forming a Rust reference (`&mut *house`), and
//  2. Reading/writing through a place expression (`(*house).floors`).
//
// Both make rustc emit a null/alignment validity check when debug assertions
// are enabled, which `abort()`s (SIGABRT) where the C simply faults (SIGSEGV).
// That is an observable divergence for `run(NULL, ..)` — see ERRORS.md rows
// E10/E11. Going through a raw-ref (`&raw`, which never dereferences) plus
// `ptr::read`/`ptr::write` compiles to the same plain load/store the C emits and
// faults identically in every profile.
#[inline]
unsafe fn get_floors(house: *const house_t) -> c_int {
    unsafe { (&raw const (*house).floors).read() }
}
#[inline]
unsafe fn get_bedrooms(house: *const house_t) -> c_int {
    unsafe { (&raw const (*house).bedrooms).read() }
}
#[inline]
unsafe fn get_bathrooms(house: *const house_t) -> c_double {
    unsafe { (&raw const (*house).bathrooms).read() }
}

// static void add_floor(house_t *house)
unsafe fn add_floor(house: *mut house_t) {
    // house->floors++;
    unsafe {
        let p = &raw mut (*house).floors;
        p.write(p.read().wrapping_add(1));
    }
}

// static void add_bedrooms(house_t *house, int extra_bedrooms)
unsafe fn add_bedrooms(house: *mut house_t, extra_bedrooms: c_int) {
    // house->bedrooms += extra_bedrooms;
    unsafe {
        let p = &raw mut (*house).bedrooms;
        p.write(p.read().wrapping_add(extra_bedrooms));
    }
}

// static void print_house(house_t *house)
unsafe fn print_house(house: *const house_t) {
    // printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
    const FMT: &[u8] =
        b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    unsafe {
        ffi::printf(
            FMT.as_ptr() as *const c_char,
            get_floors(house),
            get_bedrooms(house),
            get_bathrooms(house),
        );
    }
}

// static bool parse_val(const char *str, int *val)
fn parse_val(str: *const c_char, val: &mut c_int) -> bool {
    set_errno(0);
    let mut endp: *mut c_char = str as *mut c_char;
    let tmp: c_long = unsafe { ffi::strtol(str, &mut endp, 10) };
    if endp != (str as *mut c_char) && get_errno() == 0 && tmp >= C_INT_MIN && tmp <= C_INT_MAX {
        // *val = tmp;  (implicit long -> int conversion)
        *val = tmp as c_int;
        true
    } else {
        false
    }
}

// void run(house_t *the_house, int extra_bedrooms)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(the_house: *mut house_t, extra_bedrooms: c_int) {
    unsafe {
        print_house(the_house);
        add_floor(the_house);
        print_house(the_house);
        // the_house->bathrooms += 1.0;
        let p = &raw mut (*the_house).bathrooms;
        p.write(p.read() + 1.0);
        print_house(the_house);
        add_bedrooms(the_house, extra_bedrooms);
        print_house(the_house);
    }
}

// void driver(const char *in)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    let mut x: c_int = 0;
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
        const MSG: &[u8] = b"An error occurred\n\0";
        unsafe {
            ffi::printf(MSG.as_ptr() as *const c_char);
        }
    }
}
