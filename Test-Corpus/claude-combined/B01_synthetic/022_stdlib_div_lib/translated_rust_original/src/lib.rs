use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    // Mirror the C behavior: div_t result = div(x, y);
    // C's div() truncates toward zero; Rust's `/` and `%` on i32 do the same.
    let quot: c_int = x / y;
    let rem: c_int = x % y;
    // Use printf so output is byte-identical to the C version.
    let fmt = b"quotient: %d, remainder: %d\n\0";
    unsafe {
        printf(fmt.as_ptr(), quot, rem);
    }
}
