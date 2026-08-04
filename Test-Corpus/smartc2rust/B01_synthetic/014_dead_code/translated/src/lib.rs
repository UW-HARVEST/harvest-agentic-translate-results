
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int};

fn rust_print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn rust_bad() {
    rust_print_line(Some("bad()"));
}

fn rust_helper_good() {
    rust_print_line(Some("helperGood()"));
}

fn rust_good() {
    rust_print_line(Some("good()"));
    rust_helper_good();
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