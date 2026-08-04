use std::ffi::{c_char, CStr};

#[unsafe(no_mangle)]
pub extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    if s1.is_null() || s2.is_null() {
        return;
    }

    let s1 = unsafe { CStr::from_ptr(s1) }.to_bytes();
    let s2 = unsafe { CStr::from_ptr(s2) }.to_bytes();

    let result = s1
        .iter()
        .position(|b| s2.contains(b))
        .unwrap_or(s1.len());

    println!("{}", result);
}
