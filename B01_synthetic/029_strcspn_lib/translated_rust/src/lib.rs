use std::ffi::{c_char, CStr};

#[unsafe(no_mangle)]
pub extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    unsafe {
        let s1 = CStr::from_ptr(s1).to_bytes();
        let s2 = CStr::from_ptr(s2).to_bytes();
        let n = s1.iter().take_while(|b| !s2.contains(b)).count();
        println!("{n}");
    }
}
