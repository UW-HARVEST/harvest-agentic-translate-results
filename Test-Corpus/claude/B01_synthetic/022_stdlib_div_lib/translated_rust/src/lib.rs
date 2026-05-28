use std::ffi::{c_char, c_int};

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    // C's div() performs truncated integer division.
    // Rust's / and % on i32 also truncate toward zero, matching C's div semantics.
    let quot: c_int = x / y;
    let rem: c_int = x % y;

    // Use libc's printf to produce byte-identical output to the C version.
    // The format string must be a null-terminated C string.
    let fmt = b"quotient: %d, remainder: %d\n\0";
    unsafe {
        libc::printf(fmt.as_ptr() as *const c_char, quot, rem);
    }
}
