use std::ffi::{c_char, c_int};

unsafe extern "C" {
    #[link_name = "__isoc99_scanf"]
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(string: *const c_char) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let result = x | !y;

    // SAFETY: The static format strings match the C argument types.
    unsafe {
        printf(c"%d".as_ptr(), result);
        puts(c"".as_ptr());
    }
}

#[cfg_attr(not(test), unsafe(no_mangle))]
pub extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    let mut y: c_int = 0;

    // SAFETY: Each destination points to a live C int.
    unsafe {
        scanf(c"%d".as_ptr(), &mut x);
        scanf(c"%d".as_ptr(), &mut y);
    }

    driver(x, y);
    0
}
