// Rust translation of c_src/src/driver.c
//
// Behaviour is intended to be byte-for-byte identical to the C original,
// including its quirks. `printf` from libc is called directly so that the
// output shares the C runtime's stdout buffer, keeping interleaving with any
// C caller's output identical as well.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_double, c_int, c_long};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    // glibc / musl expose `errno` through this thread-local accessor.
    fn __errno_location() -> *mut c_int;
}

fn errno_get() -> c_int {
    unsafe { *__errno_location() }
}

fn errno_set(value: c_int) {
    unsafe {
        *__errno_location() = value;
    }
}

const ERANGE: c_int = 34;

/// Mirrors the C `house_t` struct: `int`, `int`, `double`.
#[repr(C)]
pub struct house_t {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: c_double,
}

// static void add_floor(house_t *house)
fn add_floor(house: &mut house_t) {
    house.floors = house.floors.wrapping_add(1);
}

// static void add_bedrooms(house_t *house, int extra_bedrooms)
fn add_bedrooms(house: &mut house_t, extra_bedrooms: c_int) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

// static void print_house(house_t *house)
fn print_house(house: &house_t) {
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

/// `void run(house_t *the_house, int extra_bedrooms)` — non-static in the C
/// source, so it is part of the library's exported ABI.
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

/// Reimplementation of `strtol(str, &endp, 10)`.
///
/// Returns `(value, consumed, range_error)` where `consumed` is the number of
/// bytes the conversion advanced `endp` by (zero when no conversion could be
/// performed, matching `endp == str`).
fn strtol_base10(s: *const c_char) -> (c_long, usize, bool) {
    let byte = |i: usize| -> u8 { unsafe { *s.add(i) as u8 } };

    let mut i: usize = 0;

    // Leading whitespace, as classified by isspace() in the "C" locale.
    loop {
        let c = byte(i);
        if c == b' ' || (0x09..=0x0d).contains(&c) {
            i += 1;
        } else {
            break;
        }
    }

    let negative = match byte(i) {
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

    // Magnitude limit: LONG_MAX, or |LONG_MIN| when negative.
    let cutoff: u64 = if negative {
        (c_long::MAX as u64) + 1
    } else {
        c_long::MAX as u64
    };
    let cut_div = cutoff / 10;
    let cut_rem = cutoff % 10;

    let mut acc: u64 = 0;
    let mut digits = 0usize;
    let mut overflow = false;

    loop {
        let c = byte(i);
        if !c.is_ascii_digit() {
            break;
        }
        let d = u64::from(c - b'0');
        if !overflow {
            if acc > cut_div || (acc == cut_div && d > cut_rem) {
                overflow = true;
            } else {
                acc = acc * 10 + d;
            }
        }
        digits += 1;
        i += 1;
    }

    if digits == 0 {
        // No conversion performed: strtol leaves endp == str and returns 0.
        return (0, 0, false);
    }

    let value = if overflow {
        if negative {
            c_long::MIN
        } else {
            c_long::MAX
        }
    } else if negative {
        (acc as c_long).wrapping_neg()
    } else {
        acc as c_long
    };

    (value, i, overflow)
}

// static bool parse_val(const char *str, int *val)
fn parse_val(s: *const c_char, val: &mut c_int) -> bool {
    errno_set(0);
    let (tmp, consumed, range_error) = strtol_base10(s);
    if range_error {
        errno_set(ERANGE);
    }
    if consumed != 0
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
            printf(MSG.as_ptr() as *const c_char);
        }
    }
}
