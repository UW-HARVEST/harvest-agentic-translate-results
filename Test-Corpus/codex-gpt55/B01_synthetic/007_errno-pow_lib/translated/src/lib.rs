use std::ffi::{c_double, c_int};

const EDOM_VALUE: c_int = libc::EDOM;
const ERANGE_VALUE: c_int = libc::ERANGE;

static DOMAIN_ERROR: &[u8] = b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0";
static RANGE_ERROR: &[u8] = b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0";

#[link(name = "m")]
unsafe extern "C" {
    #[link_name = "pow"]
    fn c_pow(base: c_double, exponent: c_double) -> c_double;
}

unsafe extern "C" {
    static mut stderr: *mut libc::FILE;
}

#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double {
    unsafe {
        *libc::__errno_location() = 0;
        let result = c_pow(base, exponent);
        let errno = *libc::__errno_location();

        if errno == EDOM_VALUE {
            libc::fprintf(
                stderr,
                DOMAIN_ERROR.as_ptr().cast(),
                base,
                exponent,
            );
            -1.0
        } else if errno == ERANGE_VALUE {
            libc::fprintf(
                stderr,
                RANGE_ERROR.as_ptr().cast(),
                base,
                exponent,
            );
            -1.0
        } else {
            result
        }
    }
}
