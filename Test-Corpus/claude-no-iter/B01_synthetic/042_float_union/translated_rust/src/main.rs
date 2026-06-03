// Translated from C: src/main.c
//
// Reads a double from stdin via scanf("%lf"), then prints the raw 64-bit
// representation as hex, the value in C99 %a format, and the value with
// %.4f.  We delegate to libc's scanf and printf so the output is
// byte-identical to the original program (in particular for the %a
// hexadecimal-float format which Rust's standard library does not
// implement).

use std::os::raw::{c_char, c_double, c_int};

#[repr(C)]
union RawDouble {
    x: u64,
    f: c_double,
}

extern "C" {
    fn scanf(fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn driver(f: c_double) {
    // Reproduce: raw_double_t u = {.f = f}; printf("%llx %a %.4f\n", u.x, f, f);
    // SAFETY: union access reads the bit pattern of the double; both
    // representations are the same size.  The printf call passes the
    // expected types matching the C format string.
    let bits: u64 = unsafe { RawDouble { f }.x };
    let fmt = b"%llx %a %.4f\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, bits as core::ffi::c_ulonglong, f, f);
    }
}

fn main() {
    // Match C's `double f = 0.0f; scanf("%lf", &f);` behaviour.  If the
    // scan fails or matches nothing, `f` keeps its initial value of 0.0.
    let mut f: c_double = 0.0;
    let scan_fmt = b"%lf\0".as_ptr() as *const c_char;
    unsafe {
        scanf(scan_fmt, &mut f as *mut c_double);
    }
    driver(f);
}
