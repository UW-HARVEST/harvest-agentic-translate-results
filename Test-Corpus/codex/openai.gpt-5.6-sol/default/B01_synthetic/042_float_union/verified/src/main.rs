use std::ffi::{c_char, c_double, c_int, c_ulonglong};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn main() {
    let mut f: c_double = 0.0;

    unsafe {
        scanf(c"%lf".as_ptr(), &mut f);
        printf(c"%llx %a %.4f\n".as_ptr(), f.to_bits() as c_ulonglong, f, f);
    }
}
