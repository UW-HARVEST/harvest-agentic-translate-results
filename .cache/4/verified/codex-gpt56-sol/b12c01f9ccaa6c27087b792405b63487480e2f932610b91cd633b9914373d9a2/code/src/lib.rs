use std::ffi::{c_char, c_int, c_long, c_void};

#[repr(C)]
pub struct House {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: f64,
}

unsafe extern "C" {
    static mut stdin: *mut c_void;

    fn __errno_location() -> *mut c_int;
    fn fgets(buffer: *mut c_char, size: c_int, stream: *mut c_void) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
    fn strlen(value: *const c_char) -> usize;
    fn strtol(value: *const c_char, end: *mut *mut c_char, base: c_int) -> c_long;
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

#[no_mangle]
pub unsafe extern "C" fn run(house: *mut House, extra_bedrooms: c_int) {
    unsafe {
        if house.is_null() {
            std::hint::black_box(strlen(house.cast()));
        }
        print_house(house);
        (*house).floors = (*house).floors.wrapping_add(1);
        print_house(house);
        (*house).bathrooms += 1.0;
        print_house(house);
        (*house).bedrooms = (*house).bedrooms.wrapping_add(extra_bedrooms);
        print_house(house);
    }
}

unsafe fn parse_val(value: *const c_char, output: *mut c_int) -> bool {
    unsafe {
        *__errno_location() = 0;
        let mut end = value.cast_mut();
        let parsed = strtol(value, &mut end, 10);
        if end.cast_const() != value
            && *__errno_location() == 0
            && parsed >= c_int::MIN as c_long
            && parsed <= c_int::MAX as c_long
        {
            *output = parsed as c_int;
            true
        } else {
            false
        }
    }
}

#[export_name = "main"]
pub unsafe extern "C" fn ffi_main() -> c_int {
    let mut input = [0 as c_char; 100];
    let mut parsed = 0;

    unsafe {
        fgets(input.as_mut_ptr(), input.len() as c_int, stdin);
        if parse_val(input.as_ptr(), &mut parsed) {
            let mut house = House {
                floors: 2,
                bedrooms: 5,
                bathrooms: 2.5,
            };
            run(&mut house, parsed);
            run(&mut house, parsed);
        } else {
            printf(ERROR_MESSAGE.as_ptr().cast());
        }
    }

    0
}
