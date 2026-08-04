use std::ffi::c_int;

extern "C" {
    fn div(numer: c_int, denom: c_int) -> libc_div_t;
    fn printf(format: *const u8, ...) -> c_int;
}

#[repr(C)]
struct libc_div_t {
    quot: c_int,
    rem: c_int,
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    unsafe {
        let result = div(x, y);
        printf(
            b"quotient: %d, remainder: %d\n\0".as_ptr(),
            result.quot,
            result.rem,
        );
    }
}
