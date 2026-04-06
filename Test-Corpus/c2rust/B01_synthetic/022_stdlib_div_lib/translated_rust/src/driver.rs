extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn div(__numer: ::core::ffi::c_int, __denom: ::core::ffi::c_int) -> div_t;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct div_t {
    pub quot: ::core::ffi::c_int,
    pub rem: ::core::ffi::c_int,
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut x: ::core::ffi::c_int, mut y: ::core::ffi::c_int) {
    let mut result: div_t = div(x, y);
    printf(
        b"quotient: %d, remainder: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        result.quot,
        result.rem,
    );
}
