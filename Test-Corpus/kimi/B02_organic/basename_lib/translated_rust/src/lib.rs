use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_char as raw_c_char;

#[unsafe(no_mangle)]
pub extern "C" fn tool_basename(path: *mut raw_c_char) -> *mut raw_c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    
    let c_str = unsafe { CStr::from_ptr(path) };
    let bytes = c_str.to_bytes();
    
    let s1 = bytes.iter().rposition(|&b| b == b'/');
    let s2 = bytes.iter().rposition(|&b| b == b'\\');
    
    let offset = match (s1, s2) {
        (Some(i1), Some(i2)) => std::cmp::max(i1, i2) + 1,
        (Some(i), None) => i + 1,
        (None, Some(i)) => i + 1,
        (None, None) => 0,
    };
    
    unsafe { path.add(offset) }
}