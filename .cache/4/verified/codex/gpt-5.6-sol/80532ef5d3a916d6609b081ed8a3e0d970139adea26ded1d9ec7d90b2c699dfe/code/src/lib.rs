use std::ffi::{c_char, c_int, c_long, c_void};
use std::ptr;

#[repr(C)]
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

unsafe extern "C" {
    static mut stdin: *mut c_void;

    fn __errno_location() -> *mut c_int;
    fn fgets(buffer: *mut c_char, size: c_int, stream: *mut c_void) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
    fn strtol(input: *const c_char, end: *mut *mut c_char, base: c_int) -> c_long;
}

unsafe fn print_the_house(house: *const House) {
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
    let house = ptr::addr_of_mut!(THE_HOUSE);

    unsafe {
        print_the_house(house);
        (*house).floors = (*house).floors.wrapping_add(1);
        print_the_house(house);
        (*house).bathrooms += 1.0;
        print_the_house(house);
        (*house).bedrooms = (*house).bedrooms.wrapping_add(extra_bedrooms);
        print_the_house(house);
    }
}

unsafe fn parse_val(input: *const c_char, value: *mut c_int) -> bool {
    let mut end = input.cast_mut();

    unsafe {
        *__errno_location() = 0;
        let parsed = strtol(input, &mut end, 10);
        if end != input.cast_mut()
            && *__errno_location() == 0
            && parsed >= c_int::MIN as c_long
            && parsed <= c_int::MAX as c_long
        {
            *value = parsed as c_int;
            true
        } else {
            false
        }
    }
}

#[unsafe(export_name = "main")]
pub unsafe extern "C" fn driver_main() -> c_int {
    let mut input = [0 as c_char; 100];
    let mut value = 0;

    unsafe {
        fgets(input.as_mut_ptr(), input.len() as c_int, stdin);
        if parse_val(input.as_ptr(), &mut value) {
            run(value);
            run(value);
        } else {
            printf(c"An error occurred\n".as_ptr());
        }
    }

    0
}
