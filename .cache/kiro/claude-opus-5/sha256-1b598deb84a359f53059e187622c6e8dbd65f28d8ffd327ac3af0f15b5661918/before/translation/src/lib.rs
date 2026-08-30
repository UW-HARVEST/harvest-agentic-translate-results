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

use std::ffi::{c_char, c_double, c_int};

unsafe extern "C" {
    // Use the platform's stdio so that output interleaves and buffers exactly
    // like the original C library does (important when a C caller also prints).
    fn printf(fmt: *const c_char, ...) -> c_int;
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
/// Mutable process-wide state, just like the C original: mutations made by one
/// call to `driver`/`run` are visible to the next one.
static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

fn add_floor(house: &mut House) {
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: c_int) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

/// Borrow the global house. Single-threaded access only, matching the C code,
/// which has no synchronization either.
fn the_house() -> &'static mut House {
    unsafe { &mut *(&raw mut THE_HOUSE) }
}

fn add_floor_to_the_house() {
    add_floor(the_house());
}

fn print_the_house() {
    let house = *the_house();
    const FMT: &[u8] = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    unsafe {
        printf(
            FMT.as_ptr() as *const c_char,
            house.floors,
            house.bedrooms,
            house.bathrooms,
        );
    }
}

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

/// Outcome of emulating `strtol(str, &endp, 10)`.
struct StrToLong {
    value: i64,
    /// Whether any conversion was performed, i.e. `endp != str` afterwards.
    converted: bool,
    /// Whether the conversion overflowed, i.e. `errno == ERANGE` afterwards.
    range_error: bool,
}

/// Faithful emulation of glibc `strtol(str, &endp, 10)` for the subset of
/// behavior the caller observes: the converted value, whether `endp` moved,
/// and whether `errno` was set to `ERANGE`.
fn strtol_base10(bytes: &[u8]) -> StrToLong {
    let mut i = 0usize;

    // Skip leading whitespace (isspace in the "C" locale).
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    // Optional sign.
    let negative = match bytes.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };

    // Digit sequence.
    let digits_start = i;
    let mut acc: i128 = 0;
    let mut range_error = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        if !range_error {
            acc = acc * 10 + i128::from(bytes[i] - b'0');
            if acc > i128::from(i64::MAX) + 1 {
                range_error = true;
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: *endptr is set back to str and the return
        // value is 0.
        return StrToLong {
            value: 0,
            converted: false,
            range_error: false,
        };
    }

    let signed = if negative { -acc } else { acc };
    let value = if signed > i128::from(i64::MAX) {
        range_error = true;
        i64::MAX
    } else if signed < i128::from(i64::MIN) {
        range_error = true;
        i64::MIN
    } else {
        signed as i64
    };

    StrToLong {
        value,
        converted: true,
        range_error,
    }
}

/// static bool parse_val(const char *str, int *val)
fn parse_val(str: *const c_char, val: &mut c_int) -> bool {
    // Walk the NUL-terminated string exactly as the C code's strtol would.
    let bytes = unsafe {
        let mut len = 0usize;
        while *str.add(len) != 0 {
            len += 1;
        }
        std::slice::from_raw_parts(str as *const u8, len)
    };

    let r = strtol_base10(bytes);
    let tmp = r.value;

    // endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX
    if r.converted
        && !r.range_error
        && tmp >= i64::from(c_int::MIN)
        && tmp <= i64::from(c_int::MAX)
    {
        *val = tmp as c_int;
        true
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(in_: *const c_char) {
    let mut x: c_int = 0;
    if parse_val(in_, &mut x) {
        run(x);
        run(x);
    } else {
        const FMT: &[u8] = b"An error occurred\n\0";
        unsafe {
            printf(FMT.as_ptr() as *const c_char);
        }
    }
}
