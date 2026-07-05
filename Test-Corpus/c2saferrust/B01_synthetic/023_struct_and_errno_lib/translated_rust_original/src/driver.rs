




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
fn add_floor(house: *mut house_t) {
    if let Some(house) = unsafe { house.as_mut() } {
        house.floors += 1;
    }
}

fn add_bedrooms(house: &mut house_t, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn print_house(house: &house_t) {
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors,
        house.bedrooms,
        house.bathrooms,
    );
}

#[no_mangle]
pub fn run(the_house: &mut house_t, extra_bedrooms: ::core::ffi::c_int) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    the_house.bathrooms += 1.0f64;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

fn parse_val(str: *const ::core::ffi::c_char, val: &mut ::core::ffi::c_int) -> bool {
    if str.is_null() {
        return false;
    }

    let s = unsafe { CStr::from_ptr(str) };
    let bytes = s.to_bytes();

    if bytes.is_empty() {
        return false;
    }

    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t,
        Err(_) => return false,
    };

    match text.parse::<::core::ffi::c_int>() {
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
        let mut the_house = house_t {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        println!("An error occurred");
    }
}

pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
