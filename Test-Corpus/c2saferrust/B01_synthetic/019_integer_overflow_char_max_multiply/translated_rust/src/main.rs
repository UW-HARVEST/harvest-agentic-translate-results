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
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn printLine(line: &str) {
    println!("{}", line);
}

#[no_mangle]
pub fn printHexCharLine(char_hex: ::core::ffi::c_char) {
    println!("{:02x}", char_hex as u8);
}

#[no_mangle]
pub fn bad() {
    let data: i8 = i8::MAX;
    if data > 0 {
        let result = data.wrapping_mul(2);
        printHexCharLine(result);
    }
}

fn goodG2B() {
    let data: i8 = 2;
    if data > 0 {
        let result = data * 2;
        printHexCharLine(result);
    }
}

fn goodB2G() {
    let data: i8 = CHAR_MAX as i8;
    if data > 0 {
        if (data as i32) < CHAR_MAX / 2 {
            let result = ((data as i32) * 2) as i8;
            printHexCharLine(result);
        } else {
            printLine("data value is too large to perform arithmetic safely.");
        }
    }
}

#[no_mangle]
pub fn good() {
    goodG2B();
    goodB2G();
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

pub const __SCHAR_MAX__: ::core::ffi::c_int = 127 as ::core::ffi::c_int;
pub const CHAR_MAX: ::core::ffi::c_int = __SCHAR_MAX__;
pub fn main() {
    std::process::exit(main_0())
}

