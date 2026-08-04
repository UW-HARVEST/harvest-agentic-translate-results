
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int};

#[unsafe(no_mangle)]
pub extern "C" fn main_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if argc != 2 {
        println!("Error: should only be a single (integer) argument!");
        return 1;
    }

    // Safely extract argv[1] as a Rust &str at the FFI boundary.
    let arg1 = unsafe {
        let ptr = *argv.offset(1);
        if ptr.is_null() {
            println!("Error: first argument must be an integer!");
            return 1;
        }
        match std::ffi::CStr::from_ptr(ptr).to_str() {
            Ok(s) => s.to_owned(),
            Err(_) => {
                println!("Error: first argument must be an integer!");
                return 1;
            }
        }
    };

    // Emulate strtol semantics: skip leading whitespace, then parse
    // as many valid digits (with optional sign) as possible.
    let trimmed = arg1.trim_start();
    let bytes = trimmed.as_bytes();
    let sign_len = match bytes.first() {
        Some(&b'+') | Some(&b'-') => 1,
        _ => 0,
    };
    let digits_end = sign_len
        + bytes[sign_len..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();

    if digits_end == sign_len {
        println!("Error: first argument must be an integer!");
        return 1;
    }

    let val: i32 = match trimmed[..digits_end].parse::<i64>() {
        Ok(v) => v as i32,
        Err(_) => {
            println!("Error: first argument must be an integer!");
            return 1;
        }
    };

    let mut val = val;
    loop {
        println!("{}", val);
        if val.rem_euclid(10) == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }

    0
}