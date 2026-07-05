
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int};

fn rust_print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn rust_bad() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = 0;
    rust_print_int_line(int_sum);
    let _ = int_one.wrapping_add(int_two);
    rust_print_int_line(int_sum);
}

fn rust_good() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = int_one + int_two;
    rust_print_int_line(0);
    rust_print_int_line(int_sum);
}

fn rust_print_line(line: Option<&str>) {
    if let Some(l) = line {
        println!("{}", l);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    rust_print_line(Some("Calling good()..."));
    rust_good();
    rust_print_line(Some("Finished good()"));
    rust_print_line(Some("Calling bad()..."));
    rust_bad();
    rust_print_line(Some("Finished bad()"));
    0
}

