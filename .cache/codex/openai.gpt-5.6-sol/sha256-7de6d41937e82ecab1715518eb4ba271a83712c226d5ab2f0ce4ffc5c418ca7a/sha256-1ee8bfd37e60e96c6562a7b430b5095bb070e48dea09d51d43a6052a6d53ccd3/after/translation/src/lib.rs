use std::ffi::{c_char, c_double, c_int, c_long};
use std::ptr;

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

unsafe extern "C" {
    fn __errno_location() -> *mut c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn strtol(value: *const c_char, end: *mut *mut c_char, base: c_int) -> c_long;
}

unsafe fn print_the_house() {
    let house = ptr::addr_of!(THE_HOUSE);
    unsafe {
        printf(
            c"The house has %d floors, %d bedrooms, and %.1f bathrooms\n".as_ptr(),
            (*house).floors,
            (*house).bedrooms,
            (*house).bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(extra_bedrooms: c_int) {
    unsafe {
        print_the_house();

        let house = ptr::addr_of_mut!(THE_HOUSE);
        (*house).floors = (*house).floors.wrapping_add(1);
        print_the_house();

        (*house).bathrooms += 1.0;
        print_the_house();

        (*house).bedrooms = (*house).bedrooms.wrapping_add(extra_bedrooms);
        print_the_house();
    }
}

unsafe fn parse_val(value: *const c_char, result: *mut c_int) -> bool {
    unsafe {
        *__errno_location() = 0;
        let mut end = value.cast_mut();
        let parsed = strtol(value, &mut end, 10);

        if end != value.cast_mut()
            && *__errno_location() == 0
            && parsed >= c_int::MIN as c_long
            && parsed <= c_int::MAX as c_long
        {
            *result = parsed as c_int;
            true
        } else {
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    unsafe {
        let mut extra_bedrooms = 0;
        if parse_val(input, &mut extra_bedrooms) {
            run(extra_bedrooms);
            run(extra_bedrooms);
        } else {
            printf(c"An error occurred\n".as_ptr());
        }
    }
}
