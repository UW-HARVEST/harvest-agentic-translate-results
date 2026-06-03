// Translation of c_src/src/pow.c to Rust.
// Preserves the exact behavior of the original C code, including the
// libm pow() call, errno-based error checks, and stderr output format.

use std::os::raw::{c_double, c_int};

extern "C" {
    fn pow(base: c_double, exponent: c_double) -> c_double;
    // dprintf writes formatted output to the given file descriptor.
    // We use this to preserve the exact %.2f formatting that the
    // original C code uses with fprintf(stderr, ...).
    fn dprintf(fd: c_int, format: *const u8, ...) -> c_int;
    fn __errno_location() -> *mut c_int;
}

const STDERR_FILENO: c_int = 2;

fn errno() -> c_int {
    unsafe { *__errno_location() }
}

fn set_errno(value: c_int) {
    unsafe {
        *__errno_location() = value;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double {
    // Calculate power
    set_errno(0);
    let result = unsafe { pow(base, exponent) };
    let err = errno();
    if err == libc::EDOM {
        let fmt = b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0";
        unsafe {
            dprintf(STDERR_FILENO, fmt.as_ptr(), base, exponent);
        }
        return -1.0;
    } else if err == libc::ERANGE {
        let fmt = b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0";
        unsafe {
            dprintf(STDERR_FILENO, fmt.as_ptr(), base, exponent);
        }
        return -1.0;
    }

    result
}
