use std::ffi::{c_double, c_int};

unsafe extern "C" {
    fn pow(base: c_double, exponent: c_double) -> c_double;
    fn __errno_location() -> *mut c_int;
    static mut stderr: *mut libc::FILE;
}

const EDOM: c_int = 33;
const ERANGE: c_int = 34;

#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double {
    unsafe {
        *__errno_location() = 0;
        let result = pow(base, exponent);
        let err = *__errno_location();
        if err == EDOM {
            libc::fprintf(
                stderr,
                b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0"
                    .as_ptr() as *const libc::c_char,
                base,
                exponent,
            );
            return -1.0;
        } else if err == ERANGE {
            libc::fprintf(
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
