use std::ffi::{c_char, c_int};

#[repr(C)]
struct DivResult {
    quot: c_int,
    rem: c_int,
}

unsafe extern "C" {
    fn div(numerator: c_int, denominator: c_int) -> DivResult;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_int, y: c_int) {
    let result = unsafe { div(x, y) };
    unsafe {
        printf(
            c"quotient: %d, remainder: %d\n".as_ptr(),
            result.quot,
            result.rem,
        );
    }
}
