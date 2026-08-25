use std::ffi::{c_char, c_double, c_int, c_void};

const EDOM: c_int = 33;
const ERANGE: c_int = 34;

const DOMAIN_ERROR: &[u8] =
    b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0";
const RANGE_ERROR: &[u8] = b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0";

#[link(name = "m")]
unsafe extern "C" {
    fn pow(base: c_double, exponent: c_double) -> c_double;
}

unsafe extern "C" {
    fn __errno_location() -> *mut c_int;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    static mut stderr: *mut c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double {
    unsafe {
        *__errno_location() = 0;
        let result = pow(base, exponent);

        if *__errno_location() == EDOM {
            fprintf(
                stderr,
                DOMAIN_ERROR.as_ptr().cast::<c_char>(),
                base,
                exponent,
            );
            -1.0
        } else if *__errno_location() == ERANGE {
            fprintf(
                stderr,
                RANGE_ERROR.as_ptr().cast::<c_char>(),
                base,
                exponent,
            );
            -1.0
        } else {
            result
        }
    }
}
