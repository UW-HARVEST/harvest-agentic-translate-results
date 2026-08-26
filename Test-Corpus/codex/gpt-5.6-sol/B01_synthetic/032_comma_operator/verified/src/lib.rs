use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;

    #[link_name = "__isoc99_scanf"]
    fn scanf(format: *const c_char, ...) -> c_int;
}

const OUTPUT_FORMAT: &[u8] = b"%d %d\n\0";
const INPUT_FORMAT: &[u8] = b"%d\0";

#[no_mangle]
pub unsafe extern "C" fn driver(x: c_int) {
    let mut i = 0_i32;
    let mut j = 0_i32;

    while i < x {
        unsafe {
            printf(OUTPUT_FORMAT.as_ptr().cast(), i, j);
        }
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

#[cfg_attr(not(test), no_mangle)]
pub unsafe extern "C" fn main() -> c_int {
    let mut x = 0_i32;

    unsafe {
        scanf(INPUT_FORMAT.as_ptr().cast(), &mut x);
        driver(x);
    }

    0
}
