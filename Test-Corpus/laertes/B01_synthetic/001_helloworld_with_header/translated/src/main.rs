#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn helloworld() -> libc::c_int;
}
unsafe fn main_0() -> libc::c_int {
    return helloworld();
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
