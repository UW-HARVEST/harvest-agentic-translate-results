
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::c_int;

fn rust_print_hex(p: &[u8]) {
    for &b in p {
        print!("{:02x}", b);
    }
    println!();
}

fn rust_driver(x: i32) {
    let raw = x.to_ne_bytes();
    rust_print_hex(&raw);
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> c_int {
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        if let Ok(x) = input.trim().parse::<i32>() {
            rust_driver(x);
        }
    }
    0
}