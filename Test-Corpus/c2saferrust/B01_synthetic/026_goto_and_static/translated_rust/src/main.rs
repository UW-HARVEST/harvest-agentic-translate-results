#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]


use std::io;

#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn scanf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
static mut y: ::core::ffi::c_int = 123 as ::core::ffi::c_int;
fn multi_stage(x: i32, z: i32) -> i32 {
    let mut result = 0;

    if x != 1 {
        println!("Error: x != 1");
        result = 1;
    } else if unsafe { y } != 2 {
        println!("Error: x == 1 but y != 2");
        result = 2;
    } else if z != 3 {
        println!("Error: x == 1 and y == 2, but z != 3");
        result = 3;
    } else {
        println!("Ok!");
        return result;
    }

    println!("Operation failed");
    result
}

fn main_0() -> i32 {
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("failed to read input");

    let mut parts = input.split_whitespace();
    let x: i32 = parts
        .next()
        .expect("missing x")
        .parse()
        .expect("invalid x");
    let new_y: i32 = parts
        .next()
        .expect("missing y")
        .parse()
        .expect("invalid y");
    let z: i32 = parts
        .next()
        .expect("missing z")
        .parse()
        .expect("invalid z");

    unsafe {
        y = new_y;
    }

    let result = multi_stage(x, z);
    println!("Result: {}", result);
    0
}

pub fn main() {
    std::process::exit(main_0());
}

