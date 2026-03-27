use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let quot = x / y;
    let rem = x % y;
    unsafe {
        printf(
            b"quotient: %d, remainder: %d\n\0".as_ptr(),
            quot,
            rem,
        );
    }
}
