use std::ffi::{c_char, c_int, c_long};

#[derive(Clone, Copy)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

const HOUSE_FORMAT: &[u8] = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
const ERROR_MESSAGE: &[u8] = b"An error occurred\n\0";

unsafe extern "C" {
    fn __errno_location() -> *mut c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn strtol(value: *const c_char, end: *mut *mut c_char, base: c_int) -> c_long;
}

unsafe fn print_the_house() {
    let house = unsafe { THE_HOUSE };
    unsafe {
        printf(
            HOUSE_FORMAT.as_ptr().cast(),
            house.floors,
            house.bedrooms,
            house.bathrooms,
        );
    }
}

/// Runs one update cycle on the process-global house.
///
/// # Safety
///
/// Calls that access the global house must be externally synchronized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(extra_bedrooms: c_int) {
    unsafe {
        print_the_house();
        THE_HOUSE.floors = THE_HOUSE.floors.wrapping_add(1);
        print_the_house();
        THE_HOUSE.bathrooms += 1.0;
        print_the_house();
        THE_HOUSE.bedrooms = THE_HOUSE.bedrooms.wrapping_add(extra_bedrooms);
        print_the_house();
    }
}

unsafe fn parse_val(value: *const c_char, parsed: *mut c_int) -> bool {
    unsafe {
        *__errno_location() = 0;
    }
    let mut end = value.cast_mut();
    let temporary = unsafe { strtol(value, &mut end, 10) };

    if end != value.cast_mut()
        && unsafe { *__errno_location() } == 0
        && temporary >= c_int::MIN as c_long
        && temporary <= c_int::MAX as c_long
    {
        unsafe {
            *parsed = temporary as c_int;
        }
        true
    } else {
        false
    }
}

/// Parses the input and runs two update cycles when parsing succeeds.
///
/// # Safety
///
/// `input` must point to a readable NUL-terminated string. Calls that access
/// the global house must be externally synchronized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    let mut value = 0;
    if unsafe { parse_val(input, &mut value) } {
        unsafe {
            run(value);
            run(value);
        }
    } else {
        unsafe {
            printf(ERROR_MESSAGE.as_ptr().cast());
        }
    }
}
