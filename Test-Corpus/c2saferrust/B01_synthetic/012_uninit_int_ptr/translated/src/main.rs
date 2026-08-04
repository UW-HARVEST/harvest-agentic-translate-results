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
}
#[no_mangle]
pub fn printIntPtrLine(int_number: *mut i32) {
    if let Some(value) = unsafe { int_number.as_ref() } {
        println!("{}", value);
    }
}

#[no_mangle]
pub fn bad() {
    let data: Option<i32> = None;
    let ptr = data
        .as_ref()
        .map_or(::core::ptr::null_mut(), |v| v as *const i32 as *mut i32);
    printIntPtrLine(ptr);
}

#[no_mangle]
pub fn good() {
    let mut data: i32 = 5;
    printIntPtrLine(&mut data);
}

fn main_0() -> i32 {
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();

    let x: i32 = input
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }

    0
}

pub fn main() {
    std::process::exit(main_0())
}

