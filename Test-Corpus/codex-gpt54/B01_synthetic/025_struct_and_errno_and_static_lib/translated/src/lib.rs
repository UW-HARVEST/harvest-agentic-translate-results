use std::cell::UnsafeCell;
use std::ffi::{c_char, c_double, c_int, c_long};
use std::ptr;

#[repr(C)]
#[derive(Copy, Clone)]
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

fn the_house() -> *mut House {
    THE_HOUSE.0.get()
}

fn add_floor(house: *mut House) {
    unsafe {
        (*house).floors += 1;
    }
}

fn add_bedrooms(house: *mut House, extra_bedrooms: c_int) {
    unsafe {
        (*house).bedrooms += extra_bedrooms;
    }
}

fn add_floor_to_the_house() {
    add_floor(the_house());
}

fn print_the_house() {
    unsafe {
        libc::printf(
            c"The house has %d floors, %d bedrooms, and %.1f bathrooms\n".as_ptr(),
            (*the_house()).floors,
            (*the_house()).bedrooms,
            (*the_house()).bathrooms,
        );
    }
}

fn parse_val(str_: *const c_char, val: *mut c_int) -> bool {
    unsafe {
        *libc::__errno_location() = 0;
        let mut endp = str_ as *mut c_char;
        let tmp: c_long = libc::strtol(str_, &mut endp, 10);
        if endp != str_ as *mut c_char
            && *libc::__errno_location() == 0
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
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        (*the_house()).bathrooms += 1.0;
    }
    print_the_house();
    add_bedrooms(the_house(), extra_bedrooms);
    print_the_house();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    let mut x: c_int = 0;
    if parse_val(input, ptr::addr_of_mut!(x)) {
        run(x);
        run(x);
    } else {
        unsafe {
            libc::printf(c"An error occurred\n".as_ptr());
        }
    }
}
