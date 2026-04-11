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
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn scanf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn div(__numer: libc::c_int, __denom: libc::c_int) -> div_t;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct div_t {
    pub quot: libc::c_int,
    pub rem: libc::c_int,
}
unsafe fn main_0() -> libc::c_int {
    let mut x: libc::c_int = 1 as libc::c_int;
    let mut y: libc::c_int = 1 as libc::c_int;
    scanf(
        b"%d %d\0" as *const u8 as *const libc::c_char,
        &raw mut x,
        &raw mut y,
    );
    let mut result: div_t = div(x, y);
    printf(
        b"quotient: %d, remainder: %d\n\0" as *const u8 as *const libc::c_char,
        result.quot,
        result.rem,
    );
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
