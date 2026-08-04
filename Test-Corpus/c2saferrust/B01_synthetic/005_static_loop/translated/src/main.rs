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

use std::sync::atomic::{AtomicI32, Ordering};

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
#[no_mangle]
pub fn static_sum(update: i32) -> i32 {
    static SUM: AtomicI32 = AtomicI32::new(0);
    SUM.fetch_add(update, Ordering::SeqCst) + update
}

fn main_0(argc: i32, argv: &[*mut ::core::ffi::c_char]) -> i32 {
    if argc != 2 {
        eprintln!("Error: should only be a single (integer) argument!");
        return 1;
    }

    let arg_ptr = argv[1];
    let arg = match unsafe { std::ffi::CStr::from_ptr(arg_ptr) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Error: first argument must be an integer!");
            return 1;
        }
    };

    let stride: i32 = match arg.parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("Error: first argument must be an integer!");
            return 1;
        }
    };

    for i in 0..10 {
        println!("{}", static_sum(i * stride));
    }

    0
}

pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args()
        .map(|arg| {
            let mut bytes = arg.into_bytes();
            bytes.push(0);
            bytes
        })
        .collect();

    let mut args_ptrs: Vec<*mut ::core::ffi::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut ::core::ffi::c_char)
        .collect();

    let exit_code = main_0(args_ptrs.len() as ::core::ffi::c_int, &args_ptrs);
    ::std::process::exit(exit_code as i32);
}

