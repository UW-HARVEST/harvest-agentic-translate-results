// Translated from C (MIT Lincoln Laboratory)
// Reproduces pow.c byte-for-byte using libm's pow() and errno semantics.

use std::ffi::CString;
use std::os::raw::c_double;

extern "C" {
    fn pow(base: c_double, exponent: c_double) -> c_double;
}

#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double {
    // Calculate power
    unsafe {
        *libc::__errno_location() = 0;
    }
    let result = unsafe { pow(base, exponent) };
    let err = unsafe { *libc::__errno_location() };
    if err == libc::EDOM {
        let fmt = CString::new(
            "Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n",
        )
        .unwrap();
        unsafe {
            libc::fprintf(
                libc_stderr(),
                fmt.as_ptr(),
                base,
                exponent,
            );
        }
        return -1.0;
    } else if err == libc::ERANGE {
        let fmt = CString::new(
            "Range error: pow(%.2f, %.2f) caused overflow or underflow.\n",
        )
        .unwrap();
        unsafe {
            libc::fprintf(
                libc_stderr(),
                fmt.as_ptr(),
                base,
                exponent,
            );
        }
        return -1.0;
    }

    result
}

#[inline]
fn libc_stderr() -> *mut libc::FILE {
    extern "C" {
        static stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}
