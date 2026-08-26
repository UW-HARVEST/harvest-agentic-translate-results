use std::os::raw::{c_char, c_int};

const INPUT_FORMAT: &[u8] = b"%d\0";
const OUTPUT_FORMAT: &[u8] = b"%d\n\0";

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn run_driver(x: c_int) {
    let y = x.wrapping_mul(2).wrapping_add(300);

    unsafe {
        printf(OUTPUT_FORMAT.as_ptr().cast(), y);
    }
}

#[no_mangle]
pub extern "C" fn driver(x: c_int) {
    run_driver(x);
}

#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut x: c_int = 0;

    unsafe {
        scanf(INPUT_FORMAT.as_ptr().cast(), &mut x);
    }
    run_driver(x);
    0
}
