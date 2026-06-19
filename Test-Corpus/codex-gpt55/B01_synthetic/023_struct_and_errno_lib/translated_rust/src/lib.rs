use std::ffi::{c_char, c_int, c_long};

const INT_MIN_C: c_long = c_int::MIN as c_long;
const INT_MAX_C: c_long = c_int::MAX as c_long;

#[repr(C)]
pub struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
}

#[cfg(any(target_os = "linux", target_os = "android"))]
unsafe fn errno_location() -> *mut c_int {
    unsafe extern "C" {
        fn __errno_location() -> *mut c_int;
    }

    unsafe { __errno_location() }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe fn errno_location() -> *mut c_int {
    unsafe extern "C" {
        fn __error() -> *mut c_int;
    }

    unsafe { __error() }
}

unsafe fn set_errno(value: c_int) {
    unsafe {
        *errno_location() = value;
    }
}

unsafe fn get_errno() -> c_int {
    unsafe { *errno_location() }
}

unsafe fn add_floor(house: *mut House) {
    unsafe {
        (*house).floors += 1;
    }
}

unsafe fn add_bedrooms(house: *mut House, extra_bedrooms: c_int) {
    unsafe {
        (*house).bedrooms += extra_bedrooms;
    }
}

unsafe fn print_house(house: *mut House) {
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
pub unsafe extern "C" fn run(the_house: *mut House, extra_bedrooms: c_int) {
    unsafe {
        print_house(the_house);
        add_floor(the_house);
        print_house(the_house);
        (*the_house).bathrooms += 1.0;
        print_house(the_house);
        add_bedrooms(the_house, extra_bedrooms);
        print_house(the_house);
    }
}

unsafe fn parse_val(str_ptr: *const c_char, val: *mut c_int) -> bool {
    unsafe {
        set_errno(0);
        let mut endp = str_ptr as *mut c_char;
        let tmp = strtol(str_ptr, &mut endp, 10);
        if endp != str_ptr as *mut c_char
            && get_errno() == 0
            && tmp >= INT_MIN_C
            && tmp <= INT_MAX_C
        {
            *val = tmp as c_int;
            true
        } else {
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    unsafe {
        let mut x: c_int = 0;
        if parse_val(input, &mut x) {
            let mut the_house = House {
                floors: 2,
                bedrooms: 5,
                bathrooms: 2.5,
            };
            run(&mut the_house, x);
            run(&mut the_house, x);
        } else {
            printf(c"An error occurred\n".as_ptr());
        }
    }
}
