use std::os::raw::{c_char, c_double, c_int, c_ulonglong};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn driver(f: c_double) {
    let bits = f.to_bits() as c_ulonglong;

    unsafe {
        printf(
            b"%llx %a %.4f\n\0".as_ptr() as *const c_char,
            bits,
            f,
            f,
        );
    }
}

fn main() {
    let mut f: c_double = 0.0;

    unsafe {
        scanf(b"%lf\0".as_ptr() as *const c_char, &mut f as *mut c_double);
    }

    driver(f);
}
