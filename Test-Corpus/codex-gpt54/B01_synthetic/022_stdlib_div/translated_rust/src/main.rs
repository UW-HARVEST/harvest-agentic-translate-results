use std::ffi::c_char;
use std::ffi::c_int;

#[repr(C)]
struct DivT {
    quot: c_int,
    rem: c_int,
}

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn div(numer: c_int, denom: c_int) -> DivT;
}

fn main() {
    let mut x: c_int = 1;
    let mut y: c_int = 1;

    unsafe {
        scanf(c"%d %d".as_ptr(), &mut x, &mut y);
        let result = div(x, y);
        printf(
            c"quotient: %d, remainder: %d\n".as_ptr(),
            result.quot,
            result.rem,
        );
    }
}
