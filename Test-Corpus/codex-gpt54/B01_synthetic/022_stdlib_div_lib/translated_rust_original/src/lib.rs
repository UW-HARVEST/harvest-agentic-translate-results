use std::ffi::{c_char, c_int};

#[repr(C)]
struct DivT {
    quot: c_int,
    rem: c_int,
}

unsafe extern "C" {
    fn div(numer: c_int, denom: c_int) -> DivT;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    static FORMAT: &[u8] = b"quotient: %d, remainder: %d\n\0";

    unsafe {
        let result = div(x, y);
        let _ = printf(FORMAT.as_ptr().cast(), result.quot, result.rem);
    }
}
