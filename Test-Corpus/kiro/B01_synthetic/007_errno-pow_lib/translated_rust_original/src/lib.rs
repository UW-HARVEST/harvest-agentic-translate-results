use std::ffi::c_double;

extern "C" {
    fn pow(base: c_double, exponent: c_double) -> c_double;
    fn fprintf(stream: *mut libc_FILE, format: *const i8, ...) -> i32;
    fn __errno_location() -> *mut i32;
    static stderr: *mut libc_FILE;
}

// Opaque type for C FILE
#[repr(C)]
struct libc_FILE {
    _opaque: [u8; 0],
}

const EDOM: i32 = 33;
const ERANGE: i32 = 34;

unsafe fn get_errno() -> i32 {
    unsafe { *__errno_location() }
}

unsafe fn set_errno(val: i32) {
    unsafe { *__errno_location() = val }
}

#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double {
    unsafe {
        set_errno(0);
        let result = pow(base, exponent);
        let err = get_errno();
        if err == EDOM {
            fprintf(
                stderr,
                b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0"
                    .as_ptr() as *const i8,
                base,
                exponent,
            );
            return -1.0;
        } else if err == ERANGE {
            fprintf(
                stderr,
                b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0".as_ptr()
                    as *const i8,
                base,
                exponent,
            );
            return -1.0;
        }
        result
    }
}
