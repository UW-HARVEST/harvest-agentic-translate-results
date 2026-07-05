
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

fn rust_driver(x: i32) {
    let mut j: i32 = 0;
    for i in 0..x {
        println!("{} {}", i, j);
        j += 2;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main_replacement_marker() {}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> std::os::raw::c_int {
    let mut input = String::new();
    let x: i32 = match std::io::stdin().read_line(&mut input) {
        Ok(_) => input.trim().parse::<i32>().unwrap_or(0),
        Err(_) => 0,
    };
    rust_driver(x);
    0
}