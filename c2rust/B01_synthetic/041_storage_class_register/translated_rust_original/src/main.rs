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
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut x: ::core::ffi::c_int) {
    let mut y: ::core::ffi::c_int = 2 as ::core::ffi::c_int * x;
    y += 300 as ::core::ffi::c_int;
    printf(b"%d\n\0" as *const u8 as *const ::core::ffi::c_char, y);
}
unsafe fn main_0() -> ::core::ffi::c_int {
    let mut x: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut x,
    );
    driver(x);
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
