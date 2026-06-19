use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn driver(x: c_int) {
    let mut y = x.wrapping_mul(2);
    y = y.wrapping_add(300);

    unsafe {
        printf(b"%d\n\0".as_ptr().cast(), y);
    }
}

fn main() {
    let mut x: c_int = 0;

    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut x as *mut c_int);
    }

    driver(x);
}
