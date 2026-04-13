use std::ffi::{CStr, c_char};
use std::os::raw::c_char as c_char_type;

#[unsafe(no_mangle)]
pub extern "C" fn driver(s1: *const c_char_type, s2: *const c_char_type) {
    let s1_str = unsafe {
        CStr::from_ptr(s1)
    };
    let s2_str = unsafe {
        CStr::from_ptr(s2)
    };
    
    let s1_bytes = s1_str.to_bytes();
    let s2_bytes = s2_str.to_bytes();
    
    let mut len = 0;
    for (i, &c) in s1_bytes.iter().enumerate() {
        if s2_bytes.contains(&c) {
            len = i;
            break;
        }
        len = i + 1;
    }
    
    println!("{}", len);
}