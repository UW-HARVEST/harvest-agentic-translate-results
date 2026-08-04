use std::ffi::{CStr, c_char};

fn foo(in_str: &CStr, c: u8) -> i32 {
    in_str.to_bytes().iter().filter(|&&b| b == c).count() as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(in_ptr: *const c_char) {
    if in_ptr.is_null() {
        return;
    }
    let in_str = unsafe { CStr::from_ptr(in_ptr) };
    println!("A: {}", foo(in_str, b'A'));
    println!("x: {}", foo(in_str, b'x'));
}
