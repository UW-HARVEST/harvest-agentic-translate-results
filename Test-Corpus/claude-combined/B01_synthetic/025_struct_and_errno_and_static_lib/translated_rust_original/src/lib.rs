// Translation of c_src/src/driver.c to Rust.
//
// The C source contains:
//   - a module-private static `house_t the_house` initialised to
//     {2, 5, 2.5}
//   - a module-private `parse_val` that uses strtol with the same
//     accept/reject rules as the C code
//   - public functions `run(int)` and `driver(const char *)`
//
// To get byte-identical stdout we forward print formatting to the
// C library `printf`, matching the original code exactly.

use std::ffi::c_char;
use std::os::raw::{c_double, c_int, c_long};

#[repr(C)]
#[derive(Copy, Clone)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

// Module-local mutable state matching the C `static house_t the_house`.
static mut THE_HOUSE: HouseT = HouseT {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
    fn __errno_location() -> *mut c_int;
}

#[inline]
fn errno_set(v: c_int) {
    unsafe { *__errno_location() = v; }
}

#[inline]
fn errno_get() -> c_int {
    unsafe { *__errno_location() }
}

fn add_floor(house: &mut HouseT) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut HouseT, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    unsafe {
        let h = &raw mut THE_HOUSE;
        add_floor(&mut *h);
    }
}

fn print_the_house() {
    unsafe {
        let fmt = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
        let h = &raw const THE_HOUSE;
        printf(
            fmt.as_ptr() as *const c_char,
            (*h).floors,
            (*h).bedrooms,
            (*h).bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        let h = &raw mut THE_HOUSE;
        (*h).bathrooms += 1.0;
    }
    print_the_house();
    unsafe {
        let h = &raw mut THE_HOUSE;
        add_bedrooms(&mut *h, extra_bedrooms);
    }
    print_the_house();
}

fn parse_val(str_ptr: *const c_char, val: &mut c_int) -> bool {
    // Match the C exactly: errno = 0; char *endp = (char *)str;
    // long tmp = strtol(str, &endp, 10);
    // if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) ...
    errno_set(0);
    let mut endp: *mut c_char = str_ptr as *mut c_char;
    let tmp: c_long = unsafe { strtol(str_ptr, &mut endp as *mut *mut c_char, 10) };
    let int_min = c_int::MIN as c_long;
    let int_max = c_int::MAX as c_long;
    if endp != (str_ptr as *mut c_char)
        && errno_get() == 0
        && tmp >= int_min
        && tmp <= int_max
    {
        *val = tmp as c_int;
        true
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(in_ptr: *const c_char) {
    let mut x: c_int = 0;
    if parse_val(in_ptr, &mut x) {
        run(x);
        run(x);
    } else {
        unsafe {
            let fmt = b"An error occurred\n\0";
            printf(fmt.as_ptr() as *const c_char);
        }
    }
}
