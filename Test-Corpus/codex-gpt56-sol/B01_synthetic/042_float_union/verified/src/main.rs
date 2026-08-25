use std::ffi::{c_char, c_double, c_int, c_ulonglong};

extern "C" {
    #[link_name = "__isoc99_scanf"]
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn main() {
    let mut f: c_double = 0.0;

    // C's floating-point parser and formatter preserve scanf/printf behavior.
    unsafe {
        scanf(c"%lf".as_ptr(), &mut f);
        printf(c"%llx %a %.4f\n".as_ptr(), f.to_bits() as c_ulonglong, f, f);
    }
}
