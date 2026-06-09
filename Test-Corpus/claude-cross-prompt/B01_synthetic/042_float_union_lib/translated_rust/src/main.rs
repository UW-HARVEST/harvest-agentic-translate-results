// Rust translation of c_src/src/driver.c with a main entry point.
//
// The original C code provides a library function `driver(double f)` that
// prints a double in three formats. To make this an executable that produces
// byte-identical output, we read doubles from stdin (using scanf semantics,
// which reads across whitespace including newlines) and feed each one to
// `driver`.
//
// We delegate the actual formatting to libc's printf so the `%a` and `%.4f`
// output exactly matches what the C program would produce on the same system.

use std::os::raw::{c_char, c_double, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn scanf(fmt: *const c_char, ...) -> c_int;
}

fn driver(f: c_double) {
    // raw_double_t union: reinterpret the double's bits as a uint64_t.
    let x: u64 = f.to_bits();
    // printf("%llx %a %.4f\n", u.x, f, f);
    let fmt = b"%llx %a %.4f\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, x, f, f);
    }
}

fn main() {
    let fmt = b"%lf\0".as_ptr() as *const c_char;
    loop {
        let mut value: c_double = 0.0;
        let r = unsafe { scanf(fmt, &mut value as *mut c_double) };
        if r != 1 {
            break;
        }
        driver(value);
    }
}
