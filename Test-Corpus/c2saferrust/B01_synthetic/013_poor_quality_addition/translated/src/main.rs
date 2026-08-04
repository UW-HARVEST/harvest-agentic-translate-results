#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]






#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn printLine(line: &str) {
    println!("{}", line);
}

#[no_mangle]
pub fn printIntLine(int_number: i32) {
    println!("{}", int_number);
}

#[no_mangle]
pub fn bad() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = int_one + int_two;
    println!("{}", int_sum);
    println!("{}", int_sum);
}

#[no_mangle]
pub fn good() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
    printIntLine(int_sum);
    int_sum = int_one + int_two;
    printIntLine(int_sum);
}

fn main_0(_argc: i32, _argv: &mut [String]) -> i32 {
    printLine("Calling good()...");
    good();
    printLine("Finished good()");
    printLine("Calling bad()...");
    bad();
    printLine("Finished bad()");
    0
}

pub fn main() {
    let mut args: Vec<String> = ::std::env::args().collect();
    let argc = args.len() as i32;
    ::std::process::exit(main_0(argc, &mut args) as i32)
}

