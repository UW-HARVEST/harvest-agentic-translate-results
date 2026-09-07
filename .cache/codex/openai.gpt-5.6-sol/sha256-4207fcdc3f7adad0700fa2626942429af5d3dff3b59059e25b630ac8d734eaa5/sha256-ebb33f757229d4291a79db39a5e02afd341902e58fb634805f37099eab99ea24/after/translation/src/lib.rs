use std::ffi::{c_char, c_int, c_uint};
use std::mem::size_of;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    const HEX_FORMAT: &[u8] = b"%02x\0";
    const NEWLINE: &[u8] = b"\n\0";

    let bytes = (&x as *const c_int).cast::<u8>();
    for index in 0..size_of::<c_int>() {
        // C's variadic integer promotion passes each unsigned byte as unsigned int.
        unsafe {
            printf(
                HEX_FORMAT.as_ptr().cast::<c_char>(),
                *bytes.add(index) as c_uint,
            );
        }
    }
    unsafe {
        printf(NEWLINE.as_ptr().cast::<c_char>());
    }
}
