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

use std::ffi::CStr;

#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn scanf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn printLine(line: Option<&CStr>) {
    if let Some(line) = line {
        println!("{}", line.to_string_lossy());
    }
}

#[no_mangle]
pub fn bad() {
    let data: Option<&CStr> = None;
    printLine(data);
}

#[no_mangle]
pub fn good() {
    let data = CStr::from_bytes_with_nul(b"string\0").unwrap();
    printLine(Some(data));
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

