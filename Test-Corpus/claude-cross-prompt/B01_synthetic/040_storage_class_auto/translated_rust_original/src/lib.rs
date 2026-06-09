use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
    fn scanf(fmt: *const u8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y: c_int = 2 * x;
    y += 300;
    unsafe {
        printf(b"%d\n\0".as_ptr(), y);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    unsafe {
        scanf(b"%d\0".as_ptr(), &mut x as *mut c_int);
    }
    driver(x);
    0
}
