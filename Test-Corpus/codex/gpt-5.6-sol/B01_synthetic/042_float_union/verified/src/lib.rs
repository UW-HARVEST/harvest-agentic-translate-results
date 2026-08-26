use std::ffi::{c_char, c_double, c_int, c_ulonglong};

unsafe extern "C" {
    #[link_name = "__isoc99_scanf"]
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(f: c_double) {
    unsafe {
        printf(c"%llx %a %.4f\n".as_ptr(), f.to_bits() as c_ulonglong, f, f);
    }
}

#[cfg_attr(not(test), unsafe(export_name = "main"))]
pub unsafe extern "C" fn c_main() -> c_int {
    let mut f: c_double = 0.0;

    unsafe {
        scanf(c"%lf".as_ptr(), &mut f);
        driver(f);
    }

    0
}
