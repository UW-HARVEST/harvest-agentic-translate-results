use std::ffi::{c_char, CStr};
use std::os::raw::c_int;

fn foo(in_str: &str, c: char) -> usize {
    in_str.chars().filter(|&ch| ch == c).count()
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(input) };
    let in_str = c_str.to_str().unwrap_or("");
    
    println!("A: {}", foo(in_str, 'A'));
    println!("x: {}", foo(in_str, 'x'));
}