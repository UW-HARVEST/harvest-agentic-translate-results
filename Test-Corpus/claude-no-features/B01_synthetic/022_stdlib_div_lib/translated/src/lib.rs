use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[repr(C)]
struct DivT {
    quot: c_int,
    rem: c_int,
}

fn div(numer: c_int, denom: c_int) -> DivT {
    DivT {
        quot: numer / denom,
        rem: numer % denom,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let result = div(x, y);
    unsafe {
        printf(
            b"quotient: %d, remainder: %d\n\0".as_ptr(),
            result.quot,
            result.rem,
        );
    }
}
