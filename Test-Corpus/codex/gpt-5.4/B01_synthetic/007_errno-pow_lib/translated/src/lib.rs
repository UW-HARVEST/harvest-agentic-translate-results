use libc::{c_char, c_int, FILE, EDOM, ERANGE};

static DOMAIN_ERROR_FORMAT: &[u8] =
    b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0";
static RANGE_ERROR_FORMAT: &[u8] =
    b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0";

unsafe extern "C" {
    #[link_name = "pow"]
    fn c_pow(base: f64, exponent: f64) -> f64;

    #[link_name = "fprintf"]
    fn c_fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;

    #[link_name = "__errno_location"]
    fn errno_location() -> *mut c_int;

    static mut stderr: *mut FILE;
}

#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: f64, exponent: f64) -> f64 {
    unsafe {
        *errno_location() = 0;

        let result = c_pow(base, exponent);

        if *errno_location() == EDOM {
            c_fprintf(stderr, DOMAIN_ERROR_FORMAT.as_ptr().cast(), base, exponent);
            return -1.0;
        } else if *errno_location() == ERANGE {
            c_fprintf(stderr, RANGE_ERROR_FORMAT.as_ptr().cast(), base, exponent);
            return -1.0;
        }

        result
    }
}
