use std::ffi::{CStr, c_char};

#[unsafe(no_mangle)]
pub extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let c_s1 = unsafe { CStr::from_ptr(s1) };
    let c_s2 = unsafe { CStr::from_ptr(s2) };

    let b1 = c_s1.to_bytes();
    let b2 = c_s2.to_bytes();

    let count = b1.iter().position(|&b| b2.contains(&b)).unwrap_or(b1.len());

    println!("{}", count);
}
