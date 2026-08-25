use std::ffi::{c_char, c_int};
use std::mem::size_of;

unsafe extern "C" {
    #[link_name = "__isoc99_scanf"]
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn putchar(character: c_int) -> c_int;
}

#[no_mangle]
pub unsafe extern "C" fn driver(x: c_int) {
    let bytes = unsafe {
        std::slice::from_raw_parts((&x as *const c_int).cast::<u8>(), size_of::<c_int>())
    };

    for byte in bytes {
        unsafe {
            printf(b"%02x\0".as_ptr().cast(), c_int::from(*byte));
        }
    }
    unsafe {
        putchar(c_int::from(b'\n'));
    }
}

#[no_mangle]
pub unsafe extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut x);
        driver(x);
    }
    0
}
