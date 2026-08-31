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

// Use the platform's C stdio so that output bytes *and* buffering/flush-at-exit
// semantics match the original C library exactly.
unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// `typedef struct { int floors; int bedrooms; double bathrooms; } house_t;`
#[derive(Clone, Copy)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

/// `static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};`
///
/// Mutable process-wide state, exactly like the C original: mutations made by
/// one call to `run`/`driver` are visible to every later call.
static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

/// `static void add_floor(house_t *house)`
fn add_floor(house: &mut House) {
    house.floors = house.floors.wrapping_add(1);
}

/// `static void add_bedrooms(house_t *house, int extra_bedrooms)`
fn add_bedrooms(house: &mut House, extra_bedrooms: c_int) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

/// `static void add_floor_to_the_house()`
fn add_floor_to_the_house() {
    add_floor(unsafe { &mut *(&raw mut THE_HOUSE) });
}

/// `static void print_the_house()`
fn print_the_house() {
    const FMT: &[u8] =
        b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    let house = unsafe { *(&raw const THE_HOUSE) };
    unsafe {
        c_printf(
            FMT.as_ptr() as *const c_char,
            house.floors,
            house.bedrooms,
            house.bathrooms,
        );
    }
}

/// `void run(int extra_bedrooms)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        let house = &mut *(&raw mut THE_HOUSE);
        house.bathrooms += 1.0;
    }
    print_the_house();
    add_bedrooms(unsafe { &mut *(&raw mut THE_HOUSE) }, extra_bedrooms);
    print_the_house();
}

/// `true` for the bytes that C's `isspace` accepts in the "C" locale.
fn is_c_space(b: u8) -> bool {
    b == b' ' || (0x09..=0x0d).contains(&b)
}

/// Outcome of a `strtol(str, &endp, 10)` call, as far as `parse_val` observes it.
struct StrToLong {
    value: c_long,
    /// Offset of `endp` from `str`; `0` means `endp == str` (no conversion).
    end_offset: usize,
    /// Whether `errno` was set to `ERANGE`.
    erange: bool,
}

/// Faithful re-implementation of `strtol(str, &endp, 10)`.
///
/// On failure to convert, `endp` is left equal to `str` and `0` is returned,
/// with `errno` untouched. On overflow, `LONG_MAX`/`LONG_MIN` is returned and
/// `errno` is set to `ERANGE`, with `endp` still advanced past the digits.
unsafe fn strtol_base10(str: *const c_char) -> StrToLong {
    let byte_at = |i: usize| -> u8 { unsafe { *str.add(i) as u8 } };

    let mut i: usize = 0;
    while is_c_space(byte_at(i)) {
        i += 1;
    }

    let negative = match byte_at(i) {
        b'-' => {
            i += 1;
            true
        }
        b'+' => {
            i += 1;
            false
        }
        _ => false,
    };

    // Largest magnitude representable for the requested sign.
    let limit: u64 = if negative {
        1u64 << 63 // -(unsigned long)LONG_MIN
    } else {
        c_long::MAX as u64
    };
    let cutoff = limit / 10;
    let cutlim = limit % 10;

    let digits_start = i;
    let mut acc: u64 = 0;
    let mut overflow = false;
    while byte_at(i).is_ascii_digit() {
        let digit = u64::from(byte_at(i) - b'0');
        if overflow || acc > cutoff || (acc == cutoff && digit > cutlim) {
            overflow = true;
        } else {
            acc = acc * 10 + digit;
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: endp == str, errno untouched.
        return StrToLong {
            value: 0,
            end_offset: 0,
            erange: false,
        };
    }

    let value = if overflow {
        if negative { c_long::MIN } else { c_long::MAX }
    } else if negative {
        (acc as c_long).wrapping_neg()
    } else {
        acc as c_long
    };

    StrToLong {
        value,
        end_offset: i,
        erange: overflow,
    }
}

/// `static bool parse_val(const char *str, int *val)`
unsafe fn parse_val(str: *const c_char, val: &mut c_int) -> bool {
    // errno = 0;
    let mut errno: c_int = 0;
    // char *endp = (char *)str; long tmp = strtol(str, &endp, 10);
    let res = unsafe { strtol_base10(str) };
    let endp_offset = res.end_offset;
    let tmp = res.value;
    if res.erange {
        errno = ERANGE;
    }

    // if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX)
    if endp_offset != 0
        && errno == 0
        && tmp >= c_long::from(c_int::MIN)
        && tmp <= c_long::from(c_int::MAX)
    {
        *val = tmp as c_int;
        true
    } else {
        false
    }
}

const ERANGE: c_int = 34;

/// `void driver(const char *in)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    let mut x: c_int = 0;
    if unsafe { parse_val(in_, &mut x) } {
        unsafe {
            run(x);
            run(x);
        }
    } else {
        const MSG: &[u8] = b"An error occurred\n\0";
        unsafe {
            c_printf(MSG.as_ptr() as *const c_char);
        }
    }
}
