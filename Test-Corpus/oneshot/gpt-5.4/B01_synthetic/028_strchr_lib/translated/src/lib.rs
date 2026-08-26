use std::ffi::{c_char, CStr};

fn foo(in_ptr: *const c_char, c: u8) -> i32 {
    if in_ptr.is_null() {
        return 0;
    }
    let bytes = unsafe { CStr::from_ptr(in_ptr) }.to_bytes();
    bytes.iter().filter(|&&b| b == c).count() as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(in_ptr: *const c_char) {
    println!("A: {}", foo(in_ptr, b'A'));
    println!("x: {}", foo(in_ptr, b'x'));
}
