
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
pub fn driver(x: ::core::ffi::c_int, y: ::core::ffi::c_int) {
    let quot = x / y;
    let rem = x % y;
    println!("quotient: {}, remainder: {}", quot, rem);
}

