#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]

use std::io;

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
fn main_0() -> i32 {
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();

    let mut parts = input.split_whitespace();
    let x: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let y: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);

    let quot = x / y;
    let rem = x % y;

    println!("quotient: {}, remainder: {}", quot, rem);
    0
}

pub fn main() {
    std::process::exit(main_0());
}

