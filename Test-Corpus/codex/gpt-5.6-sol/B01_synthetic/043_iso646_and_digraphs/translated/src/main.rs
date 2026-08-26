use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(string: *const c_char) -> c_int;
}

fn driver(x: c_int, y: c_int) {
    let result = x | !y;

    // SAFETY: Both format strings are static and null-terminated, and result
    // has the C int type required by "%d".
    unsafe {
        printf(c"%d".as_ptr(), result);
        puts(c"".as_ptr());
    }
}

fn main() {
    let mut x: c_int = 0;
    let mut y: c_int = 0;

    // SAFETY: The format string is static and null-terminated, and each
    // destination points to a live C int.
    unsafe {
        scanf(c"%d".as_ptr(), &mut x);
        scanf(c"%d".as_ptr(), &mut y);
    }

    driver(x, y);
}
