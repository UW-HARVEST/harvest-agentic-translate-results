use std::os::raw::{c_char, c_int};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn driver(x: c_int) {
    let y = x.wrapping_mul(2).wrapping_add(300);

    // C's variadic printf requires the value to retain its C int type.
    unsafe {
        printf(b"%d\n\0".as_ptr().cast(), y);
    }
}

fn main() {
    let mut x: c_int = 0;

    // Preserve scanf's whitespace handling, conversion behavior, and ignored result.
    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut x);
    }

    driver(x);
}
