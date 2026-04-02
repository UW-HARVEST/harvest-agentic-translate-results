use std::ffi::c_char;
use std::sync::Mutex;

struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

static THE_HOUSE: Mutex<House> = Mutex::new(House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
});

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn print_the_house() {
    let h = THE_HOUSE.lock().unwrap();
    unsafe {
        libc::printf(
            b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0".as_ptr() as *const c_char,
            h.floors,
            h.bedrooms,
            h.bathrooms,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: i32) {
    print_the_house();
    {
        let mut h = THE_HOUSE.lock().unwrap();
        add_floor(&mut h);
    }
    print_the_house();
    {
        let mut h = THE_HOUSE.lock().unwrap();
        h.bathrooms += 1.0;
    }
    print_the_house();
    {
        let mut h = THE_HOUSE.lock().unwrap();
        add_bedrooms(&mut h, extra_bedrooms);
    }
    print_the_house();
}

fn parse_val(s: *const c_char, val: &mut i32) -> bool {
    unsafe {
        *libc::__errno_location() = 0;
        let mut endp: *mut c_char = s as *mut c_char;
        let tmp = libc::strtol(s, &mut endp, 10);
        if endp != s as *mut c_char
            && *libc::__errno_location() == 0
            && tmp >= i32::MIN as libc::c_long
            && tmp <= i32::MAX as libc::c_long
        {
            *val = tmp as i32;
            true
        } else {
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    let mut x: i32 = 0;
    if parse_val(input, &mut x) {
        run(x);
        run(x);
    } else {
        unsafe {
            libc::printf(b"An error occurred\n\0".as_ptr() as *const c_char);
        }
    }
}
