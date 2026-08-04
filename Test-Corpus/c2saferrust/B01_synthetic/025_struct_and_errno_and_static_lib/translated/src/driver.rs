





use std::ffi::CStr;

extern "C" {
    fn __errno_location() -> *mut ::core::ffi::c_int;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strtol(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct house_t {
    pub floors: ::core::ffi::c_int,
    pub bedrooms: ::core::ffi::c_int,
    pub bathrooms: ::core::ffi::c_double,
}
pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
pub const INT_MIN: ::core::ffi::c_int = -__INT_MAX__ - 1 as ::core::ffi::c_int;
static mut the_house: house_t = house_t {
    floors: 2 as ::core::ffi::c_int,
    bedrooms: 5 as ::core::ffi::c_int,
    bathrooms: 2.5f64,
};
fn add_floor(house: &mut house_t) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut house_t, extra_bedrooms: ::core::ffi::c_int) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    unsafe {
        add_floor(&mut the_house);
    }
}

fn print_the_house() {
    unsafe {
        println!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
            the_house.floors,
            the_house.bedrooms,
            the_house.bathrooms,
        );
    }
}

#[no_mangle]
pub fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        the_house.bathrooms += 1.0;
        print_the_house();
        add_bedrooms(&mut the_house, extra_bedrooms);
    }
    print_the_house();
}

fn parse_val(str: *const ::core::ffi::c_char, val: &mut ::core::ffi::c_int) -> bool {
    if str.is_null() {
        return false;
    }

    let s = match unsafe { CStr::from_ptr(str) }.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    match s.trim().parse::<::core::ffi::c_int>() {
        Ok(parsed) => {
            *val = parsed;
            true
        }
        Err(_) => false,
    }
}

#[no_mangle]
pub fn driver(in_0: *const ::core::ffi::c_char) {
    let mut x: ::core::ffi::c_int = 0;
    if unsafe { parse_val(in_0, &mut x) } {
        run(x);
        run(x);
    } else {
        eprintln!("An error occurred");
    }
}

pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
