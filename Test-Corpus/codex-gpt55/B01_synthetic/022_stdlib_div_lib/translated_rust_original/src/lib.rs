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
    unsafe {
        let result = div(x, y);
        printf(
            c"quotient: %d, remainder: %d\n".as_ptr(),
            result.quot,
            result.rem,
        );
    }
}
