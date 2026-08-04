
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int};

#[derive(Default)]
struct Test {
    a: i32,
    b: i32,
}

fn rust_find_container_of_a(t: &Test) -> &Test {
    t
}

fn rust_find_container_of_b(t: &Test) -> &Test {
    t
}

#[unsafe(no_mangle)]
pub extern "C" fn container_of_main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    let args: Vec<String> = std::env::args().collect();

    let a: i32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let b: i32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    let t = Test { a, b };

    let sum = rust_find_container_of_a(&t).a + rust_find_container_of_b(&t).b;
    println!("{}", sum);

    0
}