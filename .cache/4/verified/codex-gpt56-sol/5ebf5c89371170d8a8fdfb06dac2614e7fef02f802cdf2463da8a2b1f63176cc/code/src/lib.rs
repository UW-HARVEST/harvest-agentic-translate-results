use std::ffi::{c_char, c_int};

unsafe extern "C" {
    #[link_name = "__isoc99_scanf"]
    fn c_scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn putchar(character: c_int) -> c_int;
}

fn print_hex(bytes: &[u8]) {
    for byte in bytes {
        unsafe {
            printf(c"%02x".as_ptr(), c_int::from(*byte));
        }
    }

    unsafe {
        putchar(c_int::from(b'\n'));
    }
}

#[no_mangle]
pub extern "C" fn driver(x: f32) {
    print_hex(&x.to_ne_bytes());
}

#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut x = 0.0_f32;

    unsafe {
        c_scanf(c"%f".as_ptr(), &mut x);
    }
    driver(x);
    0
}
