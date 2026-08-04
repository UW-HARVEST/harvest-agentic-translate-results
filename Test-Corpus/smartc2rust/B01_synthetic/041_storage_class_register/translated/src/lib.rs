
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::BufRead;

fn rust_driver(x: i32) {
    let y: i32 = 2 * x + 300;
    println!("{}", y);
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> core::ffi::c_int {
    let stdin = std::io::stdin();
    let x: i32 = stdin
        .lock()
        .lines()
        .next()
        .and_then(|line| line.ok())
        .and_then(|line| line.trim().parse().ok())
        .unwrap_or(0);
    rust_driver(x);
    0
}