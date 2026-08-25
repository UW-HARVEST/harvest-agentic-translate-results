use std::ffi::{c_char, c_double, c_int};
use std::ptr;

const EDOM: c_int = 33;
const ERANGE: c_int = 34;

#[repr(C)]
struct CFile {
    _private: [u8; 0],
}

unsafe extern "C" {
    static mut stderr: *mut CFile;

    fn __errno_location() -> *mut c_int;
    fn fprintf(stream: *mut CFile, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn strtod(input: *const c_char, end: *mut *mut c_char) -> c_double;
}

#[link(name = "m")]
unsafe extern "C" {
    fn pow(base: c_double, exponent: c_double) -> c_double;
}

unsafe fn set_errno(value: c_int) {
    // SAFETY: callers invoke this through the same libc ABI used by the C code.
    unsafe {
        *__errno_location() = value;
    }
}

unsafe fn errno() -> c_int {
    // SAFETY: callers invoke this through the same libc ABI used by the C code.
    unsafe { *__errno_location() }
}

#[cfg_attr(not(test), no_mangle)]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if argv.is_null() {
        let mut end = ptr::null_mut();
        // C terminates with SIGSEGV when it first dereferences a null argv.
        unsafe {
            strtod(ptr::null(), &mut end);
        }
    }

    if argc != 3 {
        // SAFETY: this intentionally has the same pointer requirements as C main.
        unsafe {
            fprintf(
                stderr,
                c"Usage: %s base exponent\n".as_ptr(),
                *argv.wrapping_add(0),
            );
        }
        return 1;
    }

    let mut endptr1 = ptr::null_mut();
    let mut endptr2 = ptr::null_mut();

    unsafe {
        set_errno(0);
    }
    // SAFETY: this intentionally has the same pointer requirements as C main.
    let base = unsafe { strtod(*argv.wrapping_add(1), &mut endptr1) };
    let conversion_errno = unsafe { errno() };
    if conversion_errno == ERANGE {
        unsafe {
            fprintf(
                stderr,
                c"Range error while converting base '%s'\n".as_ptr(),
                *argv.wrapping_add(1),
            );
        }
        return 1;
    } else if unsafe { *endptr1 != 0 } {
        unsafe {
            fprintf(
                stderr,
                c"Invalid numeric input for base: '%s'\n".as_ptr(),
                *argv.wrapping_add(1),
            );
        }
        return 1;
    }

    unsafe {
        set_errno(0);
    }
    // SAFETY: this intentionally has the same pointer requirements as C main.
    let exponent = unsafe { strtod(*argv.wrapping_add(2), &mut endptr2) };
    let conversion_errno = unsafe { errno() };
    if conversion_errno == ERANGE {
        unsafe {
            fprintf(
                stderr,
                c"Range error while converting exponent '%s'\n".as_ptr(),
                *argv.wrapping_add(2),
            );
        }
        return 1;
    } else if unsafe { *endptr2 != 0 } {
        unsafe {
            fprintf(
                stderr,
                c"Invalid numeric input for exponent: '%s'\n".as_ptr(),
                *argv.wrapping_add(2),
            );
        }
        return 1;
    }

    unsafe {
        set_errno(0);
    }
    let result = unsafe { pow(base, exponent) };
    let power_errno = unsafe { errno() };
    if power_errno == EDOM {
        unsafe {
            fprintf(
                stderr,
                c"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n".as_ptr(),
                base,
                exponent,
            );
        }
        return 1;
    } else if power_errno == ERANGE {
        unsafe {
            fprintf(
                stderr,
                c"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n".as_ptr(),
                base,
                exponent,
            );
        }
        return 1;
    }

    unsafe {
        printf(c"Result: %.2f\n".as_ptr(), result);
    }
    0
}
