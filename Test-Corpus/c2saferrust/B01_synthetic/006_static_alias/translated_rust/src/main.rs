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
#[no_mangle]
pub unsafe extern "C" fn static_alias(
    outer: *mut ::core::ffi::c_int,
) -> *mut ::core::ffi::c_int {
    use std::sync::atomic::{AtomicI32, Ordering};

    static INNER: AtomicI32 = AtomicI32::new(1);

    let outer_ref = outer.as_mut().expect("outer must not be null");
    let inner_val = INNER.load(Ordering::SeqCst);

    if *outer_ref >= inner_val {
        let _ = INNER.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| Some(v + *outer_ref));
        outer
    } else {
        *outer_ref += inner_val;
        outer
    }
}

fn main_0(argc: i32, argv: &[*mut ::core::ffi::c_char]) -> i32 {
    if argc != 3 {
        println!("Error: should only be two (integer) arguments!");
        return 1;
    }

    let first_arg = match argv.get(1) {
        Some(ptr) => *ptr,
        None => {
            println!("Error: should only be two (integer) arguments!");
            return 1;
        }
    };
    let second_arg = match argv.get(2) {
        Some(ptr) => *ptr,
        None => {
            println!("Error: should only be two (integer) arguments!");
            return 1;
        }
    };

    let initial_value: i32 = unsafe {
        let cstr = CStr::from_ptr(first_arg);
        match cstr.to_str().ok().and_then(|s| s.parse::<i32>().ok()) {
            Some(v) => v,
            None => {
                println!("Error: first argument must be an integer!");
                return 1;
            }
        }
    };

    let iterations: i32 = unsafe {
        let cstr = CStr::from_ptr(second_arg);
        match cstr.to_str().ok().and_then(|s| s.parse::<i32>().ok()) {
            Some(v) => v,
            None => {
                println!("Error: second argument must be an integer!");
                return 1;
            }
        }
    };

    let mut running_sum = initial_value;
    let mut i = 0;
    while i < iterations {
        let ptr = unsafe { static_alias(&mut running_sum as *mut i32) };
        let value = unsafe { *ptr };
        println!("{}", value);
        i += 1;
    }

    0
}

pub fn main() {
    let mut args: Vec<std::ffi::CString> = std::env::args()
        .map(|arg| std::ffi::CString::new(arg).expect("Failed to convert argument into CString."))
        .collect();

    let mut argv: Vec<*mut ::core::ffi::c_char> = args
        .iter_mut()
        .map(|arg| arg.as_ptr() as *mut ::core::ffi::c_char)
        .collect();

    let argc = argv.len() as i32;
    let exit_code = main_0(argc, &argv);
    std::process::exit(exit_code);
}

