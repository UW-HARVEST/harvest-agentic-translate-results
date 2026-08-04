
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::BufRead;

fn rust_print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn rust_bad() {
    let source: [i32; 10] = [0; 10];
    let data: Vec<i32> = source.iter().copied().collect();
    rust_print_int_line(data[0]);
}

fn rust_good() {
    let source: [i32; 10] = [0; 10];
    let data: Vec<i32> = source.iter().copied().collect();
    rust_print_int_line(data[0]);
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> std::os::raw::c_int {
    let stdin = std::io::stdin();
    let x: i32 = stdin
        .lock()
        .lines()
        .next()
        .and_then(|line| line.ok())
        .and_then(|s| s.trim().parse::<i32>().ok())
        .unwrap_or(0);

    if x != 0 {
        rust_good();
    } else {
        rust_bad();
    }
    0
}