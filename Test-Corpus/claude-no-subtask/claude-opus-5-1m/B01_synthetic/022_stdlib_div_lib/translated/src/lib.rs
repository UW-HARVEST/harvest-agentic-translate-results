use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    // Replicate C's div(x, y): quotient truncates toward zero, remainder has the
    // sign of the dividend. Rust's `/` and `%` on signed integers match this
    // behavior (C99 truncation semantics). Division by zero in C is undefined
    // behavior; the C source has no guard, so we mirror that exactly.
    let quot: c_int = x / y;
    let rem: c_int = x % y;
    unsafe {
        let fmt = b"quotient: %d, remainder: %d\n\0".as_ptr() as *const core::ffi::c_char;
        libc::printf(fmt, quot, rem);
    }
}
