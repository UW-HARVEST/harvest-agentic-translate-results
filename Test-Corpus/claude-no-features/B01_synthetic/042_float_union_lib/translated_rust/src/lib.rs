// Translation of c_src/src/driver.c to Rust.
//
// The C function uses printf with the format string "%llx %a %.4f\n", which
// must produce byte-identical output. To guarantee that, we call into libc's
// printf directly rather than reimplementing the float-to-string conversions
// in Rust.

use std::ffi::c_char;
use std::os::raw::c_double;

extern "C" {
    fn printf(format: *const c_char, ...) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: c_double) {
    // Reinterpret the bits of the double as a uint64_t (matches the C union).
    let x: u64 = f.to_bits();

    // Format string "%llx %a %.4f\n\0" — must be null-terminated for C.
    let fmt = b"%llx %a %.4f\n\0".as_ptr() as *const c_char;

    unsafe {
        printf(fmt, x, f, f);
    }
}
