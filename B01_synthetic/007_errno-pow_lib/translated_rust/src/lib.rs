use std::ffi::c_double;

extern "C" {
    fn pow(base: c_double, exponent: c_double) -> c_double;
    fn fprintf(stream: *mut libc::FILE, format: *const libc::c_char, ...) -> libc::c_int;
    static mut stderr: *mut libc::FILE;
}

#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double {
    unsafe {
        *libc::__errno_location() = 0;
        let result = pow(base, exponent);
        let errno_val = *libc::__errno_location();
        if errno_val == libc::EDOM {
            fprintf(
                stderr,
                b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0"
                    .as_ptr() as *const libc::c_char,
                base,
                exponent,
            );
            return -1.0;
        } else if errno_val == libc::ERANGE {
            fprintf(
                stderr,
                b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0".as_ptr()
                    as *const libc::c_char,
                base,
                exponent,
            );
            return -1.0;
        }
        result
    }
}
