
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::BufRead;

fn rust_print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn rust_bad() {
    // In safe Rust, uninitialized variables cannot exist. The closest
    // memory-safe equivalent to an uninitialized pointer is `None`.
    let data: Option<&str> = None;
    rust_print_line(data);
}

fn rust_good() {
    let data: Option<&str> = Some("string");
    rust_print_line(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> std::os::raw::c_int {
    let x: i32 = std::io::stdin()
        .lock()
        .lines()
        .next()
        .and_then(|res| res.ok())
        .and_then(|line| line.trim().parse::<i32>().ok())
        .unwrap_or(0);

    if x != 0 {
        rust_good();
    } else {
        rust_bad();
    }
    0
}