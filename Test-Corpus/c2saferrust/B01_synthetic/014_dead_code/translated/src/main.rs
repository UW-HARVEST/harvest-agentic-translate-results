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
pub fn bad() {
    println!("bad()");
}

fn helperGood() {
    println!("helperGood()");
}

#[no_mangle]
pub fn good() {
    println!("good()");
    helperGood();
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
    let mut args: Vec<String> = std::env::args().collect();
    let argc = args.len() as i32;
    let exit_code = main_0(argc, args.as_mut_slice());
    std::process::exit(exit_code);
}

