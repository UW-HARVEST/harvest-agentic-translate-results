#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]
#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn scanf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn div(__numer: ::core::ffi::c_int, __denom: ::core::ffi::c_int) -> div_t;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct div_t {
    pub quot: ::core::ffi::c_int,
    pub rem: ::core::ffi::c_int,
}
unsafe fn main_0() -> ::core::ffi::c_int {
    let mut x: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut y: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    scanf(
        b"%d %d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut x,
        &raw mut y,
    );
    let mut result: div_t = div(x, y);
    printf(
        b"quotient: %d, remainder: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        result.quot,
        result.rem,
    );
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
