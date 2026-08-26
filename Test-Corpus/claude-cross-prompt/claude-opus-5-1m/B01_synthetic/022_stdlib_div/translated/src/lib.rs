use std::ffi::c_int;

#[repr(C)]
struct DivT {
    quot: c_int,
    rem: c_int,
}

extern "C" {
    fn scanf(fmt: *const u8, ...) -> c_int;
    fn printf(fmt: *const u8, ...) -> c_int;
    fn div(numer: c_int, denom: c_int) -> DivT;
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut x: c_int = 1;
    let mut y: c_int = 1;
    unsafe {
        scanf(b"%d %d\0".as_ptr(), &mut x as *mut c_int, &mut y as *mut c_int);
        let result = div(x, y);
        printf(
            b"quotient: %d, remainder: %d\n\0".as_ptr(),
            result.quot,
            result.rem,
        );
    }
    0
}
