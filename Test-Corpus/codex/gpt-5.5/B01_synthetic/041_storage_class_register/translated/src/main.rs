use std::os::raw::{c_char, c_int};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn driver(x: c_int) {
    let mut y = x.wrapping_mul(2);
    y = y.wrapping_add(300);
    println!("{}", y);
}

fn main() {
    let mut x: c_int = 0;
    unsafe {
        scanf(b"%d\0".as_ptr().cast::<c_char>(), &mut x);
    }
    driver(x);
}
