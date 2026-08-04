
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

fn helloworld() -> core::ffi::c_int {
    println!("Hello World!");
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> core::ffi::c_int {
    helloworld()
}