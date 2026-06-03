use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

// Mirror C's div_t struct: { int quot; int rem; }
#[repr(C)]
struct DivT {
    quot: c_int,
    rem: c_int,
}

unsafe extern "C" {
    fn div(numer: c_int, denom: c_int) -> DivT;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    // Use C's div() to match div_t behavior exactly (including UB on y == 0).
    let result = unsafe { div(x, y) };
    // Use C's printf for byte-identical output.
    let fmt = b"quotient: %d, remainder: %d\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, result.quot, result.rem);
    }
}
