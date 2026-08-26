use std::ffi::{c_char, c_double, c_int, c_long};

#[repr(C)]
pub struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

unsafe extern "C" {
    fn __errno_location() -> *mut c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn strtol(input: *const c_char, end: *mut *mut c_char, base: c_int) -> c_long;
}

const HOUSE_FORMAT: &[u8] = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
const ERROR_MESSAGE: &[u8] = b"An error occurred\n\0";

unsafe fn print_house(house: *const House) {
    unsafe {
        printf(
            HOUSE_FORMAT.as_ptr().cast(),
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
        (*the_house).floors += 1;
        print_house(the_house);
        (*the_house).bathrooms += 1.0;
        print_house(the_house);
        (*the_house).bedrooms += extra_bedrooms;
        print_house(the_house);
    }
}

unsafe fn parse_val(input: *const c_char, value: *mut c_int) -> bool {
    unsafe {
        *__errno_location() = 0;
        let mut end = input.cast_mut();
        let parsed = strtol(input, &mut end, 10);

        if end != input.cast_mut()
            && *__errno_location() == 0
            && parsed >= c_int::MIN.into()
            && parsed <= c_int::MAX.into()
        {
            *value = parsed as c_int;
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
            let mut house = House {
                floors: 2,
                bedrooms: 5,
                bathrooms: 2.5,
            };
            run(&mut house, extra_bedrooms);
            run(&mut house, extra_bedrooms);
        } else {
            printf(ERROR_MESSAGE.as_ptr().cast());
        }
    }
}
