// Copyright 2025 MIT Lincoln Laboratory
// Translation of c_src/src/driver.c to Rust.
//
// The translation preserves byte-identical stdout output by delegating
// formatting and I/O to libc's printf, and integer parsing to libc's strtol,
// matching the behavior of the original C implementation exactly.

use std::ffi::c_char;
use std::os::raw::{c_double, c_int, c_long};

#[repr(C)]
#[derive(Copy, Clone)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

// Global mutable state, exactly matching:
//   static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};
static mut THE_HOUSE: HouseT = HouseT {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

unsafe extern "C" {
    // libc: int printf(const char *format, ...);
    fn printf(format: *const c_char, ...) -> c_int;

    // libc: long strtol(const char *nptr, char **endptr, int base);
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;

    // errno is a thread-local int accessed via __errno_location() on glibc.
    // We use the libc crate's accessor to avoid platform-specific assumptions.
}

fn add_floor(house: &mut HouseT) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut HouseT, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    // Safety: faithfully reproduces the original C, which has no thread-safety.
    unsafe {
        add_floor(&mut *core::ptr::addr_of_mut!(THE_HOUSE));
    }
}

fn print_the_house() {
    // Use libc's printf so the formatting and stdio buffering match the C
    // implementation exactly, producing byte-identical output.
    let fmt = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    unsafe {
        let h = &*core::ptr::addr_of!(THE_HOUSE);
        printf(
            fmt.as_ptr() as *const c_char,
            h.floors,
            h.bedrooms,
            h.bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        let h = &mut *core::ptr::addr_of_mut!(THE_HOUSE);
        h.bathrooms += 1.0;
    }
    print_the_house();
    unsafe {
        let h = &mut *core::ptr::addr_of_mut!(THE_HOUSE);
        add_bedrooms(h, extra_bedrooms);
    }
    print_the_house();
}

fn parse_val(s: *const c_char, val: *mut c_int) -> bool {
    // Reproduce:
    //   errno = 0;
    //   char *endp = (char *)str;
    //   long tmp = strtol(str, &endp, 10);
    //   if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
    //       *val = tmp;
    //       return true;
    //   } else {
    //       return false;
    //   }
    unsafe {
        // Reset errno to 0 prior to the call (matches `errno = 0;`).
        *libc::__errno_location() = 0;

        let mut endp: *mut c_char = s as *mut c_char;
        let tmp: c_long = strtol(s, &mut endp as *mut *mut c_char, 10);
        let err = *libc::__errno_location();

        if endp != (s as *mut c_char)
            && err == 0
            && tmp >= c_int::MIN as c_long
            && tmp <= c_int::MAX as c_long
        {
            *val = tmp as c_int;
            true
        } else {
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    let mut x: c_int = 0;
    if parse_val(input, &mut x as *mut c_int) {
        run(x);
        run(x);
    } else {
        let msg = b"An error occurred\n\0";
        unsafe {
            printf(msg.as_ptr() as *const c_char);
        }
    }
}
