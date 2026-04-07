#![no_main]

use std::io::{self, Read};

fn print_int_line(n: i32) {
    println!("{}", n);
}

#[no_mangle]
pub extern "C" fn printLine(line: *const std::os::raw::c_char) {
    if !line.is_null() {
        let c_str = unsafe { std::ffi::CStr::from_ptr(line) };
        if let Ok(s) = c_str.to_str() {
            println!("{}", s);
        }
    }
}

#[no_mangle]
pub extern "C" fn printIntLine(n: i32) {
    print_int_line(n);
}

#[no_mangle]
pub extern "C" fn bad() {
    let source = [0i32; 10];
    let mut data = vec![0i32; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

#[no_mangle]
pub extern "C" fn good() {
    let source = [0i32; 10];
    let mut data = vec![0i32; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut x: i32 = 0;

    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_ok() {
        let token = input.split_whitespace().next();
        if let Some(s) = token {
            if let Ok(v) = s.parse::<i32>() {
                x = v;
            }
        }
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
