use std::os::raw::{c_char, c_int};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn scan_decimal(value: &mut c_int) {
    // SAFETY: the format is NUL-terminated and %d expects a valid int pointer.
    unsafe {
        scanf(b"%d\0".as_ptr().cast(), value);
    }
}

fn driver(x: c_int, y: c_int) {
    let result = x | !y;
    println!("{result}");
}

fn main() {
    let mut x: c_int = 0;
    let mut y: c_int = 0;
    scan_decimal(&mut x);
    scan_decimal(&mut y);
    driver(x, y);
}
