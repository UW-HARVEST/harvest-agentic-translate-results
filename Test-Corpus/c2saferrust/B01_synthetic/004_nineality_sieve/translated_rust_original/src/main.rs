#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]

use std::ffi::CStr;

#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strtol(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
}
fn main_0(argc: i32, argv: &[*mut ::core::ffi::c_char]) -> i32 {
    if argc != 2 {
        println!("Error: should only be a single (integer) argument!");
        return 1;
    }

    let arg_ptr = argv[1];
    let arg = match unsafe { ::std::ffi::CStr::from_ptr(arg_ptr) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            println!("Error: first argument must be an integer!");
            return 1;
        }
    };

    let mut val: i32 = match arg.parse() {
        Ok(v) => v,
        Err(_) => {
            println!("Error: first argument must be an integer!");
            return 1;
        }
    };

    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val += 1;
    }

    0
}

pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = std::env::args()
        .map(|arg| {
            let mut bytes = arg.into_bytes();
            bytes.push(0);
            bytes
        })
        .collect();

    let args_ptrs: Vec<*mut ::core::ffi::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut ::core::ffi::c_char)
        .collect();

    let exit_code = main_0(args_ptrs.len() as i32, &args_ptrs);
    std::process::exit(exit_code);
}

