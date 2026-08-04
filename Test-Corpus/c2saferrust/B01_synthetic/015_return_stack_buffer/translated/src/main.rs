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

use std::ffi::CString;

#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn scanf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn printLine(line: &CStr) {
    println!("{}", line.to_string_lossy());
}

fn helperBad() -> Option<CString> {
    Some(CString::new("helperBad string").unwrap())
}

#[no_mangle]
pub fn bad() {
    if let Some(s) = helperBad() {
        printLine(s.as_c_str());
    }
}

fn helperGood1() -> CString {
    CString::new("helperGood1 string").unwrap()
}

#[no_mangle]
pub fn good() {
    let s = helperGood1();
    printLine(s.as_c_str());
}

fn main_0() -> i32 {
    let mut input = String::new();
    let x = match io::stdin().read_line(&mut input) {
        Ok(_) => input
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0),
        Err(_) => 0,
    };

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

