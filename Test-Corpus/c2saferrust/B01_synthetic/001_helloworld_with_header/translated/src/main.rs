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
    fn helloworld() -> ::core::ffi::c_int;
}
fn main_0() -> i32 {
    unsafe { helloworld() }
}

pub fn main() {
    std::process::exit(main_0())
}

