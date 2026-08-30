use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn driver(x: c_int) {
    let y = x.wrapping_mul(2).wrapping_add(300);
    unsafe {
        printf(c"%d\n".as_ptr(), y);
    }
}

fn main() {
    let mut x: c_int = 0;
    unsafe {
        scanf(c"%d".as_ptr(), &mut x);
    }
    driver(x);
}
