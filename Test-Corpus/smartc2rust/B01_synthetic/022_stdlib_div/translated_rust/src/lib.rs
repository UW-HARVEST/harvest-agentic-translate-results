
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::{self, Read, Write};

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return 0;
    }
    let mut iter = input.split_ascii_whitespace();
    let x: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let y: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let quot = x / y;
    let rem = x % y;
    println!("quotient: {}, remainder: {}", quot, rem);
    let _ = io::stdout().flush();
    0
}


