use std::cell::UnsafeCell;
use std::ffi::{c_char, c_double, c_int, c_long};

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

struct GlobalHouse(UnsafeCell<House>);

unsafe impl Sync for GlobalHouse {}

static THE_HOUSE: GlobalHouse = GlobalHouse(UnsafeCell::new(House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
}));

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

unsafe fn add_floor_to_the_house() {
    unsafe {
        add_floor(THE_HOUSE.0.get());
    }
}

unsafe fn print_the_house() {
    unsafe {
        let house = THE_HOUSE.0.get();
        libc::printf(
            b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0".as_ptr()
                as *const c_char,
            (*house).floors,
            (*house).bedrooms,
            (*house).bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    unsafe {
        print_the_house();
        add_floor_to_the_house();
        print_the_house();
        (*THE_HOUSE.0.get()).bathrooms += 1.0;
        print_the_house();
        add_bedrooms(THE_HOUSE.0.get(), extra_bedrooms);
        print_the_house();
    }
}

unsafe fn errno_location() -> *mut c_int {
    unsafe { libc::__errno_location() }
}

unsafe fn parse_val(str_: *const c_char, val: *mut c_int) -> bool {
    unsafe {
        *errno_location() = 0;
        let mut endp = str_ as *mut c_char;
        let tmp = libc::strtol(str_, &mut endp, 10);
        if endp != str_ as *mut c_char
            && *errno_location() == 0
            && tmp >= c_int::MIN as c_long
            && tmp <= c_int::MAX as c_long
        {
            *val = tmp as c_int;
            true
        } else {
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(in_: *const c_char) {
    unsafe {
        let mut x: c_int = 0;
        if parse_val(in_, &mut x) {
            run(x);
            run(x);
        } else {
            libc::printf(b"An error occurred\n\0".as_ptr() as *const c_char);
        }
    }
}
