use std::ffi::c_int;

extern "C" {
    fn pow(base: f64, exponent: f64) -> f64;
    fn __errno_location() -> *mut c_int;
    fn fprintf(stream: *mut libc_FILE, format: *const u8, ...) -> c_int;
    static stderr: *mut libc_FILE;
}

#[repr(C)]
struct libc_FILE {
    _opaque: [u8; 0],
}

const EDOM: c_int = 33;
const ERANGE: c_int = 34;

unsafe fn set_errno(val: c_int) {
    unsafe { *__errno_location() = val };
}

unsafe fn get_errno() -> c_int {
    unsafe { *__errno_location() }
}

#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: f64, exponent: f64) -> f64 {
    unsafe {
        set_errno(0);
        let result = pow(base, exponent);
        let err = get_errno();
        if err == EDOM {
            fprintf(
                stderr,
                b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0"
                    .as_ptr(),
                base,
                exponent,
            );
            return -1.0;
        } else if err == ERANGE {
            fprintf(
                stderr,
                b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0".as_ptr(),
                base,
                exponent,
            );
            return -1.0;
        }
        result
    }
}
