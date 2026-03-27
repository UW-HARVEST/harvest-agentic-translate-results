use std::ffi::c_char;
use std::ffi::c_int;

#[repr(C)]
pub struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn add_floor(house: &mut HouseT) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut HouseT, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn print_house(house: &HouseT) {
    unsafe {
        libc::printf(
            b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0".as_ptr() as *const c_char,
            house.floors as c_int,
            house.bedrooms as c_int,
            house.bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(the_house: *mut HouseT, extra_bedrooms: c_int) {
    let house = unsafe { &mut *the_house };
    print_house(house);
    add_floor(house);
    print_house(house);
    house.bathrooms += 1.0;
    print_house(house);
    add_bedrooms(house, extra_bedrooms);
    print_house(house);
}

fn parse_val(str: *const c_char, val: &mut c_int) -> bool {
    unsafe {
        *libc::__errno_location() = 0;
        let mut endp: *mut c_char = str as *mut c_char;
        let tmp = libc::strtol(str, &mut endp, 10);
        if endp != str as *mut c_char
            && *libc::__errno_location() == 0
            && tmp >= c_int::MIN as libc::c_long
            && tmp <= c_int::MAX as libc::c_long
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
    if parse_val(input, &mut x) {
        let mut the_house = HouseT {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        unsafe {
            libc::printf(b"An error occurred\n\0".as_ptr() as *const c_char);
        }
    }
}
