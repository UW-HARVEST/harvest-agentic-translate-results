use std::ffi::{c_char, c_int, c_long};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct house_t {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: f64,
}

extern "C" {
    static stdin: *mut libc::FILE;
}

fn add_floor(house: &mut house_t) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut house_t, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn print_house(house: &house_t) {
    let fmt = b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0";
    unsafe {
        libc::printf(
            fmt.as_ptr() as *const c_char,
            house.floors,
            house.bedrooms,
            house.bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(the_house: *mut house_t, extra_bedrooms: c_int) {
    let house = unsafe { &mut *the_house };
    print_house(house);
    add_floor(house);
    print_house(house);
    house.bathrooms += 1.0;
    print_house(house);
    add_bedrooms(house, extra_bedrooms);
    print_house(house);
}

fn parse_val(s: *const c_char, val: *mut c_int) -> bool {
    unsafe {
        *libc::__errno_location() = 0;
        let mut endp: *mut c_char = s as *mut c_char;
        let tmp: c_long = libc::strtol(s, &mut endp as *mut *mut c_char, 10);
        if endp as *const c_char != s
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
pub extern "C" fn main() -> c_int {
    unsafe {
        let mut input: [c_char; 100] = [0; 100];
        libc::fgets(input.as_mut_ptr(), 100, stdin);
        let mut x: c_int = 0;
        if parse_val(input.as_ptr(), &mut x) {
            let mut the_house = house_t {
                floors: 2,
                bedrooms: 5,
                bathrooms: 2.5,
            };
            run(&mut the_house as *mut house_t, x);
            run(&mut the_house as *mut house_t, x);
        } else {
            let msg = b"An error occurred\n\0";
            libc::printf(msg.as_ptr() as *const c_char);
        }
    }
    0
}
