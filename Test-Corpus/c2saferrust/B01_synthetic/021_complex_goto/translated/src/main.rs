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
fn foo(mut x: i32, mut y: i32) {
    while x > 0 || y > 0 {
        println!("loop");
        let special_case = x == 1 && y == 4;

        if !special_case && x > 0 {
            println!("x");
            x -= 1;
        }

        loop {
            if y == 0 {
                break;
            }

            println!("y");
            y -= 1;

            if x < 3 {
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            } else {
                break;
            }
        }
    }
}

fn main_0() -> i32 {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let mut parts = input.split_whitespace();
    let x: i32 = parts.next().unwrap().parse().unwrap();
    let y: i32 = parts.next().unwrap().parse().unwrap();

    foo(x, y);
    0
}

pub fn main() {
    std::process::exit(main_0());
}

