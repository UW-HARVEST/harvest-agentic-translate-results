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
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn printLine(line: &str) {
    println!("{}", line);
}

#[no_mangle]
pub fn printIntLine(int_number: i32) {
    println!("{}", int_number);
}

#[no_mangle]
pub fn bad() {
    let mut data = vec![0i32; 10];
    let source = [0i32; 10];
    data.copy_from_slice(&source);
    printIntLine(data[0]);
}

#[no_mangle]
pub fn good() {
    let data = vec![0; 10];
    printIntLine(data[0]);
}

fn main_0() -> i32 {
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();

    let x: i32 = input.trim().parse().unwrap_or(0);

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

