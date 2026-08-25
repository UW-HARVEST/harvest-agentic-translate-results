use std::os::raw::{c_char, c_int};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[no_mangle]
pub extern "C" fn driver(x: c_int) {
    let y = x.wrapping_mul(2).wrapping_add(300);

    unsafe {
        printf(b"%d\n\0".as_ptr().cast(), y);
    }
}

#[export_name = "main"]
pub extern "C" fn exported_main() -> c_int {
    let mut x: c_int = 0;

    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut x);
    }

    driver(x);
    0
}
