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
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
fn main_0(argc: i32, argv: &[*mut ::core::ffi::c_char]) -> i32 {
    if argc > 4 || argc == 1 {
        print!("Error: there should be one to three arguments passed:\n");
        print!("<string> [start] [stop]\n");
        return 1;
    }

    let s = unsafe {
        match std::ffi::CStr::from_ptr(argv[1]).to_str() {
            Ok(v) => v,
            Err(_) => {
                print!("Error: input string is not valid UTF-8\n");
                return 1;
            }
        }
    };

    let len = s.len();
    let start: i32;
    let stop: i32;

    if argc >= 3 {
        let start_arg = unsafe {
            match std::ffi::CStr::from_ptr(argv[2]).to_str() {
                Ok(v) => v,
                Err(_) => {
                    print!("Second argument must be an integer!");
                    return 1;
                }
            }
        };

        start = match start_arg.parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                print!("Second argument must be an integer!");
                return 1;
            }
        };

        if start < 0 || start as usize > len {
            print!("Error: start is off the end of the string!\n");
            return 1;
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        let stop_arg = unsafe {
            match std::ffi::CStr::from_ptr(argv[3]).to_str() {
                Ok(v) => v,
                Err(_) => {
                    print!("Third argument must be an integer!");
                    return 1;
                }
            }
        };

        stop = match stop_arg.parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                print!("Third argument must be an integer!");
                return 1;
            }
        };

        if stop < 0 || stop as usize > len {
            print!("Error: stop is off the end of the string!\n");
            return 1;
        }
        if stop <= start {
            print!("Error: stop must come after start!\n");
            return 1;
        }
    } else {
        stop = len as i32;
    }

    println!("{}", &s[start as usize..stop as usize]);
    0
}

pub fn main() {
    let mut args_storage: Vec<Vec<u8>> = std::env::args()
        .map(|arg| {
            let mut bytes = arg.into_bytes();
            bytes.push(0);
            bytes
        })
        .collect();

    let mut argv: Vec<*mut ::core::ffi::c_char> = args_storage
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut ::core::ffi::c_char)
        .collect();

    let code = main_0(argv.len() as i32, &argv);
    std::process::exit(code);
}

