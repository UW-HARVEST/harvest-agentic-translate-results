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
pub type __uint64_t = u64;
pub type uint64_t = __uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub union raw_double_t {
    pub x: uint64_t,
    pub f: ::core::ffi::c_double,
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut f: ::core::ffi::c_double) {
    let mut u: raw_double_t = raw_double_t { f: f };
    printf(
        b"%llx %a %.4f\n\0" as *const u8 as *const ::core::ffi::c_char,
        u.x,
        f,
        f,
    );
}
unsafe fn main_0() -> ::core::ffi::c_int {
    let mut f: ::core::ffi::c_double = 0.0f64;
    scanf(
        b"%lf\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut f,
    );
    driver(f);
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
