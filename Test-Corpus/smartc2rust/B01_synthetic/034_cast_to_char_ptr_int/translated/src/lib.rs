

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::{self, Write};

fn rust_print_hex(bytes: &[u8]) {
    for b in bytes {
        print!("{:02x}", b);
    }
    println!();
}

fn rust_driver(x: i32) {
    rust_print_hex(&x.to_ne_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    io::stdout().flush().ok();
    let mut input = String::new();
    let x: i32 = match io::stdin().read_line(&mut input) {
        Ok(_) => input.trim().parse::<i32>().unwrap_or(0),
        Err(_) => 0,
    };
    rust_driver(x);
    0
}

