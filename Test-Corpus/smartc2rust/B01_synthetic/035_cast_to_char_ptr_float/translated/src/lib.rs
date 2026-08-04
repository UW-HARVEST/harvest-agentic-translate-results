
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::{self, BufRead};


#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    let mut x: f32 = 0.0;
    let stdin = io::stdin();
    let mut line = String::new();
    if stdin.lock().read_line(&mut line).is_ok() {
        if let Ok(v) = line.trim().parse::<f32>() {
            x = v;
        }
    }
    driver(x);
    0
}



fn print_hex(p: &[u8]) {
    for &byte in p {
        print!("{:02x}", byte);
    }
    println!();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    print_hex(&x.to_ne_bytes());
}